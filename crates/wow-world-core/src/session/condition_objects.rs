// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::WorldObject;

impl crate::session::HubRef<'_> {
    pub fn build_condition_player_object_like_cpp(&self) -> Option<WorldObject> {
        self.player_condition_access_like_cpp()
            .build_condition_player_object_like_cpp()
    }

    pub fn condition_player_unit_snapshot_like_cpp(
        &self,
    ) -> Option<wow_conditions::ConditionUnitSnapshot> {
        self.player_condition_access_like_cpp()
            .condition_player_unit_snapshot_like_cpp()
    }

    pub fn condition_player_snapshot_like_cpp(&self) -> wow_conditions::ConditionPlayerSnapshot {
        self.player_condition_access_like_cpp()
            .condition_player_snapshot_like_cpp()
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_condition_access_like_cpp(
        &self,
    ) -> crate::session::PlayerConditionAccessLikeCpp<'_> {
        self.core
            .player_condition_access_with_selected_fixture_refs_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                crate::session::PlayerConditionFixtureRefsLikeCpp::new(
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
            )
    }
}
