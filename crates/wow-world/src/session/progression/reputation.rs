//! Represented reputation standings and their published changes.
//!
//! Moved out of the Session root under #611. The gain arithmetic moved to
//! `wow_world_social::reputation_gain` (#1363 slice 7b).

use super::*;

impl WorldSession {
    pub fn set_reputation_rates_like_cpp(&mut self, rates: ReputationRatesLikeCpp) {
        self.config.reputation_rates = rates;
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
