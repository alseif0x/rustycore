// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell cast packets — CMSG_CAST_SPELL / SMSG_SPELL_START / SMSG_SPELL_GO.
//!
//! Packet structures mirror C++ `WorldPackets::Spells` in
//! `SpellPackets.h` / `SpellPackets.cpp`.
//!
//! `CastSpellRequest` parses the full `SpellCastRequestPkt` so we correctly
//! advance the buffer even for fields we don't yet use (optionalReagents,
//! MoveUpdate, SpellWeights, etc.).
//!
//! `SpellStartPkt` and `SpellGoPkt` serialize the shared C++ `SpellCastData`
//! payload, while `SpellGoPkt` can also append the full combat-log suffix for
//! advanced viewers.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position};

use crate::packets::movement::MovementInfo;
use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};
mod cast_interruption;
pub use cast_interruption::{SpellFailedOtherPkt, SpellFailurePkt};

mod cast_payload;
#[cfg(test)]
mod cast_tests;
mod cast_types;

pub use cast_types::{
    CreatureImmunities, MissileTrajectoryResult, RuneData, SpellCastData, SpellHealPrediction,
    SpellPowerData,
};

/// C++ `WorldPackets::Spells::CancelAura`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelAura {
    pub spell_id: i32,
    pub caster_guid: ObjectGuid,
}

impl ClientPacket for CancelAura {
    const OPCODE: ClientOpcodes = ClientOpcodes::CancelAura;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let spell_id = pkt.read_int32()?;
        let caster_guid = pkt.read_packed_guid()?;
        Ok(Self {
            spell_id,
            caster_guid,
        })
    }
}

/// C++ `WorldPackets::Spells::CancelAutoRepeatSpell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelAutoRepeatSpell;

impl ClientPacket for CancelAutoRepeatSpell {
    const OPCODE: ClientOpcodes = ClientOpcodes::CancelAutoRepeatSpell;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

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

/// C++ `WorldPackets::Spells::CancelGrowthAura`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelGrowthAura;

impl ClientPacket for CancelGrowthAura {
    const OPCODE: ClientOpcodes = ClientOpcodes::CancelGrowthAura;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Spells::CancelModSpeedNoControlAuras`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelModSpeedNoControlAuras {
    pub target_guid: ObjectGuid,
}

impl ClientPacket for CancelModSpeedNoControlAuras {
    // The inspected 3.4.3 table marks this as the shared unresolved `0xBADD`
    // placeholder. Route it from the existing 0xBADD opcode branch by payload
    // shape and mover GUID until that table is resolved.
    const OPCODE: ClientOpcodes = ClientOpcodes::SetLootSpecialization;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let target_guid = pkt.read_packed_guid()?;
        Ok(Self { target_guid })
    }
}

/// C++ `WorldPackets::Spells::CancelMountAura`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelMountAura;

impl ClientPacket for CancelMountAura {
    const OPCODE: ClientOpcodes = ClientOpcodes::CancelMountAura;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Spells::ClearTarget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearTarget {
    pub guid: ObjectGuid,
}

impl ServerPacket for ClearTarget {
    const OPCODE: ServerOpcodes = ServerOpcodes::ClearTarget;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
    }
}

/// C++ `WorldPackets::Spells::SetActionButton`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetActionButton {
    pub action: u32,
    pub index: u8,
}

impl ClientPacket for SetActionButton {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetActionButton;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            action: pkt.read_uint32()?,
            index: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Spells::CancelQueuedSpell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelQueuedSpell;

impl ClientPacket for CancelQueuedSpell {
    const OPCODE: ClientOpcodes = ClientOpcodes::CancelQueuedSpell;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::Spells::SelfRes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfRes {
    pub spell_id: i32,
}

impl ClientPacket for SelfRes {
    const OPCODE: ClientOpcodes = ClientOpcodes::SelfRes;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let spell_id = pkt.read_int32()?;
        Ok(Self { spell_id })
    }
}

// ── Sub-structures ────────────────────────────────────────────────

/// SpellCastVisual as serialized by TrinityCore: one signed visual ID.
#[derive(Debug, Clone, Default)]
pub struct SpellCastVisual {
    pub spell_visual_id: u32,
    /// Kept for callers that still carry this value, but not serialized by the
    /// local C++ branch (`ScriptVisualID` is commented out there).
    pub script_visual_id: u32,
}

impl SpellCastVisual {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            spell_visual_id: pkt.read_uint32()?,
            script_visual_id: 0,
        })
    }

    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.spell_visual_id);
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

/// C++ `WorldPackets::Spells::PlaySpellVisualKit`.
///
/// Unlike `PlaySpellVisual`, this packet names one unit and one prebuilt
/// visual-kit record. `Unit::SendPlaySpellVisualKit` sends it to the unit's
/// visible set, including the controlling player when requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaySpellVisualKit {
    pub unit: ObjectGuid,
    pub kit_record_id: i32,
    pub kit_type: i32,
    pub duration: u32,
    pub mounted_visual: bool,
}

impl ServerPacket for PlaySpellVisualKit {
    const OPCODE: ServerOpcodes = ServerOpcodes::PlaySpellVisualKit;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit);
        pkt.write_int32(self.kit_record_id);
        pkt.write_int32(self.kit_type);
        pkt.write_uint32(self.duration);
        pkt.write_bit(self.mounted_visual);
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
    /// Optional movement status embedded in the cast request.
    pub move_update: Option<MovementInfo>,
    /// C++ `SpellCastTargets::HasTraj()` input: `HandleCastSpellOpcode` copies
    /// `MissileTrajectory.Speed` into `m_targets`, and `HasTraj()` is
    /// `m_speed != 0`. The pitch/speed values themselves are not retained.
    pub has_trajectory_like_cpp: bool,
    /// C++ `SpellCastTargets::m_pitch` from `MissileTrajectory.Pitch`.
    pub trajectory_pitch_like_cpp: f32,
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
        let pitch = pkt.read_float()?;
        let speed = pkt.read_float()?;

        let _crafting_npc = pkt.read_packed_guid()?;

        let currencies_count = pkt.read_uint32()? as usize;
        let reagents_count = pkt.read_uint32()? as usize;
        let removed_mods_count = pkt.read_uint32()? as usize;

        // C++ SpellExtraCurrencyCost: CurrencyID + Count.
        for _ in 0..currencies_count {
            let _currency_id = pkt.read_int32()?;
            let _count = pkt.read_int32()?;
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

        let move_update = if has_move_update {
            Some(MovementInfo::read(pkt)?)
        } else {
            None
        };

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
            move_update,
            has_trajectory_like_cpp: speed != 0.0,
            trajectory_pitch_like_cpp: pitch,
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

/// Per-target spell miss reason (`SpellMissInfo` in TrinityCore).
///
/// These discriminants are serialized directly as the C++ `uint8` wire value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpellMissReason {
    None = 0,
    Miss = 1,
    Resist = 2,
    Dodge = 3,
    Parry = 4,
    Block = 5,
    Evade = 6,
    Immune = 7,
    Immune2 = 8,
    Deflect = 9,
    Absorb = 10,
    Reflect = 11,
}

/// C++ `WorldPackets::Spells::SpellMissStatus`.
///
/// `reflect_status` is serialized only when `reason` is [`SpellMissReason::Reflect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellMissStatus {
    pub reason: SpellMissReason,
    pub reflect_status: SpellMissReason,
}

impl SpellMissStatus {
    /// Construct a non-reflect miss status.
    pub const fn new(reason: SpellMissReason) -> Self {
        Self {
            reason,
            reflect_status: SpellMissReason::None,
        }
    }

    /// Construct a reflected result and its outcome against the original caster.
    pub const fn reflected(reflect_status: SpellMissReason) -> Self {
        Self {
            reason: SpellMissReason::Reflect,
            reflect_status,
        }
    }

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.reason as u8);
        if self.reason == SpellMissReason::Reflect {
            pkt.write_uint8(self.reflect_status as u8);
        }
    }
}

/// A failed spell target paired with the status serialized for that target.
///
/// Keeping the GUID and status together guarantees that the parallel C++
/// `MissTargets` and `MissStatus` vectors have matching lengths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellMissTarget {
    pub target: ObjectGuid,
    pub status: SpellMissStatus,
}

impl SpellMissTarget {
    pub const fn new(target: ObjectGuid, reason: SpellMissReason) -> Self {
        Self {
            target,
            status: SpellMissStatus::new(reason),
        }
    }

    pub const fn reflected(target: ObjectGuid, reflect_status: SpellMissReason) -> Self {
        Self {
            target,
            status: SpellMissStatus::reflected(reflect_status),
        }
    }
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
/// C++ emits this during `Spell::prepare` even for instant casts, followed by
/// `SpellGoPkt` when the cast executes.
pub struct SpellStartPkt {
    pub caster: ObjectGuid,
    pub cast_id: ObjectGuid,
    pub original_cast_id: ObjectGuid,
    pub spell_id: i32,
    pub visual: SpellCastVisual,
    /// C++ `Spell::SendSpellStart` flags (at minimum
    /// `CAST_FLAG_HAS_TRAJECTORY` for an ordinary unit cast).
    pub cast_flags: u32,
    pub cast_flags_ex: u32,
    /// Cast time in milliseconds (0 for instant).
    pub cast_time_ms: u32,
    pub target: SpellTargetData,
    /// Optional C++ `SpellCastData` fields beyond the common cast header.
    pub cast_data: SpellCastData,
}

impl ServerPacket for SpellStartPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellStart;

    fn write(&self, pkt: &mut WorldPacket) {
        cast_payload::write_spell_cast_data(
            pkt,
            &self.caster,
            &self.cast_id,
            &self.original_cast_id,
            self.spell_id,
            &self.visual,
            self.cast_flags,
            self.cast_flags_ex,
            self.cast_time_ms,
            &self.target,
            &self.cast_data,
            &[], // no hit targets in SPELL_START
            &[], // no miss targets in SPELL_START
        );
    }
}

// ── SMSG_SPELL_GO ────────────────────────────────────────────────

/// One C++ `SpellLogPowerData` row embedded in advanced combat-log packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellLogPowerData {
    pub power_type: i32,
    pub amount: i32,
    pub cost: i32,
}

/// C++ `SpellCastLogData`, appended only to the full combat-log packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCastLogData {
    pub health: i64,
    pub attack_power: i32,
    pub spell_power: i32,
    pub armor: i32,
    pub power_data: Vec<SpellLogPowerData>,
}

impl SpellCastLogData {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int64(self.health);
        pkt.write_int32(self.attack_power);
        pkt.write_int32(self.spell_power);
        pkt.write_int32(self.armor);
        assert!(
            self.power_data.len() <= 0x1ff,
            "SpellCastLogData power count exceeds the C++ 9-bit field"
        );
        pkt.write_bits(self.power_data.len() as u32, 9);
        pkt.flush_bits();
        for power in &self.power_data {
            pkt.write_int32(power.power_type);
            pkt.write_int32(power.amount);
            pkt.write_int32(power.cost);
        }
    }
}

/// `SMSG_SPELL_GO` — spell completes and effects are applied.
///
/// Instant represented casts send this immediately after `SpellStartPkt`.
pub struct SpellGoPkt {
    pub caster: ObjectGuid,
    pub cast_id: ObjectGuid,
    pub original_cast_id: ObjectGuid,
    pub spell_id: i32,
    pub visual: SpellCastVisual,
    pub cast_flags: u32,
    pub cast_flags_ex: u32,
    /// C++ `SpellCastData::CastTime`; for `SMSG_SPELL_GO` this is the
    /// server's wrapping `getMSTime()` timestamp, not the cast duration.
    pub cast_time_ms: u32,
    pub target: SpellTargetData,
    /// Optional C++ `SpellCastData` fields beyond the common cast header.
    pub cast_data: SpellCastData,
    /// GUIDs that were hit by the spell.
    pub hit_targets: Vec<ObjectGuid>,
    /// Failed targets and their per-target miss result.
    pub miss_targets: Vec<SpellMissTarget>,
}

impl SpellGoPkt {
    fn write_with_log_data(&self, pkt: &mut WorldPacket, log_data: Option<&SpellCastLogData>) {
        // SpellCastData (`SMSG_SPELL_GO` carries the server timestamp here).
        cast_payload::write_spell_cast_data(
            pkt,
            &self.caster,
            &self.cast_id,
            &self.original_cast_id,
            self.spell_id,
            &self.visual,
            self.cast_flags,
            self.cast_flags_ex,
            self.cast_time_ms,
            &self.target,
            &self.cast_data,
            &self.hit_targets,
            &self.miss_targets,
        );

        // C++ `CombatLogServerPacket::WriteLogDataBit`, `FlushBits`, then
        // `WriteLogData` only for the full packet.
        pkt.write_bit(log_data.is_some());
        pkt.flush_bits();
        if let Some(log_data) = log_data {
            log_data.write(pkt);
        }
    }

    /// Serialize the C++ `GetFullLogPacket()` representation for an advanced
    /// combat-log viewer.
    pub fn to_full_log_bytes_like_cpp(&self, log_data: &SpellCastLogData) -> Vec<u8> {
        let mut pkt = WorldPacket::new_server(ServerOpcodes::SpellGo);
        self.write_with_log_data(&mut pkt, Some(log_data));
        pkt.into_data()
    }
}

impl ServerPacket for SpellGoPkt {
    const OPCODE: ServerOpcodes = ServerOpcodes::SpellGo;

    fn write(&self, pkt: &mut WorldPacket) {
        self.write_with_log_data(pkt, None);
    }
}

// ── SMSG_CAST_FAILED ─────────────────────────────────────────────

/// `SMSG_CAST_FAILED` — generic failure response for a spell cast.
/// Sent when the player tries to cast a spell they don't know.
pub struct CastFailed {
    pub cast_id: ObjectGuid,
    pub spell_id: i32,
    pub visual: SpellCastVisual,
    /// SpellCastResult failure reason (0 = SpellCastResult::Ok, but we use non-zero).
    /// Common: 2 = NotKnown, 70 = NotReady, 5 = BadTargets
    pub reason: i32,
    pub fail_arg1: i32,
    pub fail_arg2: i32,
}

impl ServerPacket for CastFailed {
    // C++: SpellPackets.h CastFailed / SMSG_CAST_FAILED.
    const OPCODE: ServerOpcodes = ServerOpcodes::CastFailed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.cast_id);
        pkt.write_int32(self.spell_id);
        self.visual.write(pkt);
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
#[path = "spell/tests/mod.rs"]
mod tests;
