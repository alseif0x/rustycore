//! Represented item durability and repair cost.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the C++ `sDurabilityCostsStore` equivalent for this session.
    pub fn set_durability_costs_store(&mut self, store: Arc<DurabilityCostsStore>) {
        self.durability_costs_store = Some(store);
    }
    /// Set the C++ `sDurabilityQualityStore` equivalent for this session.
    pub fn set_durability_quality_store(&mut self, store: Arc<DurabilityQualityStore>) {
        self.durability_quality_store = Some(store);
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
    /// C++ `Player::DurabilityLossAll` (`Player.cpp:4522-4544`).
    ///
    /// Equipment is always processed; `inventory` additionally walks the
    /// backpack slots and every bag's contents, matching C++.
    pub(in crate::session) fn apply_represented_durability_loss_all_like_cpp(
        &mut self,
        percent: f64,
        inventory: bool,
    ) -> usize {
        let targets = self.represented_durability_targets_like_cpp(inventory);
        let mut affected = 0;
        for (guid, slot, equipped) in targets {
            if self.apply_represented_durability_loss_item_like_cpp(guid, slot, equipped, percent) {
                affected += 1;
            }
        }
        affected
    }
    /// C++ `Player::DurabilityPointsLossAll` (`Player.cpp:4566-4588`).
    pub(in crate::session) fn apply_represented_durability_points_loss_all_like_cpp(
        &mut self,
        points: i32,
        inventory: bool,
    ) -> usize {
        let targets = self.represented_durability_targets_like_cpp(inventory);
        let mut affected = 0;
        for (guid, slot, equipped) in targets {
            if self.apply_represented_durability_points_loss_like_cpp(
                guid,
                slot,
                equipped,
                i64::from(points),
            ) {
                affected += 1;
            }
        }
        affected
    }
    /// C++ `Spell::EffectDurabilityDamage` slot branch: one top-level
    /// `INVENTORY_SLOT_BAG_0` slot, resolved through `GetItemByPos`.
    pub(in crate::session) fn apply_represented_durability_points_loss_at_slot_like_cpp(
        &mut self,
        slot: u8,
        points: i32,
    ) -> bool {
        let Some(item) = self.resolved_inventory_item_like_cpp(slot) else {
            return false;
        };
        let equipped = is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot));
        self.apply_represented_durability_points_loss_like_cpp(
            item.guid,
            slot,
            equipped,
            i64::from(points),
        )
    }
    /// C++ `Spell::EffectDurabilityDamagePCT` slot branch.
    pub(in crate::session) fn apply_represented_durability_loss_at_slot_like_cpp(
        &mut self,
        slot: u8,
        percent: f64,
    ) -> bool {
        let Some(item) = self.resolved_inventory_item_like_cpp(slot) else {
            return false;
        };
        let equipped = is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot));
        self.apply_represented_durability_loss_item_like_cpp(item.guid, slot, equipped, percent)
    }
    /// The `DurabilityLossAll`/`DurabilityPointsLossAll` target set.
    fn represented_durability_targets_like_cpp(
        &self,
        inventory: bool,
    ) -> Vec<(ObjectGuid, u8, bool)> {
        let Some(items) = self.resolved_inventory_item_objects_like_cpp() else {
            return Vec::new();
        };
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(
                self.resolved_player_inventory_slot_count_like_cpp()
                    .unwrap_or(0),
            )
            .min(INVENTORY_SLOT_ITEM_END);
        items
            .iter()
            .filter_map(|(guid, item)| {
                let slot = item.slot();
                if item.container_guid().is_empty() {
                    if is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)) {
                        return Some((*guid, slot, true));
                    }
                    if inventory && (INVENTORY_SLOT_ITEM_START..inventory_end).contains(&slot) {
                        return Some((*guid, slot, false));
                    }
                    return None;
                }
                inventory.then_some((*guid, slot, false))
            })
            .collect()
    }
    /// C++ `Player::DurabilityLoss` (`Player.cpp:4546-4562`).
    fn apply_represented_durability_loss_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: u8,
        equipped: bool,
        percent: f64,
    ) -> bool {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return false;
        };
        let max_durability = item.data().max_durability;
        if max_durability == 0 {
            return false;
        }
        let percent =
            percent / f64::from(self.represented_durability_loss_aura_multiplier_like_cpp());
        let points = ((f64::from(max_durability) * percent) as u32).max(1);
        self.apply_represented_durability_points_loss_like_cpp(
            item_guid,
            slot,
            equipped,
            i64::from(points),
        )
    }
    /// C++ `Player::DurabilityPointsLoss` (`Player.cpp:4590-4620`).
    ///
    /// Mods are removed *before* the durability write so the represented
    /// `_ApplyItemMods` gate still sees the item as unbroken, then restored
    /// after, exactly like C++.
    fn apply_represented_durability_points_loss_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: u8,
        equipped: bool,
        points: i64,
    ) -> bool {
        if self.represented_prevent_durability_loss_like_cpp() {
            return false;
        }
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return false;
        };
        let max_durability = i64::from(item.data().max_durability);
        let old_durability = i64::from(item.data().durability);
        let new_durability = (old_durability - points).clamp(0, max_durability);
        if old_durability == new_durability {
            return false;
        }
        if new_durability == 0 && old_durability > 0 && equipped {
            self.record_represented_item_mods_like_cpp(item_guid, slot, false);
        }
        let updated = self.apply_inventory_item_object_updates_like_cpp(
            item_guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::SetDurability(
                u32::try_from(new_durability).unwrap_or(0),
            )],
        );
        if new_durability > 0 && old_durability == 0 && equipped {
            self.record_represented_item_mods_like_cpp(item_guid, slot, true);
        }
        updated
    }
    /// C++ `Spell::EffectDurabilityDamage` (`SpellEffects.cpp:4316-4352`).
    pub(in crate::session) fn apply_durability_damage_effect_like_cpp(
        &mut self,
        damage: i32,
        slot: i32,
        target_guid: ObjectGuid,
    ) {
        if self.player_guid() != Some(target_guid) {
            return;
        }
        if slot < 0 {
            self.apply_represented_durability_points_loss_all_like_cpp(damage, slot < -1);
            return;
        }
        let Ok(slot) = u8::try_from(slot) else {
            return;
        };
        if slot >= INVENTORY_SLOT_BAG_END {
            return;
        }
        self.apply_represented_durability_points_loss_at_slot_like_cpp(slot, damage);
    }
    /// C++ `Spell::EffectDurabilityDamagePCT` (`SpellEffects.cpp:4354-4373`).
    pub(in crate::session) fn apply_durability_damage_pct_effect_like_cpp(
        &mut self,
        damage: i32,
        slot: i32,
        target_guid: ObjectGuid,
    ) {
        if self.player_guid() != Some(target_guid) {
            return;
        }
        if slot < 0 {
            self.apply_represented_durability_loss_all_like_cpp(
                f64::from(damage) / 100.0,
                slot < -1,
            );
            return;
        }
        let Ok(slot) = u8::try_from(slot) else {
            return;
        };
        if slot >= INVENTORY_SLOT_BAG_END || damage <= 0 {
            return;
        }
        self.apply_represented_durability_loss_at_slot_like_cpp(slot, f64::from(damage) / 100.0);
    }
    /// C++ `GetTotalAuraMultiplier(SPELL_AURA_MOD_DURABILITY_LOSS)`.
    fn represented_durability_loss_aura_multiplier_like_cpp(&self) -> f32 {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_DURABILITY_LOSS,
        )
        .unwrap_or_default()
        .into_iter()
        .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
    }
    /// C++ `HasAuraType(SPELL_AURA_PREVENT_DURABILITY_LOSS)`.
    fn represented_prevent_durability_loss_like_cpp(&self) -> bool {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_PREVENT_DURABILITY_LOSS,
        )
        .is_some_and(|effects| !effects.is_empty())
    }
    /// Get the durability cost store reference.
    pub fn durability_costs_store(&self) -> Option<&Arc<DurabilityCostsStore>> {
        self.durability_costs_store.as_ref()
    }
    /// Get the durability quality store reference.
    pub fn durability_quality_store(&self) -> Option<&Arc<DurabilityQualityStore>> {
        self.durability_quality_store.as_ref()
    }
    /// C++ `Item::CalculateDurabilityRepairCost`.
    pub(crate) fn item_durability_repair_cost_like_cpp(
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
    /// C++ `Player::DurabilityRepair(pos, takeCost, discountMod)` for one represented item.
    pub(crate) async fn repair_inventory_item_durability_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_guid: ObjectGuid,
        take_cost: bool,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return false;
        };
        let top_level_inventory_item = inventory_items
            .iter()
            .find(|(_, item)| item.guid == item_guid)
            .map(|(slot, item)| (*slot, item.clone()));
        let Some(item_object) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return false;
        };

        let top_level_inventory_item = top_level_inventory_item.map(|(_, item)| item);
        let item_entry_id = top_level_inventory_item
            .as_ref()
            .map(|item| item.entry_id)
            .unwrap_or_else(|| item_object.object().entry());
        let item_db_guid = top_level_inventory_item
            .as_ref()
            .map(|item| item.db_guid)
            .unwrap_or_else(|| item_guid.counter() as u64);
        let current_durability = item_object.data().durability;
        let max_durability = item_object.data().max_durability;
        let cost = self.item_durability_repair_cost_like_cpp(
            item_entry_id,
            current_durability,
            max_durability,
            discount,
            repair_cost_rate,
        );

        if take_cost && cost != 0 {
            let Some(money_persistence) = self
                .begin_exclusive_player_money_persistence_like_cpp()
                .await
            else {
                return false;
            };
            let Some(old_money) = self.resolved_player_money_like_cpp() else {
                self.kick("canonical Player money owner is unavailable before durability repair");
                return false;
            };
            if old_money < cost {
                return false;
            }
            let new_money = old_money - cost;

            let money_persistence = if let Some(port) =
                self.player_lifecycle_port_like_cpp().map(Arc::clone)
            {
                let Some(player_guid) = self.player_guid() else {
                    return false;
                };
                let request = wow_persistence::PlayerMoneyTransactionRequestLikeCpp {
                    player_guid: player_guid.counter() as u64,
                    money_after: new_money,
                    durability_repairs: vec![wow_persistence::PlayerDurabilityRepairSaveLikeCpp {
                        item_db_guid,
                        durability: max_durability,
                    }],
                };
                let Some(money_persistence) = self
                    .await_exclusive_player_money_transaction_outcome_like_cpp(
                        money_persistence,
                        port.persist_money_transaction_like_cpp(request),
                        old_money,
                        new_money,
                        "single item durability repair",
                    )
                    .await
                else {
                    return false;
                };
                money_persistence
            } else {
                // Unit fixtures do not install a CharacterDatabase. A live
                // session always has one; without it no detached SQL payout
                // can originate from this session either.
                money_persistence
            };

            // The committed transaction repaired the item and charged money.
            // Publish both runtime fields before the guard opens; otherwise a
            // cancelled criteria drain can leave durable durability repaired
            // while the live item remains broken.
            if !self.stage_player_money_change_like_cpp(old_money, new_money) {
                self.kick("canonical Player money owner became unavailable after durability-repair COMMIT");
                return false;
            }
            let repaired = self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid);
            drop(money_persistence);
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
            return repaired;
        } else if let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) {
            match port
                .persist_durability_repair_like_cpp(
                    wow_persistence::PlayerDurabilityRepairSaveLikeCpp {
                        item_db_guid,
                        durability: max_durability,
                    },
                )
                .await
            {
                wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
                wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                    warn!(
                        item_guid = item_guid.counter(),
                        error = %reason,
                        "failed to persist represented item durability repair"
                    );
                    return false;
                }
            }
        }

        self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid)
    }
    #[cfg(test)]
    pub(crate) async fn repair_inventory_item_durability_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        take_cost: bool,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        self.repair_inventory_item_durability_with_generator_like_cpp(
            generators.item.as_ref(),
            item_guid,
            take_cost,
            discount,
            repair_cost_rate,
        )
        .await
    }
    fn apply_inventory_item_durability_repair_runtime_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return false;
        };
        let top_level_slot = inventory_items
            .iter()
            .find_map(|(&slot, item)| (item.guid == item_guid).then_some(slot));
        let Some(item_object) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return false;
        };
        let max_durability = item_object.data().max_durability;
        let was_broken = item_object.is_broken();
        let equipped_slot = top_level_slot
            .filter(|slot| is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, *slot)));
        let updated = self.apply_inventory_item_object_updates_like_cpp(
            item_guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::SetDurability(
                max_durability,
            )],
        );
        if updated {
            if let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) {
                let mut item_data_mask = UpdateMask::new(ITEM_DATA_BITS);
                item_data_mask.set(ITEM_DATA_DURABILITY_BIT);
                let durability_update = ItemValuesUpdate {
                    changed_object_type_mask: 1 << TYPEID_ITEM,
                    object_data: None,
                    item_data: Some(ItemDataUpdate {
                        mask: item_data_mask,
                        values: item.data().clone(),
                    }),
                };
                if let Some(update) = item_values_update_to_update_object(
                    item_guid,
                    self.player_map_id_like_cpp(),
                    &durability_update,
                ) {
                    self.send_packet(&update);
                }
            }
            if let Some(slot) = equipped_slot
                && was_broken
            {
                self.record_represented_item_mods_like_cpp(item_guid, slot, true);
                self.send_represented_item_bonus_player_stat_update_like_cpp();
            }
        }
        updated
    }
    /// C++ `Player::DurabilityRepairAll(takeCost=true, guildBank=false)` for represented items.
    pub(crate) async fn repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let Some(repair_items) =
            self.repairable_inventory_item_costs_like_cpp(discount, repair_cost_rate)
        else {
            return false;
        };
        if repair_items.is_empty() {
            return true;
        }
        let total_cost = repair_items
            .iter()
            .fold(0u64, |total, (_, cost)| total.saturating_add(*cost));
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return false;
        };
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return false;
        };
        let planned_repairs = repair_items
            .iter()
            .filter_map(|(item_guid, _)| {
                let item = item_objects.get(item_guid)?;
                let db_guid = inventory_items
                    .values()
                    .find(|inventory_item| inventory_item.guid == *item_guid)
                    .map(|inventory_item| inventory_item.db_guid)
                    .unwrap_or_else(|| item_guid.counter() as u64);
                Some((*item_guid, db_guid, item.data().max_durability))
            })
            .collect::<Vec<_>>();
        if planned_repairs.len() != repair_items.len() {
            return false;
        }

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return false;
        };
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return false;
        };
        if old_money < total_cost {
            return false;
        }
        let new_money = old_money - total_cost;

        let money_persistence =
            if let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) {
                let Some(player_guid) = self.player_guid() else {
                    return false;
                };
                let request = wow_persistence::PlayerMoneyTransactionRequestLikeCpp {
                    player_guid: player_guid.counter() as u64,
                    money_after: new_money,
                    durability_repairs: planned_repairs
                        .iter()
                        .map(|&(_, item_db_guid, durability)| {
                            wow_persistence::PlayerDurabilityRepairSaveLikeCpp {
                                item_db_guid,
                                durability,
                            }
                        })
                        .collect(),
                };
                let Some(money_persistence) = self
                    .await_exclusive_player_money_transaction_outcome_like_cpp(
                        money_persistence,
                        port.persist_money_transaction_like_cpp(request),
                        old_money,
                        new_money,
                        "all-items durability repair",
                    )
                    .await
                else {
                    return false;
                };
                money_persistence
            } else {
                money_persistence
            };

        // Money and every durability row share one COMMIT. Mirror all of those
        // rows in runtime while admission is still fenced and before the first
        // cancellation point.
        if !self.stage_player_money_change_like_cpp(old_money, new_money) {
            self.kick(
                "canonical Player money owner became unavailable after durability-repair COMMIT",
            );
            return false;
        }
        let mut repaired_any = false;
        for (item_guid, _, _) in planned_repairs {
            repaired_any |= self.apply_inventory_item_durability_repair_runtime_like_cpp(item_guid);
        }
        drop(money_persistence);
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
        repaired_any
    }
    #[cfg(test)]
    pub(crate) async fn repair_all_inventory_item_durability_with_player_money_like_cpp(
        &mut self,
        discount: f32,
        repair_cost_rate: f32,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        self.repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp(
            generators.item.as_ref(),
            discount,
            repair_cost_rate,
        )
        .await
    }
    pub(in crate::session) fn repairable_inventory_item_costs_like_cpp(
        &self,
        discount: f32,
        repair_cost_rate: f32,
    ) -> Option<Vec<(ObjectGuid, u64)>> {
        let mut repair_items = Vec::new();
        let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(INVENTORY_DEFAULT_SIZE)
            .min(PLAYER_SLOT_END as u8);

        for (&slot, inventory_item) in &inventory_items {
            if !((slot < EQUIPMENT_SLOT_END)
                || (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
                || (INVENTORY_SLOT_ITEM_START..inventory_end).contains(&slot))
            {
                continue;
            }
            let Some(item_object) = item_objects.get(&inventory_item.guid) else {
                continue;
            };
            let cost = self.item_durability_repair_cost_like_cpp(
                inventory_item.entry_id,
                item_object.data().durability,
                item_object.data().max_durability,
                discount,
                repair_cost_rate,
            );
            if cost != 0 {
                repair_items.push((inventory_item.guid, cost));
            }
        }

        let represented_bag_guids: HashSet<_> = inventory_items
            .iter()
            .filter_map(|(&slot, item)| {
                ((INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot))
                    .then_some(item.guid)
            })
            .collect();
        for item_object in item_objects.values() {
            if !represented_bag_guids.contains(&item_object.container_guid())
                || item_object.slot() as usize >= MAX_BAG_SIZE
            {
                continue;
            }
            let cost = self.item_durability_repair_cost_like_cpp(
                item_object.object().entry(),
                item_object.data().durability,
                item_object.data().max_durability,
                discount,
                repair_cost_rate,
            );
            if cost != 0 {
                repair_items.push((item_object.object().guid(), cost));
            }
        }

        Some(repair_items)
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
