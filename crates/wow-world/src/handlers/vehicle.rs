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
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::vehicle::{
    EjectPassenger, MoveChangeVehicleSeats, MoveDismissVehicle, RequestVehicleExit,
    RequestVehicleNextSeat, RequestVehiclePrevSeat, RequestVehicleSwitchSeat, RideVehicleInteract,
};

use crate::session::WorldSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleHandlerAction {
    Noop,
    Reject,
    ValidateMovementAndExitVehicle,
    ChangeSeat { seat_id: i8, next: bool },
    ValidateMovementAndChangeSeat { next: bool },
    HandleSpellClick { vehicle: ObjectGuid, seat_id: i8 },
    EnterVehicle { vehicle: ObjectGuid },
    ExitVehicle,
}

pub fn move_dismiss_vehicle_action_like_cpp(charmed_vehicle: ObjectGuid) -> VehicleHandlerAction {
    if charmed_vehicle.is_empty() {
        VehicleHandlerAction::Noop
    } else {
        VehicleHandlerAction::ValidateMovementAndExitVehicle
    }
}

pub fn request_adjacent_vehicle_seat_action_like_cpp(
    has_vehicle_base: bool,
    can_switch_from_current_seat: bool,
    next: bool,
) -> VehicleHandlerAction {
    if !has_vehicle_base {
        return VehicleHandlerAction::Noop;
    }
    if !can_switch_from_current_seat {
        return VehicleHandlerAction::Reject;
    }
    VehicleHandlerAction::ChangeSeat { seat_id: -1, next }
}

pub fn move_change_vehicle_seats_action_like_cpp(
    has_vehicle_base: bool,
    can_switch_from_current_seat: bool,
    vehicle_base_guid: ObjectGuid,
    status_guid: ObjectGuid,
    dst_vehicle: ObjectGuid,
    dst_seat_index: u8,
    dst_vehicle_exists_with_empty_seat: bool,
) -> VehicleHandlerAction {
    if !has_vehicle_base {
        return VehicleHandlerAction::Noop;
    }
    if !can_switch_from_current_seat {
        return VehicleHandlerAction::Reject;
    }
    if vehicle_base_guid != status_guid {
        return VehicleHandlerAction::Noop;
    }
    if dst_vehicle.is_empty() {
        return VehicleHandlerAction::ValidateMovementAndChangeSeat {
            next: dst_seat_index != u8::MAX,
        };
    }
    if dst_vehicle_exists_with_empty_seat {
        return VehicleHandlerAction::HandleSpellClick {
            vehicle: dst_vehicle,
            seat_id: dst_seat_index as i8,
        };
    }
    VehicleHandlerAction::Noop
}

pub fn request_vehicle_switch_seat_action_like_cpp(
    has_vehicle_base: bool,
    can_switch_from_current_seat: bool,
    vehicle_base_guid: ObjectGuid,
    requested_vehicle: ObjectGuid,
    seat_index: u8,
    requested_vehicle_exists_with_empty_seat: bool,
) -> VehicleHandlerAction {
    if !has_vehicle_base {
        return VehicleHandlerAction::Noop;
    }
    if !can_switch_from_current_seat {
        return VehicleHandlerAction::Reject;
    }
    if vehicle_base_guid == requested_vehicle {
        return VehicleHandlerAction::ChangeSeat {
            seat_id: seat_index as i8,
            next: true,
        };
    }
    if requested_vehicle_exists_with_empty_seat {
        return VehicleHandlerAction::HandleSpellClick {
            vehicle: requested_vehicle,
            seat_id: seat_index as i8,
        };
    }
    VehicleHandlerAction::Noop
}

pub fn ride_vehicle_interact_action_like_cpp(
    vehicle: ObjectGuid,
    target_is_player_with_vehicle_kit: bool,
    target_is_raid_member: bool,
    target_is_within_interaction_distance: bool,
    map_exists: bool,
    map_is_battle_arena: bool,
) -> VehicleHandlerAction {
    if !target_is_player_with_vehicle_kit
        || !target_is_raid_member
        || !target_is_within_interaction_distance
        || !map_exists
        || map_is_battle_arena
    {
        return VehicleHandlerAction::Noop;
    }
    VehicleHandlerAction::EnterVehicle { vehicle }
}

pub fn eject_passenger_action_like_cpp(
    player_has_vehicle_kit: bool,
    passenger: ObjectGuid,
    passenger_found: bool,
    passenger_on_same_vehicle: bool,
    passenger_seat_is_ejectable: bool,
) -> VehicleHandlerAction {
    if !player_has_vehicle_kit
        || !passenger.is_unit()
        || !passenger_found
        || !passenger_on_same_vehicle
    {
        return VehicleHandlerAction::Reject;
    }
    if passenger_seat_is_ejectable {
        VehicleHandlerAction::ExitVehicle
    } else {
        VehicleHandlerAction::Reject
    }
}

pub fn request_vehicle_exit_action_like_cpp(
    has_vehicle: bool,
    current_seat_can_enter_or_exit: bool,
) -> VehicleHandlerAction {
    if !has_vehicle {
        return VehicleHandlerAction::Noop;
    }
    if current_seat_can_enter_or_exit {
        VehicleHandlerAction::ExitVehicle
    } else {
        VehicleHandlerAction::Reject
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveDismissVehicle,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_dismiss_vehicle",
    }
}

impl WorldSession {
    /// C++ `HandleMoveDismissVehicle`.
    ///
    /// Full `Player::ExitVehicle` requires live charm/passenger ownership. Until that runtime
    /// exists, this handler intentionally mirrors the C++ early-return shape by doing nothing
    /// when the session has no represented passenger vehicle.
    pub async fn handle_move_dismiss_vehicle(&mut self, _packet: MoveDismissVehicle) {}

    /// C++ `HandleRequestVehiclePrevSeat`.
    pub async fn handle_request_vehicle_prev_seat(&mut self, _packet: RequestVehiclePrevSeat) {}

    /// C++ `HandleRequestVehicleNextSeat`.
    pub async fn handle_request_vehicle_next_seat(&mut self, _packet: RequestVehicleNextSeat) {}

    /// C++ `HandleMoveChangeVehicleSeats`.
    pub async fn handle_move_change_vehicle_seats(&mut self, _packet: MoveChangeVehicleSeats) {}

    /// C++ `HandleRequestVehicleSwitchSeat`.
    pub async fn handle_request_vehicle_switch_seat(&mut self, _packet: RequestVehicleSwitchSeat) {}

    /// C++ `HandleRideVehicleInteract`.
    pub async fn handle_ride_vehicle_interact(&mut self, _packet: RideVehicleInteract) {}

    /// C++ `HandleEjectPassenger`.
    pub async fn handle_eject_passenger(&mut self, _packet: EjectPassenger) {}

    /// C++ `HandleRequestVehicleExit`.
    pub async fn handle_request_vehicle_exit(&mut self, _packet: RequestVehicleExit) {
        self.represented_request_vehicle_exit_like_cpp();
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestVehiclePrevSeat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_vehicle_prev_seat",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestVehicleNextSeat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_vehicle_next_seat",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveChangeVehicleSeats,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_change_vehicle_seats",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestVehicleSwitchSeat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_vehicle_switch_seat",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RideVehicleInteract,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_ride_vehicle_interact",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::EjectPassenger,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_eject_passenger",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestVehicleExit,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_vehicle_exit",
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
