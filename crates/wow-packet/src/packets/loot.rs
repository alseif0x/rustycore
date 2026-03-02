// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet definitions.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

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
            requests.push(LootItemRequest { object, loot_list_id });
        }
        let is_soft_interact = pkt.has_bit()?;
        Ok(Self { requests, is_soft_interact })
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

// ── LootItemData ─────────────────────────────────────────────────

/// One item entry in a loot window.
#[derive(Debug, Clone)]
pub struct LootItemData {
    pub loot_list_id: u8,
    pub ui_type: u8,        // 0=normal, 1=can_never_steal, 2=owner_only, etc.
    pub quantity: u32,
    pub item_id: i32,
    pub item_context: u8,
    pub bonus_list_ids: Vec<i32>,
    pub can_loot: bool,
}

impl LootItemData {
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.loot_list_id);
        pkt.write_uint8(self.ui_type);
        pkt.write_int32(self.item_id);
        pkt.write_uint32(self.quantity);
        pkt.write_uint8(self.item_context);
        pkt.write_int32(self.bonus_list_ids.len() as i32);
        for &bid in &self.bonus_list_ids {
            pkt.write_int32(bid);
        }
        pkt.write_bit(self.can_loot);
        pkt.write_bit(false); // is_over_threshold
        pkt.write_bit(false); // is_already_looted
        pkt.write_bit(false); // allow_loot_list_access (PvP loot roll)
        pkt.write_bit(false); // needs_quest
        pkt.write_bit(false); // is_clan_roll
        pkt.write_bit(false); // unused
        pkt.write_bit(false); // is_encounter_loot
        pkt.flush_bits();
    }
}

// ── LootResponse (SMSG_LOOT_RESPONSE) ────────────────────────────

/// Server sends loot window contents to the client.
#[derive(Debug, Clone)]
pub struct LootResponse {
    pub owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub failure_reason: u8, // 0 = no error, 1 = already looted, etc.
    pub acquire_reason: u8,
    pub loot_method: u8,
    pub threshold: u8,
    pub coins: u32,
    pub items: Vec<LootItemData>,
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
        pkt.write_int32(self.items.len() as i32);
        pkt.write_int32(0i32); // currencies count
        pkt.write_bit(self.acquired);
        pkt.write_bit(self.ae_looting);
        pkt.flush_bits();
        for item in &self.items {
            item.write(pkt);
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

// ── SLootRelease (SMSG_LOOT_RELEASE) ─────────────────────────────

/// Server acknowledges loot window close.
#[derive(Debug, Clone)]
pub struct SLootRelease {
    pub unit: ObjectGuid,
    pub loot_obj: ObjectGuid,
}

impl ServerPacket for SLootRelease {
    const OPCODE: ServerOpcodes = ServerOpcodes::LootRelease;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit);
        pkt.write_packed_guid(&self.loot_obj);
    }
}

// ── In-memory loot tracking ──────────────────────────────────────

/// Server-side loot state for one dead creature.
#[derive(Debug, Clone)]
pub struct CreatureLoot {
    pub loot_guid: ObjectGuid,
    pub coins: u32,
    pub items: Vec<LootEntry>,
    pub looted_by_player: bool,
}

#[derive(Debug, Clone)]
pub struct LootEntry {
    pub loot_list_id: u8,
    pub item_id: u32,
    pub quantity: u32,
    pub taken: bool,
}
