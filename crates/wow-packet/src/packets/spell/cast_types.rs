// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Typed optional fields carried by C++ `WorldPackets::Spells::SpellCastData`.
//!
//! These types intentionally model only the payload which was previously
//! hardcoded to its empty representation by the Rust packet adapter. The
//! common cast header and hit/miss target vectors remain on `SpellStartPkt`
//! and `SpellGoPkt` so existing packet ownership and call sites stay clear.

use wow_core::ObjectGuid;

use super::TargetLocation;

/// One C++ `SpellPowerData` row in `SpellCastData::RemainingPower`.
///
/// C++ writes `Cost` as an `int32` followed by `Type` as an `int8`. The
/// adapter calls the first field `amount` because that is the payload's
/// remaining amount on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellPowerData {
    pub amount: i32,
    pub power_type: i8,
}

/// C++ `RuneData` carried when `RemainingRunes` is present.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuneData {
    pub start: u8,
    pub count: u8,
    pub cooldowns: Vec<u8>,
}

/// C++ `MissileTrajectoryResult`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MissileTrajectoryResult {
    pub travel_time: u32,
    pub pitch: f32,
}

/// C++ `CreatureImmunities`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreatureImmunities {
    pub school: u32,
    pub value: u32,
}

/// C++ `SpellHealPrediction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellHealPrediction {
    pub beacon_guid: ObjectGuid,
    pub points: u32,
    pub prediction_type: u8,
}

/// Optional fields in the 3.4.3 `SpellCastData` wire payload.
///
/// The default value is the empty C++ payload and preserves the bytes emitted
/// by the former Rust serializer. `caster_unit` is intentionally optional at
/// the API boundary: when it is `None`, the serializer repeats the packet's
/// existing `caster` GUID as C++ callers historically did for unit casts. A
/// `Some` value is available for an item/source caster whose unit caster is a
/// different GUID.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellCastData {
    pub caster_unit: Option<ObjectGuid>,
    pub remaining_power: Vec<SpellPowerData>,
    pub remaining_runes: Option<RuneData>,
    pub missile_trajectory: MissileTrajectoryResult,
    pub ammo_display_id: Option<i32>,
    pub ammo_inventory_type: Option<i32>,
    pub dest_loc_spell_cast_index: u8,
    pub target_points: Vec<TargetLocation>,
    pub immunities: CreatureImmunities,
    pub heal_prediction: SpellHealPrediction,
}
