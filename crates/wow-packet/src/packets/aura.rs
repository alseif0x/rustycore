// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Aura system packets — SMSG_AURA_UPDATE.
//!
//! Handles aura application, updates, and removal.

use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;

use crate::world_packet::WorldPacket;
use crate::ServerPacket;

// ── AuraData ──────────────────────────────────────────────────

/// A single aura entry: slot, spell, duration, stacks, etc.
#[derive(Debug, Clone)]
pub struct AuraData {
    /// Aura slot (0-254)
    pub slot: u8,
    /// Spell ID
    pub spell_id: i32,
    /// Aura flags bitmask: has_effect0/1/2, negative, cant_cancel, passive
    pub aura_flags: u32,
    /// Total duration in milliseconds
    pub duration_total: u32,
    /// Remaining duration countdown in milliseconds
    pub duration_remaining: u32,
    /// Stack count
    pub stack_count: u8,
    /// Caster's GUID
    pub caster_guid: ObjectGuid,
    /// Optional effect bonuses
    pub effect_data: Option<Vec<u32>>,
}

impl AuraData {
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.slot);
        pkt.write_int32(self.spell_id);
        pkt.write_uint32(self.aura_flags);
        pkt.write_uint32(self.duration_total);
        pkt.write_uint32(self.duration_remaining);
        pkt.write_uint8(self.stack_count);
        pkt.write_packed_guid(&self.caster_guid);

        // Optional effect data
        if let Some(ref effects) = self.effect_data {
            pkt.write_uint32(effects.len() as u32);
            for &eff in effects {
                pkt.write_uint32(eff);
            }
        } else {
            pkt.write_uint32(0);
        }
    }
}

// ── AuraUpdate (SMSG_AURA_UPDATE) ─────────────────────────────

/// Server notifies client of aura changes: additions and removals.
#[derive(Debug, Clone)]
pub struct AuraUpdate {
    /// Target's GUID
    pub target_guid: ObjectGuid,
    /// Updated/added auras
    pub updated_auras: Vec<AuraData>,
    /// Removed aura slots (just slot numbers)
    pub removed_aura_slots: Vec<u8>,
}

impl ServerPacket for AuraUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuraUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        // Write target GUID
        pkt.write_packed_guid(&self.target_guid);

        // Write count and updated auras
        pkt.write_uint32(self.updated_auras.len() as u32);
        for aura in &self.updated_auras {
            aura.write(pkt);
        }

        // Write count and removed slot indices
        pkt.write_uint32(self.removed_aura_slots.len() as u32);
        for &slot in &self.removed_aura_slots {
            pkt.write_uint8(slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aura_update_write() {
        let aura = AuraData {
            slot: 0,
            spell_id: 1234,
            aura_flags: 0x00000001,
            duration_total: 30000,
            duration_remaining: 30000,
            stack_count: 1,
            caster_guid: ObjectGuid::EMPTY,
            effect_data: None,
        };

        let update = AuraUpdate {
            target_guid: ObjectGuid::EMPTY,
            updated_auras: vec![aura],
            removed_aura_slots: vec![],
        };

        let mut pkt = WorldPacket::new_server(ServerOpcodes::AuraUpdate);
        update.write(&mut pkt);

        assert!(pkt.size() > 4, "Packet should have data beyond opcode");
    }
}
