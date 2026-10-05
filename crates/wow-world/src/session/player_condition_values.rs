// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player condition values: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use std::sync::Arc;

use super::WorldSession;
use super::is_player_meeting_condition_like_cpp;
use wow_data::PlayerConditionContextLikeCpp;

impl WorldSession {
    /// Select the condition-projection participants without reading them.
    /// Callers that own a later evaluation point can pass this inert view
    /// across the boundary and project it there.
    pub(crate) fn player_condition_projection_cx_like_cpp(
        &self,
    ) -> wow_world_application::PlayerConditionProjectionCxLikeCpp<'_> {
        wow_world_application::PlayerConditionProjectionCxLikeCpp::new(
            self.core
                .player_condition_access_with_selected_fixture_refs_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    wow_world_core::session::PlayerConditionFixtureRefsLikeCpp::new(
                        &self.fixtures.identity.player_race,
                        &self.fixtures.identity.player_class,
                        &self.fixtures.identity.player_level,
                        &self.fixtures.identity.player_gender,
                        &self
                            .fixtures
                            .progression
                            .represented_primary_specialization_id_like_cpp,
                        &self.fixtures.movement.player_position,
                        &self.fixtures.identity.player_zone_id_like_cpp,
                        &self.fixtures.identity.player_area_id_like_cpp,
                        &self
                            .fixtures
                            .identity
                            .player_zone_area_authority_complete_like_cpp,
                        &self.fixtures.combat.player_pvp_hostile_like_cpp,
                        &self.fixtures.combat.player_pvp_end_timer_like_cpp,
                        &self.fixtures.combat.player_contested_pvp_timer_like_cpp,
                        &self.fixtures.identity.represented_is_outdoors_like_cpp,
                        &self.fixtures.combat.player_health_like_cpp,
                        &self.fixtures.combat.player_max_health_like_cpp,
                        &self.fixtures.combat.player_alive_like_cpp,
                        &self.fixtures.vehicles.taxi_destinations_like_cpp,
                        &self.fixtures.vehicles.taxi_flight_state_like_cpp,
                        &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
                        &self.fixtures.vehicles.taxi_mounted_like_cpp,
                        &self.fixtures.auras.visible_auras,
                        &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                        &self
                            .fixtures
                            .auras
                            .player_spell_hit_aura_authority_tombstoned_like_cpp,
                        &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                        &self
                            .fixtures
                            .progression
                            .player_skill_test_fixture_like_cpp
                            .player_skill_records_like_cpp,
                    ),
                ),
            &self.inventory,
            &self.social,
            self.catalogs.chr_specialization_store().map(Arc::as_ref),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.quest_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.instances,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.battleground,
            self.catalogs.inventory_valuation_catalog_view_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.reputation_state_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .battleground
                .represented_battleground_status_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_complete_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.combat.in_combat,
            #[cfg(any(test, feature = "test-fixtures"))]
            cfg!(test),
        )
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RepresentedPlayerConditionContextLikeCpp {
    projected: wow_world_application::RepresentedPlayerConditionContextLikeCpp,
}

/// Represented subset of C++ `Player::m_unitData` item-level cap fields
/// consumed by `Item::GetItemLevel(Player const*)`.
pub(crate) type RepresentedItemLevelCapsLikeCpp = wow_entities::PlayerItemLevelCapsLikeCpp;

impl RepresentedPlayerConditionContextLikeCpp {
    pub(crate) fn as_context<'a>(
        &'a self,
        session: &'a WorldSession,
    ) -> Option<PlayerConditionContextLikeCpp<'a>> {
        session
            .player_condition_projection_cx_like_cpp()
            .condition_context_like_cpp(&self.projected)
    }
}

impl WorldSession {
    pub(crate) fn represented_player_condition_context_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerConditionContextLikeCpp> {
        Some(RepresentedPlayerConditionContextLikeCpp {
            projected: self
                .player_condition_projection_cx_like_cpp()
                .project_like_cpp()?,
        })
    }

    pub(crate) fn represented_meets_player_condition_id_like_cpp(
        &self,
        player_condition_id: u32,
    ) -> bool {
        if player_condition_id == 0 {
            return true;
        }

        let Some(store) = self.catalogs.player_condition_store.as_ref() else {
            return false;
        };
        let Some(condition) = store.get(player_condition_id) else {
            return true;
        };

        let Some(context) = self.represented_player_condition_context_like_cpp() else {
            return false;
        };
        context
            .as_context(self)
            .is_some_and(|context| is_player_meeting_condition_like_cpp(condition, &context))
    }
}
