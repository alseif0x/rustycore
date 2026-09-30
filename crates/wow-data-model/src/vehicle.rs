use wow_constants::VehicleExitParameter;
use wow_core::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleAccessory {
    pub accessory_entry: u32,
    pub is_minion: bool,
    pub summon_time_ms: u32,
    pub seat_id: i8,
    pub summoned_type: u8,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VehicleTemplate {
    pub despawn_delay_ms: i32,
}
