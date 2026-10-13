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
    pub(crate) fn set_vehicle_template_store(&mut self, store: Arc<VehicleTemplateStoreLikeCpp>) {
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

        if !wow_world_spell::known_spells_like_cpp(self).contains(&mount.source_spell_id) {
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
