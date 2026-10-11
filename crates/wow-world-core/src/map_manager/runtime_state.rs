//! Runtime state items of mod.
//!
//! Separated from mod.rs under #711; every item keeps its name,
//! signature and body.

use super::*;

/// C++ `BASE_ATTACK_TIME` (`UnitDefines.h:30`). Creature base/ranged attack time
/// is clamped to this when the template value is 0 (`ObjectMgr.cpp:1100-1104`); a
/// 0 attack time crashes the 3.4.3 client's swing-timer math on the first tick.
pub(super) const BASE_ATTACK_TIME_LIKE_CPP: u32 = 2_000;

pub(super) const fn power_type_from_u8_like_cpp(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

/// Live snapshot of a chase victim, taken by the tick driver before the creature
/// is borrowed mutably. C++ `ChaseMovementGenerator` holds a live `Unit*`; the
/// Rust runtime has no object accessor inside the creature step, so the caller
/// supplies the same facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseTargetSnapshotLikeCpp {
    pub guid: ObjectGuid,
    pub position: Position,
    pub combat_reach: f32,
    pub in_world: bool,
    /// C++ `Unit::isInAccessiblePlaceFor(Creature const*)` branches on the
    /// victim's `IsInWater()`: in water it asks the chaser's `CanEnterWater()`,
    /// otherwise `CanWalk() || CanFly()`.
    ///
    /// `None` means the runtime cannot answer it for this victim — creature
    /// entities carry no liquid state and the terrain layer exposes heights
    /// only. See `chase_unit_snapshot_like_cpp` for how that is degraded.
    pub in_water: Option<bool>,
}

/// C++ `NOMINAL_MELEE_RANGE` (`ObjectDefines.h:44`).
pub(super) const NOMINAL_MELEE_RANGE_LIKE_CPP: f32 = 5.0;

/// C++ `Position::GetAbsoluteAngle`: the world bearing from `from` to `to`.
pub(super) fn absolute_angle_like_cpp(from: Position, to: Position) -> f32 {
    wow_movement::normalize_orientation_like_cpp((to.y - from.y).atan2(to.x - from.x))
}

/// What one chase tick produced for the caller to publish.
#[derive(Debug, Clone, PartialEq)]
pub enum ChaseTickOutcomeLikeCpp {
    /// Nothing to send this tick.
    Idle,
    /// A superseded spline was stopped; publish `SMSG_ON_MONSTER_MOVE` stop.
    Stopped(MoveSplineStopResult),
    /// A new chase spline was launched.
    Launched(Position, MoveSpline),
}

/// Runtime selector proxy for an active generator whose concrete lifecycle
/// still lives in `wow_entities::MotionSubsystem`.
///
/// #1263 F6-8A moved the type, together with the canonical runtime state that
/// stores it, into `wow-entities`; the name is re-exported here so the legacy
/// map-manager modules keep resolving it unchanged.
pub use wow_entities::{
    ActiveTauntLikeCpp, RuntimeRepresentedActiveGeneratorLikeCpp,
    RuntimeRepresentedActiveKeyLikeCpp,
};

/// Candidate-routing rule for a [`RuntimeEvent`].
///
/// Each variant maps to one of the C++ `MessageDistDeliverer` distribution
/// modes.  The final HaveAtClient / phase gate is applied by each session
/// (`SendIfVisibleLikeCpp`, Slice 4A.1b) — do NOT duplicate visibility or
/// phase logic here.
#[derive(Debug, Clone, PartialEq)]
pub enum RecipientRule {
    /// Broadcast to all sessions whose visible range overlaps the source
    /// position.  Mirrors C++ `MessageDistDeliverer` with a range constraint.
    NearbyVisible {
        source_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        range: f32,
        required_3d: bool,
    },
    /// Same visibility fanout as `NearbyVisible`, but committed combat
    /// transitions are queued on each session's durable FIFO rail.
    NearbyVisibleDurable {
        source_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        range: f32,
        required_3d: bool,
    },
    /// One creature spell cast whose START plus viewer-selected basic/full GO
    /// frames must be published and consumed as a single durable unit.
    ///
    /// `RuntimeEvent::packet_bytes` carries START and the two GO fields carry
    /// the basic/full alternatives. Routing and all companion payloads are
    /// deliberately coupled here until `RuntimeEvent` grows a first-class
    /// packet-batch payload; independent events would allow a session drain
    /// between the two committed frames.
    NearbyVisibleDurableSpellCast {
        source_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        range: f32,
        required_3d: bool,
        basic_go_packet_bytes: Vec<u8>,
        full_go_packet_bytes: Vec<u8>,
    },
    /// Broadcast to every session on the map regardless of distance.
    /// Mirrors C++ map-wide broadcast.
    MapBroadcastVisible { map_id: u16, instance_id: u32 },
    /// Send to exactly one player session identified by GUID.
    ExplicitPlayer(ObjectGuid),
    /// Send only to the session that owns the source entity (self-delivery).
    SelfOnly,
}

/// A single routing-annotated packet produced during a tick.
///
/// `packet_bytes` is the already-serialised wire payload.  The routing
/// decision (who receives it) is encoded in `recipients`.
#[derive(Debug, Clone)]
pub struct RuntimeEvent {
    pub source_guid: ObjectGuid,
    pub recipients: RecipientRule,
    pub packet_bytes: Vec<u8>,
}

/// An ordered list of [`RuntimeEvent`]s produced by a single tick pass,
/// ready to be consumed by a routing layer (Slice 4A.1b+).
#[derive(Debug, Clone, Default)]
pub struct RuntimePlan {
    pub events: Vec<RuntimeEvent>,
}

/// Which C++ `Unit::Set*AnimKitId` setter to mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureAnimKitSlotLikeCpp {
    Ai,
    Movement,
    Melee,
}
