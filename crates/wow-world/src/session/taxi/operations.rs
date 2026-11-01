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
    pub(crate) fn set_player_transport_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        crate::session::hub_mut(self).set_player_transport_guid_like_cpp(guid)
    }
    pub(crate) fn should_send_init_transport_like_cpp(
        &self,
        transport_guid: ObjectGuid,
        transport_phase_shift: &PhaseShift,
    ) -> bool {
        crate::session::hub_ref(self).player_transport_guid_like_cpp() != Some(transport_guid)
            && wow_world_entities::can_see_phase_shift_like_cpp(self, transport_phase_shift)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/taxi/operations/f3_shims.rs"]
mod f3_shims;
