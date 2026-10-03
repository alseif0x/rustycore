// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow access to the canonical inputs needed to publish a Player's registry position.

use crate::session::state::SessionCore;
use wow_core::Position;

/// Borrows the canonical Player and the exact fixture inputs used by registry synchronization.
/// The fields stay private so callers can perform the operation without receiving a mutable
/// SessionCore or a general-purpose view of its state.
pub struct PlayerRegistrySyncAccessLikeCpp<'a> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_position: &'a Option<Position>,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_health: &'a u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_max_health: &'a u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_alive: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_transport: &'a Option<Box<crate::session::PlayerTransportLoginStateLikeCpp>>,
}

impl SessionCore {
    /// Build the narrow registry synchronization capability from borrowed fixture values.
    pub fn player_registry_sync_access_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_position: &'a Option<Position>,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_health: &'a u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_max_health: &'a u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_alive: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_transport: &'a Option<Box<crate::session::PlayerTransportLoginStateLikeCpp>>,
    ) -> PlayerRegistrySyncAccessLikeCpp<'a> {
        PlayerRegistrySyncAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_health,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_max_health,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_alive,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_transport,
        }
    }
}

impl PlayerRegistrySyncAccessLikeCpp<'_> {
    fn player_position_like_cpp(&self) -> Option<Position> {
        self.core.player_position_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixture_position,
        )
    }

    fn resolved_player_vitals_like_cpp(&self) -> Option<(u32, u32, bool)> {
        self.core.resolved_player_vitals_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixture_health,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixture_max_health,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixture_alive,
        )
    }

    fn player_level_like_cpp(&self) -> u8 {
        self.core.player_level_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixture_level,
        )
    }

    pub fn update_registry_position(&self) {
        let (Some(guid), Some(pos), Some(reg)) = (
            self.core.player_guid(),
            self.player_position_like_cpp(),
            &self.core.player_registry,
        ) else {
            return;
        };
        let map_id = self.core.player_map_id_like_cpp();
        let Some(is_alive) = self
            .resolved_player_vitals_like_cpp()
            .map(|(_, _, is_alive)| is_alive)
        else {
            return;
        };
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let _ = reg.publish_movement_for_control_channel(
            guid,
            &self.core.session_command_tx,
            crate::session::directory::PlayerMovementDirectoryUpdate {
                position: pos,
                map_id,
                instance_id,
                is_in_world: self.core.player_is_in_world_for_registry_like_cpp(),
                level: self.player_level_like_cpp(),
                is_alive,
                transport: self.core.player_transport_info_with_fixture_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    self.fixture_transport,
                ),
            },
        );
    }
}
