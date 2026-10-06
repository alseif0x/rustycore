//! Represented reputation standings and their published changes.
//!
//! Moved out of the Session root under #611. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_reputation_rates_like_cpp(&mut self, rates: ReputationRatesLikeCpp) {
        self.config.reputation_rates = rates;
    }
    pub(crate) fn reputation_rates_like_cpp(&self) -> ReputationRatesLikeCpp {
        self.config.reputation_rates_like_cpp()
    }
    #[allow(dead_code)]
    pub(crate) fn reputation_rank_like_cpp(
        &self,
        faction_entry: &FactionEntry,
        standing: i32,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        reputation_to_rank_like_cpp(
            faction_entry,
            standing,
            self.catalogs.friendship_rep_reaction_store.as_deref(),
        )
    }
    pub fn set_paragon_reputation_store(&mut self, store: Arc<ParagonReputationStore>) {
        self.catalogs.paragon_reputation_store = Some(store);
        crate::session::hub_mut(self).initialize_reputation_mgr_like_cpp();
    }
    pub fn set_reputation_reward_rate_store(
        &mut self,
        store: Arc<ReputationRewardRateStoreLikeCpp>,
    ) {
        self.catalogs.reputation_reward_rate_store = Some(store);
    }
    pub fn set_reputation_spillover_template_store(
        &mut self,
        store: Arc<RepSpilloverTemplateStoreLikeCpp>,
    ) {
        self.catalogs.reputation_spillover_template_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn apply_represented_first_login_reputation_like_cpp(&mut self) -> usize {
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        crate::session::hub_mut(self)
            .apply_represented_first_login_reputation_with_catalogs_like_cpp(&player_bootstrap)
    }
    pub(in crate::session) fn calculate_kill_reputation_gain_like_cpp(
        &self,
        creature_level: u8,
        rep: i32,
        faction_id: u32,
    ) -> i32 {
        self.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Kill,
            u32::from(creature_level),
            rep,
            faction_id,
            false,
        )
    }
    pub(crate) fn reputation_gain_percent_before_reward_rate_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        creature_or_quest_level: u32,
        rep: i32,
        faction_id: u32,
        no_quest_bonus: bool,
    ) -> Option<f32> {
        let mut percent = 100.0f32;
        let mut rep_mod = if no_quest_bonus {
            0.0
        } else {
            crate::session::hub_ref(self).resolved_total_represented_aura_modifier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModReputationGain,
            )? as f32
        };

        if source == ReputationGainSourceLikeCpp::Kill {
            rep_mod += self.resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
                RepresentedAuraEffectLikeCpp::ModFactionReputationGain,
                faction_id as i32,
            )? as f32;
        }

        percent += if rep > 0 { rep_mod } else { -rep_mod };

        let reputation_rates = self.config.reputation_rates_like_cpp();
        let low_level_rate = match source {
            ReputationGainSourceLikeCpp::Kill => reputation_rates.low_level_kill,
            ReputationGainSourceLikeCpp::Quest
            | ReputationGainSourceLikeCpp::DailyQuest
            | ReputationGainSourceLikeCpp::WeeklyQuest
            | ReputationGainSourceLikeCpp::MonthlyQuest
            | ReputationGainSourceLikeCpp::RepeatableQuest => reputation_rates.low_level_quest,
            ReputationGainSourceLikeCpp::Spell => 1.0,
        };
        if low_level_rate != 1.0
            && creature_or_quest_level
                < u32::from(
                    crate::session::hub_ref(self)
                        .gray_level(crate::session::hub_ref(self).player_level_like_cpp()),
                )
        {
            percent *= low_level_rate;
        }

        (percent > 0.0).then_some(percent)
    }
    pub(crate) fn calculate_reputation_gain_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        creature_or_quest_level: u32,
        rep: i32,
        faction_id: u32,
        no_quest_bonus: bool,
    ) -> i32 {
        let Some(mut percent) = self.reputation_gain_percent_before_reward_rate_like_cpp(
            source,
            creature_or_quest_level,
            rep,
            faction_id,
            no_quest_bonus,
        ) else {
            return 0;
        };

        if let Some(rep_rate) = crate::session::hub_ref(self)
            .reputation_reward_rate_for_source_like_cpp(source, faction_id)
        {
            if rep_rate <= 0.0 {
                return 0;
            }
            percent *= rep_rate;
        }

        percent = self.apply_recruit_a_friend_reputation_bonus_like_cpp(source, percent);

        (rep as f32 * percent / 100.0) as i32
    }
    pub(crate) fn apply_recruit_a_friend_reputation_bonus_like_cpp(
        &self,
        source: ReputationGainSourceLikeCpp,
        mut percent: f32,
    ) -> f32 {
        if source != ReputationGainSourceLikeCpp::Spell
            && self.gets_recruit_a_friend_reputation_bonus_like_cpp()
        {
            percent *= 1.0
                + self
                    .config
                    .reputation_rates_like_cpp()
                    .recruit_a_friend_bonus;
        }
        percent
    }
    fn gets_recruit_a_friend_reputation_bonus_like_cpp(&self) -> bool {
        self.gets_recruit_a_friend_bonus_like_cpp(false)
    }
    #[cfg(test)]
    pub(crate) async fn reputation_changed_like_cpp(&mut self, faction_id: u32, change: i32) {
        self.quest_state
            .enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::ReputationChanged {
                    faction_id,
                    change,
                },
            );
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/progression/reputation/f3_shims.rs"]
mod f3_shims;
