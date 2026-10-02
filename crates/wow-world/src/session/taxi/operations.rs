//! Represented taxi, transport and vehicle operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_vehicle_store(&mut self, store: Arc<VehicleStore>) {
        self.catalogs.vehicle_store = Some(store);
    }
    pub fn set_vehicle_seat_store(&mut self, store: Arc<VehicleSeatStore>) {
        self.catalogs.vehicle_seat_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_vehicle_template_store(&mut self, store: Arc<VehicleTemplateStoreLikeCpp>) {
        self.catalogs.vehicle_template_store = Some(store);
    }
    pub fn set_vehicle_accessory_store(&mut self, store: Arc<VehicleAccessoryStoreLikeCpp>) {
        self.catalogs.vehicle_accessory_store = Some(store);
    }
    #[allow(dead_code)]
    pub(crate) fn represented_taxi_edge_distance_like_cpp(
        &self,
        destination_has_required_team_flag: bool,
        destination_condition_id: u32,
        distance: u32,
    ) -> u32 {
        if !destination_has_required_team_flag {
            return u16::MAX as u32;
        }

        if !self.represented_meets_player_condition_id_like_cpp(destination_condition_id) {
            return u16::MAX as u32;
        }

        distance
    }
    #[allow(dead_code)]
    pub(crate) fn represented_taxi_usable_mount_displays_like_cpp(
        &self,
        flying_mount_id: u32,
    ) -> Vec<i32> {
        let Some(mount) = self
            .catalogs
            .mount_store
            .as_ref()
            .and_then(|store| store.get_by_id(flying_mount_id))
        else {
            return Vec::new();
        };

        if !self
            .known_spells_like_cpp()
            .contains(&mount.source_spell_id)
        {
            return Vec::new();
        }

        let Some(displays) = self
            .catalogs
            .mount_x_display_store
            .as_ref()
            .and_then(|store| store.displays_for_mount_like_cpp(mount.id))
        else {
            return Vec::new();
        };

        displays
            .iter()
            .filter(|display| {
                display.player_condition_id == 0
                    || self.represented_mount_x_display_usable_like_cpp(display.player_condition_id)
            })
            .map(|display| display.creature_display_info_id)
            .collect()
    }
    pub(in crate::session) fn record_represented_vehicle_seat_action_like_cpp(
        &mut self,
        action: crate::handlers::vehicle::VehicleHandlerAction,
    ) -> bool {
        match action {
            crate::handlers::vehicle::VehicleHandlerAction::ChangeSeat { seat_id, next } => {
                #[cfg(test)]
                self.fixtures
                    .vehicles
                    .represented_vehicle_seat_change_requests_like_cpp
                    .push(RepresentedVehicleSeatChangeRequestLikeCpp { seat_id, next });
                #[cfg(not(test))]
                let _ = (seat_id, next);
                true
            }
            crate::handlers::vehicle::VehicleHandlerAction::ValidateMovementAndChangeSeat {
                next,
            } => {
                #[cfg(test)]
                self.fixtures
                    .vehicles
                    .represented_vehicle_seat_change_requests_like_cpp
                    .push(RepresentedVehicleSeatChangeRequestLikeCpp { seat_id: -1, next });
                #[cfg(not(test))]
                let _ = next;
                true
            }
            crate::handlers::vehicle::VehicleHandlerAction::HandleSpellClick {
                vehicle,
                seat_id,
            } => {
                let plan = self
                    .represented_handle_spell_click_plan_with_seat_like_cpp(vehicle, Some(seat_id));
                if plan.casts.is_empty() {
                    return false;
                }
                #[cfg(test)]
                self.fixtures
                    .vehicles
                    .represented_vehicle_seat_spell_click_requests_like_cpp
                    .push(RepresentedVehicleSeatSpellClickRequestLikeCpp {
                        vehicle_guid: vehicle,
                        seat_id,
                        planned_casts: plan.casts.len(),
                        exact_context_unrepresented: plan.exact_context_unrepresented,
                    });
                true
            }
            _ => false,
        }
    }
    pub(crate) fn represented_ride_vehicle_interact_like_cpp(
        &mut self,
        vehicle_guid: ObjectGuid,
    ) -> bool {
        const INTERACTION_DISTANCE_LIKE_CPP: f32 = 5.0;

        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(player_position) = crate::session::hub_ref(self).player_position_like_cpp() else {
            return false;
        };
        let Some(registry) = self.player_registry() else {
            return false;
        };
        let Some(target) = registry.vehicle_interaction_snapshot(vehicle_guid) else {
            return false;
        };

        let target_is_player_with_vehicle_kit = vehicle_guid.is_player() && target.has_vehicle_kit;
        let target_is_raid_member =
            self.represented_player_is_same_raid_with_like_cpp(player_guid, vehicle_guid);
        let current_map_id = self.core.player_map_id_like_cpp();
        let current_instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let target_is_within_interaction_distance = target.map_id == current_map_id
            && target.instance_id == current_instance_id
            && target
                .position
                .is_within_dist(&player_position, INTERACTION_DISTANCE_LIKE_CPP);
        let current_map_entry = self
            .catalogs
            .map_store()
            .and_then(|store| store.get(u32::from(current_map_id)));
        let map_exists = current_map_entry.is_some();
        let map_is_battle_arena =
            current_map_entry.is_some_and(|entry| entry.instance_type == wow_data::map::MAP_ARENA);

        let action = crate::handlers::vehicle::ride_vehicle_interact_action_like_cpp(
            vehicle_guid,
            target_is_player_with_vehicle_kit,
            target_is_raid_member,
            target_is_within_interaction_distance,
            map_exists,
            map_is_battle_arena,
        );
        match action {
            crate::handlers::vehicle::VehicleHandlerAction::EnterVehicle { vehicle } => {
                #[cfg(test)]
                self.fixtures
                    .vehicles
                    .represented_vehicle_enter_requests_like_cpp
                    .push(RepresentedVehicleEnterRequestLikeCpp {
                        vehicle_guid: vehicle,
                    });
                #[cfg(not(test))]
                let _ = vehicle;
                true
            }
            _ => false,
        }
    }
    pub(crate) fn represented_player_reject_battleground_object_vehicle_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        if crate::session::hub_ref(self)
            .player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_none()
        {
            return false;
        }

        self.world_entities.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::Vehicle,
            },
        );
        true
    }
    pub(crate) fn represented_set_taxi_benchmark_mode_like_cpp(&mut self, enable: bool) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        let changed = self
            .core
            .mutate_canonical_player_like_cpp(|player| {
                if enable {
                    player.set_player_flag(PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP);
                } else {
                    player.remove_player_flag(PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP);
                }
            })
            .is_some();

        if changed {
            self.sync_player_registry_state_like_cpp();
        }

        self.core
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP)
            .unwrap_or(false)
            == enable
    }
    #[cfg(test)]
    pub(crate) fn is_in_taxi_flight_like_cpp(&self) -> bool {
        crate::session::hub_ref(self)
            .resolved_is_in_taxi_flight_like_cpp()
            .expect("test Player taxi owner must resolve")
    }
    pub(crate) fn set_player_transport_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        crate::session::hub_mut(self).set_player_transport_guid_like_cpp(guid)
    }
    pub(crate) fn should_send_init_transport_like_cpp(
        &self,
        transport_guid: ObjectGuid,
        transport_phase_shift: &PhaseShift,
    ) -> bool {
        crate::session::hub_ref(self).player_transport_guid_like_cpp() != Some(transport_guid)
            && self.can_see_phase_shift_like_cpp(transport_phase_shift)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/taxi/operations/f3_shims.rs"]
mod f3_shims;
