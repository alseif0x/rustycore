// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trainer packets: CMSG_TRAINER_LIST, CMSG_TRAINER_BUY_SPELL, SMSG_TRAINER_LIST,
//! SMSG_TRAINER_BUY_FAILED, SMSG_LEARNED_SPELLS.
//!
//! C# reference: Game/Networking/Packets/NPCPackets.cs, SpellPackets.cs
//! Handler ref:  Game/Handlers/NPCHandler.cs

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::{ClientPacket, ServerPacket, WorldPacket};
use crate::world_packet::PacketError;

// ── CMSG_TRAINER_LIST (0x34ad) ───────────────────────────────────────

/// Client requests the trainer spell list. Sent when opening a trainer window.
pub struct TrainerListRequest {
    pub trainer_guid: ObjectGuid,
}

impl ClientPacket for TrainerListRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::TrainerList;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let trainer_guid = packet.read_packed_guid()?;
        Ok(Self { trainer_guid })
    }
}

// ── CMSG_TRAINER_BUY_SPELL (0x34ae) ─────────────────────────────────

/// Client requests to buy a spell from a trainer.
pub struct TrainerBuySpellRequest {
    pub trainer_guid: ObjectGuid,
    pub trainer_id: i32,
    pub spell_id: i32,
}

impl ClientPacket for TrainerBuySpellRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::TrainerBuySpell;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let trainer_guid = packet.read_packed_guid()?;
        let trainer_id = packet.read_int32()?;
        let spell_id = packet.read_int32()?;
        Ok(Self { trainer_guid, trainer_id, spell_id })
    }
}

// ── SMSG_TRAINER_LIST (0x26df) ───────────────────────────────────────

/// A single spell entry in the trainer list.
pub struct TrainerListSpell {
    pub spell_id: i32,
    pub money_cost: u32,
    pub req_skill_line: i32,
    pub req_skill_rank: i32,
    /// Up to 3 required abilities (MaxTrainerspellAbilityReqs = 3).
    pub req_ability: [i32; 3],
    /// 0 = unavailable, 1 = available (but not known), 2 = known.
    pub usable: u8,
    pub req_level: u8,
}

/// SMSG_TRAINER_LIST — full trainer spell list response.
///
/// C# write order:
/// ```
/// WritePackedGuid(TrainerGUID)
/// WriteInt32(TrainerType)
/// WriteInt32(TrainerID)
/// WriteInt32(Spells.Count)
/// foreach spell: { SpellID, MoneyCost, ReqSkillLine, ReqSkillRank, ReqAbility[0..3], Usable, ReqLevel }
/// WriteBits(Greeting.len(), 11)
/// FlushBits()
/// WriteString(Greeting)
/// ```
pub struct TrainerListPacket {
    pub trainer_guid: ObjectGuid,
    /// 0 = class trainer, 2 = pet trainer, etc.
    pub trainer_type: i32,
    pub trainer_id: i32,
    pub spells: Vec<TrainerListSpell>,
    pub greeting: String,
}

impl ServerPacket for TrainerListPacket {
    const OPCODE: ServerOpcodes = ServerOpcodes::TrainerList;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.trainer_guid);
        pkt.write_int32(self.trainer_type);
        pkt.write_int32(self.trainer_id);
        pkt.write_int32(self.spells.len() as i32);

        for spell in &self.spells {
            pkt.write_int32(spell.spell_id);
            pkt.write_uint32(spell.money_cost);
            pkt.write_int32(spell.req_skill_line);
            pkt.write_int32(spell.req_skill_rank);
            for &req_ab in &spell.req_ability {
                pkt.write_int32(req_ab);
            }
            pkt.write_uint8(spell.usable);
            pkt.write_uint8(spell.req_level);
        }

        pkt.write_bits(self.greeting.len() as u32, 11);
        pkt.flush_bits();
        pkt.write_string(&self.greeting);
    }
}

// ── SMSG_TRAINER_BUY_FAILED (0x26e0) ────────────────────────────────

/// Server response when a trainer purchase fails.
///
/// reason: 0 = "Trainer service unavailable.", 1 = "Not enough money."
pub struct TrainerBuyFailed {
    pub trainer_guid: ObjectGuid,
    pub spell_id: i32,
    pub reason: i32,
}

impl ServerPacket for TrainerBuyFailed {
    const OPCODE: ServerOpcodes = ServerOpcodes::TrainerBuyFailed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.trainer_guid);
        pkt.write_int32(self.spell_id);
        pkt.write_int32(self.reason);
    }
}

// ── SMSG_LEARNED_SPELLS (0x2c4a) ────────────────────────────────────

/// A single entry in the LearnedSpells packet.
pub struct LearnedSpellEntry {
    pub spell_id: i32,
}

/// SMSG_LEARNED_SPELLS — sent after a player learns one or more spells.
///
/// C# write order (SpellPackets.cs LearnedSpells):
/// ```
/// WriteInt32(ClientLearnedSpellData.Count)
/// WriteUInt32(SpecializationID)
/// WriteBit(SuppressMessaging)
/// FlushBits()
/// foreach spell:
///   WriteInt32(SpellID)
///   WriteBit(IsFavorite)      // false
///   WriteBit(field_8.HasValue) // false
///   WriteBit(Superceded.HasValue) // false
///   WriteBit(TraitDefinitionID.HasValue) // false
///   FlushBits()
/// ```
pub struct LearnedSpells {
    pub spells: Vec<LearnedSpellEntry>,
    pub suppress_messaging: bool,
}

impl LearnedSpells {
    /// Create a packet that tells the client about one newly learned spell.
    pub fn single(spell_id: i32) -> Self {
        Self {
            spells: vec![LearnedSpellEntry { spell_id }],
            suppress_messaging: false,
        }
    }
}

impl ServerPacket for LearnedSpells {
    const OPCODE: ServerOpcodes = ServerOpcodes::LearnedSpells;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.spells.len() as i32);
        pkt.write_uint32(0); // SpecializationID = 0
        pkt.write_bit(self.suppress_messaging);
        pkt.flush_bits();

        for spell in &self.spells {
            pkt.write_int32(spell.spell_id);
            pkt.write_bit(false); // IsFavorite
            pkt.write_bit(false); // field_8.HasValue
            pkt.write_bit(false); // Superceded.HasValue
            pkt.write_bit(false); // TraitDefinitionID.HasValue
            pkt.flush_bits();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trainer_list_empty_serializes() {
        let pkt = TrainerListPacket {
            trainer_guid: ObjectGuid::EMPTY,
            trainer_type: 0,
            trainer_id: 1,
            spells: vec![],
            greeting: String::new(),
        };
        let bytes = pkt.to_bytes();
        // At minimum: opcode(2) + packed_guid + i32*3 + bits(11) + flush + empty string
        assert!(bytes.len() >= 12, "TrainerListPacket too small: {} bytes", bytes.len());
    }

    #[test]
    fn trainer_buy_failed_serializes() {
        let pkt = TrainerBuyFailed {
            trainer_guid: ObjectGuid::EMPTY,
            spell_id: 12345,
            reason: 1,
        };
        let bytes = pkt.to_bytes();
        assert!(bytes.len() > 4);
    }

    #[test]
    fn learned_spells_single_serializes() {
        let pkt = LearnedSpells::single(100);
        let bytes = pkt.to_bytes();
        // opcode(2) + count(4) + spec_id(4) + bit/flush(1) + spell_id(4) + bits/flush(1)
        assert!(bytes.len() >= 14);
    }
}
