//! Seats packets.
//!
//! Separated from vehicle.rs under #693.

use super::*;

pub const MAX_VEHICLE_SEATS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleSeat {
    pub seat_info: VehicleSeatInfo,
    pub seat_addon: VehicleSeatAddon,
    pub passenger: PassengerInfo,
}

impl VehicleSeat {
    pub const fn new(seat_info: VehicleSeatInfo, seat_addon: VehicleSeatAddon) -> Self {
        Self {
            seat_info,
            seat_addon,
            passenger: PassengerInfo::empty(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.passenger.is_empty()
    }
}

pub fn vehicle_base_movement_flags_like_cpp(vehicle_flags: u32) -> MovementFlag2 {
    let mut movement_flags = MovementFlag2::empty();
    if vehicle_flags & VehicleFlag::NoStrafe as u32 != 0 {
        movement_flags |= MovementFlag2::NO_STRAFE;
    }
    if vehicle_flags & VehicleFlag::NoJumping as u32 != 0 {
        movement_flags |= MovementFlag2::NO_JUMPING;
    }
    if vehicle_flags & VehicleFlag::FullSpeedTurning as u32 != 0 {
        movement_flags |= MovementFlag2::FULL_SPEED_TURNING;
    }
    if vehicle_flags & VehicleFlag::AllowPitching as u32 != 0 {
        movement_flags |= MovementFlag2::ALWAYS_ALLOW_PITCHING;
    }
    if vehicle_flags & VehicleFlag::FullSpeedPitching as u32 != 0 {
        movement_flags |= MovementFlag2::FULL_SPEED_PITCHING;
    }
    movement_flags
}

pub fn calculate_passenger_position(offset: Position, transport: Position) -> Position {
    Position::new(
        transport.x + offset.x * transport.orientation.cos()
            - offset.y * transport.orientation.sin(),
        transport.y
            + offset.y * transport.orientation.cos()
            + offset.x * transport.orientation.sin(),
        transport.z + offset.z,
        normalize_orientation(transport.orientation + offset.orientation),
    )
}

pub fn calculate_passenger_offset(global: Position, transport: Position) -> Position {
    let mut x = global.x - transport.x;
    let mut y = global.y - transport.y;
    let z = global.z - transport.z;
    let orientation = normalize_orientation(global.orientation - transport.orientation);

    let inx = x;
    let iny = y;
    let tan = transport.orientation.tan();
    let denom = transport.orientation.cos() + transport.orientation.sin() * tan;
    y = (iny - inx * tan) / denom;
    x = (inx + iny * tan) / denom;

    Position::new(x, y, z, orientation)
}

pub(super) fn normalize_orientation(mut orientation: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    orientation %= tau;
    if orientation < 0.0 {
        orientation += tau;
    }
    orientation
}
