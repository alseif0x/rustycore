//! Represented spell-click interaction with observed creatures.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn represented_can_see_spell_click_on_creature_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> RepresentedCanSeeSpellClickOutcomeLikeCpp {
        let owner = self.core.quest_objective_access_like_cpp();
        let npc_access = self
            .core
            .npc_interaction_access_with_selected_refs_like_cpp(
                self.catalogs.factions.store.as_deref(),
                self.catalogs.factions.template_store.as_deref(),
                self.catalogs.friendship_rep_reaction_store.as_deref(),
                #[cfg(any(test, feature = "test-fixtures"))]
                wow_world_core::session::NpcInteractionFixtureRefsLikeCpp::new(
                    &self.fixtures.movement.player_position,
                    &self.fixtures.identity.player_faction_template_like_cpp,
                    &self.fixtures.identity.player_race,
                    &self.fixtures.identity.player_class,
                    &self.fixtures.combat.player_health_like_cpp,
                    &self.fixtures.combat.player_max_health_like_cpp,
                    &self.fixtures.combat.player_alive_like_cpp,
                    &self.fixtures.progression.reputation_state_like_cpp,
                    &self.fixtures.vehicles.taxi_destinations_like_cpp,
                    &self.fixtures.vehicles.taxi_flight_state_like_cpp,
                    &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
                    &self.fixtures.vehicles.taxi_mounted_like_cpp,
                ),
            );
        let player_access = self
            .core
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
            );
        let condition_projection = self.player_condition_projection_cx_like_cpp();
        wow_world_quest::represented_can_see_spell_click_on_like_cpp(
            &self.world_entities,
            &owner,
            &npc_access,
            &player_access,
            &condition_projection,
            &self.social,
            self.catalogs.chr.specialization_store.as_deref(),
            creature_guid,
            self.catalogs
                .spell_catalogs
                .npc_spell_click_store
                .as_deref(),
            self.catalogs.condition_store.as_deref(),
            self.catalogs.player_condition_store.as_ref(),
            self.catalogs.area_table_store.as_ref(),
            cfg!(test),
        )
    }
}
