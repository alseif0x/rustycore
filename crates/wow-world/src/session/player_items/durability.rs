//! Represented item durability and repair cost.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the C++ `sDurabilityCostsStore` equivalent for this session.
    pub fn set_durability_costs_store(&mut self, store: Arc<DurabilityCostsStore>) {
        self.catalogs.durability_costs_store = Some(store);
    }
    /// Set the C++ `sDurabilityQualityStore` equivalent for this session.
    pub fn set_durability_quality_store(&mut self, store: Arc<DurabilityQualityStore>) {
        self.catalogs.durability_quality_store = Some(store);
    }
    pub fn set_repair_cost_rate_like_cpp(&mut self, rate: f32) {
        self.config.repair_cost_rate_like_cpp = rate.max(0.0);
    }
    /// Set the C++ `RATE_DURABILITY_LOSS_ON_DEATH` fraction
    /// (`DurabilityLoss.OnDeath / 100`).
    pub fn set_durability_loss_on_death_rate_like_cpp(&mut self, rate: f32) {
        self.config.durability_loss_on_death_rate_like_cpp = rate.clamp(0.0, 1.0);
    }
    /// Set the C++ `CONFIG_STATS_LIMITS_*` values (`World.cpp:1664-1668`).
    pub fn set_stats_limits_like_cpp(&mut self, limits: wow_data::StatsLimitsLikeCpp) {
        self.config.stats_limits_like_cpp = limits;
    }
}
