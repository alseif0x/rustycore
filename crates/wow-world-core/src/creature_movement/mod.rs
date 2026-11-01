// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The creature movement body over canonical ownership.
//!
//! #1263 F6-8D3a-2: C++ keeps `Unit::movespline`, `Unit::i_motionMaster` and
//! the concrete generators on the live `Creature` object. Their state lives in
//! the canonical `Creature` (`CreatureRuntimeLikeCpp`, F6-8A/D1); the
//! generator, spline and effect bodies that drive it are written once against
//! [`CreatureMovementLikeCpp`], a borrow of that canonical creature.
//!
//! #1263 F6-8E-2: moved out of the deleted legacy `map_manager` module. The
//! legacy bridge that lent a cached `CreatureCreateData` projection beside the
//! creature is gone, so the borrow is always the canonical incarnation alone.

mod home_and_chase;
mod point_and_effects;
mod queries;
mod random_and_waypoint;
mod spline;

use std::ops::{Deref, DerefMut};

use rand::rngs::StdRng;
use wow_constants::movement::MovementFlag;
use wow_constants::{
    CreatureRandomMovementType as ConstantsCreatureRandomMovementType, UnitMoveType,
    UnitStandStateType, UnitState,
};
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    AllowedPositionZCaps, Creature, CreatureAiState, DEFAULT_HEIGHT_SEARCH, DistractMovementAction,
    EVENT_CHARGE_PREPATH, GenericMovementInform, INVALID_HEIGHT, MovementGeneratorKind,
    MovementGeneratorType, PointMovementAction, PointMovementInform, RotateMovementUpdate,
    Z_OFFSET_FIND_HEIGHT, allowed_position_z_from_ground_like_cpp,
};
use wow_movement::generators::CreatureRandomMovementType as MovementCreatureRandomMovementType;
use wow_movement::{
    ChaseMovementGenerator, HomeMovementGenerator, MoveSpline, MoveSplineFlag, MoveSplineInit,
    MoveSplineLaunchInput, MoveSplineStopResult, PathGenerator, PathType,
    RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP, RandomMovementAction, RandomMovementGenerator,
    RandomPathResult, RandomUnitSnapshot, WaypointAnimation, WaypointLaunchPlan,
    WaypointMovementAction, WaypointMovementGenerator, WaypointPath, WaypointRandomAtPathEnd,
    WaypointUnitSnapshot, compute_random_destination_like_cpp,
};
use wow_recastdetour::{
    DetourOwnerCapabilitiesLikeCpp, DetourPolyPath, MAX_POINT_PATH_LENGTH_LIKE_CPP,
    PathQueryFilterContext,
};

use crate::pathfinder::{
    CreaturePathQueryLikeCpp, path_generator_from_detour_with_normalizer_like_cpp,
    path_type_from_detour_like_cpp, point_path_limit_for_distance_like_cpp,
    random_path_result_from_path_type_like_cpp,
};
use crate::terrain::LiveTerrainHeights;

pub use queries::CreatureMovementQueriesLikeCpp;

/// A mutable borrow of one canonical creature for the movement bodies.
pub struct CreatureMovementLikeCpp<'a> {
    pub creature: &'a mut Creature,
}

impl<'a> CreatureMovementLikeCpp<'a> {
    /// Borrow one canonical incarnation.
    pub fn canonical_like_cpp(creature: &'a mut Creature) -> Self {
        Self { creature }
    }

    /// C++ `Unit::GetSpeedRate(mtype)`: `Unit::m_speed_rate` of the incarnation.
    fn speed_rate_like_cpp(&self, move_type: UnitMoveType) -> f32 {
        self.creature.unit().speed_rate()[move_type as usize]
    }

    /// Draw access to the creature-owned runtime RNG. The state has one owner
    /// (the canonical runtime state); this borrow only reaches it.
    pub(crate) fn runtime_rng_mut(&mut self) -> &mut StdRng {
        self.creature
            .runtime_like_cpp_mut()
            .runtime_rng_like_cpp_mut()
    }

    pub(super) fn walk_speed_like_cpp(&self) -> f32 {
        (self.speed_rate_like_cpp(UnitMoveType::Walk) * 2.5).max(0.01)
    }

    pub(super) fn run_speed_like_cpp(&self) -> f32 {
        (self.speed_rate_like_cpp(UnitMoveType::Run) * 7.0).max(0.01)
    }
}

impl Deref for CreatureMovementLikeCpp<'_> {
    type Target = Creature;

    fn deref(&self) -> &Creature {
        self.creature
    }
}

impl DerefMut for CreatureMovementLikeCpp<'_> {
    fn deref_mut(&mut self) -> &mut Creature {
        self.creature
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
pub(crate) const NOMINAL_MELEE_RANGE_LIKE_CPP: f32 = 5.0;

/// C++ `Position::GetAbsoluteAngle`: the world bearing from `from` to `to`.
pub(crate) fn absolute_angle_like_cpp(from: Position, to: Position) -> f32 {
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

fn position_to_i32_tuple(position: Position) -> (i32, i32, i32) {
    (position.x as i32, position.y as i32, position.z as i32)
}
