#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::PlayerCreateInfoCastSpellStoreLikeCpp;

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
        #[cfg(any(test, feature = "test-fixtures"))]
        self.core
            .packet_publication_access_like_cpp()
            .broadcast_to_movement_set_in_range_and_connection_like_cpp(
                bytes,
                range,
                realm_connection,
                &self.fixtures.movement.player_position,
            );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        self.core
            .packet_publication_access_like_cpp()
            .broadcast_to_movement_set_in_range_and_connection_like_cpp(
                bytes,
                range,
                realm_connection,
            );
    }
}
