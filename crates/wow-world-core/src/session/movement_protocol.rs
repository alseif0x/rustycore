// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;

pub type TeleportToOptionsLikeCpp = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveSplineDoneTaxiActionLikeCpp {
    InvalidMovement,
    InProgressNoFlightGenerator,
    InProgressNoTeleport,
    TeleportRequested,
    FinalCleanup,
    IgnoredUnexpectedFinalPath,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentedTaxiFlightNodeLikeCpp {
    pub map_id: u16,
    pub position: wow_core::Position,
    pub teleport_flag: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveSplineDoneTaxiEventLikeCpp {
    pub spline_id: i32,
    pub action: MoveSplineDoneTaxiActionLikeCpp,
    pub destination_node_id: Option<u32>,
    pub teleport_map_id: Option<u16>,
    pub teleport_position: Option<wow_core::Position>,
    pub honorless_target_cast: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveTeleportAckActionLikeCpp {
    NotBeingTeleportedNear,
    WrongMover,
    MissingDestination,
    MissingPlayerOwner,
    Accepted,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveTeleportAckEventLikeCpp {
    pub mover_guid: ObjectGuid,
    pub ack_index: i32,
    pub move_time: i32,
    pub action: MoveTeleportAckActionLikeCpp,
    pub destination_map_id: Option<u16>,
    pub destination_position: Option<wow_core::Position>,
    pub old_zone_id: Option<u32>,
    pub new_zone_id: Option<u32>,
    pub new_area_id: Option<u32>,
    pub honorless_target_cast: bool,
    pub pvp_disabled: bool,
    pub pet_resummon_requested: bool,
    pub delayed_operations_processed: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentedTaxiFlightStateLikeCpp {
    pub current_node: RepresentedTaxiFlightNodeLikeCpp,
    pub node_after_teleport: Option<RepresentedTaxiFlightNodeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementAckEventLikeCpp {
    pub opcode: ClientOpcodes,
    pub mover_guid: ObjectGuid,
    pub ack_index: Option<i32>,
    pub movement_force_id: Option<ObjectGuid>,
    pub movement_force_type: Option<u8>,
    pub adjusted_time: Option<u32>,
    pub speed: Option<f32>,
    pub time_skipped: Option<u32>,
    pub spline_id: Option<i32>,
    pub accepted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementSpeedAckActionLikeCpp {
    Accepted,
    SkippedPending,
    Corrected,
    Kicked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitMoveTypeLikeCpp {
    Walk = 0,
    Run = 1,
    RunBack = 2,
    Swim = 3,
    SwimBack = 4,
    TurnRate = 5,
    Flight = 6,
    FlightBack = 7,
    PitchRate = 8,
}

impl UnitMoveTypeLikeCpp {
    pub const COUNT: usize = 9;

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementSpeedAckEventLikeCpp {
    pub opcode: ClientOpcodes,
    pub move_type: Option<UnitMoveTypeLikeCpp>,
    pub ack_speed: f32,
    pub expected_speed: Option<f32>,
    pub remaining_forced_changes: Option<u8>,
    pub action: MovementSpeedAckActionLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementFallDamageEvent {
    pub z_diff: f32,
    pub damage: u32,
    pub final_damage: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementUnderMapDamageEvent {
    pub z: f32,
    pub min_height: f32,
    pub damage: u32,
}
