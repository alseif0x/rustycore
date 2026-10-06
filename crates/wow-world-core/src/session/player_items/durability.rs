use wow_constants::ItemClass;

impl crate::session::state::SessionCatalogs {
    /// C++ `Item::CalculateDurabilityRepairCost`.
    pub fn item_durability_repair_cost_like_cpp(
        &self,
        item_id: u32,
        current_durability: u32,
        max_durability: u32,
        discount: f32,
        repair_cost_rate: f32,
    ) -> u64 {
        if max_durability == 0 {
            return 0;
        }

        debug_assert!(
            max_durability >= current_durability,
            "C++ Item::CalculateDurabilityRepairCost asserts max durability >= current durability"
        );
        if current_durability >= max_durability {
            return 0;
        }

        let item = match self
            .items
            .store
            .as_ref()
            .and_then(|store| store.get(item_id))
        {
            Some(item) => item,
            None => return 0,
        };
        let stats = match self
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.random_property_template(item_id))
        {
            Some(stats) => stats,
            None => return 0,
        };
        if stats.quality < 0 {
            return 0;
        }

        let durability_cost = match self
            .durability_costs_store
            .as_ref()
            .and_then(|store| store.get(u32::from(stats.item_level)))
        {
            Some(cost) => cost,
            None => return 0,
        };
        let durability_quality_entry_id = (stats.quality as u32 + 1) * 2;
        let durability_quality = match self
            .durability_quality_store
            .as_ref()
            .and_then(|store| store.get(durability_quality_entry_id))
        {
            Some(quality) => quality,
            None => return 0,
        };

        let subclass = item.subclass_id as usize;
        let multiplier = if item.class_id == ItemClass::Weapon as u8 {
            durability_cost
                .weapon_sub_class_cost
                .get(subclass)
                .copied()
                .unwrap_or(0)
        } else if item.class_id == ItemClass::Armor as u8 {
            durability_cost
                .armor_sub_class_cost
                .get(subclass)
                .copied()
                .unwrap_or(0)
        } else {
            0
        };

        let lost_durability = max_durability - current_durability;
        let rounded =
            (lost_durability as f32 * multiplier as f32 * durability_quality.data * 1.0f32).round();
        let cost = (rounded * discount * repair_cost_rate) as u64;

        if cost == 0 { 1 } else { cost }
    }
}

impl crate::session::state::SessionWorldConfig {
    pub fn repair_cost_rate_like_cpp(&self) -> f32 {
        self.repair_cost_rate_like_cpp
    }

    #[must_use]
    pub fn durability_loss_on_death_rate_like_cpp(&self) -> f32 {
        self.durability_loss_on_death_rate_like_cpp
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn item_template_max_durability(&self, item_id: u32) -> u32 {
        self.items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.max_durability)
            .unwrap_or(0)
    }
}

impl crate::session::state::SessionWorldConfig {
    #[must_use]
    pub fn stats_limits_like_cpp(&self) -> wow_data::StatsLimitsLikeCpp {
        self.stats_limits_like_cpp
    }
}
