//! Represented item durability and repair cost.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

mod loss;
mod damage_effects;
mod repair_cost;
mod repair_item;
mod repair_all;

impl WorldSession {
    /// Set the C++ `sDurabilityCostsStore` equivalent for this session.
    pub fn set_durability_costs_store(&mut self, store: Arc<DurabilityCostsStore>) {
        self.items.install_durability_costs_store(store);
    }
    /// Set the C++ `sDurabilityQualityStore` equivalent for this session.
    pub fn set_durability_quality_store(&mut self, store: Arc<DurabilityQualityStore>) {
        self.items.install_durability_quality_store(store);
    }
    pub fn set_repair_cost_rate_like_cpp(&mut self, rate: f32) {
        self.repair_cost_rate_like_cpp = rate.max(0.0);
    }
    pub(crate) fn repair_cost_rate_like_cpp(&self) -> f32 {
        self.repair_cost_rate_like_cpp
    }
    /// Set the C++ `RATE_DURABILITY_LOSS_ON_DEATH` fraction
    /// (`DurabilityLoss.OnDeath / 100`).
    pub fn set_durability_loss_on_death_rate_like_cpp(&mut self, rate: f32) {
        self.durability_loss_on_death_rate_like_cpp = rate.clamp(0.0, 1.0);
    }
    #[must_use]
    pub(crate) fn durability_loss_on_death_rate_like_cpp(&self) -> f32 {
        self.durability_loss_on_death_rate_like_cpp
    }
    /// Set the C++ `CONFIG_STATS_LIMITS_*` values (`World.cpp:1664-1668`).
    pub fn set_stats_limits_like_cpp(&mut self, limits: wow_data::StatsLimitsLikeCpp) {
        self.stats_limits_like_cpp = limits;
    }
    #[must_use]
    pub(crate) fn stats_limits_like_cpp(&self) -> wow_data::StatsLimitsLikeCpp {
        self.stats_limits_like_cpp
    }
    /// Get the durability cost store reference.
    pub fn durability_costs_store(&self) -> Option<&Arc<DurabilityCostsStore>> {
        self.items.durability_costs_store()
    }
    /// Get the durability quality store reference.
    pub fn durability_quality_store(&self) -> Option<&Arc<DurabilityQualityStore>> {
        self.items.durability_quality_store()
    }
    pub fn item_template_max_durability(&self, item_id: u32) -> u32 {
        self.items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.max_durability)
            .unwrap_or(0)
    }
}
