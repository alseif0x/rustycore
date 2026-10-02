// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use tracing::trace;
use wow_constants::{ClientOpcodes, MovementFlag, ServerOpcodes};
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

#[cfg(any(test, feature = "test-fixtures"))]
pub fn canonical_taxi_flight_node_like_cpp(
    node: RepresentedTaxiFlightNodeLikeCpp,
) -> wow_entities::PlayerTaxiFlightNodeLikeCpp {
    wow_entities::PlayerTaxiFlightNodeLikeCpp {
        map_id: node.map_id,
        position: node.position,
        teleport_flag: node.teleport_flag,
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub fn represented_taxi_flight_node_like_cpp(
    node: wow_entities::PlayerTaxiFlightNodeLikeCpp,
) -> RepresentedTaxiFlightNodeLikeCpp {
    RepresentedTaxiFlightNodeLikeCpp {
        map_id: node.map_id,
        position: node.position,
        teleport_flag: node.teleport_flag,
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub fn canonical_taxi_flight_state_like_cpp(
    flight: RepresentedTaxiFlightStateLikeCpp,
) -> wow_entities::PlayerTaxiFlightStateLikeCpp {
    wow_entities::PlayerTaxiFlightStateLikeCpp {
        current_node: canonical_taxi_flight_node_like_cpp(flight.current_node),
        node_after_teleport: flight
            .node_after_teleport
            .map(canonical_taxi_flight_node_like_cpp),
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub fn represented_taxi_flight_state_like_cpp(
    flight: wow_entities::PlayerTaxiFlightStateLikeCpp,
) -> RepresentedTaxiFlightStateLikeCpp {
    RepresentedTaxiFlightStateLikeCpp {
        current_node: represented_taxi_flight_node_like_cpp(flight.current_node),
        node_after_teleport: flight
            .node_after_teleport
            .map(represented_taxi_flight_node_like_cpp),
    }
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

pub fn movement_speed_ack_move_type_like_cpp(
    opcode: ClientOpcodes,
) -> Option<UnitMoveTypeLikeCpp> {
    match opcode {
        ClientOpcodes::MoveForceWalkSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Walk),
        ClientOpcodes::MoveForceRunSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Run),
        ClientOpcodes::MoveForceRunBackSpeedChangeAck => Some(UnitMoveTypeLikeCpp::RunBack),
        ClientOpcodes::MoveForceSwimSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Swim),
        ClientOpcodes::MoveForceSwimBackSpeedChangeAck => Some(UnitMoveTypeLikeCpp::SwimBack),
        ClientOpcodes::MoveForceTurnRateChangeAck => Some(UnitMoveTypeLikeCpp::TurnRate),
        ClientOpcodes::MoveForceFlightSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Flight),
        ClientOpcodes::MoveForceFlightBackSpeedChangeAck => Some(UnitMoveTypeLikeCpp::FlightBack),
        ClientOpcodes::MoveForcePitchRateChangeAck => Some(UnitMoveTypeLikeCpp::PitchRate),
        _ => None,
    }
}

pub fn player_movement_speed_opcodes_like_cpp(
    move_type: UnitMoveTypeLikeCpp,
) -> Option<(ServerOpcodes, ServerOpcodes)> {
    match move_type {
        UnitMoveTypeLikeCpp::Run => Some((
            ServerOpcodes::MoveSetRunSpeed,
            ServerOpcodes::MoveUpdateRunSpeed,
        )),
        UnitMoveTypeLikeCpp::Flight => Some((
            ServerOpcodes::MoveSetFlightSpeed,
            ServerOpcodes::MoveUpdateFlightSpeed,
        )),
        UnitMoveTypeLikeCpp::Swim => Some((
            ServerOpcodes::MoveSetSwimSpeed,
            ServerOpcodes::MoveUpdateSwimSpeed,
        )),
        UnitMoveTypeLikeCpp::RunBack => Some((
            ServerOpcodes::MoveSetRunBackSpeed,
            ServerOpcodes::MoveUpdateRunBackSpeed,
        )),
        UnitMoveTypeLikeCpp::SwimBack => Some((
            ServerOpcodes::MoveSetSwimBackSpeed,
            ServerOpcodes::MoveUpdateSwimBackSpeed,
        )),
        UnitMoveTypeLikeCpp::FlightBack => Some((
            ServerOpcodes::MoveSetFlightBackSpeed,
            ServerOpcodes::MoveUpdateFlightBackSpeed,
        )),
        _ => None,
    }
}

pub fn creature_movement_spline_speed_opcode_like_cpp(
    move_type: UnitMoveTypeLikeCpp,
) -> Option<ServerOpcodes> {
    Some(match move_type {
        UnitMoveTypeLikeCpp::Walk => ServerOpcodes::MoveSplineSetWalkSpeed,
        UnitMoveTypeLikeCpp::Run => ServerOpcodes::MoveSplineSetRunSpeed,
        UnitMoveTypeLikeCpp::RunBack => ServerOpcodes::MoveSplineSetRunBackSpeed,
        UnitMoveTypeLikeCpp::Swim => ServerOpcodes::MoveSplineSetSwimSpeed,
        UnitMoveTypeLikeCpp::SwimBack => ServerOpcodes::MoveSplineSetSwimBackSpeed,
        UnitMoveTypeLikeCpp::TurnRate => ServerOpcodes::MoveSplineSetTurnRate,
        UnitMoveTypeLikeCpp::Flight => ServerOpcodes::MoveSplineSetFlightSpeed,
        UnitMoveTypeLikeCpp::FlightBack => ServerOpcodes::MoveSplineSetFlightBackSpeed,
        UnitMoveTypeLikeCpp::PitchRate => ServerOpcodes::MoveSplineSetPitchRate,
    })
}

pub const PLAYER_BASE_MOVE_SPEED_LIKE_CPP: [f32; UnitMoveTypeLikeCpp::COUNT] = [
    2.5,      // MOVE_WALK
    7.0,      // MOVE_RUN
    4.5,      // MOVE_RUN_BACK
    4.722222, // MOVE_SWIM
    2.5,      // MOVE_SWIM_BACK
    3.141594, // MOVE_TURN_RATE
    7.0,      // MOVE_FLIGHT
    4.5,      // MOVE_FLIGHT_BACK
    3.14,     // MOVE_PITCH_RATE
];

impl crate::session::HubMut<'_> {
    pub fn set_represented_can_fly_like_cpp(&mut self, enable: bool) -> bool {
        let Some(mut movement_flags) = self.shared().resolved_player_movement_flags_like_cpp()
        else {
            return false;
        };
        let currently_enabled = movement_flags.contains(MovementFlag::CAN_FLY);
        if enable == currently_enabled {
            return false;
        }

        if enable {
            movement_flags.insert(MovementFlag::CAN_FLY);
            movement_flags.remove(MovementFlag::SWIMMING | MovementFlag::SPLINE_ELEVATION);
        } else {
            movement_flags.remove(MovementFlag::CAN_FLY | MovementFlag::MASK_MOVING_FLY);
            if let Some(position) = self.shared().player_position_like_cpp() {
                self.set_fall_information_like_cpp(0, position.z);
            }
        }
        self.set_player_movement_flags_like_cpp(movement_flags);

        self.send_player_move_set_flag_like_cpp(if enable {
            ServerOpcodes::MoveSetCanFly
        } else {
            ServerOpcodes::MoveUnsetCanFly
        });
        true
    }

    pub fn set_represented_can_swim_to_fly_transition_like_cpp(
        &mut self,
        enable: bool,
    ) -> bool {
        let canonical_changed = self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_can_transition_between_swim_and_fly_like_cpp(enable)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let changed = canonical_changed.unwrap_or_else(|| {
            if self.core.player_handle_like_cpp.is_some()
                || self
                    .fixtures
                    .movement
                    .represented_can_swim_to_fly_transition_like_cpp
                    == enable
            {
                return false;
            }
            self.fixtures
                .movement
                .represented_can_swim_to_fly_transition_like_cpp = enable;
            true
        });
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let Some(changed) = canonical_changed else {
            return false;
        };
        if !changed {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical_changed.is_some() {
            self.fixtures
                .movement
                .represented_can_swim_to_fly_transition_like_cpp = enable;
        }
        self.send_player_move_set_flag_like_cpp(if enable {
            ServerOpcodes::MoveEnableTransitionBetweenSwimAndFly
        } else {
            ServerOpcodes::MoveDisableTransitionBetweenSwimAndFly
        });
        true
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::MovementState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_out_of_bounds_like_cpp(&self) -> bool {
        self.player_out_of_bounds_like_cpp
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_min_height_like_cpp(&self, position: wow_core::Position) -> f32 {
        let map_id = self.core.player_map_id_like_cpp();
        self.core
            .map_manager
            .as_ref()
            .and_then(|manager| {
                manager
                    .read()
                    .ok()
                    .map(|manager| manager.min_height_like_cpp(map_id, 0, position.x, position.y))
            })
            .unwrap_or(crate::map_manager::DEFAULT_MIN_HEIGHT_LIKE_CPP)
    }

    pub fn trace_anticheat_violation_like_cpp(
        &self,
        rule: &'static str,
        opcode: Option<ClientOpcodes>,
        severity: &'static str,
    ) {
        trace!(
            target: "anticheat.violation",
            rule,
            account = self.core.account_id,
            character = ?self.core.player_guid(),
            ?opcode,
            severity,
            "anticheat.violation"
        );
    }
}

pub const TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP: TeleportToOptionsLikeCpp = 0x08;
pub const TELE_TO_SEAMLESS_LIKE_CPP: TeleportToOptionsLikeCpp = 0x80;
