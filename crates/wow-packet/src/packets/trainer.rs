// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trainer packets: CMSG_TRAINER_LIST, CMSG_TRAINER_BUY_SPELL, SMSG_TRAINER_LIST,
//! SMSG_TRAINER_BUY_FAILED, SMSG_LEARNED_SPELLS, SMSG_SUPERCEDED_SPELLS,
//! SMSG_UNLEARNED_SPELLS.
//!
//! C# reference: Game/Networking/Packets/NPCPackets.cs, SpellPackets.cs
//! Handler ref:  Game/Handlers/NPCHandler.cs

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::world_packet::PacketError;
use crate::{ClientPacket, ServerPacket, WorldPacket};

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
        Ok(Self {
            trainer_guid,
            trainer_id,
            spell_id,
        })
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
/// ```text
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
    pub superceded: Option<i32>,
}

/// SMSG_LEARNED_SPELLS — sent after a player learns one or more spells.
///
/// C# write order (SpellPackets.cs LearnedSpells):
/// ```text
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
            spells: vec![LearnedSpellEntry {
                spell_id,
                superceded: None,
            }],
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
            write_learned_spell_info_like_cpp(pkt, spell.spell_id, spell.superceded);
        }
    }
}

// ── SMSG_SUPERCEDED_SPELLS (0x2c49) ─────────────────────────────────

/// SMSG_SUPERCEDED_SPELLS — sent when a lower rank replaces an unlearned spell.
///
/// C++ `Player::SendSupercededSpell(oldSpell, newSpell)` writes one
/// `LearnedSpellInfo` with `SpellID = newSpell` and `Superceded = oldSpell`.
pub struct SupercededSpells {
    pub spells: Vec<LearnedSpellEntry>,
}

impl SupercededSpells {
    pub fn single(old_spell_id: i32, new_spell_id: i32) -> Self {
        Self {
            spells: vec![LearnedSpellEntry {
                spell_id: new_spell_id,
                superceded: Some(old_spell_id),
            }],
        }
    }
}

impl ServerPacket for SupercededSpells {
    const OPCODE: ServerOpcodes = ServerOpcodes::SupercededSpells;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.spells.len() as u32);
        for spell in &self.spells {
            write_learned_spell_info_like_cpp(pkt, spell.spell_id, spell.superceded);
        }
    }
}

// ── SMSG_UNLEARNED_SPELLS (0x2c4b) ──────────────────────────────────

/// SMSG_UNLEARNED_SPELLS — sent after a player unlearns one or more spells.
///
/// C++ write order (SpellPackets.cpp UnlearnedSpells):
/// ```text
/// WriteUInt32(SpellID.Count)
/// foreach spell:
///   WriteUInt32(spellId)
/// WriteBit(SuppressMessaging)
/// FlushBits()
/// ```
pub struct UnlearnedSpells {
    pub spell_ids: Vec<u32>,
    pub suppress_messaging: bool,
}

impl UnlearnedSpells {
    pub fn single(spell_id: u32, suppress_messaging: bool) -> Self {
        Self {
            spell_ids: vec![spell_id],
            suppress_messaging,
        }
    }
}

impl ServerPacket for UnlearnedSpells {
    const OPCODE: ServerOpcodes = ServerOpcodes::UnlearnedSpells;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.spell_ids.len() as u32);
        for spell_id in &self.spell_ids {
            pkt.write_uint32(*spell_id);
        }
        pkt.write_bit(self.suppress_messaging);
        pkt.flush_bits();
    }
}

fn write_learned_spell_info_like_cpp(
    pkt: &mut WorldPacket,
    spell_id: i32,
    superceded: Option<i32>,
) {
    pkt.write_int32(spell_id);
    pkt.write_bit(false); // IsFavorite
    pkt.write_bit(false); // field_8.HasValue
    pkt.write_bit(superceded.is_some());
    pkt.write_bit(false); // TraitDefinitionID.HasValue
    pkt.flush_bits();

    if let Some(old_spell_id) = superceded {
        pkt.write_int32(old_spell_id);
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
        assert!(
            bytes.len() >= 12,
            "TrainerListPacket too small: {} bytes",
            bytes.len()
        );
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

    #[test]
    fn superceded_spells_single_serializes_like_cpp() {
        let pkt = SupercededSpells::single(200, 100);
        let mut bytes = WorldPacket::from_bytes(&pkt.to_bytes());

        assert_eq!(
            bytes.read_uint16().expect("opcode"),
            ServerOpcodes::SupercededSpells as u16
        );
        assert_eq!(bytes.read_uint32().expect("count"), 1);
        assert_eq!(bytes.read_int32().expect("new spell id"), 100);
        assert!(!bytes.read_bit().expect("IsFavorite"));
        assert!(!bytes.read_bit().expect("field_8.HasValue"));
        assert!(bytes.read_bit().expect("Superceded.HasValue"));
        assert!(!bytes.read_bit().expect("TraitDefinitionID.HasValue"));
        bytes.flush_bits();
        assert_eq!(bytes.read_int32().expect("old spell id"), 200);
        assert!(bytes.is_empty());
    }

    #[test]
    fn unlearned_spells_single_serializes_like_cpp() {
        let pkt = UnlearnedSpells::single(100, true);
        let mut bytes = WorldPacket::from_bytes(&pkt.to_bytes());

        assert_eq!(
            bytes.read_uint16().expect("opcode"),
            ServerOpcodes::UnlearnedSpells as u16
        );
        assert_eq!(bytes.read_uint32().expect("count"), 1);
        assert_eq!(bytes.read_uint32().expect("spell id"), 100);
        assert!(bytes.read_bit().expect("SuppressMessaging"));
        bytes.flush_bits();
        assert!(bytes.is_empty());
    }
}
