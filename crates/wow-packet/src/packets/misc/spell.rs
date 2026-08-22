// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell, aura and talent packets.

use super::*;

/// Response to GetUndeleteCharacterCooldownStatus.
/// Tells the client whether character undelete is on cooldown.
pub struct UndeleteCooldownStatusResponse {
    pub on_cooldown: bool,
    pub max_cooldown: i32,
    pub current_cooldown: i32,
}

impl UndeleteCooldownStatusResponse {
    /// No cooldown — character undelete is available.
    pub fn no_cooldown() -> Self {
        Self {
            on_cooldown: false,
            max_cooldown: 0,
            current_cooldown: 0,
        }
    }
}

impl ServerPacket for UndeleteCooldownStatusResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::UndeleteCooldownStatusResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.on_cooldown);
        pkt.write_int32(self.max_cooldown);
        pkt.write_int32(self.current_cooldown);
    }
}

// ── ServerTimeOffset (SMSG 0x2714) ───────────────────────────────────

/// C++ `WorldPackets::Talent::TalentInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TalentInfoLikeCpp {
    pub talent_id: u32,
    pub rank: u8,
}

/// C++ `WorldPackets::Talent::UpdateTalentData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTalentData {
    pub unspent_talent_points: u32,
    pub active_group: u8,
    pub groups: Vec<TalentGroupInfoLikeCpp>,
    pub is_pet_talents: bool,
}

impl Default for UpdateTalentData {
    fn default() -> Self {
        Self {
            unspent_talent_points: 0,
            active_group: 0,
            groups: vec![TalentGroupInfoLikeCpp::default()],
            is_pet_talents: false,
        }
    }
}

impl ServerPacket for UpdateTalentData {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateTalentData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.unspent_talent_points); // UnspentTalentPoints
        pkt.write_uint8(self.active_group); // ActiveGroup
        pkt.write_uint32(self.groups.len() as u32); // TalentGroupInfos.Count

        for group in &self.groups {
            pkt.write_uint8(group.talents.len() as u8);
            pkt.write_uint32(group.talents.len() as u32);
            pkt.write_uint8(group.glyph_ids.len() as u8);
            pkt.write_uint32(group.glyph_ids.len() as u32);
            pkt.write_uint8(group.spec_id);

            for talent in &group.talents {
                pkt.write_uint32(talent.talent_id);
                pkt.write_uint8(talent.rank);
            }

            for glyph_id in group.glyph_ids {
                pkt.write_uint16(glyph_id);
            }
        }

        pkt.write_bit(self.is_pet_talents);
    }
}

// ── SendKnownSpells (SMSG 0x2c27) ──────────────────────────────────

/// Known spells list sent during login.
///
/// TrinityCore C++ `WorldPackets::Spells::SendKnownSpells::Write` format:
/// ```text
/// [bit]  InitialLogin
/// [i32]  KnownSpells.Count
/// [i32]  FavoriteSpells.Count
/// [i32 × N] KnownSpells (spell IDs)
/// [i32 × M] FavoriteSpells (spell IDs)
/// ```
pub struct SendKnownSpells {
    pub initial_login: bool,
    pub known_spells: Vec<i32>,
    pub favorite_spells: Vec<i32>,
}

impl SendKnownSpells {
    /// Empty spell list for fresh characters.
    pub fn empty() -> Self {
        Self {
            initial_login: true,
            known_spells: Vec::new(),
            favorite_spells: Vec::new(),
        }
    }
}

impl ServerPacket for SendKnownSpells {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendKnownSpells;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.initial_login);
        pkt.write_int32(self.known_spells.len() as i32);
        pkt.write_int32(self.favorite_spells.len() as i32);
        for &spell_id in &self.known_spells {
            pkt.write_int32(spell_id);
        }
        for &spell_id in &self.favorite_spells {
            pkt.write_int32(spell_id);
        }
    }
}

// ── SendUnlearnSpells (SMSG 0x2c2b) ────────────────────────────────

/// Unlearned spells list. Empty for fresh characters.
pub struct SendUnlearnSpells;

impl ServerPacket for SendUnlearnSpells {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendUnlearnSpells;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Spells.Count
    }
}

// ── SendSpellHistory (SMSG 0x2c28) ──────────────────────────────────

/// One C++ `WorldPackets::Spells::SpellHistoryEntry`.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellHistoryEntry {
    pub spell_id: u32,
    pub item_id: u32,
    pub category: u32,
    pub recovery_time_ms: i32,
    pub category_recovery_time_ms: i32,
    pub mod_rate: f32,
    pub on_hold: bool,
}

/// Spell cooldown history.
pub struct SendSpellHistory {
    pub entries: Vec<SpellHistoryEntry>,
}

impl SendSpellHistory {
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl ServerPacket for SendSpellHistory {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendSpellHistory;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.entries.len() as u32);
        for entry in &self.entries {
            pkt.write_uint32(entry.spell_id);
            pkt.write_uint32(entry.item_id);
            pkt.write_uint32(entry.category);
            pkt.write_int32(entry.recovery_time_ms);
            pkt.write_int32(entry.category_recovery_time_ms);
            pkt.write_float(entry.mod_rate);
            pkt.write_bit(false); // unused622_1
            pkt.write_bit(false); // unused622_2
            pkt.write_bit(entry.on_hold);
            pkt.flush_bits();
        }
    }
}

// ── SendSpellCharges (SMSG 0x2c2a) ──────────────────────────────────

/// One C++ `WorldPackets::Spells::SpellChargeEntry`.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellChargeEntry {
    pub category: u32,
    pub next_recovery_time_ms: u32,
    pub charge_mod_rate: f32,
    pub consumed_charges: u8,
}

/// Spell charges.
pub struct SendSpellCharges {
    pub entries: Vec<SpellChargeEntry>,
}

impl SendSpellCharges {
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl ServerPacket for SendSpellCharges {
    const OPCODE: ServerOpcodes = ServerOpcodes::SendSpellCharges;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.entries.len() as u32);
        for entry in &self.entries {
            pkt.write_uint32(entry.category);
            pkt.write_uint32(entry.next_recovery_time_ms);
            pkt.write_float(entry.charge_mod_rate);
            pkt.write_uint8(entry.consumed_charges);
        }
    }
}

// ── UpdateActionButtons (SMSG 0x25e0) ───────────────────────────────

/// C++ `WorldPackets::Talent::GlyphBinding`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphBindingLikeCpp {
    pub spell_id: u32,
    pub glyph_id: u16,
}

/// C++ `WorldPackets::Talent::ActiveGlyphs`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveGlyphs {
    pub glyphs: Vec<GlyphBindingLikeCpp>,
    pub is_full_update: bool,
}

impl ServerPacket for ActiveGlyphs {
    const OPCODE: ServerOpcodes = ServerOpcodes::ActiveGlyphs;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.glyphs.len() as u32);
        for glyph in &self.glyphs {
            pkt.write_uint32(glyph.spell_id);
            pkt.write_uint16(glyph.glyph_id);
        }
        pkt.write_bit(self.is_full_update);
        pkt.flush_bits();
    }
}

// ── LoadEquipmentSet (SMSG 0x270e) ───────────────────────────────────

/// C++ `WorldPackets::Spells::AuraDataInfo`.
#[derive(Debug, Clone)]
pub struct AuraDataInfoLikeCpp {
    pub cast_id: ObjectGuid,
    pub spell_id: i32,
    pub flags: u16,
    pub active_flags: u32,
    pub caster_guid: ObjectGuid,
    pub cast_level: u16,
    pub applications: u8,
    pub duration_ms: Option<u32>,
    pub remaining_ms: Option<u32>,
    pub points: Vec<f32>,
}

impl AuraDataInfoLikeCpp {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.cast_id);
        pkt.write_int32(self.spell_id);
        pkt.write_int32(0); // SpellCastVisual::SpellXSpellVisualID
        pkt.write_uint16(self.flags);
        pkt.write_uint32(self.active_flags);
        pkt.write_uint16(self.cast_level);
        pkt.write_uint8(self.applications);
        pkt.write_int32(0); // ContentTuningID
        let has_cast_unit = self.flags & 0x0001 == 0;
        pkt.write_bit(has_cast_unit); // CastUnit
        pkt.write_bit(self.duration_ms.is_some()); // Duration
        pkt.write_bit(self.remaining_ms.is_some()); // Remaining
        pkt.write_bit(false); // TimeMod
        pkt.write_bits(self.points.len().min(usize::from(u8::MAX)) as u32, 6);
        pkt.write_bits(0, 6); // EstimatedPoints
        pkt.write_bit(false); // ContentTuning
        if has_cast_unit {
            pkt.write_packed_guid(&self.caster_guid);
        }
        if let Some(duration_ms) = self.duration_ms {
            pkt.write_uint32(duration_ms);
        }
        if let Some(remaining_ms) = self.remaining_ms {
            pkt.write_uint32(remaining_ms);
        }
        for point in &self.points {
            pkt.write_float(*point);
        }
    }
}

/// C++ `WorldPackets::Spells::AuraInfo`.
#[derive(Debug, Clone)]
pub struct AuraInfoLikeCpp {
    pub slot: u8,
    pub aura_data: Option<AuraDataInfoLikeCpp>,
}

impl AuraInfoLikeCpp {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.slot);
        pkt.write_bit(self.aura_data.is_some());
        pkt.flush_bits();
        if let Some(aura_data) = &self.aura_data {
            aura_data.write(pkt);
        }
    }
}

/// C++ `WorldPackets::Spells::AuraUpdate`.
pub struct AuraUpdate {
    pub unit_guid: ObjectGuid,
    pub update_all: bool,
    pub auras: Vec<AuraInfoLikeCpp>,
}

impl AuraUpdate {
    /// Full aura update with no auras.
    pub fn empty_for(guid: ObjectGuid) -> Self {
        Self {
            unit_guid: guid,
            update_all: true,
            auras: Vec::new(),
        }
    }

    pub fn full_for(guid: ObjectGuid, auras: Vec<AuraInfoLikeCpp>) -> Self {
        Self {
            unit_guid: guid,
            update_all: true,
            auras,
        }
    }
}

impl ServerPacket for AuraUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuraUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.update_all);
        pkt.write_bits(self.auras.len().min(0x1FF) as u32, 9);
        for aura in &self.auras {
            aura.write(pkt);
        }
        pkt.write_packed_guid(&self.unit_guid);
    }
}

// ── Battle pet journal lock packets ─────────────────────────────────

/// Spell modifier data: empty for fresh characters with no talents/auras.
///
/// The same struct is used for both `SetFlatSpellModifier` (0x2c33) and
/// `SetPctSpellModifier` (0x2c34) — only the opcode differs.
///
/// C++ `WorldPackets::Spells::SetSpellModifier` format:
/// ```text
/// [i32] Modifiers.Count
/// for each SpellModifierInfo:
///     [u8]  ModIndex
///     [i32] ModifierData.Count
///     for each SpellModifierData:
///         [f32] ModifierValue
///         [u8]  ClassIndex
/// ```
pub struct SetSpellModifier {
    /// Which opcode to use (Flat or Pct).
    opcode: ServerOpcodes,
}

impl SetSpellModifier {
    /// Empty flat spell modifiers (no modifier entries).
    pub fn flat_empty() -> Self {
        Self {
            opcode: ServerOpcodes::SetFlatSpellModifier,
        }
    }

    /// Empty percent spell modifiers (no modifier entries).
    pub fn pct_empty() -> Self {
        Self {
            opcode: ServerOpcodes::SetPctSpellModifier,
        }
    }

    /// Build the packet bytes (custom opcode, can't use the trait const).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut pkt = WorldPacket::new_server(self.opcode);
        pkt.write_int32(0); // Modifiers.Count = 0
        pkt.data().to_vec()
    }
}

// ── SetProficiency (SMSG 0x2735) ───────────────────────────────────
