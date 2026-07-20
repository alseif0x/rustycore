// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet definitions.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;
pub use wow_loot::{CreatureLoot, LootEntry, LootEntryFlags, NotNormalLootItem};

use crate::packets::item::ItemInstance;
use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

pub const LOOT_ERROR_DIDNT_KILL_LIKE_CPP: u8 = 0;
pub const LOOT_ERROR_TOO_FAR_LIKE_CPP: u8 = 4;
pub const LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP: u8 = 10;
pub const LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP: u8 = 12;
pub const LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP: u8 = 13;
pub const LOOT_ERROR_MASTER_OTHER_LIKE_CPP: u8 = 14;
pub const LOOT_ERROR_NO_LOOT_LIKE_CPP: u8 = 17;

pub const LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP: u8 = 2;
pub const LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP: u8 = LOOT_ERROR_NO_LOOT_LIKE_CPP;

pub const LOOT_TYPE_NONE_LIKE_CPP: u8 = 0;
pub const LOOT_TYPE_CORPSE_LIKE_CPP: u8 = 1;
pub const LOOT_TYPE_PICKPOCKETING_LIKE_CPP: u8 = 2;
pub const LOOT_TYPE_FISHING_LIKE_CPP: u8 = 3;
pub const LOOT_TYPE_DISENCHANTING_LIKE_CPP: u8 = 4;
pub const LOOT_TYPE_ITEM_LIKE_CPP: u8 = 5;
pub const LOOT_TYPE_SKINNING_LIKE_CPP: u8 = 6;
pub const LOOT_TYPE_GATHERING_NODE_LIKE_CPP: u8 = 8;
pub const LOOT_TYPE_CHEST_LIKE_CPP: u8 = 9;
pub const LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP: u8 = 14;
pub const LOOT_TYPE_FISHINGHOLE_LIKE_CPP: u8 = 20;
pub const LOOT_TYPE_INSIGNIA_LIKE_CPP: u8 = 21;
pub const LOOT_TYPE_FISHING_JUNK_LIKE_CPP: u8 = 22;
pub const LOOT_TYPE_PROSPECTING_LIKE_CPP: u8 = 23;
pub const LOOT_TYPE_MILLING_LIKE_CPP: u8 = 24;

// ── LootUnit (CMSG_LOOT_UNIT) ────────────────────────────────────

/// Client requests to loot a unit (dead creature).
#[derive(Debug, Clone)]
pub struct LootUnit {
    pub unit: ObjectGuid,
}

impl ClientPacket for LootUnit {
    const OPCODE: ClientOpcodes = ClientOpcodes::LootUnit;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let unit = pkt.read_packed_guid()?;
        Ok(Self { unit })
    }
}

// ── LootItemPkt (CMSG_LOOT_ITEM) ─────────────────────────────────

/// Client requests to take a specific item from a loot window.
#[derive(Debug, Clone)]
pub struct LootItemPkt {
    pub requests: Vec<LootItemRequest>,
    pub is_soft_interact: bool,
}

#[derive(Debug, Clone)]
pub struct LootItemRequest {
    pub object: ObjectGuid,
    pub loot_list_id: u8,
}

impl ClientPacket for LootItemPkt {
    const OPCODE: ClientOpcodes = ClientOpcodes::LootItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = pkt.read_uint32()?;
        let mut requests = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let object = pkt.read_packed_guid()?;
            let loot_list_id = pkt.read_uint8()?;
            requests.push(LootItemRequest {
                object,
                loot_list_id,
            });
        }
        let is_soft_interact = pkt.has_bit()?;
        Ok(Self {
            requests,
            is_soft_interact,
        })
    }
}

// ── LootRelease (CMSG_LOOT_RELEASE) ──────────────────────────────

/// Client closes the loot window.
#[derive(Debug, Clone)]
pub struct LootRelease {
    pub unit: ObjectGuid,
}

impl ClientPacket for LootRelease {
    const OPCODE: ClientOpcodes = ClientOpcodes::LootRelease;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let unit = pkt.read_packed_guid()?;
        Ok(Self { unit })
    }
}

// ── LootMoney (CMSG_LOOT_MONEY) ─────────────────────────────────

/// Client requests the money from the current loot view.
#[derive(Debug, Clone)]
pub struct LootMoney {
    pub is_soft_interact: bool,
}

impl ClientPacket for LootMoney {
    const OPCODE: ClientOpcodes = ClientOpcodes::LootMoney;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let is_soft_interact = pkt.has_bit()?;
        Ok(Self { is_soft_interact })
    }
}

// ── LootRoll (CMSG_LOOT_ROLL) ───────────────────────────────────

/// Client votes on a pending group loot roll.
#[derive(Debug, Clone)]
pub struct LootRoll {
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub roll_type: u8,
}

impl ClientPacket for LootRoll {
    const OPCODE: ClientOpcodes = ClientOpcodes::LootRoll;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let loot_obj = pkt.read_packed_guid()?;
        let loot_list_id = pkt.read_uint8()?;
        let roll_type = pkt.read_uint8()?;
        Ok(Self {
            loot_obj,
            loot_list_id,
            roll_type,
        })
    }
}

// ── MasterLootItem (CMSG_MASTER_LOOT_ITEM) ───────────────────────

/// Client-side master-loot assignment request.
#[derive(Debug, Clone)]
pub struct MasterLootItem {
    pub target: ObjectGuid,
    pub loot: Vec<LootItemRequest>,
}

impl ClientPacket for MasterLootItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::MasterLootItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = pkt.read_uint32()?;
        let target = pkt.read_packed_guid()?;
        let mut loot = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let object = pkt.read_packed_guid()?;
            let loot_list_id = pkt.read_uint8()?;
            loot.push(LootItemRequest {
                object,
                loot_list_id,
            });
        }
        Ok(Self { target, loot })
    }
}

// ── SetLootSpecialization (CMSG_SET_LOOT_SPECIALIZATION) ─────────

/// Client selects a loot specialization.
#[derive(Debug, Clone)]
pub struct SetLootSpecialization {
    pub spec_id: u32,
}

impl ClientPacket for SetLootSpecialization {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetLootSpecialization;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let spec_id = pkt.read_uint32()?;
        Ok(Self { spec_id })
    }
}

// ── LootItemData ─────────────────────────────────────────────────

/// One item entry in a loot window.
#[derive(Debug, Clone)]
pub struct LootItemData {
    pub item_type: u8,
    pub ui_type: u8,
    pub can_trade_to_tap_list: bool,
    pub loot: ItemInstance,
    pub loot_list_id: u8,
    pub quantity: u32,
    pub loot_item_type: u8,
}

impl LootItemData {
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(u32::from(self.item_type), 2);
        pkt.write_bits(u32::from(self.ui_type), 3);
        pkt.write_bit(self.can_trade_to_tap_list);
        pkt.flush_bits();
        self.loot.write(pkt);
        pkt.write_uint32(self.quantity);
        pkt.write_uint8(self.loot_item_type);
        pkt.write_uint8(self.loot_list_id);
    }
}

// ── LootCurrencyData ─────────────────────────────────────────────

/// One currency entry in a loot window.
#[derive(Debug, Clone)]
pub struct LootCurrencyData {
    pub currency_id: u32,
    pub quantity: u32,
    pub loot_list_id: u8,
    pub ui_type: u8,
}

impl LootCurrencyData {
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.currency_id);
        pkt.write_uint32(self.quantity);
        pkt.write_uint8(self.loot_list_id);
        pkt.write_bits(u32::from(self.ui_type), 3);
        pkt.flush_bits();
    }
}

// ── LootResponse (SMSG_LOOT_RESPONSE) ────────────────────────────

/// Server sends loot window contents to the client.
#[derive(Debug, Clone)]
pub struct LootResponse {
    pub owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub failure_reason: u8,
    pub acquire_reason: u8,
    pub loot_method: u8,
    pub threshold: u8,
    pub coins: u32,
    pub items: Vec<LootItemData>,
    pub currencies: Vec<LootCurrencyData>,
    pub acquired: bool,
    pub ae_looting: bool,
}

impl ServerPacket for LootResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.owner);
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_uint8(self.failure_reason);
        pkt.write_uint8(self.acquire_reason);
        pkt.write_uint8(self.loot_method);
        pkt.write_uint8(self.threshold);
        pkt.write_uint32(self.coins);
        pkt.write_uint32(self.items.len() as u32);
        pkt.write_uint32(self.currencies.len() as u32);
        pkt.write_bit(self.acquired);
        pkt.write_bit(self.ae_looting);
        pkt.flush_bits();
        for item in &self.items {
            item.write(pkt);
        }
        for currency in &self.currencies {
            currency.write(pkt);
        }
    }
}

// ── LootRemoved (SMSG_LOOT_REMOVED) ──────────────────────────────

/// Server notifies client that a loot item was removed from the window.
#[derive(Debug, Clone)]
pub struct LootRemoved {
    pub owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
}

impl ServerPacket for LootRemoved {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootRemoved;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.owner);
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_uint8(self.loot_list_id);
    }
}

// ── LootList (SMSG_LOOT_LIST) ────────────────────────────────────

/// Server notifies allowed looters about the current loot owner/list state.
#[derive(Debug, Clone)]
pub struct LootList {
    pub owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub master: Option<ObjectGuid>,
    pub round_robin_winner: Option<ObjectGuid>,
}

impl ServerPacket for LootList {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootList;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.owner);
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_bit(self.master.is_some());
        pkt.write_bit(self.round_robin_winner.is_some());
        pkt.flush_bits();

        if let Some(master) = self.master {
            pkt.write_packed_guid(&master);
        }

        if let Some(round_robin_winner) = self.round_robin_winner {
            pkt.write_packed_guid(&round_robin_winner);
        }
    }
}

// ── SLootRelease (SMSG_LOOT_RELEASE) ─────────────────────────────

/// Server acknowledges loot window close.
#[derive(Debug, Clone)]
pub struct SLootRelease {
    pub loot_obj: ObjectGuid,
    pub owner: ObjectGuid,
}

impl ServerPacket for SLootRelease {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootRelease;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_packed_guid(&self.owner);
    }
}

// ── LootReleaseAll (SMSG_LOOT_RELEASE_ALL) ───────────────────────

/// Server tells the client to close all loot windows.
#[derive(Debug, Clone)]
pub struct LootReleaseAll;

impl ServerPacket for LootReleaseAll {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootReleaseAll;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── LootMoneyNotify (SMSG_LOOT_MONEY_NOTIFY) ─────────────────────

/// Server notifies the client that money was looted.
#[derive(Debug, Clone)]
pub struct LootMoneyNotify {
    pub money: u64,
    pub money_mod: u64,
    pub sole_looter: bool,
}

impl ServerPacket for LootMoneyNotify {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootMoneyNotify;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.money);
        pkt.write_uint64(self.money_mod);
        pkt.write_bit(self.sole_looter);
        pkt.flush_bits();
    }
}

// ── CoinRemoved (SMSG_COIN_REMOVED) ──────────────────────────────

/// Server notifies the client that coins were removed from the loot window.
#[derive(Debug, Clone)]
pub struct CoinRemoved {
    pub loot_obj: ObjectGuid,
}

impl ServerPacket for CoinRemoved {
    const OPCODE: ServerOpcodes = ServerOpcodes::CoinRemoved;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.loot_obj);
    }
}

// ── AELootTargets (SMSG_AE_LOOT_TARGETS) ─────────────────────────

/// Server tells the client how many area-loot targets will be streamed.
#[derive(Debug, Clone)]
pub struct AELootTargets {
    pub count: u32,
}

impl ServerPacket for AELootTargets {
    const OPCODE: ServerOpcodes = ServerOpcodes::AeLootTargets;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.count);
    }
}

// ── AELootTargetsAck (SMSG_AE_LOOT_TARGET_ACK) ───────────────────

/// Server acknowledges one area-loot target response.
#[derive(Debug, Clone)]
pub struct AELootTargetsAck;

impl ServerPacket for AELootTargetsAck {
    const OPCODE: ServerOpcodes = ServerOpcodes::AeLootTargetAck;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

// ── StartLootRoll (SMSG_START_LOOT_ROLL) ────────────────────────

#[derive(Debug, Clone)]
pub struct StartLootRoll {
    pub loot_obj: ObjectGuid,
    pub map_id: i32,
    pub roll_time_ms: u32,
    pub method: u8,
    pub valid_rolls: u8,
    pub loot_roll_ineligible_reason: [u32; 4],
    pub item: LootItemData,
    pub dungeon_encounter_id: i32,
}

impl ServerPacket for StartLootRoll {
    const OPCODE: ServerOpcodes = ServerOpcodes::StartLootRoll;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_int32(self.map_id);
        pkt.write_uint32(self.roll_time_ms);
        pkt.write_uint8(self.valid_rolls);
        for reason in self.loot_roll_ineligible_reason {
            pkt.write_uint32(reason);
        }
        pkt.write_uint8(self.method);
        pkt.write_int32(self.dungeon_encounter_id);
        self.item.write(pkt);
    }
}

// ── LootRollBroadcast (SMSG_LOOT_ROLL) ───────────────────────────

#[derive(Debug, Clone)]
pub struct LootRollBroadcast {
    pub loot_obj: ObjectGuid,
    pub player: ObjectGuid,
    pub roll: i32,
    pub roll_type: u8,
    pub item: LootItemData,
    pub autopassed: bool,
    pub off_spec: bool,
    pub dungeon_encounter_id: i32,
}

impl ServerPacket for LootRollBroadcast {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootRoll;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_packed_guid(&self.player);
        pkt.write_int32(self.roll);
        pkt.write_uint8(self.roll_type);
        pkt.write_int32(self.dungeon_encounter_id);
        self.item.write(pkt);
        pkt.write_bit(self.autopassed);
        pkt.write_bit(self.off_spec);
        pkt.flush_bits();
    }
}

// ── LootRollWon (SMSG_LOOT_ROLL_WON) ─────────────────────────────

#[derive(Debug, Clone)]
pub struct LootRollWon {
    pub loot_obj: ObjectGuid,
    pub winner: ObjectGuid,
    pub roll: i32,
    pub roll_type: u8,
    pub item: LootItemData,
    pub main_spec: bool,
    pub dungeon_encounter_id: i32,
}

impl ServerPacket for LootRollWon {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootRollWon;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_packed_guid(&self.winner);
        pkt.write_int32(self.roll);
        pkt.write_uint8(self.roll_type);
        pkt.write_int32(self.dungeon_encounter_id);
        self.item.write(pkt);
        pkt.write_bit(self.main_spec);
        pkt.flush_bits();
    }
}

// ── LootAllPassed (SMSG_LOOT_ALL_PASSED) ─────────────────────────

#[derive(Debug, Clone)]
pub struct LootAllPassed {
    pub loot_obj: ObjectGuid,
    pub item: LootItemData,
    pub dungeon_encounter_id: i32,
}

impl ServerPacket for LootAllPassed {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootAllPassed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_int32(self.dungeon_encounter_id);
        self.item.write(pkt);
    }
}

// ── LootRollsComplete (SMSG_LOOT_ROLLS_COMPLETE) ─────────────────

#[derive(Debug, Clone)]
pub struct LootRollsComplete {
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub dungeon_encounter_id: i32,
}

impl ServerPacket for LootRollsComplete {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootRollsComplete;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_uint8(self.loot_list_id);
        pkt.write_int32(self.dungeon_encounter_id);
    }
}

// ── MasterLootCandidateList (SMSG_MASTER_LOOT_CANDIDATE_LIST) ────

#[derive(Debug, Clone)]
pub struct MasterLootCandidateList {
    pub loot_obj: ObjectGuid,
    pub players: Vec<ObjectGuid>,
}

impl ServerPacket for MasterLootCandidateList {
    const OPCODE: ServerOpcodes = ServerOpcodes::MasterLootCandidateList;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.loot_obj);
        pkt.write_uint32(self.players.len() as u32);
        for player in &self.players {
            pkt.write_packed_guid(player);
        }
    }
}

// ── In-memory loot tracking ──────────────────────────────────────

pub const LOOT_SLOT_TYPE_OWNER_LIKE_CPP: u8 = 4;

#[cfg(test)]
mod tests {
    use crate::{ClientPacket, ServerPacket};

    use super::{
        AELootTargets, AELootTargetsAck, CoinRemoved,
        LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP, LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        LootAllPassed, LootCurrencyData, LootItemData, LootList, LootMoney, LootMoneyNotify,
        LootReleaseAll, LootRemoved, LootResponse, LootRoll, LootRollBroadcast, LootRollWon,
        LootRollsComplete, MasterLootCandidateList, MasterLootItem, SLootRelease,
        SetLootSpecialization, StartLootRoll,
    };
    use crate::packets::item::ItemInstance;
    use crate::world_packet::WorldPacket;

    #[test]
    fn loot_money_reads_cpp_soft_interact_bit() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();

        let parsed = LootMoney::read(&mut pkt).expect("loot money packet should parse");
        assert!(parsed.is_soft_interact);
    }

    #[test]
    fn loot_roll_reads_cpp_loot_obj_list_id_and_vote() {
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&loot_obj);
        pkt.write_uint8(7);
        pkt.write_uint8(2);
        pkt.reset_read();

        let parsed = LootRoll::read(&mut pkt).expect("loot roll packet should parse");
        assert_eq!(parsed.loot_obj, loot_obj);
        assert_eq!(parsed.loot_list_id, 7);
        assert_eq!(parsed.roll_type, 2);
    }

    #[test]
    fn master_loot_item_reads_cpp_count_target_then_requests() {
        let target = wow_core::ObjectGuid::create_player(1, 77);
        let first = wow_core::ObjectGuid::create_item(1, 42);
        let second = wow_core::ObjectGuid::create_item(1, 43);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(2);
        pkt.write_packed_guid(&target);
        pkt.write_packed_guid(&first);
        pkt.write_uint8(3);
        pkt.write_packed_guid(&second);
        pkt.write_uint8(4);
        pkt.reset_read();

        let parsed = MasterLootItem::read(&mut pkt).expect("master loot packet should parse");
        assert_eq!(parsed.target, target);
        assert_eq!(parsed.loot.len(), 2);
        assert_eq!(parsed.loot[0].object, first);
        assert_eq!(parsed.loot[0].loot_list_id, 3);
        assert_eq!(parsed.loot[1].object, second);
        assert_eq!(parsed.loot[1].loot_list_id, 4);
    }

    #[test]
    fn set_loot_specialization_reads_cpp_spec_id() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(65);
        pkt.reset_read();

        let parsed =
            SetLootSpecialization::read(&mut pkt).expect("set loot specialization should parse");
        assert_eq!(parsed.spec_id, 65);
    }

    #[test]
    fn loot_money_notify_writes_cpp_money_money_mod_and_sole_looter() {
        let notify = LootMoneyNotify {
            money: 123,
            money_mod: 7,
            sole_looter: true,
        };
        let mut pkt = WorldPacket::new_empty();
        notify.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_uint64().unwrap(), 123);
        assert_eq!(pkt.read_uint64().unwrap(), 7);
        assert!(pkt.read_bit().unwrap());
    }

    #[test]
    fn loot_response_writes_cpp_owner_then_loot_obj() {
        let owner = wow_core::ObjectGuid::create_player(1, 7);
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let response = LootResponse {
            owner,
            loot_obj,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: 0,
            loot_method: 0,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: 0,
            items: Vec::new(),
            currencies: Vec::new(),
            acquired: true,
            ae_looting: false,
        };
        let mut pkt = WorldPacket::new_empty();
        response.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), owner);
        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    }

    #[test]
    fn loot_response_success_defaults_write_cpp_failure_reason_and_threshold() {
        let owner = wow_core::ObjectGuid::create_player(1, 7);
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let response = LootResponse {
            owner,
            loot_obj,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: 0,
            loot_method: 0,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: 0,
            items: Vec::new(),
            currencies: Vec::new(),
            acquired: true,
            ae_looting: false,
        };
        let mut pkt = WorldPacket::new_empty();
        response.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), owner);
        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(
            pkt.read_uint8().unwrap(),
            LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP
        );
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(
            pkt.read_uint8().unwrap(),
            LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP
        );
    }

    #[test]
    fn loot_currency_data_writes_cpp_shape() {
        let currency = LootCurrencyData {
            currency_id: 395,
            quantity: 7,
            loot_list_id: 3,
            ui_type: 5,
        };
        let mut pkt = WorldPacket::new_empty();
        currency.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_uint32().unwrap(), 395);
        assert_eq!(pkt.read_uint32().unwrap(), 7);
        assert_eq!(pkt.read_uint8().unwrap(), 3);
        assert_eq!(pkt.read_bits(3).unwrap(), 5);
    }

    #[test]
    fn loot_response_writes_cpp_currency_count_and_entries_after_items() {
        let owner = wow_core::ObjectGuid::create_player(1, 7);
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let response = LootResponse {
            owner,
            loot_obj,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: 0,
            loot_method: 0,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: 11,
            items: Vec::new(),
            currencies: vec![LootCurrencyData {
                currency_id: 395,
                quantity: 7,
                loot_list_id: 3,
                ui_type: 5,
            }],
            acquired: true,
            ae_looting: false,
        };
        let mut pkt = WorldPacket::new_empty();
        response.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), owner);
        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(
            pkt.read_uint8().unwrap(),
            LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP
        );
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(
            pkt.read_uint8().unwrap(),
            LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP
        );
        assert_eq!(pkt.read_uint32().unwrap(), 11);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 1);
        assert!(pkt.read_bit().unwrap());
        assert!(!pkt.read_bit().unwrap());
        pkt.reset_bits();
        assert_eq!(pkt.read_uint32().unwrap(), 395);
        assert_eq!(pkt.read_uint32().unwrap(), 7);
        assert_eq!(pkt.read_uint8().unwrap(), 3);
        assert_eq!(pkt.read_bits(3).unwrap(), 5);
    }

    #[test]
    fn loot_item_data_writes_cpp_shape() {
        let item = LootItemData {
            item_type: 0,
            ui_type: 4,
            can_trade_to_tap_list: false,
            loot: ItemInstance {
                item_id: 25,
                ..ItemInstance::default()
            },
            loot_list_id: 7,
            quantity: 2,
            loot_item_type: 0,
        };
        let mut pkt = WorldPacket::new_empty();
        item.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_bits(2).unwrap(), 0);
        assert_eq!(pkt.read_bits(3).unwrap(), 4);
        assert!(!pkt.read_bit().unwrap());
        assert_eq!(pkt.read_int32().unwrap(), 25);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert!(!pkt.read_bit().unwrap());
        pkt.reset_bits();
        assert_eq!(pkt.read_bits(6).unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 2);
        assert_eq!(pkt.read_uint8().unwrap(), 0);
        assert_eq!(pkt.read_uint8().unwrap(), 7);
    }

    #[test]
    fn loot_removed_writes_cpp_owner_then_loot_obj() {
        let owner = wow_core::ObjectGuid::create_player(1, 7);
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let removed = LootRemoved {
            owner,
            loot_obj,
            loot_list_id: 3,
        };
        let mut pkt = WorldPacket::new_empty();
        removed.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), owner);
        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(pkt.read_uint8().unwrap(), 3);
    }

    #[test]
    fn loot_list_writes_cpp_owner_loot_obj_bits_and_optional_guids() {
        let owner = wow_core::ObjectGuid::create_player(1, 7);
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let master = wow_core::ObjectGuid::create_player(1, 77);
        let round_robin_winner = wow_core::ObjectGuid::create_player(1, 78);
        let list = LootList {
            owner,
            loot_obj,
            master: Some(master),
            round_robin_winner: Some(round_robin_winner),
        };
        let mut pkt = WorldPacket::new_empty();
        list.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), owner);
        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert!(pkt.read_bit().unwrap());
        assert!(pkt.read_bit().unwrap());
        pkt.reset_bits();
        assert_eq!(pkt.read_packed_guid().unwrap(), master);
        assert_eq!(pkt.read_packed_guid().unwrap(), round_robin_winner);
    }

    #[test]
    fn loot_list_writes_cpp_absent_optional_bits_without_guids() {
        let owner = wow_core::ObjectGuid::create_player(1, 7);
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let list = LootList {
            owner,
            loot_obj,
            master: None,
            round_robin_winner: None,
        };
        let mut pkt = WorldPacket::new_empty();
        list.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), owner);
        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert!(!pkt.read_bit().unwrap());
        assert!(!pkt.read_bit().unwrap());
        pkt.reset_bits();
        assert!(pkt.read_uint8().is_err());
    }

    #[test]
    fn coin_removed_writes_cpp_loot_obj() {
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let notify = CoinRemoved { loot_obj };
        let mut pkt = WorldPacket::new_empty();
        notify.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
    }

    #[test]
    fn ae_loot_targets_writes_cpp_count_only() {
        let targets = AELootTargets { count: 3 };
        let mut pkt = WorldPacket::new_empty();
        targets.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_uint32().unwrap(), 3);
        assert!(pkt.read_uint8().is_err());
    }

    #[test]
    fn ae_loot_targets_ack_writes_cpp_empty_payload() {
        let ack = AELootTargetsAck;
        let mut pkt = WorldPacket::new_empty();
        ack.write(&mut pkt);
        pkt.reset_read();

        assert!(pkt.read_uint8().is_err());
    }

    fn roll_test_item() -> LootItemData {
        LootItemData {
            item_type: 0,
            ui_type: 5,
            can_trade_to_tap_list: false,
            loot: ItemInstance {
                item_id: 25,
                ..ItemInstance::default()
            },
            loot_list_id: 9,
            quantity: 1,
            loot_item_type: 0,
        }
    }

    #[test]
    fn start_loot_roll_writes_cpp_order() {
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let roll = StartLootRoll {
            loot_obj,
            map_id: 571,
            roll_time_ms: 60_000,
            method: 3,
            valid_rolls: 0x07,
            loot_roll_ineligible_reason: [1, 2, 3, 4],
            item: roll_test_item(),
            dungeon_encounter_id: 99,
        };
        let mut pkt = WorldPacket::new_empty();
        roll.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(pkt.read_int32().unwrap(), 571);
        assert_eq!(pkt.read_uint32().unwrap(), 60_000);
        assert_eq!(pkt.read_uint8().unwrap(), 0x07);
        assert_eq!(pkt.read_uint32().unwrap(), 1);
        assert_eq!(pkt.read_uint32().unwrap(), 2);
        assert_eq!(pkt.read_uint32().unwrap(), 3);
        assert_eq!(pkt.read_uint32().unwrap(), 4);
        assert_eq!(pkt.read_uint8().unwrap(), 3);
        assert_eq!(pkt.read_int32().unwrap(), 99);
        assert_eq!(pkt.read_bits(2).unwrap(), 0);
        assert_eq!(pkt.read_bits(3).unwrap(), 5);
    }

    #[test]
    fn loot_roll_broadcast_writes_cpp_order_and_bits() {
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let player = wow_core::ObjectGuid::create_player(1, 77);
        let broadcast = LootRollBroadcast {
            loot_obj,
            player,
            roll: -42,
            roll_type: 2,
            item: roll_test_item(),
            autopassed: true,
            off_spec: false,
            dungeon_encounter_id: 99,
        };
        let mut pkt = WorldPacket::new_empty();
        broadcast.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(pkt.read_packed_guid().unwrap(), player);
        assert_eq!(pkt.read_int32().unwrap(), -42);
        assert_eq!(pkt.read_uint8().unwrap(), 2);
        assert_eq!(pkt.read_int32().unwrap(), 99);
    }

    #[test]
    fn loot_roll_won_writes_cpp_order() {
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let winner = wow_core::ObjectGuid::create_player(1, 77);
        let won = LootRollWon {
            loot_obj,
            winner,
            roll: 98,
            roll_type: 2,
            item: roll_test_item(),
            main_spec: true,
            dungeon_encounter_id: 99,
        };
        let mut pkt = WorldPacket::new_empty();
        won.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(pkt.read_packed_guid().unwrap(), winner);
        assert_eq!(pkt.read_int32().unwrap(), 98);
        assert_eq!(pkt.read_uint8().unwrap(), 2);
        assert_eq!(pkt.read_int32().unwrap(), 99);
    }

    #[test]
    fn loot_all_passed_writes_cpp_order() {
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let passed = LootAllPassed {
            loot_obj,
            item: roll_test_item(),
            dungeon_encounter_id: 99,
        };
        let mut pkt = WorldPacket::new_empty();
        passed.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(pkt.read_int32().unwrap(), 99);
        assert_eq!(pkt.read_bits(2).unwrap(), 0);
        assert_eq!(pkt.read_bits(3).unwrap(), 5);
    }

    #[test]
    fn loot_rolls_complete_writes_cpp_order() {
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let complete = LootRollsComplete {
            loot_obj,
            loot_list_id: 7,
            dungeon_encounter_id: 99,
        };
        let mut pkt = WorldPacket::new_empty();
        complete.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(pkt.read_uint8().unwrap(), 7);
        assert_eq!(pkt.read_int32().unwrap(), 99);
    }

    #[test]
    fn master_loot_candidate_list_writes_cpp_order() {
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let first = wow_core::ObjectGuid::create_player(1, 77);
        let second = wow_core::ObjectGuid::create_player(1, 78);
        let list = MasterLootCandidateList {
            loot_obj,
            players: vec![first, second],
        };
        let mut pkt = WorldPacket::new_empty();
        list.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(pkt.read_uint32().unwrap(), 2);
        assert_eq!(pkt.read_packed_guid().unwrap(), first);
        assert_eq!(pkt.read_packed_guid().unwrap(), second);
    }

    #[test]
    fn loot_release_writes_cpp_loot_obj_then_owner() {
        let loot_obj = wow_core::ObjectGuid::create_item(1, 42);
        let owner = wow_core::ObjectGuid::create_player(1, 7);
        let release = SLootRelease { loot_obj, owner };
        let mut pkt = WorldPacket::new_empty();
        release.write(&mut pkt);
        pkt.reset_read();

        assert_eq!(pkt.read_packed_guid().unwrap(), loot_obj);
        assert_eq!(pkt.read_packed_guid().unwrap(), owner);
    }

    #[test]
    fn loot_release_all_writes_cpp_empty_payload() {
        let release = LootReleaseAll;
        let mut pkt = WorldPacket::new_empty();
        release.write(&mut pkt);
        pkt.reset_read();

        assert!(pkt.read_uint8().is_err());
    }
}
