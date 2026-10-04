// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::state::SessionCore;
use crate::session::{
    PacketPublicationAccessLikeCpp, PlayerRegistryControlBindingLikeCpp,
    PlayerRegistrySyncAccessLikeCpp, PlayerStatsAccessLikeCpp, SessionCatalogs,
    SessionWorldConfig,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::StatsFixtureRefs;
use wow_core::Position;

/// Read the canonical combat state used while selecting an equipment-set move.
pub struct EquipmentSetCombatAccessLikeCpp<'a> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    in_combat_fixture: &'a bool,
}

impl SessionCore {
    pub fn equipment_set_combat_access_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] in_combat_fixture: &'a bool,
    ) -> EquipmentSetCombatAccessLikeCpp<'a> {
        EquipmentSetCombatAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            in_combat_fixture,
        }
    }
}

impl EquipmentSetCombatAccessLikeCpp<'_> {
    pub fn resolved_in_combat_like_cpp(&self) -> Option<bool> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().subsystems().combat.has_combat());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(*self.in_combat_fixture);
        }
        canonical
    }
}

/// Finite owner access used by the Application equipment-set operation.
/// It exposes only the selected phase capabilities and never returns SessionCore.
pub struct EquipmentSetUseAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    pub fn equipment_set_use_access_like_cpp(&self) -> EquipmentSetUseAccessLikeCpp<'_> {
        EquipmentSetUseAccessLikeCpp { core: self }
    }
}

impl EquipmentSetUseAccessLikeCpp<'_> {
    pub fn player_guid_like_cpp(&self) -> Option<wow_core::ObjectGuid> {
        self.core.player_guid()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    pub fn packet_publication_access_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.core.packet_publication_access_like_cpp()
    }

    pub fn player_registry_sync_capabilities_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_position: &'a Option<Position>,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_transport: &'a Option<Box<crate::session::PlayerTransportLoginStateLikeCpp>>,
    ) -> Option<(
        PlayerRegistrySyncAccessLikeCpp<'a>,
        PlayerRegistryControlBindingLikeCpp<'a>,
    )> {
        let guid = self.core.player_guid()?;
        let registry = self.core.player_registry()?;
        let position = self.core.player_registry_sync_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_transport,
        );
        let control = self
            .core
            .player_registry_control_binding_like_cpp(guid, registry);
        Some((position, control))
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    pub fn player_stats_access_like_cpp<'a>(
        &'a self,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
    ) -> PlayerStatsAccessLikeCpp<'a> {
        self.core.player_stats_access_like_cpp(catalogs, config)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_stats_access_with_fixture_refs_like_cpp<'a>(
        &'a self,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
        player_race: &'a u8,
        player_class: &'a u8,
        player_level: &'a u8,
        fixtures: StatsFixtureRefs<'a>,
    ) -> PlayerStatsAccessLikeCpp<'a> {
        self.core.player_stats_access_with_fixture_refs_like_cpp(
            catalogs,
            config,
            player_race,
            player_class,
            player_level,
            fixtures,
        )
    }

}
