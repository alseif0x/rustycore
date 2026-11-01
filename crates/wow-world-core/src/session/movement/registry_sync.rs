// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow access to the canonical inputs needed to publish a Player's registry position.

use crate::player_directory::PlayerRegistry;
use crate::session::mailbox::SessionCommand;
use crate::session::state::SessionCore;
use wow_core::{ObjectGuid, Position};

/// Inert fixture participants lent to the final registry publication.
///
/// The builder only carries the selected mutable-vitals inputs; it reads no
/// canonical or fixture state when constructed. World/Stats lend it at the
/// publication phase, so Registry keeps its own fresh GUID and registry lookups
/// without holding a second simultaneous borrow of the fixtures that a
/// `PlayerStatsAccessLikeCpp` mutates.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct RegistrySyncInputs<'a> {
    fixture_health: &'a u32,
    fixture_max_health: &'a u32,
    fixture_alive: &'a bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> RegistrySyncInputs<'a> {
    pub fn new_like_cpp(
        fixture_health: &'a u32,
        fixture_max_health: &'a u32,
        fixture_alive: &'a bool,
    ) -> Self {
        Self {
            fixture_health,
            fixture_max_health,
            fixture_alive,
        }
    }

    /// Reborrow the same selected participants without reading state.
    pub fn reborrow_like_cpp(&self) -> RegistrySyncInputs<'_> {
        RegistrySyncInputs {
            fixture_health: self.fixture_health,
            fixture_max_health: self.fixture_max_health,
            fixture_alive: self.fixture_alive,
        }
    }
}

/// Borrows the canonical Player and the exact fixture inputs used by registry synchronization.
/// The fields stay private so callers can perform the operation without receiving a mutable
/// SessionCore or a general-purpose view of its state.
pub struct PlayerRegistrySyncAccessLikeCpp<'a> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_position: &'a Option<Position>,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_transport: &'a Option<Box<crate::session::PlayerTransportLoginStateLikeCpp>>,
}

impl SessionCore {
    /// Build the narrow registry synchronization capability from borrowed fixture values.
    ///
    /// The mutable-vitals participants are not captured here: the caller lends
    /// them through [`RegistrySyncInputs`] at the final publication phase.
    pub fn player_registry_sync_access_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_position: &'a Option<Position>,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_transport: &'a Option<
            Box<crate::session::PlayerTransportLoginStateLikeCpp>,
        >,
    ) -> PlayerRegistrySyncAccessLikeCpp<'a> {
        PlayerRegistrySyncAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_transport,
        }
    }
}

impl PlayerRegistrySyncAccessLikeCpp<'_> {
    /// Reborrow the same selected inputs without reading canonical state.
    pub fn reborrow_like_cpp(&self) -> PlayerRegistrySyncAccessLikeCpp<'_> {
        PlayerRegistrySyncAccessLikeCpp {
            core: self.core,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_position: self.fixture_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level: self.fixture_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_transport: self.fixture_transport,
        }
    }

    /// Resolve the current control binding at the caller's final sync phase.
    pub fn control_binding_if_available_like_cpp(
        &self,
    ) -> Option<crate::session::PlayerRegistryControlBindingLikeCpp<'_>> {
        let guid = self.core.player_guid()?;
        let registry = self.core.player_registry()?;
        Some(
            self.core
                .player_registry_control_binding_like_cpp(guid, registry),
        )
    }

    fn player_position_like_cpp(&self) -> Option<Position> {
        self.core.player_position_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixture_position,
        )
    }

    fn resolved_player_vitals_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] inputs: &RegistrySyncInputs<'_>,
    ) -> Option<(u32, u32, bool)> {
        self.core.resolved_player_vitals_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            inputs.fixture_health,
            #[cfg(any(test, feature = "test-fixtures"))]
            inputs.fixture_max_health,
            #[cfg(any(test, feature = "test-fixtures"))]
            inputs.fixture_alive,
        )
    }

    fn player_level_like_cpp(&self) -> u8 {
        self.core.player_level_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixture_level,
        )
    }

    pub fn update_registry_position(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] inputs: &RegistrySyncInputs<'_>,
    ) {
        let (Some(guid), Some(pos), Some(reg)) = (
            self.core.player_guid(),
            self.player_position_like_cpp(),
            &self.core.player_registry,
        ) else {
            return;
        };
        let map_id = self.core.player_map_id_like_cpp();
        let Some(is_alive) = self
            .resolved_player_vitals_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                inputs,
            )
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

/// Captures the proven session incarnation and its control-channel endpoints for one registry
/// publication sequence. The private fields prevent callers from using this as a registry view.
pub struct PlayerRegistryControlBindingLikeCpp<'a> {
    core: &'a SessionCore,
    guid: ObjectGuid,
    registry: &'a PlayerRegistry,
    command_tx: &'a flume::Sender<SessionCommand>,
}

impl SessionCore {
    /// Bind the already-proven registration identity to this session's publication rails.
    pub fn player_registry_control_binding_like_cpp<'a>(
        &'a self,
        guid: ObjectGuid,
        registry: &'a PlayerRegistry,
    ) -> PlayerRegistryControlBindingLikeCpp<'a> {
        PlayerRegistryControlBindingLikeCpp {
            core: self,
            guid,
            registry,
            command_tx: &self.session_command_tx,
        }
    }

    /// Re-resolve this session's current registry binding before publishing party state.
    pub fn sync_player_registry_party_member_party_type_like_cpp(&self) {
        let (Some(guid), Some(registry)) = (self.player_guid(), self.player_registry.as_ref())
        else {
            return;
        };
        let party_type = self.party_member_party_type_like_cpp();
        registry.publish_party_type_for_control_channel(guid, &self.session_command_tx, party_type);
    }
}

impl PlayerRegistryControlBindingLikeCpp<'_> {
    /// Replace the session incarnation's current loot-roll identities on its control rail.
    pub fn replace_loot_rolls_like_cpp(
        &self,
        identities: Vec<crate::session::mailbox::LootRollCommandIdentityLikeCpp>,
    ) -> bool {
        self.registry
            .replace_loot_rolls_for_control_channel(self.guid, self.command_tx, identities)
    }

    /// Publish party state through a fresh Core-owned GUID/registry proof.
    pub fn sync_party_member_party_type_like_cpp(&self) {
        self.core
            .sync_player_registry_party_member_party_type_like_cpp();
    }
}
