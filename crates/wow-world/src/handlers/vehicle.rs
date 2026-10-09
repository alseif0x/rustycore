//! Vehicle packet handler bodies.
//!
//! C++ refs:
//! - `WorldSession::HandleMoveDismissVehicle`
//! - `WorldSession::HandleRequestVehiclePrevSeat`
//! - `WorldSession::HandleRequestVehicleNextSeat`
//! - `WorldSession::HandleMoveChangeVehicleSeats`
//! - `WorldSession::HandleRequestVehicleSwitchSeat`
//! - `WorldSession::HandleRideVehicleInteract`
//! - `WorldSession::HandleEjectPassenger`
//! - `WorldSession::HandleRequestVehicleExit`
//!
//! `#1263 F5 remaining families`: all eight registrations now live in the
//! `ApplicationVehicle` area registrar (`wow-world-application`); the bodies
//! stay here.

use wow_core::ObjectGuid;
use wow_packet::packets::vehicle::{
    MoveChangeVehicleSeats, MoveDismissVehicle, RequestVehicleSwitchSeat, RideVehicleInteract,
};

use crate::session::WorldSession;

pub use wow_world_application::{
    VehicleHandlerAction, eject_passenger_action_like_cpp,
    move_change_vehicle_seats_action_like_cpp, move_dismiss_vehicle_action_like_cpp,
    request_adjacent_vehicle_seat_action_like_cpp, request_vehicle_exit_action_like_cpp,
    request_vehicle_switch_seat_action_like_cpp, ride_vehicle_interact_action_like_cpp,
};

#[cfg(test)]
mod test_shims;

impl WorldSession {
    /// C++ `HandleMoveDismissVehicle`.
    ///
    /// Full `Player::ExitVehicle` requires live charm/passenger ownership. Until that runtime
    /// exists, this handler mirrors the C++ early-return shape through represented charm state,
    /// then copies the validated movement status before represented exit cleanup.
    pub async fn handle_move_dismiss_vehicle(&mut self, mut packet: MoveDismissVehicle) {
        self.represented_move_dismiss_vehicle_like_cpp(&mut packet.status);
    }

    /// C++ `HandleMoveChangeVehicleSeats`.
    pub async fn handle_move_change_vehicle_seats(&mut self, mut packet: MoveChangeVehicleSeats) {
        self.represented_move_change_vehicle_seats_like_cpp(
            &mut packet.status,
            packet.dst_vehicle,
            packet.dst_seat_index,
        );
    }

    /// C++ `HandleRequestVehicleSwitchSeat`.
    pub async fn handle_request_vehicle_switch_seat(&mut self, packet: RequestVehicleSwitchSeat) {
        self.represented_request_vehicle_switch_seat_like_cpp(packet.vehicle, packet.seat_index);
    }

    /// C++ `HandleRideVehicleInteract`.
    pub async fn handle_ride_vehicle_interact(&mut self, packet: RideVehicleInteract) {
        self.represented_ride_vehicle_interact_like_cpp(packet.vehicle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::guid::HighGuid;

    fn player(counter: i64) -> ObjectGuid {
        ObjectGuid::create_global(HighGuid::Player, 0, counter)
    }

    fn creature(counter: i64) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, counter)
    }

    #[test]
    fn dismiss_vehicle_action_matches_cpp_charm_gate() {
        assert_eq!(
            move_dismiss_vehicle_action_like_cpp(ObjectGuid::EMPTY),
            VehicleHandlerAction::Noop
        );
        assert_eq!(
            move_dismiss_vehicle_action_like_cpp(creature(1)),
            VehicleHandlerAction::ValidateMovementAndExitVehicle
        );
    }

    #[test]
    fn adjacent_seat_actions_match_cpp_switch_gate() {
        assert_eq!(
            request_adjacent_vehicle_seat_action_like_cpp(false, true, true),
            VehicleHandlerAction::Noop
        );
        assert_eq!(
            request_adjacent_vehicle_seat_action_like_cpp(true, false, true),
            VehicleHandlerAction::Reject
        );
        assert_eq!(
            request_adjacent_vehicle_seat_action_like_cpp(true, true, false),
            VehicleHandlerAction::ChangeSeat {
                seat_id: -1,
                next: false,
            }
        );
    }

    #[test]
    fn move_change_vehicle_seats_action_matches_cpp_branches() {
        let base = creature(1);
        let other = creature(2);

        assert_eq!(
            move_change_vehicle_seats_action_like_cpp(
                true,
                true,
                base,
                other,
                ObjectGuid::EMPTY,
                0,
                false
            ),
            VehicleHandlerAction::Noop
        );
        assert_eq!(
            move_change_vehicle_seats_action_like_cpp(
                true,
                true,
                base,
                base,
                ObjectGuid::EMPTY,
                u8::MAX,
                false,
            ),
            VehicleHandlerAction::ValidateMovementAndChangeSeat { next: false }
        );
        assert_eq!(
            move_change_vehicle_seats_action_like_cpp(true, true, base, base, other, 3, true),
            VehicleHandlerAction::HandleSpellClick {
                vehicle: other,
                seat_id: 3,
            }
        );
    }

    #[test]
    fn switch_seat_action_matches_cpp_same_and_other_vehicle() {
        let base = creature(1);
        let other = creature(2);

        assert_eq!(
            request_vehicle_switch_seat_action_like_cpp(true, false, base, base, 1, false),
            VehicleHandlerAction::Reject
        );
        assert_eq!(
            request_vehicle_switch_seat_action_like_cpp(true, true, base, base, 1, false),
            VehicleHandlerAction::ChangeSeat {
                seat_id: 1,
                next: true,
            }
        );
        assert_eq!(
            request_vehicle_switch_seat_action_like_cpp(true, true, base, other, 2, true),
            VehicleHandlerAction::HandleSpellClick {
                vehicle: other,
                seat_id: 2,
            }
        );
    }

    #[test]
    fn ride_eject_and_exit_actions_match_cpp_gates() {
        let target = player(1);
        assert_eq!(
            ride_vehicle_interact_action_like_cpp(target, true, true, true, true, false),
            VehicleHandlerAction::EnterVehicle { vehicle: target }
        );
        assert_eq!(
            ride_vehicle_interact_action_like_cpp(target, true, false, true, true, false),
            VehicleHandlerAction::Noop
        );

        let passenger = creature(2);
        assert_eq!(
            eject_passenger_action_like_cpp(true, passenger, true, true, true),
            VehicleHandlerAction::ExitVehicle
        );
        assert_eq!(
            eject_passenger_action_like_cpp(true, passenger, true, true, false),
            VehicleHandlerAction::Reject
        );
        assert_eq!(
            eject_passenger_action_like_cpp(true, ObjectGuid::EMPTY, true, true, true),
            VehicleHandlerAction::Reject
        );

        assert_eq!(
            request_vehicle_exit_action_like_cpp(false, true),
            VehicleHandlerAction::Noop
        );
        assert_eq!(
            request_vehicle_exit_action_like_cpp(true, true),
            VehicleHandlerAction::ExitVehicle
        );
        assert_eq!(
            request_vehicle_exit_action_like_cpp(true, false),
            VehicleHandlerAction::Reject
        );
    }
}
