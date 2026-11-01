//! Taxi, transport, and vehicle Hub operations.

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::PlayerTransportLoginStateLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::movement_protocol::{
    RepresentedTaxiFlightNodeLikeCpp, canonical_taxi_flight_node_like_cpp,
    canonical_taxi_flight_state_like_cpp, represented_taxi_flight_state_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::taxi_contracts::PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP;
use crate::session::{RepresentedActivateTaxiLikeCpp, set_active_player_update_bit_like_cpp};
use wow_constants::{TypeId, UnitFlags};
use wow_core::ObjectGuid;
use wow_entities::{Vehicle, VehicleAccessory};

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::TaxiVehicleState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_on_transport_like_cpp(&mut self, on_transport: bool) {
        self.player_on_transport_like_cpp = on_transport;
    }
}

impl crate::session::HubRef<'_> {
    /// C++ `MovementHandler.cpp:408-421` allows a passenger to update only its
    /// facing when the current `VehicleSeatEntry` has `ALLOW_TURNING`.
    pub fn represented_current_vehicle_seat_allows_turning_like_cpp(&self) -> bool {
        self.player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_some_and(wow_data::vehicle_seat_flags_allow_turning_like_cpp)
    }

    pub fn send_active_player_transport_server_time_update_like_cpp(&self) {
        let Some(guid) = self.core.player_guid() else {
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
        self.core
            .send_packet(&UpdateObject::full_active_player_values_update(
                guid,
                self.core.player_map_id_like_cpp(),
                data,
            ));
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn active_player_transport_server_time_like_cpp(&self) -> i32 {
        self.active_player_update_state_like_cpp()
            .expect("test active Player owner must resolve")
            .1
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_taxi_benchmark_mode_like_cpp(&self) -> bool {
        let Some(guid) = self.core.player_guid() else {
            return false;
        };

        self.core
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP)
            .unwrap_or(false)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn taxi_destinations_like_cpp(&self) -> Vec<u32> {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| taxi.destinations_like_cpp().to_vec())
            .expect("test Player taxi owner must resolve")
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn taxi_unit_flags_like_cpp(&self) -> UnitFlags {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| UnitFlags::from_bits_retain(taxi.unit_flags_like_cpp()))
            .expect("test Player taxi owner must resolve")
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn taxi_mounted_like_cpp(&self) -> bool {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| taxi.mounted_like_cpp())
            .expect("test Player taxi owner must resolve")
    }

    pub fn player_transport_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.player_transport_state_like_cpp()
            .flatten()
            .map(|state| state.guid)
    }

    pub fn player_on_transport_state_like_cpp(&self) -> Option<bool> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.vehicles.player_on_transport_like_cpp);
        }
        self.core
            .with_owned_player_like_cpp(|player| player.gameplay_state().transport.is_some())
    }

    pub fn represented_player_has_active_vehicle_like_cpp(&self) -> bool {
        self.player_mount_vehicle_kit_snapshot_like_cpp()
            .flatten()
            .as_ref()
            .is_some_and(|vehicle_kit| {
                vehicle_kit.status() == wow_entities::VehicleStatus::Installed
            })
    }
}

impl crate::session::HubMut<'_> {
    pub(in crate::session) fn install_player_mount_vehicle_kit_like_cpp(
        &mut self,
        vehicle_kit: Vehicle,
    ) -> bool {
        let mut vehicle_kit = Some(vehicle_kit);
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.install_mount_vehicle_kit_like_cpp(
                    vehicle_kit
                        .take()
                        .expect("vehicle kit installation runs once"),
                );
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp = vehicle_kit;
            return true;
        }
        canonical
    }

    pub fn clear_player_mount_vehicle_kit_like_cpp(&mut self) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.clear_mount_vehicle_kit_like_cpp())
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp = None;
            return true;
        }
        canonical
    }

    pub fn remove_player_mount_vehicle_kit_like_cpp(&mut self) -> bool {
        self.core.remove_mount_vehicle_kit_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
        )
    }

    pub(in crate::session) fn eject_player_mount_vehicle_passenger_like_cpp(
        &mut self,
        passenger_guid: ObjectGuid,
    ) -> bool {
        if !passenger_guid.is_unit() {
            return false;
        }
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.eject_mount_vehicle_passenger_like_cpp(passenger_guid)
            })
            .unwrap_or(false);
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .fixtures
                .vehicles
                .player_mount_vehicle_kit_like_cpp
                .as_mut()
                .is_some_and(|vehicle_kit| {
                    let passenger_type_id = if passenger_guid.is_player() {
                        TypeId::Player
                    } else {
                        TypeId::Unit
                    };
                    vehicle_kit
                        .seat_info_for_passenger_like_cpp(passenger_guid)
                        .is_some_and(|seat| seat.ejectable)
                        && vehicle_kit
                            .remove_passenger_plan_like_cpp(
                                passenger_guid,
                                passenger_type_id,
                                false,
                                false,
                                false,
                                false,
                            )
                            .is_some()
                });
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_player_mount_vehicle_kit_like_cpp<R>(
        &mut self,
        update: impl FnOnce(&mut Option<Vehicle>) -> R,
    ) -> Option<R> {
        if self.core.player_handle_like_cpp.is_none() {
            return Some(update(
                &mut self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
            ));
        }
        self.core.with_owned_player_mut_like_cpp(|player| {
            update(&mut player.gameplay_state_mut().mount_vehicle_kit)
        })
    }

    pub fn create_player_mount_vehicle_kit_like_cpp(
        &mut self,
        vehicle_id: u32,
        creature_entry: u32,
    ) -> bool {
        if vehicle_id == 0 {
            return false;
        }
        let Some(player_guid) = self.core.player_guid() else {
            return false;
        };

        let Some(vehicle) = self
            .catalogs
            .vehicle_store
            .as_ref()
            .and_then(|store| store.get(vehicle_id))
        else {
            #[cfg(not(any(test, feature = "test-fixtures")))]
            return false;
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                if self.catalogs.vehicle_store.is_some() {
                    return false;
                }
                self.fixtures.vehicles.player_mount_vehicle_id_like_cpp = vehicle_id;
                let _ = self.clear_player_mount_vehicle_kit_like_cpp();
                self.fixtures
                    .vehicles
                    .player_mount_vehicle_accessories_like_cpp = self
                    .catalogs
                    .vehicle_accessory_store
                    .as_ref()
                    .and_then(|store| store.accessories_for_vehicle_like_cpp(None, creature_entry))
                    .map(<[VehicleAccessory]>::to_vec)
                    .unwrap_or_default();
                self.fixtures
                    .vehicles
                    .player_mount_vehicle_seat_count_like_cpp = 0;
                self.fixtures
                    .vehicles
                    .player_mount_vehicle_usable_seat_count_like_cpp = 0;
                return true;
            }
        };

        let seat_defs = self
            .catalogs
            .vehicle_seat_store
            .as_ref()
            .map(|store| store.seat_defs_for_vehicle_like_cpp(vehicle))
            .unwrap_or_default();
        let Some(player_position) = self.shared().player_position_like_cpp() else {
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
            .catalogs
            .vehicle_accessory_store
            .as_ref()
            .and_then(|store| store.accessories_for_vehicle_like_cpp(None, creature_entry))
            .map(<[VehicleAccessory]>::to_vec)
            .unwrap_or_default();
        let _accessory_plan =
            vehicle_kit.install_all_accessories_plan_like_cpp(false, &accessories);
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures.vehicles.player_mount_vehicle_id_like_cpp = vehicle_id;
            self.fixtures
                .vehicles
                .player_mount_vehicle_seat_count_like_cpp =
                vehicle_kit.seats().len().min(u8::MAX as usize) as u8;
            self.fixtures
                .vehicles
                .player_mount_vehicle_usable_seat_count_like_cpp =
                vehicle_kit.usable_seat_num().min(u32::from(u8::MAX)) as u8;
            self.fixtures
                .vehicles
                .player_mount_vehicle_accessories_like_cpp = _accessory_plan.accessories;
        }
        self.install_player_mount_vehicle_kit_like_cpp(vehicle_kit)
    }

    pub fn send_set_vehicle_rec_id_like_cpp(&mut self, vehicle_id: u32) {
        self.aura_removal_mount_accesses_like_cpp()
            .1
            .send_set_vehicle_rec_id_like_cpp(vehicle_id);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_active_player_transport_server_time_like_cpp(&mut self, value: i32) {
        let _ = self.mutate_active_player_update_state_like_cpp(|state| {
            state.active_transport_server_time = value;
        });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn replace_player_taxi_state_like_cpp(
        &mut self,
        state: wow_entities::PlayerTaxiState,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_taxi_state_like_cpp(state.clone())
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures.vehicles.taxi_destinations_like_cpp =
                state.destinations_like_cpp().to_vec();
            self.fixtures.vehicles.taxi_flight_state_like_cpp = state
                .flight_like_cpp()
                .map(represented_taxi_flight_state_like_cpp);
            self.fixtures.vehicles.taxi_unit_flags_like_cpp =
                UnitFlags::from_bits_retain(state.unit_flags_like_cpp());
            self.fixtures.vehicles.taxi_mounted_like_cpp = state.mounted_like_cpp();
            return true;
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_player_taxi_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerTaxiState) -> R,
    ) -> Option<R> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            let mut state = self.shared().player_taxi_state_snapshot_like_cpp()?;
            let result = f(&mut state);
            return self
                .replace_player_taxi_state_like_cpp(state)
                .then_some(result);
        }
        // PlayerTaxi mutates the owning Player's route, not a Session copy.
        self.core
            .with_owned_player_mut_like_cpp(|player| f(&mut player.gameplay_state_mut().taxi))
    }

    pub fn advance_player_taxi_flight_after_teleport_like_cpp(
        &mut self,
    ) -> Option<wow_entities::PlayerTaxiFlightNodeLikeCpp> {
        let canonical = self.core.with_owned_player_mut_like_cpp(
            wow_entities::Player::advance_taxi_flight_after_teleport_like_cpp,
        );
        if let Some(result) = canonical {
            return result;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_taxi_state_like_cpp(|taxi| {
                    taxi.advance_taxi_flight_after_teleport_like_cpp()
                })
                .flatten();
        }
        None
    }

    pub fn cleanup_player_after_taxi_flight_like_cpp(&mut self) -> bool {
        let taxi_unit_flags = (UnitFlags::REMOVE_CLIENT_CONTROL | UnitFlags::ON_TAXI).bits();
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.cleanup_after_taxi_flight_like_cpp(taxi_unit_flags)
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_taxi_state_like_cpp(|taxi| {
                    taxi.cleanup_after_taxi_flight_like_cpp(taxi_unit_flags);
                })
                .is_some();
        }
        false
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_taxi_destinations_like_cpp(&mut self, destinations: Vec<u32>) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.replace_destinations_like_cpp(destinations);
        });
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_activate_taxi_like_cpp(
        &mut self,
        request: RepresentedActivateTaxiLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .vehicles
            .represented_activate_taxi_requests_like_cpp
            .push(request);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_taxi_flight_state_like_cpp(
        &mut self,
        current_node: RepresentedTaxiFlightNodeLikeCpp,
        node_after_teleport: Option<RepresentedTaxiFlightNodeLikeCpp>,
    ) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.begin_taxi_flight_like_cpp(
                canonical_taxi_flight_node_like_cpp(current_node),
                node_after_teleport.map(canonical_taxi_flight_node_like_cpp),
            );
        });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_taxi_cleanup_state_like_cpp(&mut self, unit_flags: UnitFlags, mounted: bool) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.set_taxi_cleanup_state_like_cpp(unit_flags.bits(), mounted);
        });
    }

    pub fn set_player_transport_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
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

    pub fn set_player_transport_info_like_cpp(
        &mut self,
        info: Option<wow_packet::packets::movement::TransportInfo>,
    ) {
        let info = info.filter(|info| !info.guid.is_empty());
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures.vehicles.player_on_transport_like_cpp = info.is_some();
            self.fixtures.vehicles.player_transport_login_state_like_cpp =
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
        let _ = self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_transport_like_cpp(transport);
        });
    }
}

impl crate::session::HubMut<'_> {
    pub fn set_player_vehicle_seat_state_like_cpp(
        &mut self,
        flags: Option<i32>,
        seat_id: Option<u32>,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_vehicle_seat_like_cpp(flags, seat_id);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp = flags;
            self.fixtures.vehicles.player_vehicle_seat_id_like_cpp = seat_id;
            return true;
        }
        canonical
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_vehicle_seat_state_like_cpp(&self) -> Option<(Option<i32>, Option<u32>)> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            let state = player.gameplay_state();
            (state.vehicle_seat_flags, state.vehicle_seat_id)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some((
                self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
            ));
        }
        canonical
    }

    pub fn player_mount_vehicle_kit_snapshot_like_cpp(&self) -> Option<Option<Vehicle>> {
        self.core.mount_vehicle_kit_snapshot_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
        )
    }

    pub fn represented_current_vehicle_seat_can_switch_from_like_cpp(&self) -> bool {
        self.player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_some_and(wow_data::vehicle_seat_flags_can_switch_from_seat_like_cpp)
    }

    pub fn represented_vehicle_base_guid_for_switch_like_cpp(&self) -> Option<ObjectGuid> {
        if self
            .player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_none()
        {
            return None;
        }
        self.player_moved_unit_guid_like_cpp()
    }

    pub fn player_taxi_state_snapshot_like_cpp(&self) -> Option<wow_entities::PlayerTaxiState> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.taxi_state_like_cpp().clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                wow_entities::PlayerTaxiState::from_represented_parts_like_cpp(
                    self.fixtures.vehicles.taxi_destinations_like_cpp.clone(),
                    self.fixtures
                        .vehicles
                        .taxi_flight_state_like_cpp
                        .map(canonical_taxi_flight_state_like_cpp),
                    self.fixtures.vehicles.taxi_unit_flags_like_cpp.bits(),
                    self.fixtures.vehicles.taxi_mounted_like_cpp,
                ),
            );
        }
        canonical
    }

    pub fn resolved_is_in_taxi_flight_like_cpp(&self) -> Option<bool> {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| taxi.is_in_flight_like_cpp())
    }

    pub(crate) fn player_transport_info_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::movement::TransportInfo> {
        self.core.player_transport_info_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.player_transport_login_state_like_cpp,
        )
    }

    pub fn player_transport_state_like_cpp(
        &self,
    ) -> Option<Option<wow_entities::PlayerTransportState>> {
        self.core.player_transport_state_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.vehicles.player_transport_login_state_like_cpp,
        )
    }
}

impl crate::session::state::SessionCore {
    pub(crate) fn player_transport_state_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_transport: &Option<
            Box<crate::session::PlayerTransportLoginStateLikeCpp>,
        >,
    ) -> Option<Option<wow_entities::PlayerTransportState>> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            return Some(fixture_transport.as_ref().map(|state| {
                wow_entities::PlayerTransportState {
                    guid: state.info.guid,
                    x: state.info.x,
                    y: state.info.y,
                    z: state.info.z,
                    orientation: state.info.o,
                    seat: state.info.seat,
                    time: state.info.time,
                    prev_time: state.info.prev_time,
                    vehicle_id: state.info.vehicle_id,
                }
            }));
        }
        self.with_owned_player_like_cpp(|player| player.gameplay_state().transport.clone())
    }

    pub(crate) fn player_transport_info_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_transport: &Option<
            Box<crate::session::PlayerTransportLoginStateLikeCpp>,
        >,
    ) -> Option<wow_packet::packets::movement::TransportInfo> {
        self.player_transport_state_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_transport,
        )
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
}
