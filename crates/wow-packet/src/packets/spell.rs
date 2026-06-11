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
use wow_core::{ObjectGuid, Position};

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

/// C++ `WorldPackets::Spells::CancelCast`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelCast {
    pub cast_id: ObjectGuid,
    pub spell_id: u32,
}

impl ClientPacket for CancelCast {
    const OPCODE: ClientOpcodes = ClientOpcodes::CancelCast;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let cast_id = pkt.read_packed_guid()?;
        let spell_id = pkt.read_uint32()?;
        Ok(Self { cast_id, spell_id })
    }
}

/// C++ `WorldPackets::Spells::CancelChannelling`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelChannelling {
    pub channel_spell: i32,
    pub reason: i32,
}

impl ClientPacket for CancelChannelling {
    const OPCODE: ClientOpcodes = ClientOpcodes::CancelChannelling;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let channel_spell = pkt.read_int32()?;
        let reason = pkt.read_int32()?;
        Ok(Self {
            channel_spell,
            reason,
        })
    }
}

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

/// C++ `WorldPackets::Spells::PlaySpellVisual`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaySpellVisual {
    pub source: ObjectGuid,
    pub target: ObjectGuid,
    pub transport: ObjectGuid,
    pub target_position: Position,
    pub spell_visual_id: u32,
    pub travel_speed: f32,
    pub hit_reason: u16,
    pub miss_reason: u16,
    pub reflect_status: u16,
    pub launch_delay: f32,
    pub min_duration: f32,
    pub speed_as_time: bool,
}

impl PlaySpellVisual {
    pub fn self_target(
        source: ObjectGuid,
        target_position: Position,
        spell_visual_id: u32,
    ) -> Self {
        Self {
            source,
            target: source,
            transport: ObjectGuid::EMPTY,
            target_position,
            spell_visual_id,
            travel_speed: 0.0,
            hit_reason: 0,
            miss_reason: 0,
            reflect_status: 0,
            launch_delay: 0.0,
            min_duration: 0.0,
            speed_as_time: false,
        }
    }
}

impl ServerPacket for PlaySpellVisual {
    const OPCODE: ServerOpcodes = ServerOpcodes::PlaySpellVisual;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.source.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        for byte in self.target.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        for byte in self.transport.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_float(self.target_position.x);
        pkt.write_float(self.target_position.y);
        pkt.write_float(self.target_position.z);
        pkt.write_uint32(self.spell_visual_id);
        pkt.write_float(self.travel_speed);
        pkt.write_uint16(self.hit_reason);
        pkt.write_uint16(self.miss_reason);
        pkt.write_uint16(self.reflect_status);
        pkt.write_float(self.launch_delay);
        pkt.write_float(self.min_duration);
        pkt.write_bit(self.speed_as_time);
        pkt.flush_bits();
    }
}

/// Spell target location payload: transport GUID followed by XYZ only.
///
/// Trinity carries optional orientation separately in `SpellTargetData` rather
/// than inside this XYZ payload.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TargetLocation {
    pub transport: ObjectGuid,
    pub position: Position,
}

impl TargetLocation {
    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let transport = pkt.read_packed_guid()?;
        let x = pkt.read_float()?;
        let y = pkt.read_float()?;
        let z = pkt.read_float()?;

        Ok(Self {
            transport,
            position: Position::xyz(x, y, z),
        })
    }

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.transport);
        pkt.write_float(self.position.x);
        pkt.write_float(self.position.y);
        pkt.write_float(self.position.z);
    }
}

/// SpellTargetData — unit/item target with optional C++ target data preserved.
/// C++ ref: `WorldPackets::Spells::SpellTargetData`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellTargetData {
    /// SpellCastTargetFlags (28 bits).
    pub flags: u32,
    /// Primary unit target.
    pub unit: ObjectGuid,
    /// Item target (usually EMPTY).
    pub item: ObjectGuid,
    /// Optional source target location.
    pub src_location: Option<TargetLocation>,
    /// Optional destination target location.
    pub dst_location: Option<TargetLocation>,
    /// Optional target orientation, stored separately from XYZ locations.
    pub orientation: Option<f32>,
    /// Optional target map id.
    pub map_id: Option<i32>,
    /// Optional target name payload.
    pub name: String,
}

impl SpellTargetData {
    /// Read from wire; matches C++ `operator>>(ByteBuffer&, SpellTargetData&)`.
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.reset_bits();

        let flags = pkt.read_bits(28)?;
        let has_src = pkt.has_bit()?;
        let has_dst = pkt.has_bit()?;
        let has_orient = pkt.has_bit()?;
        let has_mapid = pkt.has_bit()?;
        let name_len = pkt.read_bits(7)? as usize;

        let unit = pkt.read_packed_guid()?;
        let item = pkt.read_packed_guid()?;

        let src_location = if has_src {
            Some(TargetLocation::read(pkt)?)
        } else {
            None
        };

        let dst_location = if has_dst {
            Some(TargetLocation::read(pkt)?)
        } else {
            None
        };

        let orientation = if has_orient {
            Some(pkt.read_float()?)
        } else {
            None
        };

        let map_id = if has_mapid {
            Some(pkt.read_int32()?)
        } else {
            None
        };

        let name = pkt.read_string(name_len)?;

        Ok(Self {
            flags,
            unit,
            item,
            src_location,
            dst_location,
            orientation,
            map_id,
            name,
        })
    }

    /// Write target data; mirrors C++ `operator<<(ByteBuffer&, SpellTargetData const&)`.
    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(self.flags, 28);
        pkt.write_bit(self.src_location.is_some());
        pkt.write_bit(self.dst_location.is_some());
        pkt.write_bit(self.orientation.is_some());
        pkt.write_bit(self.map_id.is_some());
        pkt.write_bits(self.name.len() as u32, 7);
        pkt.flush_bits();

        pkt.write_packed_guid(&self.unit);
        pkt.write_packed_guid(&self.item);

        if let Some(src_location) = self.src_location {
            src_location.write(pkt);
        }

        if let Some(dst_location) = self.dst_location {
            dst_location.write(pkt);
        }

        if let Some(orientation) = self.orientation {
            pkt.write_float(orientation);
        }

        if let Some(map_id) = self.map_id {
            pkt.write_int32(map_id);
        }

        pkt.write_string(&self.name);
    }
}

// ── SpellCraftingReagent helper (read-only, for skipping) ─────────

fn skip_crafting_reagent(pkt: &mut WorldPacket) -> Result<(), PacketError> {
    let _item_id = pkt.read_int32()?;
    let _data_slot_index = pkt.read_int32()?;
    let _quantity = pkt.read_int32()?;
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
    /// C++ `SpellCastRequest::Misc`; toys use `Misc[0]` as the item id.
    pub misc: [i32; 2],
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
        let cast_id = pkt.read_packed_guid()?;
        let misc0 = pkt.read_int32()?;
        let misc1 = pkt.read_int32()?;
        let spell_id = pkt.read_int32()?;
        let visual = SpellCastVisual::read(pkt)?;

        // MissileTrajectoryRequest: Pitch + Speed (2 floats)
        let _pitch = pkt.read_float()?;
        let _speed = pkt.read_float()?;

        let _crafting_npc = pkt.read_packed_guid()?;

        let currencies_count = pkt.read_uint32()? as usize;
        let reagents_count = pkt.read_uint32()? as usize;
        let removed_mods_count = pkt.read_uint32()? as usize;

        // Optional currencies (each: 3 i32 + 1 optional byte via bit)
        for _ in 0..currencies_count {
            let _item = pkt.read_int32()?;
            let _slot = pkt.read_int32()?;
            let _qty = pkt.read_int32()?;
            let has_extra = pkt.has_bit()?;
            if has_extra {
                let _u = pkt.read_uint8()?;
            }
        }

        // Bit section: SendCastFlags(5), hasMoveUpdate(1), weightCount(2), hasCraftingOrderID(1)
        let _send_cast_flags = pkt.read_bits(5)?;
        let has_move_update = pkt.has_bit()?;
        let weight_count = pkt.read_bits(2)? as usize;
        let has_crafting_order = pkt.has_bit()?;

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
            return Ok(Self {
                cast_id,
                misc: [misc0, misc1],
                spell_id,
                visual,
                target,
            });
        }

        // SpellWeights (each: ResetBitPos + Type(2 bits) + ID(i32) + Quantity(u32))
        for _ in 0..weight_count {
            pkt.reset_bits();
            let _ty = pkt.read_bits(2)?;
            let _id = pkt.read_int32()?;
            let _qty = pkt.read_uint32()?;
        }

        Ok(Self {
            cast_id,
            misc: [misc0, misc1],
            spell_id,
            visual,
            target,
        })
    }
}

/// CMSG_OPEN_ITEM payload.
///
/// C++ `WorldPackets::Spells::OpenItem::Read` reads `Slot` then `PackSlot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenItem {
    pub slot: u8,
    pub pack_slot: u8,
}

impl ClientPacket for OpenItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::OpenItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            slot: pkt.read_uint8()?,
            pack_slot: pkt.read_uint8()?,
        })
    }
}

/// `CMSG_SPELL_CLICK` payload.
///
/// C++ `WorldPackets::Spells::SpellClick::Read` reads the clicked unit GUID
/// followed by the `TryAutoDismount` bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellClick {
    pub unit_guid: ObjectGuid,
    pub try_auto_dismount: bool,
}

impl ClientPacket for SpellClick {
    const OPCODE: ClientOpcodes = ClientOpcodes::SpellClick;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            unit_guid: pkt.read_packed_guid()?,
            try_auto_dismount: pkt.read_bit()?,
        })
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
    original_cast_id: &ObjectGuid,
    spell_id: i32,
    visual: &SpellCastVisual,
    cast_flags_ex: u32,
    cast_time_ms: u32,
    target: &SpellTargetData,
    hit_targets: &[ObjectGuid],
) {
    // CasterGUID, CasterUnit, CastID, OriginalCastID
    pkt.write_packed_guid(caster);
    pkt.write_packed_guid(caster); // CasterUnit = same for player spells
    pkt.write_packed_guid(cast_id);
    pkt.write_packed_guid(original_cast_id);

    // SpellID + visual
    pkt.write_int32(spell_id);
    visual.write(pkt);

    // CastFlags, CastFlagsEx, CastTime
    pkt.write_uint32(0); // CastFlags
    pkt.write_uint32(cast_flags_ex);
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
    pkt.write_bits(0, 9); // RemainingPower
    pkt.write_bit(false); // RemainingRunes present?
    pkt.write_bits(0, 16); // TargetPoints
    pkt.write_bit(false); // AmmoDisplayID present?
    pkt.write_bit(false); // AmmoInventoryType present?
    pkt.flush_bits();

    // Target
    target.write(pkt);

    // HitTargets
    for guid in hit_targets {
        pkt.write_packed_guid(guid);
    }
    // (no MissTargets, MissStatus, RemainingPower, Runes, TargetPoints, Ammo)
}

// ── SMSG_SPELL_PREPARE ───────────────────────────────────────────

/// `SMSG_SPELL_PREPARE` — maps the client cast id to the server spell cast id.
///
/// C++ ref: `WorldPackets::Spells::SpellPrepare::Write`.
pub struct SpellPreparePkt {
    pub client_cast_id: ObjectGuid,
    pub server_cast_id: ObjectGuid,
}

impl ServerPacket for SpellPreparePkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellPrepare;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.client_cast_id);
        pkt.write_packed_guid(&self.server_cast_id);
    }
}

// ── SMSG_SPELL_START ─────────────────────────────────────────────

/// `SMSG_SPELL_START` — notifies client a spell cast has begun.
/// Used for spells with a cast time; for instant spells use `SpellGoPkt`.
pub struct SpellStartPkt {
    pub caster: ObjectGuid,
    pub cast_id: ObjectGuid,
    pub original_cast_id: ObjectGuid,
    pub spell_id: i32,
    pub visual: SpellCastVisual,
    pub cast_flags_ex: u32,
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
            &self.original_cast_id,
            self.spell_id,
            &self.visual,
            self.cast_flags_ex,
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
    pub original_cast_id: ObjectGuid,
    pub spell_id: i32,
    pub visual: SpellCastVisual,
    pub cast_flags_ex: u32,
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
            &self.original_cast_id,
            self.spell_id,
            &self.visual,
            self.cast_flags_ex,
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

// ── CooldownEvent (SMSG 0x26b9) ──────────────────────────────────────

/// Sent after a spell fires to notify the client that a cooldown has started.
/// The client uses this to display the GCD / cooldown animation on action buttons.
/// C# ref: SpellPackets.CooldownEvent (ConnectionType.Instance)
pub struct CooldownEvent {
    pub spell_id: i32,
    pub is_pet: bool,
}

impl ServerPacket for CooldownEvent {
    const OPCODE: ServerOpcodes = ServerOpcodes::CooldownEvent;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.spell_id);
        pkt.write_bit(self.is_pet);
        pkt.flush_bits();
    }
}

// ── SpellCooldownEntry / SpellCooldownPkt (SMSG 0x2c15) ──────────────

/// One entry in a SpellCooldownPkt.
/// C# ref: SpellPackets.SpellCooldownStruct
#[derive(Clone)]
pub struct SpellCooldownEntry {
    /// Spell ID (SrecID in C#).
    pub spell_id: i32,
    /// Remaining cooldown in milliseconds (0 = use category cooldown).
    pub cooldown_ms: u32,
    /// Cooldown modifier rate (1.0 = unmodified).
    pub mod_rate: f32,
}

/// Sends a list of spell cooldowns to the client.
/// Sent on login to restore active cooldowns, and optionally after each cast.
/// C# ref: SpellPackets.SpellCooldownPkt (ConnectionType.Instance)
pub struct SpellCooldownPkt {
    pub caster: wow_core::ObjectGuid,
    /// SpellCooldownFlags: 0x1 = IncludeGCD, 0x2 = InitialLogin, 0x4 = OnHold
    pub flags: u8,
    pub cooldowns: Vec<SpellCooldownEntry>,
}

impl ServerPacket for SpellCooldownPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellCooldown;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.caster);
        pkt.write_uint8(self.flags);
        pkt.write_uint32(self.cooldowns.len() as u32);
        for cd in &self.cooldowns {
            pkt.write_int32(cd.spell_id);
            pkt.write_uint32(cd.cooldown_ms);
            pkt.write_float(cd.mod_rate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_cast_reads_cpp_cast_id_then_spell_id() {
        let cast_id = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&cast_id);
        pkt.write_uint32(12_345);
        pkt.reset_read();

        let parsed = CancelCast::read(&mut pkt).unwrap();
        assert_eq!(parsed.cast_id, cast_id);
        assert_eq!(parsed.spell_id, 12_345);
        assert!(pkt.is_empty());
    }

    #[test]
    fn cancel_channelling_reads_cpp_channel_spell_then_reason() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(12_345);
        pkt.write_int32(40);
        pkt.reset_read();

        let parsed = CancelChannelling::read(&mut pkt).unwrap();
        assert_eq!(parsed.channel_spell, 12_345);
        assert_eq!(parsed.reason, 40);
        assert!(pkt.is_empty());
    }

    #[test]
    fn open_item_reads_cpp_slot_then_pack_slot() {
        let mut pkt = WorldPacket::from_bytes(&[0xC6, 0x32, 0xFF, 0x24]);
        pkt.skip_opcode();

        let open = OpenItem::read(&mut pkt).unwrap();
        assert_eq!(open.slot, 0xFF);
        assert_eq!(open.pack_slot, 0x24);
    }

    #[test]
    fn spell_click_reads_cpp_guid_then_try_auto_dismount_bit() {
        let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::SpellClick as u16);
        pkt.write_packed_guid(&guid);
        pkt.write_bit(true);
        pkt.flush_bits();
        pkt.reset_read();
        pkt.skip_opcode();

        let spell_click = SpellClick::read(&mut pkt).unwrap();
        assert_eq!(spell_click.unit_guid, guid);
        assert!(spell_click.try_auto_dismount);
    }

    #[test]
    fn spell_target_data_roundtrips_optional_locations_orientation_map_name() {
        let unit = ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
        let item = ObjectGuid::new(0x2122_2324_2526_2728, 0x3132_3334_3536_3738);
        let src_transport = ObjectGuid::new(0x4142_4344_4546_4748, 0x5152_5354_5556_5758);
        let dst_transport = ObjectGuid::new(0x6162_6364_6566_6768, 0x7172_7374_7576_7778);
        let target = SpellTargetData {
            flags: 0x0A_BC_DE_F0,
            unit,
            item,
            src_location: Some(TargetLocation {
                transport: src_transport,
                position: Position::xyz(1.25, -2.5, 3.75),
            }),
            dst_location: Some(TargetLocation {
                transport: dst_transport,
                position: Position::xyz(100.0, 200.5, -300.25),
            }),
            orientation: Some(4.125),
            map_id: Some(571),
            name: "FarsightTarget".to_string(),
        };

        let mut pkt = WorldPacket::new_empty();
        target.write(&mut pkt);
        pkt.reset_read();

        let parsed = SpellTargetData::read(&mut pkt).expect("target data must parse");
        assert_eq!(parsed.flags, target.flags);
        assert_eq!(parsed.unit, unit);
        assert_eq!(parsed.item, item);
        assert_eq!(parsed.src_location, target.src_location);
        assert_eq!(parsed.dst_location, target.dst_location);
        assert_eq!(parsed.orientation, target.orientation);
        assert_eq!(parsed.map_id, target.map_id);
        assert_eq!(parsed.name, target.name);
        assert!(pkt.is_empty());
    }

    #[test]
    fn spell_target_data_default_minimal_has_no_optional_payload() {
        let target = SpellTargetData::default();
        let mut pkt = WorldPacket::new_empty();
        target.write(&mut pkt);

        // 28-bit flags + 4 presence bits + 7-bit name length flushed to 5 bytes,
        // then two empty packed GUIDs (2 bytes each), matching the previous minimal shape.
        assert_eq!(pkt.data().len(), 9);

        pkt.reset_read();
        let parsed = SpellTargetData::read(&mut pkt).expect("minimal target data must parse");
        assert_eq!(parsed.flags, 0);
        assert_eq!(parsed.unit, ObjectGuid::EMPTY);
        assert_eq!(parsed.item, ObjectGuid::EMPTY);
        assert_eq!(parsed.src_location, None);
        assert_eq!(parsed.dst_location, None);
        assert_eq!(parsed.orientation, None);
        assert_eq!(parsed.map_id, None);
        assert!(parsed.name.is_empty());
        assert!(pkt.is_empty());
    }

    fn write_minimal_spell_cast_request(
        pkt: &mut WorldPacket,
        cast_id: ObjectGuid,
        misc: [i32; 2],
        spell_id: i32,
    ) {
        pkt.write_packed_guid(&cast_id);
        pkt.write_int32(misc[0]);
        pkt.write_int32(misc[1]);
        pkt.write_int32(spell_id);
        SpellCastVisual::default().write(pkt);
        pkt.write_float(0.0);
        pkt.write_float(0.0);
        pkt.write_packed_guid(&ObjectGuid::EMPTY);
        pkt.write_uint32(0);
        pkt.write_uint32(0);
        pkt.write_uint32(0);
        pkt.write_bits(0, 5);
        pkt.write_bit(false);
        pkt.write_bits(0, 2);
        pkt.write_bit(false);
        pkt.flush_bits();
        SpellTargetData::default().write(pkt);
    }

    #[test]
    fn cast_spell_request_preserves_misc_like_cpp() {
        let cast_id = ObjectGuid::create_player(1, 77);
        let mut pkt = WorldPacket::new_empty();
        write_minimal_spell_cast_request(&mut pkt, cast_id, [30_000, 9], 12_345);

        let parsed = CastSpellRequest::read(&mut pkt).unwrap();
        assert_eq!(parsed.cast_id, cast_id);
        assert_eq!(parsed.misc, [30_000, 9]);
        assert_eq!(parsed.spell_id, 12_345);
    }

    #[test]
    fn spell_prepare_writes_client_and_server_cast_ids_like_cpp() {
        let client_cast_id = ObjectGuid::create_player(1, 77);
        let server_cast_id = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Cast,
            0,
            1,
            571,
            0,
            12_345,
            9,
        );

        let bytes = SpellPreparePkt {
            client_cast_id,
            server_cast_id,
        }
        .to_bytes();

        assert_eq!(
            &bytes[0..2],
            &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
        );
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_packed_guid().unwrap(), client_cast_id);
        assert_eq!(pkt.read_packed_guid().unwrap(), server_cast_id);
        assert!(pkt.is_empty());
    }

    #[test]
    fn play_spell_visual_writes_cpp_field_order() {
        let guid = ObjectGuid::create_player(1, 77);
        let target_position = Position::new(1.25, -2.5, 3.75, 0.5);
        let bytes = PlaySpellVisual::self_target(guid, target_position, 222).to_bytes();
        let mut pkt = WorldPacket::from_bytes(&bytes);

        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::PlaySpellVisual as u16
        );
        let mut source = [0u8; 16];
        for byte in &mut source {
            *byte = pkt.read_uint8().expect("source byte");
        }
        assert_eq!(ObjectGuid::from_raw_bytes(&source), guid);
        let mut target = [0u8; 16];
        for byte in &mut target {
            *byte = pkt.read_uint8().expect("target byte");
        }
        assert_eq!(ObjectGuid::from_raw_bytes(&target), guid);
        let mut transport = [0u8; 16];
        for byte in &mut transport {
            *byte = pkt.read_uint8().expect("transport byte");
        }
        assert_eq!(ObjectGuid::from_raw_bytes(&transport), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_float().expect("target x"), 1.25);
        assert_eq!(pkt.read_float().expect("target y"), -2.5);
        assert_eq!(pkt.read_float().expect("target z"), 3.75);
        assert_eq!(pkt.read_uint32().expect("visual"), 222);
        assert_eq!(pkt.read_float().expect("travel speed"), 0.0);
        assert_eq!(pkt.read_uint16().expect("hit reason"), 0);
        assert_eq!(pkt.read_uint16().expect("miss reason"), 0);
        assert_eq!(pkt.read_uint16().expect("reflect status"), 0);
        assert_eq!(pkt.read_float().expect("launch delay"), 0.0);
        assert_eq!(pkt.read_float().expect("min duration"), 0.0);
        assert!(!pkt.read_bit().expect("speed as time"));
        assert!(pkt.is_empty());
    }

    #[test]
    fn spell_go_preserves_original_cast_id_and_cast_flags_ex_like_cpp() {
        let caster = ObjectGuid::create_player(1, 77);
        let client_cast_id = ObjectGuid::create_player(1, 99);
        let server_cast_id = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Cast,
            0,
            1,
            571,
            0,
            12_345,
            9,
        );

        let bytes = SpellGoPkt {
            caster,
            cast_id: server_cast_id,
            original_cast_id: client_cast_id,
            spell_id: 12_345,
            visual: SpellCastVisual::default(),
            cast_flags_ex: 0x08000,
            target: SpellTargetData::default(),
            hit_targets: Vec::new(),
        }
        .to_bytes();

        assert_eq!(&bytes[0..2], &(ServerOpcodes::SpellGo as u16).to_le_bytes());
        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_packed_guid().unwrap(), caster);
        assert_eq!(pkt.read_packed_guid().unwrap(), caster);
        assert_eq!(pkt.read_packed_guid().unwrap(), server_cast_id);
        assert_eq!(pkt.read_packed_guid().unwrap(), client_cast_id);
        assert_eq!(pkt.read_int32().unwrap(), 12_345);
        let visual = SpellCastVisual::read(&mut pkt).unwrap();
        assert_eq!(visual.spell_visual_id, 0);
        assert_eq!(visual.script_visual_id, 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0);
        assert_eq!(pkt.read_uint32().unwrap(), 0x08000);
    }
}
