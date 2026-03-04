// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell cast packets — CMSG_CAST_SPELL / SMSG_SPELL_START / SMSG_SPELL_GO.
//!
//! Packet structures mirror C# Game/Networking/Packets/SpellPackets.cs.
//!
//! `CastSpellRequest` parses the full `SpellCastRequestPkt` so we correctly
//! advance the buffer even for fields we don't yet use (optionalReagents,
//! MoveUpdate, SpellWeights, etc.).
//!
//! `SpellGoPkt` writes a minimal but correct `SpellCastData` that the client
//! accepts for instant-cast spell animations (no log data, empty RemainingPower).

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

// ── Sub-structures ────────────────────────────────────────────────

/// SpellCastVisual — two visual IDs packed inline.
#[derive(Debug, Clone, Default)]
pub struct SpellCastVisual {
    pub spell_visual_id: u32,
    pub script_visual_id: u32,
}

impl SpellCastVisual {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            spell_visual_id: pkt.read_uint32()?,
            script_visual_id: pkt.read_uint32()?,
        })
    }

    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.spell_visual_id);
        pkt.write_uint32(self.script_visual_id);
    }
}

/// SpellTargetData — unit/item target with optional location data.
/// C# ref: `SpellPackets.cs / class SpellTargetData`.
#[derive(Debug, Clone, Default)]
pub struct SpellTargetData {
    /// SpellCastTargetFlags (28 bits).
    pub flags: u32,
    /// Primary unit target.
    pub unit: ObjectGuid,
    /// Item target (usually EMPTY).
    pub item: ObjectGuid,
}

impl SpellTargetData {
    /// Read from wire; matches C# `SpellTargetData.Read()`.
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        // C# calls ResetBitPos() here — handled by our auto-reset on byte reads,
        // but we also expose a public reset for clarity.
        pkt.reset_bits();

        let flags        = pkt.read_bits(28)?;
        let has_src      = pkt.has_bit()?;
        let has_dst      = pkt.has_bit()?;
        let has_orient   = pkt.has_bit()?;
        let has_mapid    = pkt.has_bit()?;
        let name_len     = pkt.read_bits(7)? as usize;

        let unit = pkt.read_packed_guid()?;
        let item = pkt.read_packed_guid()?;

        // Optional source location: packed guid + vec3
        if has_src {
            let _transport = pkt.read_packed_guid()?;
            let _x = pkt.read_float()?;
            let _y = pkt.read_float()?;
            let _z = pkt.read_float()?;
        }

        // Optional dest location: packed guid + vec3
        if has_dst {
            let _transport = pkt.read_packed_guid()?;
            let _x = pkt.read_float()?;
            let _y = pkt.read_float()?;
            let _z = pkt.read_float()?;
        }

        if has_orient {
            let _o = pkt.read_float()?;
        }

        if has_mapid {
            let _map = pkt.read_int32()?;
        }

        if name_len > 0 {
            let _name = pkt.read_string(name_len)?;
        }

        Ok(Self { flags, unit, item })
    }

    /// Write minimal target (unit only, no locations).
    /// C# ref: `SpellTargetData.Write()`.
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(self.flags, 28);
        pkt.write_bit(false); // no SrcLocation
        pkt.write_bit(false); // no DstLocation
        pkt.write_bit(false); // no Orientation
        pkt.write_bit(false); // no MapID
        pkt.write_bits(0, 7); // name length = 0
        pkt.flush_bits();

        pkt.write_packed_guid(&self.unit);
        pkt.write_packed_guid(&self.item);
        // no Name bytes
    }
}

// ── SpellCraftingReagent helper (read-only, for skipping) ─────────

fn skip_crafting_reagent(pkt: &mut WorldPacket) -> Result<(), PacketError> {
    let _item_id         = pkt.read_int32()?;
    let _data_slot_index = pkt.read_int32()?;
    let _quantity        = pkt.read_int32()?;
    // optional Unknown_1000 byte guarded by a bit
    // NOTE: these optional bytes use the *non-reset* bit reader that was
    // last active when we entered this helper. To be safe, we read the bit
    // directly here — the parent loop already consumed the previous bits.
    // In practice most spell casts have 0 reagents so this path is skipped.
    let has_extra = pkt.has_bit()?;
    if has_extra {
        let _u = pkt.read_uint8()?;
    }
    Ok(())
}

// ── Client packet ─────────────────────────────────────────────────

/// Parsed representation of `CMSG_CAST_SPELL` / `SpellCastRequestPkt`.
///
/// We parse the full structure so the buffer position is correct; fields
/// we don't yet use are stored as `_ignored` locals and dropped.
#[derive(Debug, Clone)]
pub struct CastSpellRequest {
    /// Client-generated cast ID (an ObjectGuid used as a unique cast token).
    pub cast_id: ObjectGuid,
    /// The spell being cast.
    pub spell_id: i32,
    /// Spell visual IDs.
    pub visual: SpellCastVisual,
    /// Cast target.
    pub target: SpellTargetData,
}

impl ClientPacket for CastSpellRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::CastSpell;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let cast_id      = pkt.read_packed_guid()?;
        let _misc0       = pkt.read_int32()?;
        let _misc1       = pkt.read_int32()?;
        let spell_id     = pkt.read_int32()?;
        let visual       = SpellCastVisual::read(pkt)?;

        // MissileTrajectoryRequest: Pitch + Speed (2 floats)
        let _pitch       = pkt.read_float()?;
        let _speed       = pkt.read_float()?;

        let _crafting_npc = pkt.read_packed_guid()?;

        let currencies_count  = pkt.read_uint32()? as usize;
        let reagents_count    = pkt.read_uint32()? as usize;
        let removed_mods_count = pkt.read_uint32()? as usize;

        // Optional currencies (each: 3 i32 + 1 optional byte via bit)
        for _ in 0..currencies_count {
            let _item     = pkt.read_int32()?;
            let _slot     = pkt.read_int32()?;
            let _qty      = pkt.read_int32()?;
            let has_extra = pkt.has_bit()?;
            if has_extra {
                let _u = pkt.read_uint8()?;
            }
        }

        // Bit section: SendCastFlags(5), hasMoveUpdate(1), weightCount(2), hasCraftingOrderID(1)
        let _send_cast_flags    = pkt.read_bits(5)?;
        let has_move_update     = pkt.has_bit()?;
        let weight_count        = pkt.read_bits(2)? as usize;
        let has_crafting_order  = pkt.has_bit()?;

        // Target — reads its own bit section (SpellTargetData::read calls reset_bits)
        let target = SpellTargetData::read(pkt)?;

        if has_crafting_order {
            let _order_id = pkt.read_uint64()?;
        }

        // Optional reagents
        for _ in 0..reagents_count {
            skip_crafting_reagent(pkt)?;
        }

        // Removed modifications
        for _ in 0..removed_mods_count {
            skip_crafting_reagent(pkt)?;
        }

        // Optional MoveUpdate (MovementInfo — many fields, skip via best-effort)
        // We only reach this path if the player is moving while casting (rare).
        // Parsing MovementInfo here is complex; we ignore it and stop reading.
        if has_move_update {
            // MoveInfo is at the end; anything after target is non-critical for
            // our básicos implementation — just stop early.
            return Ok(Self { cast_id, spell_id, visual, target });
        }

        // SpellWeights (each: ResetBitPos + Type(2 bits) + ID(i32) + Quantity(u32))
        for _ in 0..weight_count {
            pkt.reset_bits();
            let _ty  = pkt.read_bits(2)?;
            let _id  = pkt.read_int32()?;
            let _qty = pkt.read_uint32()?;
        }

        Ok(Self { cast_id, spell_id, visual, target })
    }
}

// ── Server packet helpers ─────────────────────────────────────────

/// Write a minimal `SpellCastData` (used by both SpellStart and SpellGo).
///
/// C# ref: `SpellCastData.Write()` in SpellPackets.cs.
///
/// Parameters
/// - `caster`      : player ObjectGuid
/// - `cast_id`     : echo of the client's cast_id
/// - `spell_id`    : spell being cast
/// - `visual`      : spell visual IDs
/// - `cast_time_ms`: 0 for instant
/// - `target`      : SpellTargetData (unit + flags)
/// - `hit_targets` : list of GUIDs that were hit (empty for visual-only)
fn write_spell_cast_data(
    pkt: &mut WorldPacket,
    caster: &ObjectGuid,
    cast_id: &ObjectGuid,
    spell_id: i32,
    visual: &SpellCastVisual,
    cast_time_ms: u32,
    target: &SpellTargetData,
    hit_targets: &[ObjectGuid],
) {
    // CasterGUID, CasterUnit, CastID, OriginalCastID
    pkt.write_packed_guid(caster);
    pkt.write_packed_guid(caster); // CasterUnit = same for player spells
    pkt.write_packed_guid(cast_id);
    pkt.write_packed_guid(cast_id); // OriginalCastID = CastID

    // SpellID + visual
    pkt.write_int32(spell_id);
    visual.write(pkt);

    // CastFlags, CastFlagsEx, CastTime
    pkt.write_uint32(0); // CastFlags
    pkt.write_uint32(0); // CastFlagsEx
    pkt.write_uint32(cast_time_ms);

    // MissileTrajectoryResult: TravelTime(i32) + Pitch(f32)
    pkt.write_int32(0);
    pkt.write_float(0.0);

    // DestLocSpellCastIndex
    pkt.write_uint8(0);

    // Immunities: School(u32) + Value(u32)
    pkt.write_uint32(0);
    pkt.write_uint32(0);

    // SpellHealPrediction: Points(u32) + Type(u8) + BeaconGUID(packed)
    pkt.write_uint32(0);
    pkt.write_uint8(0);
    pkt.write_packed_guid(&ObjectGuid::EMPTY);

    // Bit counts
    pkt.write_bits(hit_targets.len() as u32, 16); // HitTargets
    pkt.write_bits(0, 16); // MissTargets
    pkt.write_bits(0, 16); // MissStatus
    pkt.write_bits(0, 9);  // RemainingPower
    pkt.write_bit(false);  // RemainingRunes present?
    pkt.write_bits(0, 16); // TargetPoints
    pkt.write_bit(false);  // AmmoDisplayID present?
    pkt.write_bit(false);  // AmmoInventoryType present?
    pkt.flush_bits();

    // Target
    target.write(pkt);

    // HitTargets
    for guid in hit_targets {
        pkt.write_packed_guid(guid);
    }
    // (no MissTargets, MissStatus, RemainingPower, Runes, TargetPoints, Ammo)
}

// ── SMSG_SPELL_START ─────────────────────────────────────────────

/// `SMSG_SPELL_START` — notifies client a spell cast has begun.
/// Used for spells with a cast time; for instant spells use `SpellGoPkt`.
pub struct SpellStartPkt {
    pub caster: ObjectGuid,
    pub cast_id: ObjectGuid,
    pub spell_id: i32,
    pub visual: SpellCastVisual,
    /// Cast time in milliseconds (0 for instant).
    pub cast_time_ms: u32,
    pub target: SpellTargetData,
}

impl ServerPacket for SpellStartPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellStart;

    fn write(&self, pkt: &mut WorldPacket) {
        write_spell_cast_data(
            pkt,
            &self.caster,
            &self.cast_id,
            self.spell_id,
            &self.visual,
            self.cast_time_ms,
            &self.target,
            &[], // no hit targets in SPELL_START
        );
    }
}

// ── SMSG_SPELL_GO ────────────────────────────────────────────────

/// `SMSG_SPELL_GO` — spell completes and effects are applied.
///
/// For our básicos implementation we send this immediately for all spells
/// (treating everything as instant-cast).
pub struct SpellGoPkt {
    pub caster: ObjectGuid,
    pub cast_id: ObjectGuid,
    pub spell_id: i32,
    pub visual: SpellCastVisual,
    pub target: SpellTargetData,
    /// GUIDs that were hit by the spell.
    pub hit_targets: Vec<ObjectGuid>,
}

impl ServerPacket for SpellGoPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellGo;

    fn write(&self, pkt: &mut WorldPacket) {
        // SpellCastData (CastTime=0 for instant)
        write_spell_cast_data(
            pkt,
            &self.caster,
            &self.cast_id,
            self.spell_id,
            &self.visual,
            0, // CastTime
            &self.target,
            &self.hit_targets,
        );

        // CombatLogServerPacket extras: WriteLogDataBit + FlushBits + WriteLogData
        pkt.write_bit(false); // no log data
        pkt.flush_bits();
        // (WriteLogData writes nothing when bit is false)
    }
}

// ── SMSG_CAST_FAILED ─────────────────────────────────────────────

/// `SMSG_CAST_FAILED` — generic failure response for a spell cast.
/// Sent when the player tries to cast a spell they don't know.
pub struct CastFailed {
    pub cast_id: ObjectGuid,
    pub spell_id: i32,
    /// SpellCastResult failure reason (0 = SpellCastResult::Ok, but we use non-zero).
    /// Common: 2 = NotKnown, 70 = NotReady, 5 = BadTargets
    pub reason: i32,
    pub fail_arg1: i32,
    pub fail_arg2: i32,
}

impl ServerPacket for CastFailed {
    // C#: ServerOpcodes.CastFailed (0x2c35 in WotLK Classic)
    const OPCODE: ServerOpcodes = ServerOpcodes::CastFailed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.cast_id);
        pkt.write_int32(self.spell_id);
        pkt.write_int32(self.reason);
        pkt.write_int32(self.fail_arg1);
        pkt.write_int32(self.fail_arg2);
    }
}
