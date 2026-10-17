//! Vendor SELL persistence smoke.
//!
//! C++ anchors (TrinityCore-derived 3.4.3 reference):
//! - `ItemPackets.cpp` `SellItem::Read`: `VendorGUID`, `ItemGUID` (both
//!   packed `ObjectGuid`), `uint32 Amount`.
//! - `ItemHandler.cpp` `WorldSession::HandleSellItemOpcode`: on success it
//!   calls `ModifyMoney(SellPrice * Amount)`, removes the whole stack with
//!   `RemoveItem(bag, slot, true)` and `AddItemToBuyBackSlot(pItem)`, and sends
//!   **no** packet. `SMSG_SELL_RESPONSE` is only produced by
//!   `Player::SendSellError` for a failure.
//! - `ItemPackets.cpp` `SellResponse::Write`: packed `VendorGUID`,
//!   `uint32 ItemGUIDs.size()`, `int32 Reason`, then each packed item GUID.
//!   `ItemDefines.h` `enum SellResult` starts at `SELL_ERR_CANT_FIND_ITEM = 1`;
//!   there is no success value. Opcodes.cpp routes `SMSG_SELL_RESPONSE` on
//!   `CONNECTION_TYPE_REALM`.
//! - `Player.cpp` `Player::_SaveInventory`: every buyback-slot item whose state
//!   is not `ITEM_NEW` gets `DeleteFromInventoryDB` (character_inventory row)
//!   and `DeleteFromDB` (item_instance plus gems/transmog/gift rows), then is
//!   forced to `ITEM_NEW`. Buyback slots are never persisted, so after the
//!   logout save a sold DB-loaded item has neither row.
//! - `ItemTemplate.h` `GetSellPrice` reads `ItemSparse` DB2 `SellPrice`; the bot
//!   cannot read the client DB2 store, so the exact money delta is an optional
//!   operator input and otherwise only `delta > 0` is enforced and reported.

use super::*;

/// C++ Opcodes.h `CMSG_SELL_ITEM`.
pub(crate) const CMSG_SELL_ITEM: u16 = 0x34A2;
/// C++ Opcodes.h `SMSG_SELL_RESPONSE`.
pub(crate) const SMSG_SELL_RESPONSE: u16 = 0x26C5;
/// Linen Cloth: a common stackable trade good with a nonzero `SellPrice`
/// (13 copper in the Wrath Classic client data; verify with
/// `WOW_BOT_VENDOR_SELL_EXPECTED_MONEY=13` on a live run).
pub(crate) const DEFAULT_VENDOR_SELL_ITEM_ENTRY: u32 = 2589;
/// UnitDefines.h `UNIT_NPC_FLAG_VENDOR`.
pub(crate) const NPC_FLAG_VENDOR: u64 = 0x80;
/// CreatureData.h `CREATURE_FLAG_EXTRA_NO_SELL_VENDOR`.
pub(crate) const CREATURE_FLAG_EXTRA_NO_SELL_VENDOR: u32 = 0x0000_1000;
/// Quiet window after `CMSG_SELL_ITEM` in which any `SMSG_SELL_RESPONSE`
/// (always an error in C++) fails the phase.
pub(crate) const VENDOR_SELL_RESPONSE_WINDOW: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VendorSellPhase {
    Sell,
    VerifyRelog,
}

#[derive(Debug, Clone)]
pub(crate) struct VendorSellOptions {
    pub(crate) phase: VendorSellPhase,
    pub(crate) vendor: ResolvedCreatureTarget,
    pub(crate) target_match_radius: f32,
    pub(crate) item_entry: u32,
    pub(crate) item_guid: u64,
    pub(crate) money_before: u64,
    pub(crate) expected_money_delta: Option<u64>,
    /// Set for the relog phase from the first phase's persisted money.
    pub(crate) expected_money_after: Option<u64>,
    pub(crate) timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct VendorSellFixture {
    pub(crate) options: VendorSellOptions,
    pub(crate) original_position: CharacterPositionSnapshot,
    pub(crate) original_money: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct VendorSellCli {
    pub(crate) vendor_entry: u32,
    pub(crate) vendor_spawn_guid: u64,
    pub(crate) runtime_counter: Option<u64>,
    pub(crate) item_entry: u32,
    pub(crate) expected_money_delta: Option<u64>,
    pub(crate) timeout_secs: u64,
}

/// Flattened into `BotRunResult` so the JSON report carries `vendor_sell_*`.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct VendorSellEvidence {
    pub(crate) vendor_sell_smoke: bool,
    pub(crate) vendor_sell_smoke_passed: Option<bool>,
    pub(crate) vendor_sell_entry: Option<u32>,
    pub(crate) vendor_sell_spawn_guid: Option<u64>,
    pub(crate) vendor_sell_runtime_counter: Option<u64>,
    pub(crate) vendor_sell_item_entry: Option<u32>,
    pub(crate) vendor_sell_item_guid: Option<u64>,
    pub(crate) vendor_sell_money_before: Option<u64>,
    pub(crate) vendor_sell_money_after: Option<u64>,
    pub(crate) vendor_sell_money_delta: Option<u64>,
    pub(crate) vendor_sell_expected_money_delta: Option<u64>,
    pub(crate) vendor_sell_item_sent: bool,
    /// True when any `SMSG_SELL_RESPONSE` arrived; C++ only sends it on error.
    pub(crate) vendor_sell_response_seen: bool,
    pub(crate) vendor_sell_response_reason: Option<i32>,
    pub(crate) vendor_sell_db_persisted: bool,
    pub(crate) vendor_sell_relogin_verified: bool,
    pub(crate) vendor_sell_failure: Option<String>,
}

impl VendorSellEvidence {
    pub(crate) fn for_options(options: Option<&PostLoginSmokeOptions>) -> Self {
        let Some(PostLoginSmokeOptions::VendorSell(options)) = options else {
            return Self::default();
        };
        Self {
            vendor_sell_smoke: true,
            vendor_sell_entry: Some(options.vendor.entry),
            vendor_sell_spawn_guid: Some(options.vendor.spawn_guid),
            vendor_sell_runtime_counter: (options.vendor.guid_counter != 0)
                .then_some(options.vendor.guid_counter),
            vendor_sell_item_entry: Some(options.item_entry),
            vendor_sell_item_guid: Some(options.item_guid),
            vendor_sell_money_before: Some(options.money_before),
            vendor_sell_expected_money_delta: options.expected_money_delta,
            ..Self::default()
        }
    }

    pub(crate) fn passed(&self) -> bool {
        self.vendor_sell_smoke_passed.unwrap_or(false)
            && self.vendor_sell_item_sent
            && !self.vendor_sell_response_seen
            && self.vendor_sell_db_persisted
            && self.vendor_sell_relogin_verified
            && self.vendor_sell_money_delta.is_some_and(|delta| delta > 0)
    }
}

/// `CMSG_SELL_ITEM` body in `SellItem::Read` order.
pub(crate) fn build_sell_item_payload_like_cpp(
    vendor_packed_guid: &[u8],
    item_low: u64,
    item_high: u64,
    amount: u32,
) -> Vec<u8> {
    let mut payload = vendor_packed_guid.to_vec();
    payload.extend(build_packed_guid(item_low, item_high));
    payload.extend(amount.to_le_bytes());
    payload
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SellResponseWire {
    pub(crate) vendor_guid: (u64, u64),
    pub(crate) reason: i32,
    pub(crate) item_guids: Vec<(u64, u64)>,
}

pub(crate) fn parse_sell_response_like_cpp(payload: &[u8]) -> Result<SellResponseWire> {
    let (vendor_len, vendor_low, vendor_high) = parse_packed_guid(payload)
        .ok_or_else(|| anyhow!("SMSG_SELL_RESPONSE has an invalid packed VendorGUID"))?;
    let mut cursor = vendor_len;
    let mut take_u32 = |name: &str| -> Result<u32> {
        let bytes = payload
            .get(cursor..cursor + 4)
            .ok_or_else(|| anyhow!("SMSG_SELL_RESPONSE omitted {name}"))?;
        cursor += 4;
        Ok(u32::from_le_bytes(bytes.try_into()?))
    };
    let count = take_u32("ItemGUIDs count")?;
    let reason = take_u32("Reason")? as i32;
    if count > 64 {
        bail!("SMSG_SELL_RESPONSE item count {count} is implausible");
    }
    let mut item_guids = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (len, low, high) = payload
            .get(cursor..)
            .and_then(parse_packed_guid)
            .ok_or_else(|| anyhow!("SMSG_SELL_RESPONSE has an invalid packed item GUID"))?;
        cursor += len;
        item_guids.push((low, high));
    }
    if cursor != payload.len() {
        bail!(
            "SMSG_SELL_RESPONSE has {} trailing bytes",
            payload.len() - cursor
        );
    }
    Ok(SellResponseWire {
        vendor_guid: (vendor_low, vendor_high),
        reason,
        item_guids,
    })
}

/// `ItemDefines.h` `enum SellResult` names, for failure text.
pub(crate) fn sell_result_name(reason: i32) -> &'static str {
    match reason {
        1 => "SELL_ERR_CANT_FIND_ITEM",
        2 => "SELL_ERR_CANT_SELL_ITEM",
        3 => "SELL_ERR_CANT_FIND_VENDOR",
        4 => "SELL_ERR_YOU_DONT_OWN_THAT_ITEM",
        5 => "SELL_ERR_UNK",
        6 => "SELL_ERR_ONLY_EMPTY_BAG",
        7 => "SELL_ERR_CANT_SELL_TO_THIS_MERCHANT",
        8 => "SELL_ERR_MUST_REPAIR_DURABILITY",
        9 => "SELL_ERR_VENDOR_REFUSE_SCRAPPABLE_AZERITE",
        10 => "SELL_ERR_INTERNAL_BAG_ERROR",
        _ => "unknown SellResult",
    }
}

/// Money must strictly grow; with an operator-supplied `SellPrice * Amount`
/// it must grow by exactly that amount.
pub(crate) fn vendor_sell_money_delta(
    before: u64,
    after: u64,
    expected_delta: Option<u64>,
) -> Result<u64> {
    let delta = after
        .checked_sub(before)
        .filter(|delta| *delta > 0)
        .ok_or_else(|| anyhow!("vendor sell money did not increase: {before} -> {after}"))?;
    if let Some(expected) = expected_delta {
        if delta != expected {
            bail!("vendor sell money delta is {delta}, expected SellPrice*Amount {expected}");
        }
    }
    Ok(delta)
}

pub(crate) fn observe_vendor_sell_login_update(
    options: &VendorSellOptions,
    opcode: u16,
    payload: &[u8],
    seen: &mut Option<DiscoveredCreatureGuid>,
) -> Result<()> {
    if opcode != SMSG_UPDATE_OBJECT {
        return Ok(());
    }
    let Some(candidate) = find_vendor_sell_candidate(options, payload) else {
        return Ok(());
    };
    match seen {
        Some(previous) if (previous.low, previous.high) != (candidate.low, candidate.high) => {
            bail!(
                "vendor-sell login discovery produced two different live candidates near SQL spawn {}",
                options.vendor.spawn_guid
            )
        }
        _ => *seen = Some(candidate),
    }
    Ok(())
}

fn find_vendor_sell_candidate(
    options: &VendorSellOptions,
    payload: &[u8],
) -> Option<DiscoveredCreatureGuid> {
    find_creature_guid_near_position_in_update_object(
        payload,
        options.vendor.map_id,
        options.vendor.entry,
        options.vendor.x as f32,
        options.vendor.y as f32,
        options.vendor.z as f32,
        options.target_match_radius,
        (options.vendor.guid_counter != 0)
            .then_some(options.vendor.guid_counter & OBJECT_GUID_COUNTER_MASK),
    )
}

pub(crate) fn prepare_vendor_sell_fixture(
    bot: &config::BotConfig,
    cli: &VendorSellCli,
) -> Result<VendorSellFixture> {
    use mysql::prelude::Queryable;

    require_bot_local_account(&bot.account, "vendor-sell")?;
    if cli.item_entry == 0 || cli.item_entry > i32::MAX as u32 {
        bail!("vendor-sell item entry does not fit the 3.4.3 wire/database shape");
    }
    if cli
        .runtime_counter
        .is_some_and(|counter| counter == 0 || counter > OBJECT_GUID_COUNTER_MASK)
    {
        bail!("vendor runtime counter override must fit the nonzero 40-bit ObjectGuid counter");
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let (original_position, original_money) =
        load_offline_fixture_character(&mut characters, bot, "vendor-sell")?;
    let owned_same_entry: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM item_instance WHERE owner_guid = ? AND itemEntry = ?",
            (bot.character_guid, cli.item_entry),
        )
        .map_err(|error| anyhow!("Check existing vendor-sell fixture item: {error}"))?
        .unwrap_or(0);
    if owned_same_entry != 0 {
        bail!(
            "bot character already owns {owned_same_entry} item_instance rows of entry {}; choose an isolated --vendor-sell-item-entry",
            cli.item_entry
        );
    }
    let occupied_slots: Vec<u8> = characters
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (bot.character_guid,),
            |slot: u8| slot,
        )
        .map_err(|error| anyhow!("Load vendor-sell bot occupied backpack slots: {error}"))?;
    let item_slot = (INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
        .find(|slot| !occupied_slots.contains(slot))
        .ok_or_else(|| anyhow!("No empty default backpack slot for vendor-sell smoke"))?;
    let max_item_guid: u64 = characters
        .query_first("SELECT COALESCE(MAX(guid), 0) FROM item_instance")
        .map_err(|error| anyhow!("Load max item guid: {error}"))?
        .unwrap_or(0);
    let item_guid = max_item_guid
        .checked_add(10_000)
        .ok_or_else(|| anyhow!("item guid overflow while reserving vendor-sell fixture"))?;

    let (mut world, vendor, target_match_radius) =
        resolve_vendor_sql_spawn(cli.vendor_entry, cli.vendor_spawn_guid, cli.runtime_counter)?;
    // HandleSellItemOpcode: GetNPCIfCanInteractWith(..., UNIT_NPC_FLAG_VENDOR)
    // and the CREATURE_FLAG_EXTRA_NO_SELL_VENDOR refusal.
    let (npc_flags, flags_extra): (u64, u32) = world
        .exec_first(
            "SELECT IF(c.npcflag <> 0, c.npcflag, ct.npcflag), ct.flags_extra \
             FROM creature c JOIN creature_template ct ON ct.entry = c.id WHERE c.guid = ?",
            (cli.vendor_spawn_guid,),
        )
        .map_err(|error| anyhow!("Load vendor-sell npc flags: {error}"))?
        .ok_or_else(|| {
            anyhow!(
                "No creature_template for vendor spawn {}",
                cli.vendor_spawn_guid
            )
        })?;
    if npc_flags & NPC_FLAG_VENDOR == 0 || flags_extra & CREATURE_FLAG_EXTRA_NO_SELL_VENDOR != 0 {
        bail!(
            "vendor spawn {} cannot buy items (npcflag=0x{npc_flags:X}, flags_extra=0x{flags_extra:X})",
            cli.vendor_spawn_guid
        );
    }

    let mut transaction = characters
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start vendor-sell fixture transaction: {error}"))?;
    transaction
        .exec_drop(
            "INSERT INTO item_instance \
             (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, durability, \
              enchantments, charges, flags, randomPropertiesId, randomPropertiesSeed, context) \
             VALUES (?, ?, ?, 0, 0, 1, 0, '', '', 0, 0, 0, 0)",
            (item_guid, cli.item_entry, bot.character_guid),
        )
        .map_err(|error| anyhow!("Insert vendor-sell fixture item: {error}"))?;
    transaction
        .exec_drop(
            "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)",
            (bot.character_guid, item_slot, item_guid),
        )
        .map_err(|error| anyhow!("Insert vendor-sell fixture inventory row: {error}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET map = ?, zone = 0, instance_id = 0, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
                u32::from(vendor.map_id),
                vendor.x + 2.0,
                vendor.y,
                vendor.z,
                vendor.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|error| anyhow!("Relocate vendor-sell bot near vendor: {error}"))?;
    if transaction.affected_rows() != 1 {
        bail!("vendor-sell character relocation lost its offline ownership guard");
    }
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit vendor-sell fixture transaction: {error}"))?;
    info!(
        "Vendor-sell fixture: character={} vendor={}/{} item={}/entry={} slot={} money={}",
        bot.character_guid,
        cli.vendor_entry,
        cli.vendor_spawn_guid,
        item_guid,
        cli.item_entry,
        item_slot,
        original_money
    );
    Ok(VendorSellFixture {
        options: VendorSellOptions {
            phase: VendorSellPhase::Sell,
            vendor,
            target_match_radius,
            item_entry: cli.item_entry,
            item_guid,
            money_before: original_money,
            expected_money_delta: cli.expected_money_delta,
            expected_money_after: None,
            timeout_secs: cli.timeout_secs,
        },
        original_position,
        original_money,
    })
}

/// `(money, character_inventory rows, item_instance rows)` for the fixture item.
pub(crate) fn load_vendor_sell_db_state(
    bot: &config::BotConfig,
    item_guid: u64,
) -> Result<(u64, u64, u64)> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let money: u64 = conn
        .exec_first(
            "SELECT money FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load vendor-sell money: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    let inventory_rows: u64 = conn
        .exec_first(
            "SELECT COUNT(*) FROM character_inventory WHERE item = ?",
            (item_guid,),
        )
        .map_err(|error| anyhow!("Load vendor-sell inventory row: {error}"))?
        .unwrap_or(0);
    let instance_rows: u64 = conn
        .exec_first(
            "SELECT COUNT(*) FROM item_instance WHERE guid = ?",
            (item_guid,),
        )
        .map_err(|error| anyhow!("Load vendor-sell item_instance row: {error}"))?
        .unwrap_or(0);
    Ok((money, inventory_rows, instance_rows))
}

async fn load_vendor_sell_db_state_async(
    bot: &config::BotConfig,
    item_guid: u64,
) -> Result<(u64, u64, u64)> {
    let bot = bot.clone();
    tokio::task::spawn_blocking(move || load_vendor_sell_db_state(&bot, item_guid))
        .await
        .map_err(|error| anyhow!("Vendor-sell DB worker join failed: {error}"))?
}

/// Assert the C++ `_SaveInventory` buyback outcome: both rows deleted.
fn require_sold_item_rows_deleted(state: (u64, u64, u64), when: &str) -> Result<u64> {
    let (money, inventory_rows, instance_rows) = state;
    if inventory_rows != 0 || instance_rows != 0 {
        bail!(
            "sold item rows remain {when}: character_inventory={inventory_rows} item_instance={instance_rows}; C++ _SaveInventory deletes both for a buyback-slot item"
        );
    }
    Ok(money)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_vendor_sell_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &VendorSellOptions,
    login_discovered_target: Option<DiscoveredCreatureGuid>,
    logged_out: &mut bool,
    result: &mut BotRunResult,
) -> Result<()> {
    if options.phase == VendorSellPhase::VerifyRelog {
        let expected_after = options
            .expected_money_after
            .context("vendor-sell relog phase requires the persisted money")?;
        let money = require_sold_item_rows_deleted(
            load_vendor_sell_db_state_async(bot, options.item_guid).await?,
            "after relogin",
        )?;
        if money != expected_after {
            bail!("vendor-sell money after relogin is {money}, expected {expected_after}");
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
        let money = require_sold_item_rows_deleted(
            load_vendor_sell_db_state_async(bot, options.item_guid).await?,
            "after the relogin logout save",
        )?;
        if money != expected_after {
            bail!("vendor-sell money after relogin logout is {money}, expected {expected_after}");
        }
        result.vendor_sell.vendor_sell_money_after = Some(money);
        result.vendor_sell.vendor_sell_relogin_verified = true;
        result.vendor_sell.vendor_sell_smoke_passed = Some(true);
        return Ok(());
    }

    let (money_before, inventory_rows, instance_rows) =
        load_vendor_sell_db_state_async(bot, options.item_guid).await?;
    if (money_before, inventory_rows, instance_rows) != (options.money_before, 1, 1) {
        bail!(
            "vendor-sell fixture drifted before the sale: money/inventory/item {money_before}/{inventory_rows}/{instance_rows}, expected {}/1/1",
            options.money_before
        );
    }

    // C++ Player::CanNeverSee hides nearby objects until the active mover is
    // initialized; send the canonical ACK before the discovery window.
    let ack = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(stream, crypt, CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE, &ack).await?;
    let mut discovered = login_discovered_target;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    while discovered.is_none() && tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            Duration::from_millis(250).min(remaining),
            Duration::from_secs(5),
            "vendor-sell instance discovery",
        )
        .await?
        else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
        } else {
            observe_vendor_sell_login_update(options, opcode, &payload, &mut discovered)?;
        }
    }
    let runtime_target = resolve_vendor_runtime_target(&options.vendor, discovered)?;
    let runtime_vendor_guid = build_packed_guid(runtime_target.low, runtime_target.high);
    result.vendor_sell.vendor_sell_runtime_counter =
        Some(runtime_target.low & OBJECT_GUID_COUNTER_MASK);

    let realm = u16::try_from(realm_id()).context("REALM_ID does not fit an item ObjectGuid")?;
    let (item_low, item_high) = item_guid_raw(options.item_guid, realm);
    let payload = build_sell_item_payload_like_cpp(&runtime_vendor_guid, item_low, item_high, 1);
    send_encrypted_packet(stream, crypt, CMSG_SELL_ITEM, &payload).await?;
    result.vendor_sell.vendor_sell_item_sent = true;
    info!(
        "[Bot {}] ✅ CMSG_SELL_ITEM sent vendor counter={} item={} amount=1",
        bot_index,
        runtime_target.low & OBJECT_GUID_COUNTER_MASK,
        options.item_guid
    );
    wait_for_vendor_sell_quiet_window(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        result,
    )
    .await?;

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
    let money_after = require_sold_item_rows_deleted(
        load_vendor_sell_db_state_async(bot, options.item_guid).await?,
        "after the logout save",
    )?;
    result.vendor_sell.vendor_sell_money_after = Some(money_after);
    let delta = vendor_sell_money_delta(
        options.money_before,
        money_after,
        options.expected_money_delta,
    )?;
    result.vendor_sell.vendor_sell_money_delta = Some(delta);
    result.vendor_sell.vendor_sell_db_persisted = true;
    result.vendor_sell.vendor_sell_smoke_passed = Some(true);
    Ok(())
}

/// C++ success sends nothing; read both sockets for a bounded window and fail
/// on `SMSG_SELL_RESPONSE` (realm-routed error) or an inventory failure.
async fn wait_for_vendor_sell_quiet_window(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) -> Result<()> {
    let realm = realm_connection
        .as_mut()
        .context("vendor-sell smoke requires the preserved realm connection")?;
    let deadline = tokio::time::Instant::now() + VENDOR_SELL_RESPONSE_WINDOW;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        let routed = if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(5)),
            Duration::from_secs(5),
            "vendor-sell realm window",
        )
        .await?
        {
            Some((true, opcode, payload))
        } else {
            read_encrypted_packet_if_ready(
                stream,
                crypt,
                server_inflater,
                remaining.min(Duration::from_millis(5)),
                Duration::from_secs(5),
                "vendor-sell instance window",
            )
            .await?
            .map(|(opcode, payload)| (false, opcode, payload))
        };
        let Some((on_realm, opcode, payload)) = routed else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        match opcode {
            SMSG_TIME_SYNC_REQUEST if !on_realm => {
                let sequence = parse_time_sync_request_sequence(&payload)?;
                let response = build_time_sync_response_payload(sequence, 0);
                send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            }
            SMSG_SELL_RESPONSE => {
                result.vendor_sell.vendor_sell_response_seen = true;
                let response = parse_sell_response_like_cpp(&payload)?;
                result.vendor_sell.vendor_sell_response_reason = Some(response.reason);
                bail!(
                    "vendor sell returned SMSG_SELL_RESPONSE on {} reason={} ({}); C++ HandleSellItemOpcode only sends it on failure",
                    if on_realm { "realm" } else { "instance (C++ routes it on realm)" },
                    response.reason,
                    sell_result_name(response.reason)
                );
            }
            SMSG_INVENTORY_CHANGE_FAILURE => {
                bail!("vendor sell returned SMSG_INVENTORY_CHANGE_FAILURE");
            }
            _ => info!(
                "[Bot {}] 📦 {} vendor-sell {}",
                bot_index,
                if on_realm { "realm" } else { "instance" },
                parse_packet(opcode, &payload)
            ),
        }
    }
}

pub(crate) fn cleanup_vendor_sell_fixture(
    bot: &config::BotConfig,
    fixture: &VendorSellFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    wait_for_fixture_character_offline(&mut conn, bot.character_guid, "vendor-sell")?;
    let item_guid = fixture.options.item_guid;
    let position = &fixture.original_position;
    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start vendor-sell cleanup transaction: {error}"))?;
    transaction
        .exec_drop(
            "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
            (bot.character_guid, item_guid),
        )
        .map_err(|error| anyhow!("Delete vendor-sell inventory row: {error}"))?;
    transaction
        .exec_drop(
            "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?",
            (item_guid, bot.character_guid),
        )
        .map_err(|error| anyhow!("Delete vendor-sell item instance: {error}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET money = ?, map = ?, zone = ?, instance_id = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
                fixture.original_money,
                position.map_id,
                position.zone_id,
                position.instance_id,
                position.x,
                position.y,
                position.z,
                position.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|error| anyhow!("Restore vendor-sell money/position: {error}"))?;
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit vendor-sell cleanup: {error}"))?;
    let (money, inventory_rows, instance_rows) = load_vendor_sell_db_state(bot, item_guid)?;
    if (money, inventory_rows, instance_rows) != (fixture.original_money, 0, 0) {
        bail!(
            "vendor-sell cleanup verification found money/inventory/item {money}/{inventory_rows}/{instance_rows}, expected {}/0/0",
            fixture.original_money
        );
    }
    info!(
        "Vendor-sell fixture cleaned: character={} item={} money={}",
        bot.character_guid, item_guid, fixture.original_money
    );
    Ok(())
}

pub(crate) async fn run_vendor_sell_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    cli: VendorSellCli,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture =
        tokio::task::spawn_blocking(move || prepare_vendor_sell_fixture(&bot_for_setup, &cli))
            .await
            .map_err(|error| anyhow!("Vendor-sell setup DB worker join failed: {error}"))??;
    let cleanup = |bot: config::BotConfig, fixture: VendorSellFixture| async move {
        tokio::task::spawn_blocking(move || cleanup_vendor_sell_fixture(&bot, &fixture))
            .await
            .map_err(|error| anyhow!("Vendor-sell cleanup DB worker join failed: {error}"))?
    };

    let first = run_bot_with_post_login_smoke(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        PostLoginSmokeOptions::VendorSell(fixture.options.clone()),
    )
    .await;
    let mut combined = match first {
        Ok(result) => result,
        Err(error) => {
            if let Err(cleanup_error) = cleanup(bot.clone(), fixture.clone()).await {
                bail!("Vendor-sell login/phase failed: {error}; fixture cleanup failed: {cleanup_error}");
            }
            return Err(error.context("Vendor-sell login/phase failed"));
        }
    };

    if combined
        .vendor_sell
        .vendor_sell_smoke_passed
        .unwrap_or(false)
    {
        let mut relog = fixture.options.clone();
        relog.phase = VendorSellPhase::VerifyRelog;
        relog.expected_money_after = combined.vendor_sell.vendor_sell_money_after;
        match run_bot_with_post_login_smoke(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            PostLoginSmokeOptions::VendorSell(relog),
        )
        .await
        {
            Ok(second) => {
                combined.world_auth &= second.world_auth;
                combined.enum_characters &= second.enum_characters;
                combined.player_login_verified &= second.player_login_verified;
                combined.seen_opcodes.extend(second.seen_opcodes);
                let evidence = &mut combined.vendor_sell;
                evidence.vendor_sell_relogin_verified =
                    second.vendor_sell.vendor_sell_relogin_verified;
                evidence.vendor_sell_failure = second.vendor_sell.vendor_sell_failure;
                evidence.vendor_sell_smoke_passed = Some(
                    second.vendor_sell.vendor_sell_smoke_passed.unwrap_or(false)
                        && evidence.vendor_sell_relogin_verified,
                );
            }
            Err(error) => {
                combined.vendor_sell.vendor_sell_failure =
                    Some(format!("Vendor-sell persistence relog failed: {error}"));
                combined.vendor_sell.vendor_sell_smoke_passed = Some(false);
            }
        }
    }

    if let Err(error) = cleanup(bot.clone(), fixture.clone()).await {
        append_failure(
            &mut combined.vendor_sell.vendor_sell_failure,
            format!("Vendor-sell fixture cleanup failed: {error}"),
        );
        combined.vendor_sell.vendor_sell_smoke_passed = Some(false);
    }
    Ok(combined)
}
