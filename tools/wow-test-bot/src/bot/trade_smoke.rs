//! Two-session player TRADE smoke (#38).
//!
//! C++ anchors (TrinityCore-derived 3.4.3 reference):
//! - Opcodes.h: `CMSG_INITIATE_TRADE = 0x3156`, `CMSG_BEGIN_TRADE = 0x3157`,
//!   `CMSG_ACCEPT_TRADE = 0x315A`, `CMSG_CANCEL_TRADE = 0x315C`,
//!   `CMSG_SET_TRADE_ITEM = 0x315D`, `CMSG_SET_TRADE_GOLD = 0x315F`,
//!   `SMSG_TRADE_UPDATED = 0x2581`, `SMSG_TRADE_STATUS = 0x2582`. Opcodes.cpp
//!   routes both server packets on `CONNECTION_TYPE_INSTANCE`.
//! - `TradePackets.cpp`: `InitiateTrade::Read` (packed `Guid`),
//!   `AcceptTrade::Read` (`uint32 StateIndex`), `SetTradeItem::Read`
//!   (`uint8 TradeSlot, uint8 PackSlot, uint8 ItemSlotInPack`),
//!   `SetTradeGold::Read` (`uint64 Coinage`), `TradeStatus::Write` and
//!   `TradeUpdated::Write` (decoded below field for field).
//! - `TradeHandler.cpp`: `HandleInitiateTradeOpcode` sends `PROPOSED` with
//!   `Partner = initiator` to the target only; `HandleBeginTradeOpcode` sends
//!   `INITIATED` to both; `TradeData::SetItem`/`SetMoney` send `UNACCEPTED`
//!   and a `TRADE_UPDATED(WhichPlayer=1)` describing the setter to the
//!   partner; `HandleAcceptTradeOpcode` compares the partner's
//!   `GetServerStateIndex()` with the received `StateIndex`, sends `ACCEPTED`
//!   to the partner on the first accept, and on the second accept moves the
//!   items/money, commits `SaveInventoryAndGoldToDB` for both players in one
//!   transaction and sends `COMPLETE` to both.
//! - `TradeData.h`: both server and client state indexes start at 1; every
//!   change replaces the server index with `rand32()`.

use super::*;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Barrier;

pub(crate) const CMSG_INITIATE_TRADE: u16 = 0x3156;
pub(crate) const CMSG_BEGIN_TRADE: u16 = 0x3157;
pub(crate) const CMSG_ACCEPT_TRADE: u16 = 0x315A;
pub(crate) const CMSG_CANCEL_TRADE: u16 = 0x315C;
pub(crate) const CMSG_SET_TRADE_ITEM: u16 = 0x315D;
pub(crate) const CMSG_SET_TRADE_GOLD: u16 = 0x315F;
pub(crate) const SMSG_TRADE_UPDATED: u16 = 0x2581;
pub(crate) const SMSG_TRADE_STATUS: u16 = 0x2582;

// SharedDefines.h `enum TradeStatus`.
pub(crate) const TRADE_STATUS_PROPOSED: u8 = 1;
pub(crate) const TRADE_STATUS_INITIATED: u8 = 2;
pub(crate) const TRADE_STATUS_ACCEPTED: u8 = 4;
pub(crate) const TRADE_STATUS_UNACCEPTED: u8 = 7;
pub(crate) const TRADE_STATUS_COMPLETE: u8 = 8;
pub(crate) const TRADE_STATUS_FAILED: u8 = 12;
pub(crate) const TRADE_STATUS_WRONG_REALM: u8 = 22;
pub(crate) const TRADE_STATUS_NOT_ON_TAPLIST: u8 = 23;
pub(crate) const TRADE_STATUS_CURRENCY_NOT_TRADABLE: u8 = 24;
pub(crate) const TRADE_STATUS_NOT_ENOUGH_CURRENCY: u8 = 25;

/// Player.h `INVENTORY_SLOT_BAG_0`: the backpack as a `PackSlot`.
pub(crate) const TRADE_PACK_SLOT_BACKPACK: u8 = 255;
/// `TradeData.h` initial `_serverStateIndex`.
pub(crate) const TRADE_INITIAL_STATE_INDEX: u32 = 1;

pub(crate) fn trade_status_name(status: u8) -> &'static str {
    match status {
        0 => "PLAYER_BUSY",
        1 => "PROPOSED",
        2 => "INITIATED",
        3 => "CANCELLED",
        4 => "ACCEPTED",
        5 => "ALREADY_TRADING",
        6 => "NO_TARGET",
        7 => "UNACCEPTED",
        8 => "COMPLETE",
        9 => "STATE_CHANGED",
        10 => "TOO_FAR_AWAY",
        11 => "WRONG_FACTION",
        12 => "FAILED",
        13 => "PETITION",
        14 => "PLAYER_IGNORED",
        15 => "STUNNED",
        16 => "TARGET_STUNNED",
        17 => "DEAD",
        18 => "TARGET_DEAD",
        19 => "LOGGING_OUT",
        20 => "TARGET_LOGGING_OUT",
        21 => "RESTRICTED_ACCOUNT",
        22 => "WRONG_REALM",
        23 => "NOT_ON_TAPLIST",
        24 => "CURRENCY_NOT_TRADABLE",
        25 => "NOT_ENOUGH_CURRENCY",
        _ => "UNKNOWN",
    }
}

// ─── Client packet builders ─────────────────────────────────────────────────

/// `InitiateTrade::Read`: one packed `ObjectGuid`.
pub(crate) fn build_initiate_trade(partner_low: u64, partner_high: u64) -> Vec<u8> {
    build_packed_guid(partner_low, partner_high)
}

/// `BeginTrade` has no body.
pub(crate) fn build_begin_trade() -> Vec<u8> {
    Vec::new()
}

/// `SetTradeItem::Read`: `uint8 TradeSlot, uint8 PackSlot, uint8 ItemSlotInPack`.
pub(crate) fn build_set_trade_item(trade_slot: u8, pack_slot: u8, slot_in_pack: u8) -> [u8; 3] {
    [trade_slot, pack_slot, slot_in_pack]
}

/// `SetTradeGold::Read`: `uint64 Coinage`.
pub(crate) fn build_set_trade_gold(coinage: u64) -> [u8; 8] {
    coinage.to_le_bytes()
}

/// `AcceptTrade::Read`: `uint32 StateIndex`.
pub(crate) fn build_accept_trade(state_index: u32) -> [u8; 4] {
    state_index.to_le_bytes()
}

// ─── Server packet parsers ──────────────────────────────────────────────────

struct TradeCursor<'a> {
    payload: &'a [u8],
    offset: usize,
    packet: &'static str,
}

impl<'a> TradeCursor<'a> {
    fn new(payload: &'a [u8], packet: &'static str) -> Self {
        Self {
            payload,
            offset: 0,
            packet,
        }
    }

    fn take(&mut self, len: usize, field: &str) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .context("trade cursor overflow")?;
        let bytes = self.payload.get(self.offset..end).ok_or_else(|| {
            anyhow!(
                "malformed {}: {field} needs {len} byte(s) at offset {}, payload has {}",
                self.packet,
                self.offset,
                self.payload.len()
            )
        })?;
        self.offset = end;
        Ok(bytes)
    }

    fn u8(&mut self, field: &str) -> Result<u8> {
        Ok(self.take(1, field)?[0])
    }

    fn u32(&mut self, field: &str) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4, field)?.try_into()?))
    }

    fn i32(&mut self, field: &str) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4, field)?.try_into()?))
    }

    fn u64(&mut self, field: &str) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8, field)?.try_into()?))
    }

    fn guid(&mut self, field: &str) -> Result<(u64, u64)> {
        let (consumed, low, high) = parse_packed_guid(&self.payload[self.offset..])
            .ok_or_else(|| anyhow!("malformed {} {field} packed ObjectGuid", self.packet))?;
        self.offset += consumed;
        Ok((low, high))
    }

    /// One flushed bit-pack byte whose bits outside `used_mask` must be zero.
    fn bits(&mut self, used_mask: u8, field: &str) -> Result<u8> {
        let byte = self.u8(field)?;
        if byte & !used_mask != 0 {
            bail!(
                "malformed {}: {field} bit-pack byte {byte:#04X} has nonzero padding",
                self.packet
            );
        }
        Ok(byte)
    }

    fn finish(&self) -> Result<()> {
        if self.offset != self.payload.len() {
            bail!(
                "malformed {}: {} trailing byte(s) after offset {}",
                self.packet,
                self.payload.len() - self.offset,
                self.offset
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TradeStatusPacket {
    pub(crate) partner_is_same_bnet_account: bool,
    pub(crate) status: u8,
    pub(crate) failure_for_you: Option<bool>,
    pub(crate) bag_result: Option<i32>,
    pub(crate) item_id: Option<i32>,
    pub(crate) id: Option<u32>,
    pub(crate) partner: Option<(u64, u64)>,
    pub(crate) partner_account: Option<(u64, u64)>,
    pub(crate) trade_slot: Option<u8>,
    pub(crate) currency: Option<(i32, i32)>,
}

/// `TradeStatus::Write`: `bit PartnerIsSameBnetAccount`, `5 bits Status`,
/// then the per-status tail. `FAILED` writes one more bit before the flush;
/// every byte-write flushes the pending bit pack (`ByteBuffer::append`).
pub(crate) fn parse_trade_status_like_cpp(payload: &[u8]) -> Result<TradeStatusPacket> {
    let mut cursor = TradeCursor::new(payload, "SMSG_TRADE_STATUS");
    let head = cursor.u8("PartnerIsSameBnetAccount/Status")?;
    let status = (head >> 2) & 0x1F;
    let mut packet = TradeStatusPacket {
        partner_is_same_bnet_account: head & 0x80 != 0,
        status,
        ..TradeStatusPacket::default()
    };
    let padding = if status == TRADE_STATUS_FAILED {
        head & 0x01
    } else {
        head & 0x03
    };
    if padding != 0 {
        bail!("malformed SMSG_TRADE_STATUS: head byte {head:#04X} has nonzero padding");
    }
    match status {
        TRADE_STATUS_FAILED => {
            packet.failure_for_you = Some(head & 0x02 != 0);
            packet.bag_result = Some(cursor.i32("BagResult")?);
            packet.item_id = Some(cursor.i32("ItemID")?);
        }
        TRADE_STATUS_INITIATED => packet.id = Some(cursor.u32("ID")?),
        TRADE_STATUS_PROPOSED => {
            packet.partner = Some(cursor.guid("Partner")?);
            packet.partner_account = Some(cursor.guid("PartnerAccount")?);
        }
        TRADE_STATUS_WRONG_REALM | TRADE_STATUS_NOT_ON_TAPLIST => {
            packet.trade_slot = Some(cursor.u8("TradeSlot")?);
        }
        TRADE_STATUS_NOT_ENOUGH_CURRENCY | TRADE_STATUS_CURRENCY_NOT_TRADABLE => {
            packet.currency = Some((cursor.i32("CurrencyType")?, cursor.i32("CurrencyQuantity")?));
        }
        _ => {}
    }
    cursor.finish()?;
    Ok(packet)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TradeItemInstance {
    pub(crate) item_id: i32,
    pub(crate) random_properties_seed: i32,
    pub(crate) random_properties_id: i32,
    pub(crate) modifications: Vec<(i32, u8)>,
    pub(crate) bonus: Option<(u8, Vec<u32>)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UnwrappedTradeItem {
    pub(crate) enchant_id: i32,
    pub(crate) on_use_enchantment_id: i32,
    pub(crate) creator: (u64, u64),
    pub(crate) charges: i32,
    pub(crate) max_durability: u32,
    pub(crate) durability: u32,
    pub(crate) lock: bool,
    pub(crate) gems: Vec<(u8, TradeItemInstance)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TradeItemPacket {
    pub(crate) slot: u8,
    pub(crate) stack_count: u32,
    pub(crate) gift_creator: (u64, u64),
    pub(crate) item: TradeItemInstance,
    pub(crate) unwrapped: Option<UnwrappedTradeItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TradeUpdatedPacket {
    pub(crate) which_player: u8,
    pub(crate) id: u32,
    pub(crate) client_state_index: u32,
    pub(crate) current_state_index: u32,
    pub(crate) gold: u64,
    pub(crate) currency_type: i32,
    pub(crate) currency_quantity: i32,
    pub(crate) proposed_enchantment: i32,
    pub(crate) items: Vec<TradeItemPacket>,
}

/// `ItemPacketsCommon.cpp` `operator<<(ItemInstance)`.
fn read_item_instance(cursor: &mut TradeCursor<'_>) -> Result<TradeItemInstance> {
    let item_id = cursor.i32("ItemInstance.ItemID")?;
    let random_properties_seed = cursor.i32("ItemInstance.RandomPropertiesSeed")?;
    let random_properties_id = cursor.i32("ItemInstance.RandomPropertiesID")?;
    let has_bonus = cursor.bits(0x80, "ItemInstance.hasItemBonus")? & 0x80 != 0;
    let mod_count = cursor.bits(0xFC, "ItemModList.Values.size")? >> 2;
    let mut modifications = Vec::with_capacity(mod_count as usize);
    for _ in 0..mod_count {
        let value = cursor.i32("ItemMod.Value")?;
        let kind = cursor.u8("ItemMod.Type")?;
        modifications.push((value, kind));
    }
    let bonus = if has_bonus {
        let context = cursor.u8("ItemBonuses.Context")?;
        let count = cursor.u32("ItemBonuses.BonusListIDs.size")?;
        if count > 64 {
            bail!("malformed SMSG_TRADE_UPDATED: {count} item bonus ids");
        }
        let ids = (0..count)
            .map(|_| cursor.u32("ItemBonuses.BonusListID"))
            .collect::<Result<Vec<_>>>()?;
        Some((context, ids))
    } else {
        None
    };
    Ok(TradeItemInstance {
        item_id,
        random_properties_seed,
        random_properties_id,
        modifications,
        bonus,
    })
}

/// `TradeUpdated::Write` plus `operator<<(TradeItem)` /
/// `operator<<(UnwrappedTradeItem)`.
pub(crate) fn parse_trade_updated_like_cpp(payload: &[u8]) -> Result<TradeUpdatedPacket> {
    let mut cursor = TradeCursor::new(payload, "SMSG_TRADE_UPDATED");
    let mut packet = TradeUpdatedPacket {
        which_player: cursor.u8("WhichPlayer")?,
        id: cursor.u32("ID")?,
        client_state_index: cursor.u32("ClientStateIndex")?,
        current_state_index: cursor.u32("CurrentStateIndex")?,
        gold: cursor.u64("Gold")?,
        currency_type: cursor.i32("CurrencyType")?,
        currency_quantity: cursor.i32("CurrencyQuantity")?,
        proposed_enchantment: cursor.i32("ProposedEnchantment")?,
        items: Vec::new(),
    };
    let count = cursor.u32("Items.size")?;
    // TradeDefines.h TRADE_SLOT_COUNT.
    if count > 7 {
        bail!("malformed SMSG_TRADE_UPDATED: {count} items exceeds TRADE_SLOT_COUNT 7");
    }
    for _ in 0..count {
        let slot = cursor.u8("TradeItem.Slot")?;
        let stack_count = cursor.u32("TradeItem.StackCount")?;
        let gift_creator = cursor.guid("TradeItem.GiftCreator")?;
        let item = read_item_instance(&mut cursor)?;
        let has_unwrapped = cursor.bits(0x80, "TradeItem.hasUnwrapped")? & 0x80 != 0;
        let unwrapped = if has_unwrapped {
            let enchant_id = cursor.i32("Unwrapped.EnchantID")?;
            let on_use_enchantment_id = cursor.i32("Unwrapped.OnUseEnchantmentID")?;
            let creator = cursor.guid("Unwrapped.Creator")?;
            let charges = cursor.i32("Unwrapped.Charges")?;
            let max_durability = cursor.u32("Unwrapped.MaxDurability")?;
            let durability = cursor.u32("Unwrapped.Durability")?;
            let bits = cursor.bits(0xE0, "Unwrapped.Gems.size/Lock")?;
            let gem_count = bits >> 6;
            let lock = bits & 0x20 != 0;
            let mut gems = Vec::with_capacity(gem_count as usize);
            for _ in 0..gem_count {
                let gem_slot = cursor.u8("ItemGemData.Slot")?;
                gems.push((gem_slot, read_item_instance(&mut cursor)?));
            }
            Some(UnwrappedTradeItem {
                enchant_id,
                on_use_enchantment_id,
                creator,
                charges,
                max_durability,
                durability,
                lock,
                gems,
            })
        } else {
            None
        };
        packet.items.push(TradeItemPacket {
            slot,
            stack_count,
            gift_creator,
            item,
            unwrapped,
        });
    }
    cursor.finish()?;
    Ok(packet)
}

/// The receiver's view of the initiator's offer is complete: the fixture item
/// (one stack) in trade slot 0 and the exact gold.
pub(crate) fn trade_update_matches_offer(
    update: &TradeUpdatedPacket,
    item_entry: u32,
    gold: u64,
) -> bool {
    update.which_player == 1
        && update.gold == gold
        && update.items.len() == 1
        && update.items[0].slot == 0
        && update.items[0].stack_count == 1
        && i64::from(update.items[0].item.item_id) == i64::from(item_entry)
}

// ─── Options, sync, evidence ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TradeSmokePhase {
    Trade,
    VerifyRelog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TradeRole {
    /// Account A: owns the item, initiates, offers item + gold, accepts second.
    Initiator,
    /// Account B: receives the proposal, begins, accepts first.
    Receiver,
}

/// Expected characters-DB state once the trade committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TradeDbExpectation {
    pub(crate) initiator_guid: u64,
    pub(crate) receiver_guid: u64,
    pub(crate) item_guid: u64,
    pub(crate) item_entry: u32,
    pub(crate) initiator_money_after: u64,
    pub(crate) receiver_money_after: u64,
    /// When set, the item must be persisted inside this bag at slot 0.
    pub(crate) receiver_bag_guid: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct TradeSmokeSync {
    pub(crate) ready: Barrier,
    pub(crate) completed: Barrier,
    pub(crate) verified: Barrier,
    pub(crate) cancelled: CancellationToken,
    pub(crate) failure: StdMutex<Option<String>>,
}

impl TradeSmokeSync {
    pub(crate) fn new() -> Self {
        Self {
            ready: Barrier::new(2),
            completed: Barrier::new(2),
            verified: Barrier::new(2),
            cancelled: CancellationToken::new(),
            failure: StdMutex::new(None),
        }
    }

    pub(crate) fn cancel(&self, message: impl Into<String>) {
        let message = message.into();
        let mut failure = self.failure.lock().expect("trade failure lock");
        if failure.is_none() {
            *failure = Some(message);
        }
        self.cancelled.cancel();
    }

    pub(crate) fn cancellation_error(&self) -> Result<()> {
        if !self.cancelled.is_cancelled() {
            return Ok(());
        }
        let failure = self
            .failure
            .lock()
            .expect("trade failure lock")
            .clone()
            .unwrap_or_else(|| "peer cancelled the trade smoke".to_string());
        bail!("trade smoke cancelled: {failure}")
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TradeSmokeOptions {
    pub(crate) phase: TradeSmokePhase,
    pub(crate) role: TradeRole,
    pub(crate) character_guid: u64,
    pub(crate) partner_guid: u64,
    pub(crate) item_guid: u64,
    pub(crate) item_entry: u32,
    pub(crate) item_slot: u8,
    pub(crate) gold: u64,
    pub(crate) expected: TradeDbExpectation,
    pub(crate) timeout_secs: u64,
    pub(crate) sync: Arc<TradeSmokeSync>,
}

/// Flattened into `BotRunResult` so the JSON report carries `trade_*`.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct TradeSmokeEvidence {
    pub(crate) trade_smoke: bool,
    pub(crate) trade_smoke_passed: Option<bool>,
    pub(crate) trade_role: Option<TradeRole>,
    pub(crate) trade_partner_guid: Option<u64>,
    pub(crate) trade_item_guid: Option<u64>,
    pub(crate) trade_item_entry: Option<u32>,
    pub(crate) trade_item_slot: Option<u8>,
    pub(crate) trade_gold: Option<u64>,
    /// Every SMSG_TRADE_STATUS in arrival order, by C++ enum name.
    pub(crate) trade_statuses_seen: Vec<String>,
    pub(crate) trade_proposed_partner_verified: bool,
    pub(crate) trade_initiated_id: Option<u32>,
    pub(crate) trade_updates_seen: u32,
    /// Receiver: A's `CurrentStateIndex` from the matching TRADE_UPDATED.
    pub(crate) trade_partner_state_index: Option<u32>,
    pub(crate) trade_accept_state_index_sent: Option<u32>,
    pub(crate) trade_completed: bool,
    pub(crate) trade_db_verified_online: bool,
    pub(crate) trade_db_verified_after_logout: bool,
    pub(crate) trade_relog_verified: bool,
    pub(crate) trade_db_verified_after_relog: bool,
    pub(crate) trade_money_after: Option<u64>,
    pub(crate) trade_failure: Option<String>,
}

impl TradeSmokeEvidence {
    pub(crate) fn for_options(options: Option<&PostLoginSmokeOptions>) -> Self {
        let Some(PostLoginSmokeOptions::Trade(options)) = options else {
            return Self::default();
        };
        Self {
            trade_smoke: true,
            trade_role: Some(options.role),
            trade_partner_guid: Some(options.partner_guid),
            trade_item_guid: Some(options.item_guid),
            trade_item_entry: Some(options.item_entry),
            trade_item_slot: Some(options.item_slot),
            trade_gold: Some(options.gold),
            ..Self::default()
        }
    }

    pub(crate) fn passed(&self) -> bool {
        self.trade_smoke_passed.unwrap_or(false)
            && self.trade_completed
            && self.trade_db_verified_after_logout
            && self.trade_relog_verified
            && self.trade_db_verified_after_relog
    }
}

pub(crate) fn trade_smoke_summary(result: &BotRunResult) -> Option<String> {
    let evidence = &result.trade;
    if !evidence.trade_smoke {
        return None;
    }
    Some(format!(
        "trade_smoke role={:?} partner={:?} item={:?}/entry={:?}/slot={:?} gold={:?} statuses={:?} proposed_ok={} initiated={:?} updates={} partner_index={:?} accept_index={:?} complete={} db_online={} db_logout={} relog={} db_relog={} money_after={:?} failure={:?}",
        evidence.trade_role,
        evidence.trade_partner_guid,
        evidence.trade_item_guid,
        evidence.trade_item_entry,
        evidence.trade_item_slot,
        evidence.trade_gold,
        evidence.trade_statuses_seen,
        evidence.trade_proposed_partner_verified,
        evidence.trade_initiated_id,
        evidence.trade_updates_seen,
        evidence.trade_partner_state_index,
        evidence.trade_accept_state_index_sent,
        evidence.trade_completed,
        evidence.trade_db_verified_online,
        evidence.trade_db_verified_after_logout,
        evidence.trade_relog_verified,
        evidence.trade_db_verified_after_relog,
        evidence.trade_money_after,
        evidence.trade_failure,
    ))
}

// ─── Live phase ─────────────────────────────────────────────────────────────

#[derive(Debug)]
enum TradeEvent {
    Status(TradeStatusPacket),
    Updated(TradeUpdatedPacket),
}

/// Read packets until the next trade packet. Trade packets must arrive on the
/// instance socket (C++ `CONNECTION_TYPE_INSTANCE`); one on the realm socket
/// fails the phase. Time sync is answered so the session stays healthy.
#[allow(clippy::too_many_arguments)]
async fn next_trade_event(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &TradeSmokeOptions,
    waiting_for: &str,
    result: &mut BotRunResult,
) -> Result<TradeEvent> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        options.sync.cancellation_error()?;
        if tokio::time::Instant::now() >= deadline {
            bail!(
                "timed out after {}s waiting for {waiting_for}",
                options.timeout_secs
            );
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            Duration::from_millis(50),
            Duration::from_secs(5),
            "trade instance packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            match opcode {
                SMSG_TIME_SYNC_REQUEST => {
                    let sequence = parse_time_sync_request_sequence(&payload)?;
                    let response = build_time_sync_response_payload(sequence, 0);
                    send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response)
                        .await?;
                }
                SMSG_TRADE_STATUS => {
                    let status = parse_trade_status_like_cpp(&payload)?;
                    let name = trade_status_name(status.status);
                    result.trade.trade_statuses_seen.push(name.to_string());
                    info!("[Bot {bot_index}] 📦 SMSG_TRADE_STATUS {name} {status:?}");
                    return Ok(TradeEvent::Status(status));
                }
                SMSG_TRADE_UPDATED => {
                    let update = parse_trade_updated_like_cpp(&payload)?;
                    result.trade.trade_updates_seen += 1;
                    info!("[Bot {bot_index}] 📦 SMSG_TRADE_UPDATED {update:?}");
                    return Ok(TradeEvent::Updated(update));
                }
                _ => debug!(
                    "[Bot {bot_index}] trade wait {}",
                    parse_packet(opcode, &payload)
                ),
            }
        }
        if let Some(realm) = realm_connection.as_mut() {
            if let Some((opcode, _payload)) = read_encrypted_packet_if_ready(
                &mut realm.stream,
                &mut realm.crypt,
                &mut realm.inflater,
                Duration::from_millis(1),
                Duration::from_secs(5),
                "trade realm packet",
            )
            .await?
            {
                result.seen_opcodes.push(format!("0x{opcode:04X}"));
                if matches!(opcode, SMSG_TRADE_STATUS | SMSG_TRADE_UPDATED) {
                    bail!(
                        "{} arrived on the realm socket; C++ Opcodes.cpp routes it on CONNECTION_TYPE_INSTANCE",
                        if opcode == SMSG_TRADE_STATUS {
                            "SMSG_TRADE_STATUS"
                        } else {
                            "SMSG_TRADE_UPDATED"
                        }
                    );
                }
            }
        }
    }
}

/// Fail on any status the expected handshake cannot produce at this point.
fn reject_unexpected_status(status: &TradeStatusPacket, waiting_for: &str) -> Result<()> {
    bail!(
        "unexpected SMSG_TRADE_STATUS {} ({}) while waiting for {waiting_for}{}",
        trade_status_name(status.status),
        status.status,
        match (status.bag_result, status.item_id) {
            (Some(bag), Some(item)) => format!(" (BagResult={bag}, ItemID={item})"),
            _ => String::new(),
        }
    )
}

async fn wait_trade_barrier(
    barrier: &Barrier,
    options: &TradeSmokeOptions,
    label: &str,
) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(options.timeout_secs), async {
        tokio::select! {
            _ = barrier.wait() => Ok(()),
            _ = options.sync.cancelled.cancelled() => options.sync.cancellation_error(),
        }
    })
    .await
    .map_err(|_| anyhow!("trade smoke timed out waiting for the peer at {label}"))?
}

async fn send_trade_packet(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    opcode: u16,
    name: &str,
    payload: &[u8],
) -> Result<()> {
    send_encrypted_packet(stream, crypt, opcode, payload).await?;
    info!(
        "[Bot {bot_index}] ✅ {name} sent ({} byte body)",
        payload.len()
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_initiator(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &TradeSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let (partner_low, partner_high) = create_player_guid_raw(options.partner_guid, realm_id());
    send_trade_packet(
        bot_index,
        stream,
        crypt,
        CMSG_INITIATE_TRADE,
        "CMSG_INITIATE_TRADE",
        &build_initiate_trade(partner_low, partner_high),
    )
    .await?;

    loop {
        let waiting = "TRADE_STATUS INITIATED";
        match next_trade_event(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection,
            options,
            waiting,
            result,
        )
        .await?
        {
            TradeEvent::Status(status) if status.status == TRADE_STATUS_INITIATED => {
                result.trade.trade_initiated_id = status.id;
                break;
            }
            TradeEvent::Status(status) => reject_unexpected_status(&status, waiting)?,
            TradeEvent::Updated(_) => bail!("TRADE_UPDATED arrived before INITIATED"),
        }
    }

    send_trade_packet(
        bot_index,
        stream,
        crypt,
        CMSG_SET_TRADE_ITEM,
        "CMSG_SET_TRADE_ITEM",
        &build_set_trade_item(0, TRADE_PACK_SLOT_BACKPACK, options.item_slot),
    )
    .await?;
    send_trade_packet(
        bot_index,
        stream,
        crypt,
        CMSG_SET_TRADE_GOLD,
        "CMSG_SET_TRADE_GOLD",
        &build_set_trade_gold(options.gold),
    )
    .await?;

    // B's side never changes, so its server state index normally stays at the
    // TradeData.h initial value; a WhichPlayer=1 update would describe it.
    let mut partner_state_index = TRADE_INITIAL_STATE_INDEX;
    loop {
        let waiting = "TRADE_STATUS ACCEPTED from the receiver";
        match next_trade_event(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection,
            options,
            waiting,
            result,
        )
        .await?
        {
            TradeEvent::Status(status) if status.status == TRADE_STATUS_ACCEPTED => break,
            TradeEvent::Status(status) if status.status == TRADE_STATUS_UNACCEPTED => {}
            TradeEvent::Status(status) => reject_unexpected_status(&status, waiting)?,
            TradeEvent::Updated(update) if update.which_player == 1 => {
                partner_state_index = update.current_state_index;
            }
            TradeEvent::Updated(_) => {}
        }
    }
    result.trade.trade_partner_state_index = Some(partner_state_index);
    result.trade.trade_accept_state_index_sent = Some(partner_state_index);
    send_trade_packet(
        bot_index,
        stream,
        crypt,
        CMSG_ACCEPT_TRADE,
        "CMSG_ACCEPT_TRADE",
        &build_accept_trade(partner_state_index),
    )
    .await?;

    wait_for_trade_complete(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        options,
        result,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_receiver(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &TradeSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let expected_partner = create_player_guid_raw(options.partner_guid, realm_id());
    loop {
        let waiting = "TRADE_STATUS PROPOSED";
        match next_trade_event(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection,
            options,
            waiting,
            result,
        )
        .await?
        {
            TradeEvent::Status(status) if status.status == TRADE_STATUS_PROPOSED => {
                if status.partner != Some(expected_partner) {
                    bail!(
                        "TRADE_STATUS PROPOSED partner {:?} is not the initiator {:?}",
                        status.partner,
                        expected_partner
                    );
                }
                result.trade.trade_proposed_partner_verified = true;
                break;
            }
            TradeEvent::Status(status) => reject_unexpected_status(&status, waiting)?,
            TradeEvent::Updated(_) => bail!("TRADE_UPDATED arrived before PROPOSED"),
        }
    }

    send_trade_packet(
        bot_index,
        stream,
        crypt,
        CMSG_BEGIN_TRADE,
        "CMSG_BEGIN_TRADE",
        &build_begin_trade(),
    )
    .await?;

    let mut initiated = false;
    let mut offer_index = None;
    while offer_index.is_none() {
        let waiting = if initiated {
            "TRADE_UPDATED with the initiator's item and gold"
        } else {
            "TRADE_STATUS INITIATED"
        };
        match next_trade_event(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection,
            options,
            waiting,
            result,
        )
        .await?
        {
            TradeEvent::Status(status) if !initiated && status.status == TRADE_STATUS_INITIATED => {
                result.trade.trade_initiated_id = status.id;
                initiated = true;
            }
            TradeEvent::Status(status) if initiated && status.status == TRADE_STATUS_UNACCEPTED => {
            }
            TradeEvent::Status(status) => reject_unexpected_status(&status, waiting)?,
            TradeEvent::Updated(_) if !initiated => bail!("TRADE_UPDATED arrived before INITIATED"),
            TradeEvent::Updated(update) => {
                if update.which_player != 1 {
                    continue;
                }
                result.trade.trade_partner_state_index = Some(update.current_state_index);
                if trade_update_matches_offer(&update, options.item_entry, options.gold) {
                    offer_index = Some(update.current_state_index);
                }
            }
        }
    }
    let offer_index = offer_index.expect("loop exits with the offer index");
    result.trade.trade_accept_state_index_sent = Some(offer_index);
    send_trade_packet(
        bot_index,
        stream,
        crypt,
        CMSG_ACCEPT_TRADE,
        "CMSG_ACCEPT_TRADE",
        &build_accept_trade(offer_index),
    )
    .await?;

    wait_for_trade_complete(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        options,
        result,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn wait_for_trade_complete(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &TradeSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    loop {
        let waiting = "TRADE_STATUS COMPLETE";
        match next_trade_event(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection,
            options,
            waiting,
            result,
        )
        .await?
        {
            TradeEvent::Status(status) if status.status == TRADE_STATUS_COMPLETE => {
                result.trade.trade_completed = true;
                return Ok(());
            }
            TradeEvent::Status(status)
                if matches!(
                    status.status,
                    TRADE_STATUS_ACCEPTED | TRADE_STATUS_UNACCEPTED
                ) => {}
            TradeEvent::Status(status) => reject_unexpected_status(&status, waiting)?,
            TradeEvent::Updated(_) => {}
        }
    }
}

async fn verify_trade_db_online(options: &TradeSmokeOptions) -> Result<TradeDbState> {
    // C++ commits SaveInventoryAndGoldToDB for both players before sending
    // COMPLETE; allow a short settle for an asynchronous DB writer.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let expected = options.expected;
        let state = tokio::task::spawn_blocking(move || load_trade_db_state(&expected))
            .await
            .map_err(|error| anyhow!("trade DB worker join failed: {error}"))??;
        match check_trade_db_state(&state, &options.expected) {
            Ok(()) => return Ok(state),
            Err(error) if tokio::time::Instant::now() >= deadline => {
                return Err(error.context("trade was not committed to the characters DB online"))
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(250)).await,
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_trade_smoke_phase_inner(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &TradeSmokeOptions,
    logged_out: &mut bool,
    result: &mut BotRunResult,
) -> Result<()> {
    let ack = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(stream, crypt, CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE, &ack).await?;
    pump_instance_socket(
        stream,
        crypt,
        server_inflater,
        Duration::from_millis(500),
        "trade pre-action drain",
        result,
    )
    .await?;

    if options.phase == TradeSmokePhase::VerifyRelog {
        loot_race::logout_and_wait_routed_like_cpp(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection.as_mut(),
            bot.character_guid,
            result,
        )
        .await?;
        *logged_out = true;
        result.trade.trade_relog_verified = true;
        result.trade.trade_smoke_passed = Some(true);
        return Ok(());
    }

    wait_trade_barrier(&options.sync.ready, options, "world entry").await?;
    // Let both players be added to the shared map grid before the range check.
    pump_instance_socket(
        stream,
        crypt,
        server_inflater,
        Duration::from_secs(1),
        "trade peer-visibility drain",
        result,
    )
    .await?;

    match options.role {
        TradeRole::Initiator => {
            run_initiator(
                bot_index,
                stream,
                crypt,
                server_inflater,
                realm_connection,
                options,
                result,
            )
            .await?
        }
        TradeRole::Receiver => {
            run_receiver(
                bot_index,
                stream,
                crypt,
                server_inflater,
                realm_connection,
                options,
                result,
            )
            .await?
        }
    }
    info!("[Bot {bot_index}] ✅ trade COMPLETE observed");

    wait_trade_barrier(&options.sync.completed, options, "trade completion").await?;
    if options.role == TradeRole::Receiver {
        let state = verify_trade_db_online(options).await?;
        result.trade.trade_money_after = Some(state.receiver_money);
        result.trade.trade_db_verified_online = true;
        info!("[Bot {bot_index}] ✅ trade committed to the characters DB while online");
    }
    // Neither session logs out (and re-saves) before the online check ran.
    wait_trade_barrier(&options.sync.verified, options, "online DB verification").await?;
    if options.role == TradeRole::Initiator {
        result.trade.trade_db_verified_online = true;
    }

    loot_race::logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection.as_mut(),
        bot.character_guid,
        result,
    )
    .await?;
    *logged_out = true;
    result.trade.trade_smoke_passed = Some(true);
    Ok(())
}

/// `run_post_login_smoke_phase` arm. A failure cancels the peer at once so it
/// does not wait for its own timeout, and best-effort cancels the open trade
/// (`HandleCancelTradeOpcode`) before the generic logout path runs.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_trade_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &TradeSmokeOptions,
    logged_out: &mut bool,
    result: &mut BotRunResult,
) -> Result<()> {
    let outcome = run_trade_smoke_phase_inner(
        bot_index,
        bot,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        options,
        logged_out,
        result,
    )
    .await;
    if let Err(error) = &outcome {
        options
            .sync
            .cancel(format!("{:?} failed: {error:#}", options.role));
        if !*logged_out && !result.trade.trade_completed {
            let _ = send_encrypted_packet(stream, crypt, CMSG_CANCEL_TRADE, &[]).await;
        }
    }
    outcome
}

#[cfg(test)]
#[path = "trade_smoke_tests.rs"]
mod tests;
