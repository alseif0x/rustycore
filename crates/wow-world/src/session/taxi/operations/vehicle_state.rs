//! Canonical Player vehicle seat and mount-kit bridges, catalogs and represented fixtures.

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
                player.set_vehicle_seat_like_cpp(flags, seat_id);
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
        let canonical =
            self.with_owned_player_like_cpp(|player| player.mount_vehicle_kit_snapshot_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_mount_vehicle_kit_like_cpp.clone());
        }
        canonical
    }
    pub(in crate::session) fn install_player_mount_vehicle_kit_like_cpp(
        &mut self,
        vehicle_kit: Vehicle,
    ) -> bool {
        let mut vehicle_kit = Some(vehicle_kit);
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.install_mount_vehicle_kit_like_cpp(
                    vehicle_kit
                        .take()
                        .expect("vehicle kit installation runs once"),
                );
            })
            .is_some();
        #[cfg(test)]
        if canonical {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.player_mount_vehicle_kit_like_cpp = vehicle_kit;
            return true;
        }
        canonical
    }
    pub(in crate::session) fn clear_player_mount_vehicle_kit_like_cpp(&mut self) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.clear_mount_vehicle_kit_like_cpp())
            .is_some();
        #[cfg(test)]
        if canonical {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.player_mount_vehicle_kit_like_cpp = None;
            return true;
        }
        canonical
    }
    pub(in crate::session) fn remove_player_mount_vehicle_kit_like_cpp(&mut self) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.remove_mount_vehicle_kit_like_cpp())
            .is_some();
        #[cfg(test)]
        if canonical {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            if let Some(vehicle_kit) = self.player_mount_vehicle_kit_like_cpp.as_mut() {
                vehicle_kit.uninstall();
            }
            self.player_mount_vehicle_kit_like_cpp = None;
            return true;
        }
        canonical
    }
    pub(in crate::session) fn eject_player_mount_vehicle_passenger_like_cpp(
        &mut self,
        passenger_guid: ObjectGuid,
    ) -> bool {
        if !passenger_guid.is_unit() {
            return false;
        }
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.eject_mount_vehicle_passenger_like_cpp(passenger_guid)
            })
            .unwrap_or(false);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
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
    #[cfg(test)]
    pub(in crate::session) fn mutate_player_mount_vehicle_kit_like_cpp<R>(
        &mut self,
        update: impl FnOnce(&mut Option<Vehicle>) -> R,
    ) -> Option<R> {
        if self.player_handle_like_cpp.is_none() {
            return Some(update(&mut self.player_mount_vehicle_kit_like_cpp));
        }
        self.with_owned_player_mut_like_cpp(|player| {
            update(&mut player.gameplay_state_mut().mount_vehicle_kit)
        })
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
                let _ = self.clear_player_mount_vehicle_kit_like_cpp();
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
        self.install_player_mount_vehicle_kit_like_cpp(vehicle_kit)
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
