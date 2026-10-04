//! Hub operations for movement packet and observer publication.

use crate::session::mailbox::{SendIfVisibleLikeCppCommand, SessionCommand};
use std::time::Instant;
use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;

impl crate::session::HubMut<'_> {
    pub fn send_movement_set_collision_height_like_cpp(&mut self, reason: u8) {
        let (presentation, mut control) = self.aura_removal_mount_accesses_like_cpp();
        control.send_movement_set_collision_height_like_cpp(&presentation, reason);
    }

    pub fn send_represented_capture_point_removed_like_cpp(&mut self, gameobject_guid: ObjectGuid) {
        self.core
            .send_packet(&wow_packet::packets::misc::CapturePointRemoved {
                capture_point_guid: gameobject_guid,
            });
    }

    pub(in crate::session) fn send_player_move_set_flag_like_cpp(&mut self, opcode: ServerOpcodes) {
        let (_presentation, mut control) = self.aura_removal_mount_accesses_like_cpp();
        control.send_player_move_set_flag_like_cpp(opcode);
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
        let position = self
            .core
            .player_registry_sync_access_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.movement.player_position,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.vehicles.player_transport_login_state_like_cpp,
            );
        position.update_registry_position(
            #[cfg(any(test, feature = "test-fixtures"))]
            &crate::session::RegistrySyncInputs::new_like_cpp(
                &self.fixtures.combat.player_health_like_cpp,
                &self.fixtures.combat.player_max_health_like_cpp,
                &self.fixtures.combat.player_alive_like_cpp,
            ),
        );
    }
}
