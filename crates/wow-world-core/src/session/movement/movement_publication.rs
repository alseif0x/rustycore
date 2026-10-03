//! Hub operations for movement packet and observer publication.

use crate::session::mailbox::{SendIfVisibleLikeCppCommand, SessionCommand};
use std::time::Instant;
use tracing::warn;
use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;

impl crate::session::HubMut<'_> {
    pub fn send_movement_set_collision_height_like_cpp(&mut self, reason: u8) {
        let Some(player_guid) = self.core.player_guid() else {
            return;
        };
        let Some((_, mount_display_id, object_scale)) =
            self.shared().player_unit_presentation_snapshot_like_cpp()
        else {
            return;
        };
        let Some(collision_height) = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().collision_height_like_cpp())
            .or_else(|| {
                #[cfg(any(test, feature = "test-fixtures"))]
                {
                    return self
                        .core
                        .player_handle_like_cpp
                        .is_none()
                        .then_some(self.fixtures.movement.player_collision_height_like_cpp);
                }
                #[cfg(not(any(test, feature = "test-fixtures")))]
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

        let Some(scale_duration) = self.shared().resolved_player_scale_duration_like_cpp() else {
            return;
        };

        self.core
            .send_packet(&wow_packet::packets::movement::MoveSetCollisionHeight {
                mover_guid: player_guid,
                sequence_index,
                height: collision_height,
                scale: object_scale,
                reason,
                mount_display_id: u32::try_from(mount_display_id).unwrap_or(0),
                scale_duration,
            });

        use wow_packet::ServerPacket;
        let Some(status) = self
            .shared()
            .current_player_movement_info_like_cpp(player_guid)
        else {
            return;
        };
        self.shared().broadcast_to_movement_set_like_cpp(
            wow_packet::packets::movement::MoveUpdateCollisionHeight {
                status,
                height: collision_height,
                scale: object_scale,
            }
            .to_bytes(),
            false,
        );
    }

    pub fn send_represented_capture_point_removed_like_cpp(&mut self, gameobject_guid: ObjectGuid) {
        self.core
            .send_packet(&wow_packet::packets::misc::CapturePointRemoved {
                capture_point_guid: gameobject_guid,
            });
    }

    pub(in crate::session) fn send_player_move_set_flag_like_cpp(&mut self, opcode: ServerOpcodes) {
        use wow_packet::ServerPacket;

        let Some(player_guid) = self.core.player_guid() else {
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
        if self.core.send_tx().send(self_packet).is_err() {
            warn!("Send channel closed for account {}", self.core.account_id);
        }

        let Some(status) = self
            .shared()
            .current_player_movement_info_like_cpp(player_guid)
        else {
            return;
        };
        self.shared().broadcast_to_movement_set_like_cpp(
            wow_packet::packets::movement::MoveUpdate { info: status }.to_bytes(),
            false,
        );
    }
}
impl crate::session::state::SessionCore {
    pub(crate) fn player_position_with_fixture_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_position: &Option<wow_core::Position>,
    ) -> Option<wow_core::Position> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.unit().world().position());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return *fixture_position;
        }
        canonical
    }
}

impl crate::session::HubRef<'_> {
    /// Publish a movement-set packet from the Unit that actually moved.
    /// C++ calls `mover->SendMessageToSet`, so controlled movers must be
    /// spatially routed from their own position and carry their own GUID as
    /// the visibility source (MovementHandler.cpp:735-739).
    pub fn broadcast_from_movement_source_set_like_cpp(
        &self,
        source_guid: ObjectGuid,
        source_position: wow_core::Position,
        bytes: Vec<u8>,
        range: f32,
    ) {
        let (Some(registry), Some(_player_guid)) =
            (self.core.player_registry(), self.core.player_guid())
        else {
            return;
        };
        let map_id = self.core.player_map_id_like_cpp();
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        for registration in registry.movement_recipients_within_range(
            source_guid,
            map_id,
            instance_id,
            source_position,
            range,
        ) {
            let _ = registry.try_send_current_command(
                registration,
                SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                    queued_at: Instant::now(),
                    source_guid,
                    map_id,
                    instance_id,
                    packet_bytes: bytes.clone(),
                }),
            );
        }
    }
}
impl crate::session::HubRef<'_> {
    /// Update this session's position (and map) in the player registry.
    /// Called whenever `player_position` changes.
    pub fn update_registry_position(&self) {
        self.core
            .player_registry_sync_access_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.movement.player_position,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.combat.player_health_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.combat.player_max_health_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.combat.player_alive_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.vehicles.player_transport_login_state_like_cpp,
            )
            .update_registry_position();
    }
}
