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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimeRepresentedActiveKeyLikeCpp {
    pub(super) kind: RuntimeMovementGeneratorType,
    mode: RuntimeMovementGeneratorMode,
    priority: RuntimeMovementGeneratorPriority,
    base_unit_state: u32,
}

#[derive(Debug)]
pub(super) struct RuntimeRepresentedActiveGeneratorLikeCpp {
    state: RuntimeMovementGeneratorState,
    kind: RuntimeMovementGeneratorType,
}

impl RuntimeRepresentedActiveGeneratorLikeCpp {
    pub(super) fn from_represented(generator: MovementGeneratorRef) -> Option<Self> {
        let kind = RuntimeMovementGeneratorType::from_trinity_id(generator.kind.trinity_id())?;
        let mode = match generator.mode {
            wow_entities::MovementGeneratorMode::Default => RuntimeMovementGeneratorMode::Default,
            wow_entities::MovementGeneratorMode::Override => RuntimeMovementGeneratorMode::Override,
        };
        let priority = match generator.priority {
            wow_entities::MovementGeneratorPriority::None => RuntimeMovementGeneratorPriority::None,
            wow_entities::MovementGeneratorPriority::Normal => {
                RuntimeMovementGeneratorPriority::Normal
            }
            wow_entities::MovementGeneratorPriority::Highest => {
                RuntimeMovementGeneratorPriority::Highest
            }
        };
        Some(Self {
            state: RuntimeMovementGeneratorState {
                mode,
                priority,
                flags: RuntimeMovementGeneratorFlags::INITIALIZATION_PENDING,
                base_unit_state: generator.base_unit_state,
            },
            kind,
        })
    }

    pub(super) const fn key(&self) -> RuntimeRepresentedActiveKeyLikeCpp {
        RuntimeRepresentedActiveKeyLikeCpp {
            kind: self.kind,
            mode: self.state.mode,
            priority: self.state.priority,
            base_unit_state: self.state.base_unit_state,
        }
    }
}

impl RuntimeMovementGenerator for RuntimeRepresentedActiveGeneratorLikeCpp {
    fn state(&self) -> &RuntimeMovementGeneratorState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut RuntimeMovementGeneratorState {
        &mut self.state
    }

    fn kind(&self) -> RuntimeMovementGeneratorType {
        self.kind
    }

    fn initialize(&mut self) {
        self.state.flags.remove(
            RuntimeMovementGeneratorFlags::INITIALIZATION_PENDING
                | RuntimeMovementGeneratorFlags::DEACTIVATED,
        );
        self.state
            .flags
            .insert(RuntimeMovementGeneratorFlags::INITIALIZED);
    }

    fn reset(&mut self) {
        self.initialize();
    }

    fn update(&mut self, _diff_ms: u32) -> bool {
        !self
            .state
            .flags
            .contains(RuntimeMovementGeneratorFlags::FINALIZED)
    }

    fn deactivate(&mut self) {
        self.state
            .flags
            .insert(RuntimeMovementGeneratorFlags::DEACTIVATED);
    }

    fn finalize(&mut self, _active: bool, _movement_inform: bool) {
        self.state
            .flags
            .insert(RuntimeMovementGeneratorFlags::FINALIZED);
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ActiveTauntLikeCpp {
    pub(super) caster: ObjectGuid,
    /// `None` represents C++/DB2's permanent duration sentinel `-1`.
    pub(super) due_at_ms: Option<u64>,
    pub(super) spell_id: u32,
    pub(super) effect_mask: u32,
    pub(super) slot: u8,
}

/// Who owns the creature/combat tick for a given map at runtime.
///
/// Test/local default is `Session`: each logged-in session drives its own
/// creature and combat ticks. Production startup flips this to `GlobalLegacy`
/// by default so a global map clock drives creature runtime like C++ and
/// session-level creature ticks are skipped to avoid double resolution.
///
/// The owner lives on the shared [`MapManager`] so all sessions on the same
/// map read the same value.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RuntimeTickOwner {
    /// Per-session tick for isolated tests and explicit local diagnostics.
    #[default]
    Session,
    /// Global legacy-manager tick used by production startup by default.
    GlobalLegacy,
}

/// Initial session-local packet seam produced by `run_creatures_tick` /
/// `run_combat_tick`.
///
/// **This is a seam initial, NOT the final fanout design.** It holds only the
/// raw bytes that today are sent via `send_tx`. The real global fanout (Slice
/// 5+) will need per-destination routing, not a flat byte list.
///
/// If side effects other than packet bytes are discovered during extraction
/// they must be modelled separately, NOT smuggled through this flush path.
pub struct RuntimeOutput {
    /// Packets to be flushed to the session channel in order.
    pub packets: Vec<Vec<u8>>,
}

impl RuntimeOutput {
    pub fn new() -> Self {
        Self {
            packets: Vec::new(),
        }
    }
}

impl Default for RuntimeOutput {
    fn default() -> Self {
        Self::new()
    }
}

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

impl RuntimeOutput {
    /// Convert this `RuntimeOutput` into a [`RuntimePlan`] where every packet
    /// is addressed to the owning session only (`RecipientRule::SelfOnly`).
    ///
    /// This is the minimal bridge used by a single-session tick caller that
    /// still handles its own delivery.  Packet order is preserved exactly.
    /// `RuntimeOutput` itself is consumed (not cloned); the Slice 3 flush path
    /// (`flush_runtime_output`) is left untouched.
    pub fn into_owning_session_plan(self, source_guid: ObjectGuid) -> RuntimePlan {
        let events = self
            .packets
            .into_iter()
            .map(|packet_bytes| RuntimeEvent {
                source_guid,
                recipients: RecipientRule::SelfOnly,
                packet_bytes,
            })
            .collect();
        RuntimePlan { events }
    }
}

/// Read the runtime tick owner from a shared manager under one poison policy.
///
/// The owner decided who ticks creatures, and the two readers disagreed about a
/// poisoned lock. The session read it as `mm.read().ok()` falling back to
/// [`RuntimeTickOwner::Session`], while every tick body reads through the poison
/// with `unwrap_or_else(|poisoned| poisoned.into_inner())`. A poisoned legacy
/// lock therefore told the session "you own the tick" and the global loop "you
/// own the tick" at the same time, and the creature resolved twice (#28).
///
/// One function, one policy: read through the poison, exactly as the tick
/// bodies do. A poisoned lock means some thread panicked mid-mutation, which is
/// a reason to disconnect a session — not a reason to silently hand ownership
/// back to it.
///
/// Returning a `Copy` value is the other half of the guarantee. The owner cannot
/// be read while holding a guard, so no caller can acquire the canonical map
/// lock underneath a legacy one just to find out who owns the tick.
#[must_use]
pub fn shared_runtime_tick_owner_like_cpp(manager: &SharedMapManager) -> RuntimeTickOwner {
    manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .tick_owner()
}
