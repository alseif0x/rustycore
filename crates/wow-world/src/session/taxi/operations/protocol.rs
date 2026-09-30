//! Vehicle record packets, active transport time publication and taxi benchmark flags.

use super::*;

impl WorldSession {
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
}
