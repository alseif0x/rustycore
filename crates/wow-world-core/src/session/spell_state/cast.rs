#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;
use std::time::Instant;

#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::PlayerCreateInfoCastSpellStoreLikeCpp;
use crate::session::mailbox::{SendIfVisibleLikeCppCommand, SessionCommand};

impl crate::session::state::SessionCatalogs {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_create_cast_spell_store_like_cpp(
        &mut self,
        store: Arc<PlayerCreateInfoCastSpellStoreLikeCpp>,
    ) {
        self.player_bootstrap_catalog_test_fixture_like_cpp
            .player_create_cast_spell_store_like_cpp = Some(store);
    }
}

impl crate::session::HubRef<'_> {
    pub fn broadcast_to_movement_set_like_cpp(&self, bytes: Vec<u8>, _include_self: bool) {
        self.broadcast_to_movement_set_in_range_like_cpp(
            bytes,
            crate::map_manager::VISIBILITY_RADIUS,
        );
    }

    pub fn broadcast_to_movement_set_in_range_like_cpp(&self, bytes: Vec<u8>, range: f32) {
        self.broadcast_to_movement_set_in_range_and_connection_like_cpp(bytes, range, false);
    }

    pub fn broadcast_to_movement_set_in_range_and_connection_like_cpp(
        &self,
        bytes: Vec<u8>,
        range: f32,
        realm_connection: bool,
    ) {
        let (Some(guid), Some(registry)) = (self.core.player_guid(), self.core.player_registry())
        else {
            return;
        };
        let Some(source_position) = self.player_position_like_cpp() else {
            return;
        };
        let map_id = self.core.player_map_id_like_cpp();
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        for registration in registry.movement_recipients_within_range(
            guid,
            map_id,
            instance_id,
            source_position,
            range,
        ) {
            let command = SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid: guid,
                map_id,
                instance_id,
                packet_bytes: bytes.clone(),
            };
            let command = if realm_connection {
                SessionCommand::SendRealmIfVisibleLikeCpp(command)
            } else {
                SessionCommand::SendIfVisibleLikeCpp(command)
            };
            let _ = registry.try_send_current_command(registration, command);
        }
    }
}
