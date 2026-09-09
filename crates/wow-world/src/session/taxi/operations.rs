//! Represented taxi, transport and vehicle operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn player_vehicle_seat_state_like_cpp(
        &self,
    ) -> Option<(Option<i32>, Option<u32>)> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let state = player.gameplay_state();
            (state.vehicle_seat_flags, state.vehicle_seat_id)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.player_vehicle_seat_flags_like_cpp,
                self.player_vehicle_seat_id_like_cpp,
            ));
        }
        canonical
    }
    pub(in crate::session) fn set_player_vehicle_seat_state_like_cpp(
        &mut self,
        flags: Option<i32>,
        seat_id: Option<u32>,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                let state = player.gameplay_state_mut();
                state.vehicle_seat_flags = flags;
                state.vehicle_seat_id = seat_id;
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_vehicle_seat_flags_like_cpp = flags;
            self.player_vehicle_seat_id_like_cpp = seat_id;
            return true;
        }
        canonical
    }
    pub(in crate::session) fn player_mount_vehicle_kit_snapshot_like_cpp(
        &self,
    ) -> Option<Option<Vehicle>> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.gameplay_state().mount_vehicle_kit.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_mount_vehicle_kit_like_cpp.clone());
        }
        canonical
    }
    pub(in crate::session) fn mutate_player_mount_vehicle_kit_like_cpp<R>(
        &mut self,
        update: impl FnOnce(&mut Option<Vehicle>) -> R,
    ) -> Option<R> {
        let mut update = Some(update);
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            update.take().expect("vehicle kit mutation runs once")(
                &mut player.gameplay_state_mut().mount_vehicle_kit,
            )
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(update.take().expect("vehicle kit mutation runs once")(
                &mut self.player_mount_vehicle_kit_like_cpp,
            ));
        }
        canonical
    }
    pub fn set_vehicle_store(&mut self, store: Arc<VehicleStore>) {
        self.vehicle_store = Some(store);
    }
    pub fn set_vehicle_seat_store(&mut self, store: Arc<VehicleSeatStore>) {
        self.vehicle_seat_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_vehicle_template_store(&mut self, store: Arc<VehicleTemplateStoreLikeCpp>) {
        self.vehicle_template_store = Some(store);
    }
    pub fn set_vehicle_accessory_store(&mut self, store: Arc<VehicleAccessoryStoreLikeCpp>) {
        self.vehicle_accessory_store = Some(store);
    }
    pub(in crate::session) fn create_player_mount_vehicle_kit_like_cpp(
        &mut self,
        vehicle_id: u32,
        creature_entry: u32,
    ) -> bool {
        if vehicle_id == 0 {
            return false;
        }
        let Some(player_guid) = self.player_guid() else {
            return false;
        };

        let Some(vehicle) = self
            .vehicle_store
            .as_ref()
            .and_then(|store| store.get(vehicle_id))
        else {
            #[cfg(not(test))]
            return false;
            #[cfg(test)]
            {
                if self.vehicle_store.is_some() {
                    return false;
                }
                self.player_mount_vehicle_id_like_cpp = vehicle_id;
                let _ = self.mutate_player_mount_vehicle_kit_like_cpp(|kit| *kit = None);
                self.player_mount_vehicle_accessories_like_cpp = self
                    .vehicle_accessory_store
                    .as_ref()
                    .and_then(|store| store.accessories_for_vehicle_like_cpp(None, creature_entry))
                    .map(<[VehicleAccessory]>::to_vec)
                    .unwrap_or_default();
                self.player_mount_vehicle_seat_count_like_cpp = 0;
                self.player_mount_vehicle_usable_seat_count_like_cpp = 0;
                return true;
            }
        };

        let seat_defs = self
            .vehicle_seat_store
            .as_ref()
            .map(|store| store.seat_defs_for_vehicle_like_cpp(vehicle))
            .unwrap_or_default();
        let Some(player_position) = self.player_position_like_cpp() else {
            return false;
        };
        let mut vehicle_kit = Vehicle::new(
            player_guid,
            TypeId::Player,
            player_position,
            vehicle_id,
            creature_entry,
            seat_defs,
        );
        vehicle_kit.install();
        let accessories = self
            .vehicle_accessory_store
            .as_ref()
            .and_then(|store| store.accessories_for_vehicle_like_cpp(None, creature_entry))
            .map(<[VehicleAccessory]>::to_vec)
            .unwrap_or_default();
        let _accessory_plan =
            vehicle_kit.install_all_accessories_plan_like_cpp(false, &accessories);
        #[cfg(test)]
        {
            self.player_mount_vehicle_id_like_cpp = vehicle_id;
            self.player_mount_vehicle_seat_count_like_cpp =
                vehicle_kit.seats().len().min(u8::MAX as usize) as u8;
            self.player_mount_vehicle_usable_seat_count_like_cpp =
                vehicle_kit.usable_seat_num().min(u32::from(u8::MAX)) as u8;
            self.player_mount_vehicle_accessories_like_cpp = _accessory_plan.accessories;
        }
        self.mutate_player_mount_vehicle_kit_like_cpp(|kit| *kit = Some(vehicle_kit))
            .is_some()
    }
    pub(in crate::session) fn send_set_vehicle_rec_id_like_cpp(&mut self, vehicle_id: u32) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let vehicle_rec_id = i32::try_from(vehicle_id).unwrap_or(i32::MAX);
        let Some(sequence_index) = self.next_movement_counter_like_cpp() else {
            return;
        };

        self.send_packet(&wow_packet::packets::vehicle::MoveSetVehicleRecId {
            mover_guid: player_guid,
            sequence_index,
            vehicle_rec_id,
        });
        self.send_packet(&wow_packet::packets::vehicle::SetVehicleRecId {
            vehicle_guid: player_guid,
            vehicle_rec_id,
        });
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
    pub(in crate::session) fn send_active_player_transport_server_time_update_like_cpp(&self) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some((local_flags, transport_server_time, _)) =
            self.active_player_update_state_like_cpp()
        else {
            return;
        };

        use wow_packet::packets::update::{ActivePlayerDataValuesUpdate, UpdateObject};

        let mut data = ActivePlayerDataValuesUpdate::default();
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 38);
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 69);
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 70);
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 118);
        data.local_flags = local_flags;
        data.transport_server_time = transport_server_time;
        self.send_packet(&UpdateObject::full_active_player_values_update(
            guid,
            self.player_map_id_like_cpp(),
            data,
        ));
    }
    #[cfg(test)]
    pub(crate) fn active_player_transport_server_time_like_cpp(&self) -> i32 {
        self.active_player_update_state_like_cpp()
            .expect("test active Player owner must resolve")
            .1
    }
    #[cfg(test)]
    pub(crate) fn set_active_player_transport_server_time_like_cpp(&mut self, value: i32) {
        let _ = self.mutate_active_player_update_state_like_cpp(|state| {
            state.active_transport_server_time = value;
        });
    }
    pub(crate) fn represented_set_taxi_benchmark_mode_like_cpp(&mut self, enable: bool) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        let changed = self
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

        self.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP)
            .unwrap_or(false)
            == enable
    }
    #[cfg(test)]
    pub(crate) fn represented_taxi_benchmark_mode_like_cpp(&self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        self.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP)
            .unwrap_or(false)
    }
    pub(crate) fn player_taxi_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerTaxiState> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.taxi_state_like_cpp().clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerTaxiState {
                destinations: self.taxi_destinations_like_cpp.clone(),
                flight: self
                    .taxi_flight_state_like_cpp
                    .map(canonical_taxi_flight_state_like_cpp),
                unit_flags: self.taxi_unit_flags_like_cpp.bits(),
                mounted: self.taxi_mounted_like_cpp,
                ..Default::default()
            });
        }
        canonical
    }
    #[cfg(test)]
    pub(in crate::session) fn replace_player_taxi_state_like_cpp(
        &mut self,
        state: wow_entities::PlayerTaxiState,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_taxi_state_like_cpp(state.clone())
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.taxi_destinations_like_cpp = state.destinations;
            self.taxi_flight_state_like_cpp =
                state.flight.map(represented_taxi_flight_state_like_cpp);
            self.taxi_unit_flags_like_cpp = UnitFlags::from_bits_retain(state.unit_flags);
            self.taxi_mounted_like_cpp = state.mounted;
            return true;
        }
        canonical
    }
    pub(in crate::session) fn mutate_player_taxi_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerTaxiState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut state = self.player_taxi_state_snapshot_like_cpp()?;
            let result = f(&mut state);
            return self
                .replace_player_taxi_state_like_cpp(state)
                .then_some(result);
        }
        // PlayerTaxi mutates the owning Player's route, not a Session copy.
        self.with_owned_player_mut_like_cpp(|player| f(&mut player.gameplay_state_mut().taxi))
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_destinations_like_cpp(&mut self, destinations: Vec<u32>) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.destinations = destinations;
        });
    }
    #[cfg(test)]
    pub(crate) fn taxi_destinations_like_cpp(&self) -> Vec<u32> {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| taxi.destinations)
            .expect("test Player taxi owner must resolve")
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
    pub(crate) fn resolved_is_in_taxi_flight_like_cpp(&self) -> Option<bool> {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| taxi.flight.is_some())
    }
    #[cfg(test)]
    pub(crate) fn is_in_taxi_flight_like_cpp(&self) -> bool {
        self.resolved_is_in_taxi_flight_like_cpp()
            .expect("test Player taxi owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_flight_state_like_cpp(
        &mut self,
        current_node: RepresentedTaxiFlightNodeLikeCpp,
        node_after_teleport: Option<RepresentedTaxiFlightNodeLikeCpp>,
    ) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.flight = Some(wow_entities::PlayerTaxiFlightStateLikeCpp {
                current_node: canonical_taxi_flight_node_like_cpp(current_node),
                node_after_teleport: node_after_teleport.map(canonical_taxi_flight_node_like_cpp),
            });
        });
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_cleanup_state_like_cpp(&mut self, unit_flags: UnitFlags, mounted: bool) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.unit_flags = unit_flags.bits();
            taxi.mounted = mounted;
        });
    }
    #[cfg(test)]
    pub(crate) fn taxi_unit_flags_like_cpp(&self) -> UnitFlags {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| UnitFlags::from_bits_retain(taxi.unit_flags))
            .expect("test Player taxi owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn taxi_mounted_like_cpp(&self) -> bool {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| taxi.mounted)
            .expect("test Player taxi owner must resolve")
    }
    pub(crate) fn set_player_transport_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        self.set_player_transport_info_like_cpp(guid.filter(|guid| !guid.is_empty()).map(|guid| {
            wow_packet::packets::movement::TransportInfo {
                guid,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                o: 0.0,
                seat: -1,
                time: 0,
                prev_time: None,
                vehicle_id: None,
            }
        }));
    }
    pub(crate) fn set_player_transport_info_like_cpp(
        &mut self,
        info: Option<wow_packet::packets::movement::TransportInfo>,
    ) {
        let info = info.filter(|info| !info.guid.is_empty());
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.player_on_transport_like_cpp = info.is_some();
            self.player_transport_login_state_like_cpp =
                info.map(|info| Box::new(PlayerTransportLoginStateLikeCpp { info }));
            return;
        }
        let transport = info.map(|info| wow_entities::PlayerTransportState {
            guid: info.guid,
            x: info.x,
            y: info.y,
            z: info.z,
            orientation: info.o,
            seat: info.seat,
            time: info.time,
            prev_time: info.prev_time,
            vehicle_id: info.vehicle_id,
        });
        let _ = self.with_owned_player_mut_like_cpp(|player| {
            player.gameplay_state_mut().transport = transport;
        });
    }
    pub(crate) fn player_transport_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.player_transport_state_like_cpp()
            .flatten()
            .map(|state| state.guid)
    }
    pub(crate) fn player_transport_info_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::movement::TransportInfo> {
        self.player_transport_state_like_cpp()
            .flatten()
            .map(|state| wow_packet::packets::movement::TransportInfo {
                guid: state.guid,
                x: state.x,
                y: state.y,
                z: state.z,
                o: state.orientation,
                seat: state.seat,
                time: state.time,
                prev_time: state.prev_time,
                vehicle_id: state.vehicle_id,
            })
    }
    pub(in crate::session) fn player_transport_state_like_cpp(
        &self,
    ) -> Option<Option<wow_entities::PlayerTransportState>> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.player_transport_login_state_like_cpp
                    .as_ref()
                    .map(|state| wow_entities::PlayerTransportState {
                        guid: state.info.guid,
                        x: state.info.x,
                        y: state.info.y,
                        z: state.info.z,
                        orientation: state.info.o,
                        seat: state.info.seat,
                        time: state.info.time,
                        prev_time: state.info.prev_time,
                        vehicle_id: state.info.vehicle_id,
                    }),
            );
        }
        self.with_owned_player_like_cpp(|player| player.gameplay_state().transport.clone())
    }
    pub(in crate::session) fn player_on_transport_state_like_cpp(&self) -> Option<bool> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(self.player_on_transport_like_cpp);
        }
        self.with_owned_player_like_cpp(|player| player.gameplay_state().transport.is_some())
    }
    pub(crate) fn should_send_init_transport_like_cpp(
        &self,
        transport_guid: ObjectGuid,
        transport_phase_shift: &PhaseShift,
    ) -> bool {
        self.player_transport_guid_like_cpp() != Some(transport_guid)
            && self.can_see_phase_shift_like_cpp(transport_phase_shift)
    }
    #[cfg(test)]
    pub(crate) fn set_player_on_transport_like_cpp(&mut self, on_transport: bool) {
        self.player_on_transport_like_cpp = on_transport;
    }
    pub(in crate::session) fn represented_player_has_active_vehicle_like_cpp(&self) -> bool {
        self.player_mount_vehicle_kit_snapshot_like_cpp()
            .flatten()
            .as_ref()
            .is_some_and(|vehicle_kit| {
                vehicle_kit.status() == wow_entities::VehicleStatus::Installed
            })
    }
}
