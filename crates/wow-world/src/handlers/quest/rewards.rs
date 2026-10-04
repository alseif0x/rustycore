// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapters for the App-owned quest reward operation.

use super::*;
use crate::quest::application::QuestRewardDurablePlanLikeCpp;

mod currencies;
mod controller;
mod items;
mod validation;

impl WorldSession {
    async fn remove_quest_required_items_and_currencies_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        let stats_fixtures = wow_world_core::session::StatsFixtureRefs::new_like_cpp(
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
                &self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                &self.fixtures.auras.visible_auras,
                &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            ),
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
        operation.remove_quest_required_items_and_currencies_like_cpp(
            plan,
            quest,
            #[cfg(any(test, feature = "test-fixtures"))]
            stats_fixtures,
        ).await
    }

    fn record_represented_quest_reward_reputation_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut fixtures = wow_world_application::QuestRewardReputationFixtureRefsLikeCpp::new_like_cpp(
            &mut self.fixtures.progression.reputation_state_like_cpp,
            &self.fixtures.progression.represented_gray_level_script_overrides_like_cpp,
            &self.fixtures.movement.player_position,
            &self.fixtures.auras.player_aura_authority_complete_like_cpp,
            &self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
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
        operation.record_represented_quest_reward_reputation_like_cpp(
            quest,
            #[cfg(any(test, feature = "test-fixtures"))]
            &mut fixtures,
        );
    }

    fn apply_quest_reward_lockout_status_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        let mut player = self.core.quest_reward_player_access_like_cpp(
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let mut player = self.core.quest_reward_player_access_like_cpp();
        wow_world_application::QuestRewardCx::apply_quest_reward_lockout_status_like_cpp(
            &mut self.quest_state,
            &mut player,
            plan,
            quest,
            cfg!(test),
        );
    }

    #[cfg(test)]
    pub(super) async fn reward_represented_quest_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.reward_represented_quest_with_generator_like_cpp(
            generator.as_ref(),
            quest,
            quest_giver_guid,
            choice,
        )
        .await
    }
}
