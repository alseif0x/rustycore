//! Taxi route and mount admission, vehicle interaction and represented action recording.

use super::*;

impl WorldSession {
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
    pub(crate) fn represented_current_vehicle_seat_can_switch_from_like_cpp(&self) -> bool {
        self.player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_some_and(wow_data::vehicle_seat_flags_can_switch_from_seat_like_cpp)
    }
    /// C++ `MovementHandler.cpp:408-421` allows a passenger to update only its
    /// facing when the current `VehicleSeatEntry` has `ALLOW_TURNING`.
    pub(crate) fn represented_current_vehicle_seat_allows_turning_like_cpp(&self) -> bool {
        self.player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_some_and(wow_data::vehicle_seat_flags_allow_turning_like_cpp)
    }
    pub(in crate::session) fn represented_vehicle_base_guid_for_switch_like_cpp(
        &self,
    ) -> Option<ObjectGuid> {
        if self
            .player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_none()
        {
            return None;
        }
        self.player_moved_unit_guid_like_cpp()
    }
    pub(in crate::session) fn record_represented_vehicle_seat_action_like_cpp(
        &mut self,
        action: crate::handlers::vehicle::VehicleHandlerAction,
    ) -> bool {
        match action {
            crate::handlers::vehicle::VehicleHandlerAction::ChangeSeat { seat_id, next } => {
                #[cfg(test)]
                self.represented_vehicle_seat_change_requests_like_cpp
                    .push(RepresentedVehicleSeatChangeRequestLikeCpp { seat_id, next });
                #[cfg(not(test))]
                let _ = (seat_id, next);
                true
            }
            crate::handlers::vehicle::VehicleHandlerAction::ValidateMovementAndChangeSeat {
                next,
            } => {
                #[cfg(test)]
                self.represented_vehicle_seat_change_requests_like_cpp
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
                self.represented_vehicle_seat_spell_click_requests_like_cpp
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
        let Some(player_position) = self.player_position_like_cpp() else {
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
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let target_is_within_interaction_distance = target.map_id == current_map_id
            && target.instance_id == current_instance_id
            && target
                .position
                .is_within_dist(&player_position, INTERACTION_DISTANCE_LIKE_CPP);
        let current_map_entry = self
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
                self.represented_vehicle_enter_requests_like_cpp.push(
                    RepresentedVehicleEnterRequestLikeCpp {
                        vehicle_guid: vehicle,
                    },
                );
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
        if self
            .player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_none()
        {
            return false;
        }

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::Vehicle,
            },
        );
        true
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_activate_taxi_like_cpp(
        &mut self,
        request: RepresentedActivateTaxiLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_activate_taxi_requests_like_cpp
            .push(request);
    }
}
