//! Movement packets and updates published to the client and observers.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Update this session's position (and map) in the player registry.
    /// Called whenever `player_position` changes.
    pub(crate) fn update_registry_position(&self) {
        let (Some(guid), Some(pos), Some(reg)) = (
            self.player_guid(),
            self.player_position_like_cpp(),
            &self.player_registry,
        ) else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let Some(is_alive) = self.resolved_player_is_alive_like_cpp() else {
            return;
        };
        // Fallback to 0 (world/default instance) when no canonical map key is
        // available — mirrors C++ world-map phase where instance_id == 0.
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        let _ = reg.publish_movement_for_control_channel(
            guid,
            &self.session_command_tx,
            crate::session::directory::PlayerMovementDirectoryUpdate {
                position: pos,
                map_id,
                instance_id,
                is_in_world: self.player_is_in_world_for_registry_like_cpp(),
                level: self.player_level_like_cpp(),
                is_alive,
                transport: self.player_transport_info_like_cpp(),
            },
        );
    }
    pub(in crate::session) fn send_movement_set_collision_height_like_cpp(&mut self, reason: u8) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some((_, mount_display_id, object_scale)) =
            self.player_unit_presentation_snapshot_like_cpp()
        else {
            return;
        };
        let Some(collision_height) = self
            .with_owned_player_like_cpp(|player| player.unit().collision_height_like_cpp())
            .or_else(|| {
                #[cfg(test)]
                {
                    return self
                        .player_handle_like_cpp
                        .is_none()
                        .then_some(self.player_collision_height_like_cpp);
                }
                #[cfg(not(test))]
                {
                    None
                }
            })
        else {
            return;
        };
        let Some(sequence_index) = self.next_movement_counter_like_cpp() else {
            return;
        };

        let Some(scale_duration) = self.resolved_player_scale_duration_like_cpp() else {
            return;
        };

        self.send_packet(&wow_packet::packets::movement::MoveSetCollisionHeight {
            mover_guid: player_guid,
            sequence_index,
            height: collision_height,
            scale: object_scale,
            reason,
            mount_display_id: u32::try_from(mount_display_id).unwrap_or(0),
            scale_duration,
        });

        use wow_packet::ServerPacket;
        let Some(status) = self.current_player_movement_info_like_cpp(player_guid) else {
            return;
        };
        self.broadcast_to_movement_set_like_cpp(
            wow_packet::packets::movement::MoveUpdateCollisionHeight {
                status,
                height: collision_height,
                scale: object_scale,
            }
            .to_bytes(),
            false,
        );
    }
    fn send_represented_capture_point_removed_like_cpp(&mut self, gameobject_guid: ObjectGuid) {
        self.send_packet(&wow_packet::packets::misc::CapturePointRemoved {
            capture_point_guid: gameobject_guid,
        });
    }
    pub(in crate::session) fn send_player_move_set_flag_like_cpp(&mut self, opcode: ServerOpcodes) {
        use wow_packet::ServerPacket;

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(sequence_index) = self.next_movement_counter_like_cpp() else {
            return;
        };

        let self_packet = wow_packet::packets::movement::MoveSetFlag {
            opcode,
            mover_guid: player_guid,
            sequence_index,
        }
        .to_bytes();
        if self.send_tx().send(self_packet).is_err() {
            warn!("Send channel closed for account {}", self.account_id);
        }

        let Some(status) = self.current_player_movement_info_like_cpp(player_guid) else {
            return;
        };
        self.broadcast_to_movement_set_like_cpp(
            wow_packet::packets::movement::MoveUpdate { info: status }.to_bytes(),
            false,
        );
    }
    pub(crate) fn send_represented_capture_point_removed_from_last_update_like_cpp(
        &mut self,
    ) -> usize {
        let Some((map_id, instance_id, update_generation, removable_guids)) = self
            .visible_gameobject_guids_from_last_update_summary_like_cpp(|summary| {
                summary
                    .generic_capture_point_removed_guids
                    .as_slice()
                    .to_vec()
            })
        else {
            return 0;
        };

        let mut seen = std::collections::HashSet::new();
        let mut sent = 0;
        for guid in removable_guids {
            if !seen.insert(guid) || !self.client_visible_guids_like_cpp.contains(&guid) {
                continue;
            }
            if !self
                .represented_capture_point_removed_delivered_like_cpp
                .insert((map_id, instance_id, update_generation, guid))
            {
                continue;
            }
            self.send_represented_capture_point_removed_like_cpp(guid);
            sent += 1;
        }
        sent
    }
}
