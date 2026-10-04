// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::PlayerStatsAccessLikeCpp;
use crate::session::{PlayerRegistryControlBindingLikeCpp, PlayerRegistrySyncAccessLikeCpp};

impl PlayerStatsAccessLikeCpp<'_> {
    pub fn item_scaling_pvp_rules_enabled_like_cpp(&self) -> bool {
        self.core.player_aura_subsystem_snapshot_with_fixture_refs_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.auras.player_aura_authority_complete_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.auras.visible_auras_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
        ).map(|auras| {
            auras.runtime_applications_like_cpp().values().any(|aura| {
                aura.spell_id == crate::session::SPELL_PVP_RULES_ENABLED_LIKE_CPP
            })
        }).unwrap_or(false)
    }

    pub fn item_scaling_registry_access_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] position: &'a Option<wow_core::Position>,
        #[cfg(any(test, feature = "test-fixtures"))]
        transport: &'a Option<Box<crate::session::PlayerTransportLoginStateLikeCpp>>,
    ) -> Option<(PlayerRegistrySyncAccessLikeCpp<'a>, PlayerRegistryControlBindingLikeCpp<'a>)> {
        let (Some(guid), Some(registry)) = (self.core.player_guid(), self.core.player_registry())
        else { return None; };
        let position = self.core.player_registry_sync_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            position,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            transport,
        );
        Some((position, self.core.player_registry_control_binding_like_cpp(guid, registry)))
    }
}
