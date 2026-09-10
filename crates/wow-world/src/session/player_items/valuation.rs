//! Represented item level and price valuation.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// C++ `Item::GetBuyPrice(proto, quality, itemLevel, standardPrice)`.
    ///
    /// This preserves the contrasted branch behavior where `standardPrice`
    /// remains false even after the calculated-price path.
    fn item_buy_price_with_catalogs_like_cpp(
        &self,
        catalogs: &ItemValuationCatalogsLikeCpp,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<(u32, bool)> {
        let basic = self.items.store.as_ref()?.get(item_id)?;
        let sparse = self.items.stats_store.as_ref()?.sparse_template(item_id)?;
        let flags2 = sparse.flags[1];
        let standard_price = false;

        if (flags2 & ItemFlags2::OverrideGoldCost as u32) != 0 {
            return Some((sparse.buy_price, standard_price));
        }

        let stores = catalogs.import_prices.as_ref();
        let quality_price = match stores.quality.get(quality + 1) {
            Some(entry) => entry.data,
            None => return Some((0, standard_price)),
        };
        let (base_armor, base_weapon) =
            match crate::session_rules::item_price_base_with_catalogs_like_cpp(catalogs, item_level)
            {
                Some(base) => base,
                None => return Some((0, standard_price)),
            };

        let mut inventory_type =
            <InventoryType as num_traits::FromPrimitive>::from_i8(sparse.inventory_type)
                .unwrap_or(InventoryType::NonEquip);
        let mut base_factor = if matches!(
            inventory_type,
            InventoryType::Weapon
                | InventoryType::Weapon2Hand
                | InventoryType::WeaponMainhand
                | InventoryType::WeaponOffhand
                | InventoryType::Ranged
                | InventoryType::Thrown
                | InventoryType::RangedRight
        ) {
            base_weapon
        } else {
            base_armor
        };

        if inventory_type == InventoryType::Robe {
            inventory_type = InventoryType::Chest;
        }

        if basic.class_id == ItemClass::Gem as u8 && basic.subclass_id == 11 {
            inventory_type = InventoryType::Weapon;
            base_factor = base_weapon / 3.0;
        }

        let type_factor = match inventory_type {
            InventoryType::Head
            | InventoryType::Neck
            | InventoryType::Shoulders
            | InventoryType::Chest
            | InventoryType::Waist
            | InventoryType::Legs
            | InventoryType::Feet
            | InventoryType::Wrists
            | InventoryType::Hands
            | InventoryType::Finger
            | InventoryType::Trinket
            | InventoryType::Cloak
            | InventoryType::Holdable => {
                let armor_price = match stores.armor.get(inventory_type as u32) {
                    Some(entry) => entry,
                    None => return Some((0, standard_price)),
                };
                match basic.subclass_id {
                    0 | 1 => armor_price.cloth_modifier,
                    2 => armor_price.leather_modifier,
                    3 => armor_price.chain_modifier,
                    4 => armor_price.plate_modifier,
                    _ => 1.0,
                }
            }
            InventoryType::Shield => match stores.shield.get(2) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::WeaponMainhand => match stores.weapon.get(1) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::WeaponOffhand => match stores.weapon.get(2) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::Weapon => match stores.weapon.get(3) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::Weapon2Hand => match stores.weapon.get(4) {
                Some(entry) => entry.data,
                None => return Some((0, standard_price)),
            },
            InventoryType::Ranged | InventoryType::RangedRight | InventoryType::Relic => {
                match stores.weapon.get(5) {
                    Some(entry) => entry.data,
                    None => return Some((0, standard_price)),
                }
            }
            _ => return Some((sparse.buy_price, standard_price)),
        };

        let cost = sparse.price_variance
            * type_factor
            * base_factor
            * quality_price
            * sparse.price_random_value;
        Some((cost as u32, standard_price))
    }
    /// C++ `Item::GetSellPrice(proto, quality, itemLevel)`.
    pub(in crate::session) fn item_sell_price_with_catalogs_like_cpp(
        &self,
        catalogs: &ItemValuationCatalogsLikeCpp,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<u32> {
        let basic = self.items.store.as_ref()?.get(item_id)?;
        let sparse = self.items.stats_store.as_ref()?.sparse_template(item_id)?;

        if (sparse.flags[1] & ItemFlags2::OverrideGoldCost as u32) != 0 {
            return Some(sparse.sell_price);
        }

        let (cost, standard_price) =
            self.item_buy_price_with_catalogs_like_cpp(catalogs, item_id, quality, item_level)?;
        if standard_price {
            let price_modifier = catalogs
                .item_classes
                .get_by_old_enum(u32::from(basic.class_id))?
                .price_modifier;
            let buy_count = sparse.vendor_stack_count.max(1);
            Some((cost as f32 * price_modifier / buy_count as f32) as u32)
        } else {
            Some(sparse.sell_price)
        }
    }
    #[cfg(test)]
    pub(crate) fn item_valuation_catalogs_for_test_like_cpp(&self) -> ItemValuationCatalogsLikeCpp {
        let mut catalogs = ItemValuationCatalogsLikeCpp::default();
        if let Some(store) = &self.import_price_stores {
            catalogs.import_prices = Arc::clone(store);
        }
        if let Some(store) = &self.item_price_base_store {
            catalogs.price_base = Arc::clone(store);
        }
        if let Some(store) = &self.item_class_store {
            catalogs.item_classes = Arc::clone(store);
        }
        if let Some(store) = &self.item_currency_cost_store {
            catalogs.currency_costs = Arc::clone(store);
        }
        if let Some(store) = &self.item_disenchant_loot_store {
            catalogs.disenchant_loot = Arc::clone(store);
        }
        catalogs
    }
    #[cfg(test)]
    pub fn item_buy_price_like_cpp(
        &self,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<(u32, bool)> {
        self.item_buy_price_with_catalogs_like_cpp(
            &self.item_valuation_catalogs_for_test_like_cpp(),
            item_id,
            quality,
            item_level,
        )
    }
    #[cfg(test)]
    pub fn item_sell_price_like_cpp(
        &self,
        item_id: u32,
        quality: u32,
        item_level: u32,
    ) -> Option<u32> {
        self.item_sell_price_with_catalogs_like_cpp(
            &self.item_valuation_catalogs_for_test_like_cpp(),
            item_id,
            quality,
            item_level,
        )
    }
    pub(crate) fn set_represented_item_level_caps_like_cpp(
        &mut self,
        caps: RepresentedItemLevelCapsLikeCpp,
    ) -> bool {
        self.mutate_player_item_modifier_runtime_like_cpp(|state| state.item_level_caps = caps)
            .is_some()
    }
    pub(crate) fn set_represented_using_pvp_item_levels_like_cpp(&mut self, active: bool) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.gameplay_state_mut().using_pvp_item_levels = active
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_using_pvp_item_levels_like_cpp = active;
            return true;
        }
        false
    }
    pub(in crate::session) fn resolved_using_pvp_item_levels_like_cpp(&self) -> Option<bool> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().using_pvp_item_levels);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(self.represented_using_pvp_item_levels_like_cpp);
        }
        None
    }
    #[cfg(test)]
    pub(crate) fn represented_using_pvp_item_levels_like_cpp(&self) -> bool {
        self.resolved_using_pvp_item_levels_like_cpp()
            .expect("test Player PvP item-level owner must resolve")
    }
    pub(crate) fn update_represented_item_level_area_based_scaling_like_cpp(&mut self) -> bool {
        self.update_represented_item_level_area_based_scaling_with_publication_like_cpp(true)
            .unwrap_or(false)
    }
    pub(crate) fn update_represented_item_level_area_based_scaling_with_publication_like_cpp(
        &mut self,
        publish: bool,
    ) -> Option<bool> {
        let map_pvp_activity = self
            .map_store()
            .and_then(|store| store.get(u32::from(self.player_map_id_like_cpp())))
            .is_some_and(|entry| {
                entry.is_battleground_or_arena() || entry.activates_pvp_item_levels_like_cpp()
            });
        let pvp_activity = map_pvp_activity || self.represented_has_pvp_rules_enabled_like_cpp();
        let Some(using_pvp_item_levels) = self.resolved_using_pvp_item_levels_like_cpp() else {
            return None;
        };
        if using_pvp_item_levels == pvp_activity {
            return Some(false);
        }

        let Some((health_before, max_health_before, _)) = self.resolved_player_vitals_like_cpp()
        else {
            return None;
        };
        let Some(item_mod_targets) = self.represented_top_level_item_mod_targets_like_cpp() else {
            return None;
        };
        self.record_represented_all_item_mods_like_cpp(&item_mod_targets, false);
        if !self.set_represented_using_pvp_item_levels_like_cpp(pvp_activity) {
            return None;
        }
        self.record_represented_all_item_mods_like_cpp(&item_mod_targets, true);
        self.restore_represented_health_pct_after_item_mod_scaling_like_cpp(
            health_before,
            max_health_before,
        );
        if publish && !item_mod_targets.is_empty() {
            self.send_represented_item_bonus_player_stat_update_like_cpp();
        }
        Some(true)
    }
    pub(in crate::session) fn represented_avg_total_item_level_like_cpp(&self) -> Option<f32> {
        let (can_dual_wield, can_titan_grip) = self.inventory_equip_capabilities_like_cpp()?;
        let mut best_item_levels =
            vec![(InventoryType::NonEquip, 0u32, ObjectGuid::EMPTY); EQUIPMENT_SLOT_END as usize];
        let mut sum = 0u32;

        let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
        for (&slot, inventory_item) in &self.resolved_inventory_items_like_cpp()? {
            let runtime_item = item_objects.get(&inventory_item.guid);
            self.represented_avg_total_item_level_consume_candidate_like_cpp(
                &mut best_item_levels,
                &mut sum,
                Some(slot),
                inventory_item.entry_id,
                inventory_item.guid,
                runtime_item,
                can_dual_wield,
                can_titan_grip,
            );
        }

        for item in item_objects.values() {
            if item.is_in_trade()
                || item.container_guid().is_empty()
                || !item_objects.contains_key(&item.container_guid())
            {
                continue;
            }

            self.represented_avg_total_item_level_consume_candidate_like_cpp(
                &mut best_item_levels,
                &mut sum,
                None,
                item.object().entry(),
                item.object().guid(),
                Some(item),
                can_dual_wield,
                can_titan_grip,
            );
        }

        if !can_titan_grip
            && best_item_levels[EQUIPMENT_SLOT_MAINHAND as usize].0 == InventoryType::Weapon2Hand
        {
            sum = sum.saturating_add(best_item_levels[EQUIPMENT_SLOT_MAINHAND as usize].1);
        }

        Some(sum as f32 / 16.0)
    }
    /// C++ `Player::GetAverageItemLevel`.
    pub(crate) fn represented_average_item_level_like_cpp(&self) -> Option<f32> {
        let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let mut sum = 0.0f32;
        let mut count = 0u32;

        for slot in 0..EQUIPMENT_SLOT_END {
            if matches!(
                slot,
                EQUIPMENT_SLOT_TABARD
                    | EQUIPMENT_SLOT_RANGED
                    | EQUIPMENT_SLOT_OFFHAND
                    | EQUIPMENT_SLOT_BODY
            ) {
                continue;
            }

            if let Some(inventory_item) = inventory_items.get(&slot) {
                let runtime_item = item_objects.get(&inventory_item.guid);
                if let Some(item_level) =
                    self.represented_item_level_like_cpp(inventory_item.entry_id, runtime_item)
                {
                    sum += item_level as f32;
                }
            }

            count += 1;
        }

        Some(if count == 0 { 0.0 } else { sum / count as f32 })
    }
    fn represented_avg_total_item_level_consume_candidate_like_cpp(
        &self,
        best_item_levels: &mut [(InventoryType, u32, ObjectGuid)],
        sum: &mut u32,
        direct_slot: Option<u8>,
        entry_id: u32,
        item_guid: ObjectGuid,
        runtime_item: Option<&Item>,
        can_dual_wield: bool,
        can_titan_grip: bool,
    ) {
        let Some(storage_template) = self.item_storage_template(entry_id) else {
            return;
        };
        let Some(item_level) = self.represented_item_level_like_cpp(entry_id, runtime_item) else {
            return;
        };
        let inventory_type = storage_template.inventory_type;

        if let Some(slot) = direct_slot.filter(|slot| *slot < EQUIPMENT_SLOT_END) {
            crate::session_rules::represented_avg_total_item_level_maybe_replace_slot_like_cpp(
                best_item_levels,
                sum,
                slot,
                inventory_type,
                item_level,
                item_guid,
                false,
            );
            return;
        }

        if let Some(runtime_item) = runtime_item {
            let represented_item = InventoryItem {
                guid: item_guid,
                entry_id,
                db_guid: item_guid.counter() as u64,
                inventory_type: Some(inventory_type as u8),
            };
            if self.can_use_inventory_item_represented_with_loading_like_cpp(
                &represented_item,
                Some(runtime_item),
                false,
            ) != InventoryResult::Ok
            {
                return;
            }

            if self.represented_can_equip_unique_item_like_cpp(entry_id, runtime_item, NULL_SLOT)
                != InventoryResult::Ok
            {
                return;
            }

            if self.represented_avg_total_item_level_can_equip_item_like_cpp(
                entry_id,
                runtime_item,
                can_dual_wield,
                can_titan_grip,
            ) != InventoryResult::Ok
            {
                return;
            }
        }

        for (candidate_slot, check_duplicate_guid) in
            crate::session_rules::represented_total_avg_equipment_slot_candidates_like_cpp(
                inventory_type,
                can_dual_wield,
                can_titan_grip,
            )
        {
            crate::session_rules::represented_avg_total_item_level_maybe_replace_slot_like_cpp(
                best_item_levels,
                sum,
                candidate_slot,
                inventory_type,
                item_level,
                item_guid,
                check_duplicate_guid,
            );
        }
    }
    pub(in crate::session) fn represented_item_level_like_cpp(
        &self,
        entry_id: u32,
        runtime_item: Option<&Item>,
    ) -> Option<u32> {
        let using_pvp_item_levels = self.resolved_using_pvp_item_levels_like_cpp()?;
        let caps = self
            .player_item_modifier_runtime_snapshot_like_cpp()?
            .item_level_caps;
        let item_stats_store = self.items.stats_store.as_ref()?;
        let random_property_template = item_stats_store.random_property_template(entry_id)?;
        let sparse_template = item_stats_store.sparse_template(entry_id);
        let template_item_level = i64::from(random_property_template.item_level);
        let runtime_item_level = runtime_item
            .map(|item| i64::from(item.data().debug_item_level))
            .filter(|level| *level != 0);
        let item_level = runtime_item_level.unwrap_or_else(|| {
            let mut item_level = sparse_template
                .and_then(|template| {
                    self.represented_player_level_curve_item_level_like_cpp(template, runtime_item)
                })
                .unwrap_or(template_item_level);
            item_level += self.represented_item_level_bonus_like_cpp(runtime_item);
            let item_level_before_upgrades = item_level;
            if using_pvp_item_levels {
                item_level += i64::from(self.represented_pvp_item_level_bonus_like_cpp(entry_id));
            }

            let inventory_type = sparse_template
                .map(|template| template.inventory_type)
                .unwrap_or(random_property_template.inventory_type);
            let is_equipable =
                <InventoryType as num_traits::FromPrimitive>::from_i8(inventory_type)
                    .is_some_and(|inventory_type| inventory_type != InventoryType::NonEquip);
            if !is_equipable {
                return item_level;
            }

            if caps.min_item_level != 0
                && (caps.min_item_level_cutoff == 0
                    || item_level_before_upgrades >= i64::from(caps.min_item_level_cutoff))
                && item_level < i64::from(caps.min_item_level)
            {
                item_level = i64::from(caps.min_item_level);
            }

            let flags3 = sparse_template
                .map(|template| template.flags[2])
                .unwrap_or_default();
            let ignore_max_cap = (flags3 & ItemFlags3::IgnoreItemLevelCapInPvp as u32) != 0;
            if caps.max_item_level != 0
                && !ignore_max_cap
                && item_level > i64::from(caps.max_item_level)
            {
                item_level = i64::from(caps.max_item_level);
            }

            item_level
        });
        Some(
            item_level
                .clamp(
                    i64::from(Self::MIN_ITEM_LEVEL_LIKE_CPP),
                    i64::from(Self::MAX_ITEM_LEVEL_LIKE_CPP),
                )
                .try_into()
                .expect("clamped item level fits u32"),
        )
    }
    fn represented_player_level_curve_item_level_like_cpp(
        &self,
        template: &wow_data::item_stats::ItemSparseTemplateEntry,
        runtime_item: Option<&Item>,
    ) -> Option<i64> {
        let curve_id = template.player_level_to_item_level_curve_id_like_cpp();
        if curve_id == 0 {
            return None;
        }

        let fixed_level = runtime_item
            .map(|item| item.get_modifier(ItemModifier::TimewalkerLevel))
            .unwrap_or(0);
        let mut level = if fixed_level != 0 {
            fixed_level
        } else {
            u32::from(self.player_level_like_cpp())
        };

        if fixed_level == 0
            && let Some(levels) = self.content_tuning_store.as_ref().and_then(|store| {
                store.content_tuning_data_like_cpp(
                    template.scaling_stat_content_tuning_like_cpp(),
                    true,
                )
            })
        {
            let clamped = (level as i32).clamp(levels.min_level, levels.max_level);
            level = u32::try_from(clamped).unwrap_or(level);
        }

        let Some((curve_store, curve_point_store)) = self
            .curve_store
            .as_ref()
            .zip(self.curve_point_store.as_ref())
        else {
            return Some(0);
        };
        let curve_value =
            curve_store.curve_value_at_like_cpp(curve_point_store, curve_id, level as f32);

        Some(curve_value as i64)
    }
    fn represented_item_level_bonus_like_cpp(&self, runtime_item: Option<&Item>) -> i64 {
        let Some(item) = runtime_item else {
            return 0;
        };
        let Some(store) = self.items.bonus_db2_store.as_ref() else {
            return 0;
        };

        item.data()
            .item_bonus_key
            .bonus_list_ids
            .iter()
            .filter_map(|bonus_list_id| u16::try_from(*bonus_list_id).ok())
            .flat_map(|bonus_list_id| store.entries_for_bonus_list_like_cpp(bonus_list_id))
            .filter(|bonus| {
                <ItemBonusType as num_traits::FromPrimitive>::from_u8(bonus.bonus_type)
                    == Some(ItemBonusType::ItemLevel)
            })
            .map(|bonus| i64::from(bonus.value[0]))
            .sum()
    }
    fn represented_pvp_item_level_bonus_like_cpp(&self, entry_id: u32) -> u8 {
        self.pvp_item_store
            .as_ref()
            .map(|store| store.item_level_bonus_like_cpp(entry_id))
            .unwrap_or(0)
    }
}
