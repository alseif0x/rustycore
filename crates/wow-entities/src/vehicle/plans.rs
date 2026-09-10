//! Plans packets.
//!
//! Separated from vehicle.rs under #693.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleAccessoryInstallPlan {
    pub remove_all_passengers: bool,
    pub accessories: Vec<VehicleAccessory>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleAccessorySummonPlan {
    pub accessory: VehicleAccessory,
    pub add_accessory_unit_mask: bool,
    pub handle_spell_click_seat_id: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehiclePendingJoinAbort {
    pub passenger: ObjectGuid,
    pub seat_id: i8,
    pub target_vehicle_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VehiclePendingEventRemovalPlan {
    pub scheduled_aborts: Vec<VehiclePendingJoinAbort>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleRemoveAllPassengersPlan {
    pub pending_join_aborts: Vec<VehiclePendingJoinAbort>,
    pub remove_control_vehicle_auras: bool,
    pub forced_exit_passengers: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehiclePassengerAddPlan {
    pub accepted: bool,
    pub seat_id: Option<i8>,
    pub scheduled_abort: bool,
    pub displaced_passenger: Option<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehiclePassengerTransportReset {
    None,
    Reset,
    InheritBaseTransport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehiclePassengerRemovePlan {
    pub seat_id: i8,
    pub set_vehicle_none: bool,
    pub restore_gravity: bool,
    pub restore_interactible: bool,
    pub restore_npc_flag: bool,
    pub remove_charm: bool,
    pub transport_reset: VehiclePassengerTransportReset,
    pub cast_parachute: bool,
    pub call_ai_passenger_boarded: bool,
    pub call_on_remove_passenger_script: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehiclePassengerRelocation {
    pub passenger: ObjectGuid,
    pub position: Position,
    pub set_home_position: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleJoinAbortPlan {
    pub remove_pending_event: bool,
    pub remove_control_vehicle_aura: bool,
    pub despawn_accessory: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VehicleJoinExecutePlan {
    pub passenger: ObjectGuid,
    pub seat_id: i8,
    pub abort: Option<VehicleJoinAbortPlan>,
    pub pending_seat_aborts: VehiclePendingEventRemovalPlan,
    pub pending_passenger_aborts: VehiclePendingEventRemovalPlan,
    pub exit_existing_vehicle: bool,
    pub passenger_info: Option<PassengerInfo>,
    pub remove_npc_flag: bool,
    pub interrupt_generic_spell: bool,
    pub interrupt_autorepeat_spell: bool,
    pub remove_mount_interrupt_auras: bool,
    pub remove_mounted_auras: bool,
    pub player_drop_battleground_flag: bool,
    pub player_stop_casting_charm: bool,
    pub player_stop_casting_bind_sight: bool,
    pub player_cancel_expected_vehicle_ride_aura: bool,
    pub player_unsummon_temporary_pet: bool,
    pub set_disable_gravity: bool,
    pub transport_position: Option<Position>,
    pub set_vehicle_charm: bool,
    pub send_clear_target: bool,
    pub set_root_controlled: bool,
    pub launch_transport_enter_spline: bool,
    pub transfer_threat_to_vehicle: bool,
    pub call_ai_passenger_boarded: bool,
    pub call_on_add_passenger_script: bool,
    pub call_on_install_accessory_script: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleResetPlan {
    pub immunity_plan: VehicleImmunityPlan,
    pub accessory_install_plan: Option<VehicleAccessoryInstallPlan>,
    pub call_on_reset_script: bool,
}

pub fn vehicle_accessory_install_plan_like_cpp(
    base_type_id: TypeId,
    evading: bool,
    accessories: &[VehicleAccessory],
) -> VehicleAccessoryInstallPlan {
    VehicleAccessoryInstallPlan {
        remove_all_passengers: base_type_id == TypeId::Player || !evading,
        accessories: accessories
            .iter()
            .copied()
            .filter(|accessory| !evading || accessory.is_minion)
            .collect(),
    }
}
