// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inert selected-owner construction for the complete App quest reward.

use super::*;

impl WorldSession {
    pub(in crate::handlers::quest) async fn reward_represented_quest_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut xp_fixtures = wow_world_application::QuestXpGainFixtureRefsLikeCpp::new_like_cpp(
            wow_world_core::session::CoreXPGainFixtureRefsLikeCpp::new_like_cpp(
                &mut self.fixtures.progression.player_xp,
                &mut self.fixtures.progression.player_next_level_xp,
                &self.fixtures.movement.player_position,
                &self
                    .fixtures
                    .battleground
                    .player_battleground_type_id_like_cpp,
                &self
                    .fixtures
                    .battleground
                    .player_battleground_map_id_like_cpp,
                &self
                    .fixtures
                    .battleground
                    .represented_battleground_status_like_cpp,
                &self
                    .fixtures
                    .battleground
                    .represented_battleground_queue_slots_like_cpp,
                &self
                    .fixtures
                    .battleground
                    .represented_arena_team_id_invited_like_cpp,
                &mut self.fixtures.progression.rest_mgr_test_fixture_like_cpp,
                &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                &self
                    .fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                &self.fixtures.auras.visible_auras,
                &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            ),
            &mut self.fixtures.progression.player_character_points_like_cpp,
            &self
                .fixtures
                .progression
                .represented_gray_level_script_overrides_like_cpp,
            &self.fixtures.progression.represented_talents_like_cpp,
            &self
                .fixtures
                .progression
                .represented_active_talent_group_like_cpp,
            wow_world_core::session::StatsFixtureRefs::new_like_cpp(
                wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                    &mut self.fixtures.combat.player_health_like_cpp,
                    &mut self.fixtures.combat.player_max_health_like_cpp,
                    &mut self.fixtures.combat.player_alive_like_cpp,
                    &mut self.fixtures.combat.represented_player_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_max_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_base_mana_like_cpp,
                ),
                wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                    &self.fixtures.auras.represented_shapeshift_form_like_cpp,
                    &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                    &self
                        .fixtures
                        .auras
                        .player_spell_hit_aura_authority_tombstoned_like_cpp,
                    &self.fixtures.auras.visible_auras,
                    &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                ),
            ),
        );
        #[cfg(any(test, feature = "test-fixtures"))]
        let planning_fixtures =
            wow_world_application::QuestRewardItemPlanningFixtureRefsLikeCpp::new_like_cpp(
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
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
                &self.spell_state,
                &self.instances,
                &self.fixtures.battleground,
                &self
                    .fixtures
                    .battleground
                    .represented_battleground_status_like_cpp,
                &self
                    .fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_records_complete_like_cpp,
                &self.fixtures.combat.in_combat,
            );
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut reputation_fixtures =
            wow_world_application::QuestRewardReputationFixtureRefsLikeCpp::new_like_cpp(
                &mut self.fixtures.progression.reputation_state_like_cpp,
                &self
                    .fixtures
                    .progression
                    .represented_gray_level_script_overrides_like_cpp,
                &self.fixtures.movement.player_position,
                &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                &self
                    .fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                &self.fixtures.auras.visible_auras,
                &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            );
        let player = self.core.quest_reward_player_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_class,
        );
        let mut operation = wow_world_application::QuestRewardCx::new(
            &mut self.inventory,
            &mut self.lifecycle,
            &mut self.quest_state,
            player,
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut self.fixtures.identity.player_level,
            &self.catalogs,
            &self.config,
            &self.social,
            self.catalogs.currency_types_store.as_deref(),
            self.catalogs.quests.xp_store.as_deref(),
            cfg!(test),
        );
        operation
            .reward_quest_with_generator_like_cpp(
                item_guid_generator,
                quest,
                quest_giver_guid,
                choice.item_id,
                choice.loot_item_type,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
                #[cfg(any(test, feature = "test-fixtures"))]
                &mut xp_fixtures,
                #[cfg(any(test, feature = "test-fixtures"))]
                &mut self.fixtures.teleport,
                #[cfg(any(test, feature = "test-fixtures"))]
                &planning_fixtures,
                &self.loot,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.vehicles.player_transport_login_state_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                (
                    &self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                    &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                    &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                    &self.fixtures.pets.represented_pet_guid_like_cpp,
                ),
                #[cfg(any(test, feature = "test-fixtures"))]
                &mut reputation_fixtures,
            )
            .await
    }
}
