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
#[path = "loot/tests/mod.rs"]
mod tests;
