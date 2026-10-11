// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_skill_updates_like_cpp(&self) -> &[(u32, u32)] {
        self.quest_state
            .fixture_represented_quest_reward_skill_updates_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardSpellCastLikeCpp] {
        self.quest_state
            .fixture_represented_quest_reward_spell_casts_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_titles_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardTitleLikeCpp] {
        self.quest_state
            .fixture_represented_quest_reward_titles_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_talent_points_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardTalentPointsLikeCpp] {
        self.quest_state
            .fixture_represented_quest_reward_talent_points_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_mails_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardMailLikeCpp] {
        self.quest_state
            .fixture_represented_quest_reward_mails_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_reputations_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardReputationLikeCpp] {
        self.quest_state
            .fixture_represented_quest_reward_reputations_like_cpp()
    }
}

impl WorldSession {
    pub(crate) async fn notify_game_event_quest_complete_like_cpp(
        &mut self,
        quest_id: u32,
    ) -> GameEventQuestCompleteClientOutcomeLikeCpp {
        #[cfg(any(test, feature = "test-fixtures"))]
        let player = self.core.quest_reward_player_access_like_cpp(
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let player = self.core.quest_reward_player_access_like_cpp();
        player
            .notify_game_event_quest_complete_like_cpp(quest_id)
            .await
    }
    pub(crate) fn record_represented_rewarded_quest_row_like_cpp(&mut self, quest_id: u32) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_rewarded_row_like_cpp(quest_id, true);
        });
    }
    pub(crate) fn complete_player_quest_status_authority_load_like_cpp(&mut self) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_status_authority_complete_like_cpp(true);
        });
    }
    pub(crate) fn add_represented_quest_reward_talent_points_like_cpp(
        &mut self,
        quest_id: u32,
        points: u32,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.add_quest_rewarded_talent_points_like_cpp(points);
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.quest_state
                .fixture_record_quest_reward_talent_points_like_cpp(
                    RepresentedQuestRewardTalentPointsLikeCpp {
                        quest_id,
                        points,
                        init_talent_for_level_unrepresented: true,
                    },
                );
            return true;
        }
        let _ = quest_id;
        false
    }
    pub(crate) fn set_represented_daily_quest_completed_like_cpp_for_test(
        &mut self,
        quest_id: u32,
        completed: bool,
    ) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            if completed {
                state.set_daily_like_cpp(quest_id, true);
            } else {
                state.set_daily_like_cpp(quest_id, false);
            }
        });
        self.sync_player_registry_state_like_cpp();
    }
    /// Apply one quest-reward currency grant and return its committed delta so
    /// callers can publish it at their own phase.
    pub(crate) fn add_currency_quest_reward_like_cpp(
        &mut self,
        currency_id: u32,
        amount: u32,
        gain_source: wow_constants::currency::CurrencyGainSourceLikeCpp,
    ) -> Result<Option<wow_world_core::session::PlayerCurrencyDelta>, ()> {
        let player = self.core.quest_reward_player_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_class,
        );
        wow_world_application::add_currency_quest_reward_like_cpp(
            &mut self.inventory,
            &player,
            self.catalogs.currency_types_store.as_deref(),
            currency_id,
            amount,
            gain_source,
        )
    }
}
