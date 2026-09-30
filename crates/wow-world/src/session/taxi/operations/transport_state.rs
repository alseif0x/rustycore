//! Canonical Player transport state, movement wire projection and visibility admission.

use super::*;

impl WorldSession {
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
            player.set_transport_like_cpp(transport);
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
}
