//! Vehicle packets.
//!
//! Separated from vehicle.rs under #693.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleStatus {
    None,
    Installed,
    Uninstalling,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassengerInfo {
    pub guid: ObjectGuid,
    pub is_uninteractible: bool,
    pub is_gravity_disabled: bool,
}

impl PassengerInfo {
    pub const fn empty() -> Self {
        Self {
            guid: ObjectGuid::EMPTY,
            is_uninteractible: false,
            is_gravity_disabled: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.guid.is_empty()
    }

    pub fn reset(&mut self) {
        *self = Self::empty();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VehicleTemplate {
    pub despawn_delay_ms: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vehicle {
    base_guid: ObjectGuid,
    base_type_id: TypeId,
    base_position: Position,
    vehicle_id: u32,
    creature_entry: u32,
    pub(super) usable_seat_num: u32,
    pub(super) seats: BTreeMap<i8, VehicleSeat>,
    pub(super) status: VehicleStatus,
    pending_join_events: BTreeMap<ObjectGuid, i8>,
}

impl Vehicle {
    pub fn new(
        base_guid: ObjectGuid,
        base_type_id: TypeId,
        base_position: Position,
        vehicle_id: u32,
        creature_entry: u32,
        seat_defs: impl IntoIterator<Item = (i8, VehicleSeatInfo, VehicleSeatAddon)>,
    ) -> Self {
        let mut seats = BTreeMap::new();
        let mut usable_seat_num = 0;
        for (seat_id, seat_info, seat_addon) in seat_defs.into_iter().take(MAX_VEHICLE_SEATS) {
            if seat_info.can_enter_or_exit {
                usable_seat_num += 1;
            }
            seats.insert(seat_id, VehicleSeat::new(seat_info, seat_addon));
        }

        Self {
            base_guid,
            base_type_id,
            base_position,
            vehicle_id,
            creature_entry,
            usable_seat_num,
            seats,
            status: VehicleStatus::None,
            pending_join_events: BTreeMap::new(),
        }
    }

    pub const fn base_guid(&self) -> ObjectGuid {
        self.base_guid
    }

    pub const fn base_type_id(&self) -> TypeId {
        self.base_type_id
    }

    pub const fn base_position(&self) -> Position {
        self.base_position
    }

    pub fn set_base_position(&mut self, position: Position) {
        self.base_position = position;
    }

    pub const fn vehicle_id(&self) -> u32 {
        self.vehicle_id
    }

    pub const fn creature_entry(&self) -> u32 {
        self.creature_entry
    }

    pub const fn usable_seat_num(&self) -> u32 {
        self.usable_seat_num
    }

    pub const fn status(&self) -> VehicleStatus {
        self.status
    }

    pub fn seats(&self) -> &BTreeMap<i8, VehicleSeat> {
        &self.seats
    }

    pub fn install(&mut self) {
        self.status = VehicleStatus::Installed;
    }

    pub fn uninstall(&mut self) {
        self.status = VehicleStatus::Uninstalling;
        self.remove_all_passengers();
    }

    pub fn remove_all_passengers_plan_like_cpp(&mut self) -> VehicleRemoveAllPassengersPlan {
        let target_vehicle_available = self.status != VehicleStatus::Uninstalling;
        let pending_join_aborts = self
            .pending_join_events
            .iter()
            .map(|(passenger, seat_id)| VehiclePendingJoinAbort {
                passenger: *passenger,
                seat_id: *seat_id,
                target_vehicle_available,
            })
            .collect();
        let forced_exit_passengers = self
            .seats
            .values()
            .filter_map(|seat| (!seat.passenger.guid.is_empty()).then_some(seat.passenger.guid))
            .collect();

        self.pending_join_events.clear();
        for seat in self.seats.values_mut() {
            seat.passenger.reset();
        }

        VehicleRemoveAllPassengersPlan {
            pending_join_aborts,
            remove_control_vehicle_auras: true,
            forced_exit_passengers,
        }
    }

    pub fn install_accessory_plan_like_cpp(
        &self,
        accessory: VehicleAccessory,
    ) -> Option<VehicleAccessorySummonPlan> {
        if self.status == VehicleStatus::Uninstalling {
            return None;
        }

        Some(VehicleAccessorySummonPlan {
            accessory,
            add_accessory_unit_mask: accessory.is_minion,
            handle_spell_click_seat_id: accessory.seat_id,
        })
    }

    pub fn install_all_accessories_plan_like_cpp(
        &mut self,
        evading: bool,
        accessories: &[VehicleAccessory],
    ) -> VehicleAccessoryInstallPlan {
        let plan = vehicle_accessory_install_plan_like_cpp(self.base_type_id, evading, accessories);
        if plan.remove_all_passengers {
            self.remove_all_passengers();
        }
        plan
    }

    pub fn reset_plan_like_cpp(
        &mut self,
        evading: bool,
        base_is_alive: bool,
        is_mechanical_creature: bool,
        is_world_boss: bool,
        accessories: &[VehicleAccessory],
    ) -> Option<VehicleResetPlan> {
        if self.base_type_id != TypeId::Unit {
            return None;
        }

        let immunity_plan =
            vehicle_immunity_plan_like_cpp(self.vehicle_id, is_mechanical_creature, is_world_boss);
        let accessory_install_plan =
            base_is_alive.then(|| self.install_all_accessories_plan_like_cpp(evading, accessories));

        Some(VehicleResetPlan {
            immunity_plan,
            accessory_install_plan,
            call_on_reset_script: true,
        })
    }

    pub fn has_empty_seat(&self, seat_id: i8) -> bool {
        self.seats.get(&seat_id).is_some_and(VehicleSeat::is_empty)
            && !self.has_pending_event_for_seat(seat_id)
    }

    pub fn passenger(&self, seat_id: i8) -> Option<ObjectGuid> {
        let passenger = self.seats.get(&seat_id)?.passenger.guid;
        (!passenger.is_empty()).then_some(passenger)
    }

    pub fn seat_id_for_passenger_like_cpp(&self, passenger: ObjectGuid) -> Option<i8> {
        self.seats
            .iter()
            .find_map(|(seat_id, seat)| (seat.passenger.guid == passenger).then_some(*seat_id))
    }

    pub fn seat_info_for_passenger_like_cpp(
        &self,
        passenger: ObjectGuid,
    ) -> Option<VehicleSeatInfo> {
        self.seats
            .values()
            .find_map(|seat| (seat.passenger.guid == passenger).then_some(seat.seat_info))
    }

    pub fn seat_addon_for_passenger_like_cpp(
        &self,
        passenger: ObjectGuid,
    ) -> Option<VehicleSeatAddon> {
        self.seats
            .values()
            .find_map(|seat| (seat.passenger.guid == passenger).then_some(seat.seat_addon))
    }

    pub fn available_seat_count(&self) -> u8 {
        self.seats
            .iter()
            .filter(|(seat_id, seat)| {
                seat.is_empty()
                    && !self.has_pending_event_for_seat(**seat_id)
                    && (seat.seat_info.can_enter_or_exit || seat.seat_info.usable_by_override)
            })
            .count()
            .min(u8::MAX as usize) as u8
    }

    pub fn add_vehicle_passenger(&mut self, passenger: ObjectGuid, seat_id: i8) -> bool {
        if self.has_pending_event_for_seat(seat_id) {
            return false;
        }
        let Some(seat) = self.seats.get_mut(&seat_id) else {
            return false;
        };
        if !seat.is_empty() {
            return false;
        }

        seat.passenger.guid = passenger;
        true
    }

    pub fn add_vehicle_passenger_plan_like_cpp(
        &mut self,
        passenger: ObjectGuid,
        seat_id: i8,
    ) -> VehiclePassengerAddPlan {
        if self.status == VehicleStatus::Uninstalling {
            return VehiclePassengerAddPlan {
                accepted: false,
                seat_id: None,
                scheduled_abort: false,
                displaced_passenger: None,
            };
        }

        let selected_seat_id = if seat_id < 0 {
            self.seats
                .iter()
                .find(|(candidate_id, seat)| {
                    seat.is_empty()
                        && !self.has_pending_event_for_seat(**candidate_id)
                        && (seat.seat_info.can_enter_or_exit || seat.seat_info.usable_by_override)
                })
                .map(|(candidate_id, _)| *candidate_id)
        } else {
            self.seats.contains_key(&seat_id).then_some(seat_id)
        };

        let Some(selected_seat_id) = selected_seat_id else {
            return VehiclePassengerAddPlan {
                accepted: false,
                seat_id: None,
                scheduled_abort: true,
                displaced_passenger: None,
            };
        };

        let displaced_passenger = if seat_id >= 0 {
            self.seats.get_mut(&selected_seat_id).and_then(|seat| {
                let passenger = (!seat.passenger.guid.is_empty()).then_some(seat.passenger.guid)?;
                seat.passenger.reset();
                Some(passenger)
            })
        } else {
            None
        };

        self.add_pending_event(passenger, selected_seat_id);
        VehiclePassengerAddPlan {
            accepted: true,
            seat_id: Some(selected_seat_id),
            scheduled_abort: false,
            displaced_passenger,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn vehicle_join_execute_plan_like_cpp(
        &mut self,
        passenger: ObjectGuid,
        seat_id: i8,
        passenger_is_alive: bool,
        passenger_had_vehicle: bool,
        passenger_type_id: TypeId,
        passenger_is_uninteractible: bool,
        passenger_is_gravity_disabled: bool,
        passenger_in_battleground: bool,
        passenger_is_accessory: bool,
        base_ai_enabled: bool,
    ) -> Option<VehicleJoinExecutePlan> {
        if !self.seats.contains_key(&seat_id) {
            return None;
        }

        let pending_seat_aborts = self.remove_pending_events_for_seat_plan_like_cpp(seat_id);
        let pending_passenger_aborts =
            self.remove_pending_events_for_passenger_plan_like_cpp(passenger);

        if !passenger_is_alive {
            return Some(VehicleJoinExecutePlan {
                passenger,
                seat_id,
                abort: Some(VehicleJoinAbortPlan {
                    remove_pending_event: true,
                    remove_control_vehicle_aura: true,
                    despawn_accessory: passenger_is_accessory,
                }),
                pending_seat_aborts,
                pending_passenger_aborts,
                exit_existing_vehicle: false,
                passenger_info: None,
                remove_npc_flag: false,
                interrupt_generic_spell: false,
                interrupt_autorepeat_spell: false,
                remove_mount_interrupt_auras: false,
                remove_mounted_auras: false,
                player_drop_battleground_flag: false,
                player_stop_casting_charm: false,
                player_stop_casting_bind_sight: false,
                player_cancel_expected_vehicle_ride_aura: false,
                player_unsummon_temporary_pet: false,
                set_disable_gravity: false,
                transport_position: None,
                set_vehicle_charm: false,
                send_clear_target: false,
                set_root_controlled: false,
                launch_transport_enter_spline: false,
                transfer_threat_to_vehicle: false,
                call_ai_passenger_boarded: false,
                call_on_add_passenger_script: false,
                call_on_install_accessory_script: false,
            });
        }

        let seat = self.seats.get_mut(&seat_id)?;
        let passenger_info = PassengerInfo {
            guid: passenger,
            is_uninteractible: passenger_is_uninteractible,
            is_gravity_disabled: passenger_is_gravity_disabled,
        };
        seat.passenger = passenger_info;

        let remove_npc_flag = if seat.seat_info.can_enter_or_exit {
            debug_assert!(self.usable_seat_num > 0);
            self.usable_seat_num = self.usable_seat_num.saturating_sub(1);
            self.usable_seat_num == 0
        } else {
            false
        };
        let passenger_is_player = passenger_type_id == TypeId::Player;
        let player_unsummon_temporary_pet = passenger_is_player && !seat.seat_info.keep_pet;
        let transport_position = Position::new(
            seat.seat_info.attachment_offset.x,
            seat.seat_info.attachment_offset.y,
            seat.seat_info.attachment_offset.z,
            seat.seat_addon.seat_orientation_offset,
        );
        let set_vehicle_charm =
            self.base_type_id == TypeId::Unit && passenger_is_player && seat.seat_info.can_control;

        Some(VehicleJoinExecutePlan {
            passenger,
            seat_id,
            abort: None,
            pending_seat_aborts,
            pending_passenger_aborts,
            exit_existing_vehicle: passenger_had_vehicle,
            passenger_info: Some(passenger_info),
            remove_npc_flag,
            interrupt_generic_spell: true,
            interrupt_autorepeat_spell: true,
            remove_mount_interrupt_auras: true,
            remove_mounted_auras: true,
            player_drop_battleground_flag: passenger_is_player && passenger_in_battleground,
            player_stop_casting_charm: passenger_is_player,
            player_stop_casting_bind_sight: passenger_is_player,
            player_cancel_expected_vehicle_ride_aura: passenger_is_player,
            player_unsummon_temporary_pet,
            set_disable_gravity: seat.seat_info.disables_gravity,
            transport_position: Some(transport_position),
            set_vehicle_charm,
            send_clear_target: true,
            set_root_controlled: true,
            launch_transport_enter_spline: true,
            transfer_threat_to_vehicle: true,
            call_ai_passenger_boarded: self.base_type_id == TypeId::Unit && base_ai_enabled,
            call_on_add_passenger_script: self.base_type_id == TypeId::Unit,
            call_on_install_accessory_script: self.base_type_id == TypeId::Unit
                && passenger_is_accessory,
        })
    }

    pub fn remove_passenger(&mut self, passenger: ObjectGuid) -> Option<i8> {
        self.remove_passenger_plan_like_cpp(passenger, TypeId::Unit, false, false, false, false)
            .map(|plan| plan.seat_id)
    }

    pub fn remove_passenger_plan_like_cpp(
        &mut self,
        passenger: ObjectGuid,
        passenger_type_id: TypeId,
        base_is_in_world: bool,
        base_has_transport: bool,
        passenger_is_flying: bool,
        base_ai_enabled: bool,
    ) -> Option<VehiclePassengerRemovePlan> {
        let (&seat_id, seat) = self
            .seats
            .iter_mut()
            .find(|(_, seat)| seat.passenger.guid == passenger)?;
        let passenger_info = seat.passenger;

        let restore_npc_flag = if seat.seat_info.can_enter_or_exit {
            self.usable_seat_num = self.usable_seat_num.saturating_add(1);
            self.usable_seat_num != 0
        } else {
            false
        };
        let restore_gravity =
            seat.seat_info.disables_gravity && !passenger_info.is_gravity_disabled;
        let restore_interactible =
            seat.seat_info.passenger_not_selectable && !passenger_info.is_uninteractible;
        let remove_charm = self.base_type_id == TypeId::Unit
            && passenger_type_id == TypeId::Player
            && seat.seat_info.can_control;
        let transport_reset = if base_is_in_world {
            if base_has_transport {
                VehiclePassengerTransportReset::InheritBaseTransport
            } else {
                VehiclePassengerTransportReset::Reset
            }
        } else {
            VehiclePassengerTransportReset::None
        };

        seat.passenger.reset();

        Some(VehiclePassengerRemovePlan {
            seat_id,
            set_vehicle_none: true,
            restore_gravity,
            restore_interactible,
            restore_npc_flag,
            remove_charm,
            transport_reset,
            cast_parachute: passenger_is_flying,
            call_ai_passenger_boarded: self.base_type_id == TypeId::Unit && base_ai_enabled,
            call_on_remove_passenger_script: self.base_type_id == TypeId::Unit,
        })
    }

    pub fn remove_all_passengers(&mut self) {
        let _ = self.remove_all_passengers_plan_like_cpp();
    }

    pub fn is_vehicle_in_use(&self) -> bool {
        self.seats.values().any(|seat| !seat.is_empty())
    }

    pub fn is_controllable_vehicle(&self) -> bool {
        self.seats.values().any(|seat| seat.seat_info.can_control)
    }

    pub fn add_pending_event(&mut self, passenger: ObjectGuid, seat_id: i8) {
        self.pending_join_events.insert(passenger, seat_id);
    }

    pub fn remove_pending_events_for_passenger_plan_like_cpp(
        &mut self,
        passenger: ObjectGuid,
    ) -> VehiclePendingEventRemovalPlan {
        let Some(seat_id) = self.pending_join_events.remove(&passenger) else {
            return VehiclePendingEventRemovalPlan::default();
        };

        VehiclePendingEventRemovalPlan {
            scheduled_aborts: vec![VehiclePendingJoinAbort {
                passenger,
                seat_id,
                target_vehicle_available: true,
            }],
        }
    }

    pub fn remove_pending_events_for_passenger(&mut self, passenger: ObjectGuid) {
        let _ = self.remove_pending_events_for_passenger_plan_like_cpp(passenger);
    }

    pub fn remove_pending_events_for_seat_plan_like_cpp(
        &mut self,
        seat_id: i8,
    ) -> VehiclePendingEventRemovalPlan {
        let scheduled_aborts = self
            .pending_join_events
            .iter()
            .filter_map(|(passenger, pending_seat)| {
                (*pending_seat == seat_id).then_some(VehiclePendingJoinAbort {
                    passenger: *passenger,
                    seat_id: *pending_seat,
                    target_vehicle_available: true,
                })
            })
            .collect::<Vec<_>>();
        self.pending_join_events
            .retain(|_, pending_seat| *pending_seat != seat_id);

        VehiclePendingEventRemovalPlan { scheduled_aborts }
    }

    pub fn remove_pending_events_for_seat(&mut self, seat_id: i8) {
        let _ = self.remove_pending_events_for_seat_plan_like_cpp(seat_id);
    }

    pub fn has_pending_event_for_seat(&self, seat_id: i8) -> bool {
        self.pending_join_events
            .values()
            .any(|pending_seat| *pending_seat == seat_id)
    }

    pub fn next_empty_seat(&self, seat_id: i8, next: bool) -> Option<i8> {
        if !self.seats.contains_key(&seat_id) || self.seats.is_empty() {
            return None;
        }

        let seat_ids: Vec<i8> = self.seats.keys().copied().collect();
        let mut index = seat_ids.iter().position(|known| *known == seat_id)?;
        loop {
            index = if next {
                (index + 1) % seat_ids.len()
            } else if index == 0 {
                seat_ids.len() - 1
            } else {
                index - 1
            };

            let candidate = seat_ids[index];
            if candidate == seat_id {
                return None;
            }
            let seat = self.seats.get(&candidate)?;
            if seat.is_empty()
                && !self.has_pending_event_for_seat(candidate)
                && (seat.seat_info.can_enter_or_exit || seat.seat_info.usable_by_override)
            {
                return Some(candidate);
            }
        }
    }

    pub fn relocate_passengers_plan_like_cpp(
        &self,
        passenger_transport_offsets: &[(ObjectGuid, Position)],
    ) -> Vec<VehiclePassengerRelocation> {
        self.seats
            .values()
            .filter_map(|seat| {
                let passenger = seat.passenger.guid;
                if passenger.is_empty() {
                    return None;
                }
                let (_, offset) = passenger_transport_offsets
                    .iter()
                    .find(|(candidate, _)| *candidate == passenger)?;
                Some(VehiclePassengerRelocation {
                    passenger,
                    position: self.calculate_passenger_position(*offset),
                    set_home_position: false,
                })
            })
            .collect()
    }

    pub fn debug_info_like_cpp(&self) -> String {
        let mut output = String::from("Vehicle seats:\n");
        for (seat_id, seat) in &self.seats {
            let passenger = if seat.is_empty() {
                "empty".to_string()
            } else {
                seat.passenger.guid.to_string()
            };
            output.push_str(&format!("seat {seat_id}: {passenger}\n"));
        }

        output.push_str("Vehicle pending events:");
        if self.pending_join_events.is_empty() {
            output.push_str(" none");
        } else {
            output.push('\n');
            for (passenger, seat_id) in &self.pending_join_events {
                output.push_str(&format!("seat {seat_id}: {passenger}\n"));
            }
        }
        output
    }

    pub fn calculate_passenger_position(&self, offset: Position) -> Position {
        calculate_passenger_position(offset, self.base_position)
    }

    pub fn calculate_passenger_offset(&self, global: Position) -> Position {
        calculate_passenger_offset(global, self.base_position)
    }
}
