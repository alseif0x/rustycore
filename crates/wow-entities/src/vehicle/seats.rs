//! Seats packets.
//!
//! Separated from vehicle.rs under #693.

use super::*;

pub const MAX_VEHICLE_SEATS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleSeatAddon {
    pub seat_orientation_offset: f32,
    pub exit_parameter_x: f32,
    pub exit_parameter_y: f32,
    pub exit_parameter_z: f32,
    pub exit_parameter_o: f32,
    pub exit_parameter: VehicleExitParameter,
}

impl Default for VehicleSeatAddon {
    fn default() -> Self {
        Self {
            seat_orientation_offset: 0.0,
            exit_parameter_x: 0.0,
            exit_parameter_y: 0.0,
            exit_parameter_z: 0.0,
            exit_parameter_o: 0.0,
            exit_parameter: VehicleExitParameter::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleSeatInfo {
    pub id: u32,
    pub attachment_offset: Position,
    pub can_enter_or_exit: bool,
    pub usable_by_override: bool,
    pub can_control: bool,
    pub can_switch_from_seat: bool,
    pub ejectable: bool,
    pub disables_gravity: bool,
    pub passenger_not_selectable: bool,
    pub keep_pet: bool,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleAccessory {
    pub accessory_entry: u32,
    pub is_minion: bool,
    pub summon_time_ms: u32,
    pub seat_id: i8,
    pub summoned_type: u8,
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
