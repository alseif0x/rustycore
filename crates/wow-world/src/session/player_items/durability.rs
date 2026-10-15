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
    /// C++ `Spell::EffectDurabilityDamage` (`SpellEffects.cpp:4316-4352`).
    pub(in crate::session) fn apply_durability_damage_effect_like_cpp(
        &mut self,
        effect: u32,
        damage: i32,
        slot: i32,
        target_guid: ObjectGuid,
    ) {
        if self.player_guid() != Some(target_guid) {
            return;
        }
        let effect = i32::try_from(effect).unwrap_or(0);
        if slot < 0 {
            wow_world_inventory::apply_represented_durability_points_loss_all_like_cpp(
                self,
                damage,
                slot < -1,
            );
            // C++ logs `-1`/`-1` for the all-items branch
            // (`SpellEffects.cpp:4328`).
            wow_world_spell::record_spell_execute_log_durability_damage_like_cpp(
                self,
                effect,
                target_guid,
                -1,
                -1,
            );
            return;
        }
        let Ok(slot) = u8::try_from(slot) else {
            return;
        };
        if slot >= INVENTORY_SLOT_BAG_END {
            return;
        }
        let item_entry = self
            .resolved_inventory_item_like_cpp(slot)
            .map(|item| item.entry_id);
        if wow_world_inventory::apply_represented_durability_points_loss_at_slot_like_cpp(
            self, slot, damage,
        ) && let Some(item_entry) = item_entry
        {
            // C++ logs the item entry and the slot as `ItemID`/`Amount`
            // (`SpellEffects.cpp:4339`).
            wow_world_spell::record_spell_execute_log_durability_damage_like_cpp(
                self,
                effect,
                target_guid,
                i32::try_from(item_entry).unwrap_or(i32::MAX),
                i32::from(slot),
            );
        }
    }
    pub(crate) fn item_template_max_durability(&self, item_id: u32) -> u32 {
        self.catalogs.item_template_max_durability(item_id)
    }
}
