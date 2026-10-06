// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected-owner adapters for App quest-reward item delivery.

use super::*;

enum QuestRewardItemGrantLikeCpp {
    Fixed,
    Chosen(QuestChoiceItemLikeCpp),
    Package(QuestChoiceItemLikeCpp),
}

impl WorldSession {
    pub(super) async fn store_fixed_quest_reward_items_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        self.apply_quest_reward_item_grant_like_cpp(
            plan,
            item_guid_generator,
            quest,
            QuestRewardItemGrantLikeCpp::Fixed,
        )
        .await
    }

    pub(super) async fn store_chosen_quest_reward_item_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        self.apply_quest_reward_item_grant_like_cpp(
            plan,
            item_guid_generator,
            quest,
            QuestRewardItemGrantLikeCpp::Chosen(choice),
        )
        .await
    }

    pub(super) async fn store_quest_package_reward_items_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        self.apply_quest_reward_item_grant_like_cpp(
            plan,
            item_guid_generator,
            quest,
            QuestRewardItemGrantLikeCpp::Package(choice),
        )
        .await
    }

    async fn apply_quest_reward_item_grant_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        grant: QuestRewardItemGrantLikeCpp,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        let fixtures =
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
        let vitals = (
            &self.fixtures.combat.player_health_like_cpp,
            &self.fixtures.combat.player_max_health_like_cpp,
            &self.fixtures.combat.player_alive_like_cpp,
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
        match grant {
            QuestRewardItemGrantLikeCpp::Fixed => {
                operation
                    .store_fixed_quest_reward_items_like_cpp(
                        plan,
                        item_guid_generator,
                        quest,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        &fixtures,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        vitals,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        &self.fixtures.progression.reputation_state_like_cpp,
                    )
                    .await
            }
            QuestRewardItemGrantLikeCpp::Chosen(choice) => {
                operation
                    .store_chosen_quest_reward_item_like_cpp(
                        plan,
                        item_guid_generator,
                        quest,
                        choice.item_id,
                        choice.loot_item_type,
                        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        &fixtures,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        vitals,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        &self.fixtures.progression.reputation_state_like_cpp,
                    )
                    .await
            }
            QuestRewardItemGrantLikeCpp::Package(choice) => {
                operation
                    .store_quest_package_reward_items_like_cpp(
                        plan,
                        item_guid_generator,
                        quest,
                        choice.item_id,
                        choice.loot_item_type,
                        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        &fixtures,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        vitals,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        &self.fixtures.progression.reputation_state_like_cpp,
                    )
                    .await
            }
        }
    }
}
