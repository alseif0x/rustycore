//! Vehicle regressions.
//!
//! Separated from vehicle.rs under #683.

use super::*;
use wow_core::guid::HighGuid;

fn base_guid() -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 530, 100, 0, 1)
}

fn passenger_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_global(HighGuid::Player, 0, counter)
}

fn seat(id: u32, can_enter_or_exit: bool) -> VehicleSeatInfo {
    VehicleSeatInfo {
        id,
        attachment_offset: Position::new(0.0, 0.0, 0.0, 0.0),
        can_enter_or_exit,
        usable_by_override: false,
        can_control: false,
        can_switch_from_seat: false,
        ejectable: false,
        disables_gravity: false,
        passenger_not_selectable: false,
        keep_pet: false,
    }
}

fn vehicle() -> Vehicle {
    Vehicle::new(
        base_guid(),
        TypeId::Unit,
        Position::new(10.0, 20.0, 30.0, 1.0),
        123,
        456,
        [
            (0, seat(1000, true), VehicleSeatAddon::default()),
            (1, seat(1001, false), VehicleSeatAddon::default()),
            (
                2,
                VehicleSeatInfo {
                    id: 1002,
                    attachment_offset: Position::new(2.0, 0.0, 0.0, 0.0),
                    can_enter_or_exit: false,
                    usable_by_override: true,
                    can_control: true,
                    can_switch_from_seat: false,
                    ejectable: false,
                    disables_gravity: false,
                    passenger_not_selectable: false,
                    keep_pet: false,
                },
                VehicleSeatAddon::default(),
            ),
        ],
    )
}

mod scenarios;
