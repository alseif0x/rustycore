//! Vehicle packet handler registrations.
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

use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{PacketProcessing, SessionStatus};
use wow_packet::ClientPacket;

use crate::session::registry::PacketHandlerEntry;
use wow_packet::packets::vehicle::{
    EjectPassenger, MoveChangeVehicleSeats, MoveDismissVehicle, RequestVehicleExit,
    RequestVehicleNextSeat, RequestVehiclePrevSeat, RequestVehicleSwitchSeat, RideVehicleInteract,
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

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveDismissVehicle,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_dismiss_vehicle",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::vehicle::MoveDismissVehicle::read(&mut pkt) {
                    Ok(packet) => session.handle_move_dismiss_vehicle(packet).await,
                    Err(e) => tracing::warn!("Failed to read MoveDismissVehicle: {e}"),
                }
            })
        },
    }
}

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

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveChangeVehicleSeats,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_change_vehicle_seats",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::vehicle::MoveChangeVehicleSeats::read(&mut pkt) {
                    Ok(packet) => session.handle_move_change_vehicle_seats(packet).await,
                    Err(e) => tracing::warn!("Failed to read MoveChangeVehicleSeats: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestVehicleSwitchSeat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_vehicle_switch_seat",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::vehicle::RequestVehicleSwitchSeat::read(&mut pkt) {
                    Ok(packet) => session.handle_request_vehicle_switch_seat(packet).await,
                    Err(e) => tracing::warn!("Failed to read RequestVehicleSwitchSeat: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RideVehicleInteract,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_ride_vehicle_interact",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::vehicle::RideVehicleInteract::read(&mut pkt) {
                    Ok(packet) => session.handle_ride_vehicle_interact(packet).await,
                    Err(e) => tracing::warn!("Failed to read RideVehicleInteract: {e}"),
                }
            })
        },
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
