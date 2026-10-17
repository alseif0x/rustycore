//! Represented reputation standings and their published changes.
//!
//! Moved out of the Session root under #611. The gain arithmetic moved to
//! `wow_world_social::reputation_gain` (#1363 slice 7b).

use super::*;

impl WorldSession {
    pub fn set_reputation_rates_like_cpp(&mut self, rates: ReputationRatesLikeCpp) {
        self.config.reputation_rates = rates;
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
}

#[cfg(test)]
#[path = "../../../unit_tests/session/progression/reputation/f3_shims.rs"]
mod f3_shims;
