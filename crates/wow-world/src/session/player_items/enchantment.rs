//! Represented item enchantment state and its loaded effects.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// C++ `Item::GetDisenchantLoot`.
    ///
    /// `can_disenchant_bonus` represents `BonusData::CanDisenchant`, which is
    /// not yet a canonical Rust item-bonus subsystem.
    pub(crate) fn item_disenchant_loot_with_catalogs_like_cpp(
        &self,
        catalogs: &ItemValuationCatalogsLikeCpp,
        item_id: u32,
        quality: u32,
        item_level: u32,
        can_disenchant_bonus: bool,
    ) -> Option<(u32, u16)> {
        if !can_disenchant_bonus {
            return None;
        }

        let basic = self.items.store.as_ref()?.get(item_id)?;
        let sparse = self.items.stats_store.as_ref()?.sparse_template(item_id)?;
        let item_flags = sparse.item_flags();

        if item_flags.contains(ItemFlags::CONJURED)
            || item_flags.contains(ItemFlags::NO_DISENCHANT)
            || sparse.bonding == ItemBondingType::Quest as u8
        {
            return None;
        }

        if sparse.zone_bound[0] != 0
            || sparse.zone_bound[1] != 0
            || sparse.instance_bound != 0
            || sparse.max_stack_size() > 1
        {
            return None;
        }

        if self.item_sell_price_with_catalogs_like_cpp(catalogs, item_id, quality, item_level)
            == Some(0)
            && !catalogs.currency_costs.has_item_currency_cost(item_id)
        {
            return None;
        }

        catalogs
            .disenchant_loot
            .find_for_item_like_cpp(
                u32::from(basic.class_id),
                basic.subclass_id as i8,
                quality as u8,
                item_level,
                sparse.required_expansion,
            )
            .map(|entry| (entry.id, entry.skill_required))
    }
    #[cfg(test)]
    pub fn item_disenchant_loot_like_cpp(
        &self,
        item_id: u32,
        quality: u32,
        item_level: u32,
        can_disenchant_bonus: bool,
    ) -> Option<(u32, u16)> {
        self.item_disenchant_loot_with_catalogs_like_cpp(
            &self.item_valuation_catalogs_for_test_like_cpp(),
            item_id,
            quality,
            item_level,
            can_disenchant_bonus,
        )
    }
    /// Set the item random enchantment template store for this session.
    pub fn set_item_random_enchantment_template_store(
        &mut self,
        store: Arc<ItemRandomEnchantmentTemplateStore>,
    ) {
        self.items.random_enchantment_template_store = Some(store);
    }
    /// Get the item random enchantment template store reference.
    pub fn item_random_enchantment_template_store(
        &self,
    ) -> Option<&Arc<ItemRandomEnchantmentTemplateStore>> {
        self.items.random_enchantment_template_store.as_ref()
    }
    /// Set the item disenchant loot store for this session.
    #[cfg(test)]
    pub fn set_item_disenchant_loot_store(&mut self, store: Arc<ItemDisenchantLootStore>) {
        self.item_disenchant_loot_store = Some(store);
    }
    /// Get the item disenchant loot store reference.
    #[cfg(test)]
    pub fn item_disenchant_loot_store(&self) -> Option<&Arc<ItemDisenchantLootStore>> {
        self.item_disenchant_loot_store.as_ref()
    }
    /// Resolve C++ `sItemRandomSuffixStore.LookupEntry(abs(RandomPropertiesID))`.
    pub fn apply_enchantment_random_suffix_ref(
        &self,
        random_properties_id: i32,
    ) -> Option<ApplyEnchantmentRandomSuffixRef> {
        let id = random_properties_id.unsigned_abs();
        if id == 0 {
            return None;
        }

        self.items
            .random_suffix_store
            .as_ref()
            .and_then(|store| store.get(id))
            .map(|entry| {
                ApplyEnchantmentRandomSuffixRef::new(
                    entry.id,
                    entry.enchantments,
                    entry.allocation_pct,
                )
            })
    }
    /// Set the spell item enchantment store for this session.
    pub fn set_spell_item_enchantment_store(&mut self, store: Arc<SpellItemEnchantmentStore>) {
        self.spell_catalogs.spell_item_enchantment_store = Some(store);
    }
    pub fn set_spell_item_enchantment_condition_store(
        &mut self,
        store: Arc<SpellItemEnchantmentConditionStore>,
    ) {
        self.spell_catalogs.spell_item_enchantment_condition_store = Some(store);
    }
    /// Get the spell item enchantment store reference.
    pub fn spell_item_enchantment_store(&self) -> Option<&Arc<SpellItemEnchantmentStore>> {
        self.spell_catalogs.spell_item_enchantment_store.as_ref()
    }
    /// C++ `Player::EnchantmentFitsRequirements` for the currently equipped gems.
    fn enchantment_fits_requirements_like_cpp(
        &self,
        enchantment_condition: u32,
        except_slot: Option<u8>,
    ) -> bool {
        if enchantment_condition == 0 {
            return true;
        }
        let Some(condition) = self
            .spell_catalogs
            .spell_item_enchantment_condition_store
            .as_ref()
            .and_then(|store| store.get(enchantment_condition))
        else {
            return true;
        };

        let mut gem_counts = [0u8; 4];
        for slot in 0..EQUIPMENT_SLOT_END {
            if except_slot == Some(slot) {
                continue;
            }
            let Some(inventory_item) = self.resolved_inventory_item_like_cpp(slot) else {
                continue;
            };
            let Some(item) = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
            else {
                continue;
            };
            if item.is_broken() {
                continue;
            }
            for gem in &item.data().gems {
                let Ok(gem_item_id) = u32::try_from(gem.item_id) else {
                    continue;
                };
                let Some(gem_properties_id) = self
                    .items
                    .stats_store
                    .as_ref()
                    .and_then(|store| store.gem_properties(gem_item_id))
                    .map(u32::from)
                else {
                    continue;
                };
                let Some(gem_type) = self
                    .gem_properties_store
                    .as_ref()
                    .and_then(|store| store.get(gem_properties_id))
                    .map(|properties| properties.gem_type)
                else {
                    continue;
                };
                for (color, count) in gem_counts.iter_mut().enumerate() {
                    if gem_type & (1 << color) != 0 {
                        *count = count.saturating_add(1);
                    }
                }
            }
        }

        let mut activate = true;
        for index in 0..5 {
            let left_type = condition.lt_operand_type[index];
            if left_type == 0 {
                continue;
            }
            let Some(&left_count) = gem_counts.get(usize::from(left_type - 1)) else {
                return false;
            };
            let right_type = condition.rt_operand_type[index];
            let right_count = if right_type == 0 {
                condition.rt_operand[index]
            } else {
                let Some(&count) = gem_counts.get(usize::from(right_type - 1)) else {
                    return false;
                };
                count
            };
            activate &= match condition.operator[index] {
                2 => left_count < right_count,
                3 => left_count > right_count,
                5 => left_count >= right_count,
                _ => true,
            };
        }
        activate
    }
    #[cfg(test)]
    pub fn set_spell_enchant_proc_store(&mut self, store: Arc<SpellEnchantProcStoreLikeCpp>) {
        self.spell_catalogs.spell_enchant_proc_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn spell_enchant_proc_event_like_cpp(
        &self,
        enchantment_id: u32,
    ) -> Option<&SpellEnchantProcEntryLikeCpp> {
        self.spell_catalogs
            .spell_enchant_proc_store
            .as_ref()
            .and_then(|store| store.get_spell_enchant_proc_event_like_cpp(enchantment_id))
    }
    /// C++ `SpellMgr::IsArenaAllowedEnchancment`.
    pub fn is_arena_allowed_enchantment(&self, enchantment_id: u32) -> bool {
        self.spell_catalogs
            .spell_item_enchantment_store
            .as_ref()
            .is_some_and(|store| store.is_arena_allowed_enchantment(enchantment_id))
    }
    /// Build the entity-level `ApplyEnchantment` template from `SpellItemEnchantment.db2`.
    pub fn apply_enchantment_template_ref(
        &self,
        enchantment_id: i32,
        required_skill_value: u16,
        condition_fits: bool,
    ) -> Option<ApplyEnchantmentTemplateRef> {
        let id = u32::try_from(enchantment_id).ok()?;
        self.spell_catalogs
            .spell_item_enchantment_store
            .as_ref()
            .and_then(|store| store.get(id))
            .map(|entry| {
                let mut template = ApplyEnchantmentTemplateRef::new(enchantment_id);
                template.condition_id = u32::from(entry.condition_id);
                template.condition_fits = condition_fits;
                template.min_level = entry.min_level;
                template.required_skill_id = u32::from(entry.required_skill_id);
                template.required_skill_rank = entry.required_skill_rank;
                template.required_skill_value = required_skill_value;
                template
            })
    }
    /// Build the C++ three `SpellItemEnchantmentEntry` effect refs.
    pub fn apply_enchantment_effect_refs(
        &self,
        enchantment_id: u32,
    ) -> Option<[ApplyEnchantmentEffectRef; 3]> {
        self.spell_catalogs
            .spell_item_enchantment_store
            .as_ref()
            .and_then(|store| store.get(enchantment_id))
            .map(|entry| {
                std::array::from_fn(|index| {
                    let amount = entry.effect_points_min[index] as u32;
                    let arg = entry.effect_arg[index];
                    match <ItemEnchantmentType as num_traits::FromPrimitive>::from_u8(
                        entry.effect[index],
                    ) {
                        Some(effect_type) => {
                            ApplyEnchantmentEffectRef::known(effect_type, amount, arg)
                        }
                        None => ApplyEnchantmentEffectRef::unknown(
                            u32::from(entry.effect[index]),
                            amount,
                            arg,
                        ),
                    }
                })
            })
    }
    fn current_item_enchantment_socket_context_like_cpp(
        &self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
    ) -> Option<ApplyEnchantmentSocketContext> {
        let socket_index = match slot {
            EnchantmentSlot::EnhancementSocket => 0,
            EnchantmentSlot::EnhancementSocket2 => 1,
            EnchantmentSlot::EnhancementSocket3 => 2,
            _ => return None,
        };
        let item = self.resolved_inventory_item_object_like_cpp(item_guid)?;
        let socket_color = self
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.socket_template(item.object().entry()))
            .map(|template| u32::from(template.socket_types[socket_index]))
            .unwrap_or(0);
        let gem_requirement = item
            .data()
            .gems
            .get(socket_index)
            .and_then(|gem| u32::try_from(gem.item_id).ok())
            .and_then(|gem_item_id| {
                self.items
                    .stats_store
                    .as_ref()?
                    .socket_template(gem_item_id)
            })
            .and_then(|gem_template| {
                Some(ApplyEnchantmentGemRequirementRef::new(
                    u32::from(gem_template.required_skill_id),
                    gem_template.required_skill_rank,
                    self.resolved_player_skill_value_like_cpp(gem_template.required_skill_id)?,
                ))
            });

        if socket_color != 0 {
            return Some(ApplyEnchantmentSocketContext::colored(
                socket_color,
                gem_requirement,
            ));
        }

        let prismatic_enchantment_id =
            item.data().enchantments[EnchantmentSlot::EnhancementSocketPrismatic as usize].id;
        let prismatic_enchantment = self
            .apply_enchantment_template_ref(prismatic_enchantment_id, 0, true)
            .and_then(|mut template| {
                if let Ok(skill_id) = u16::try_from(template.required_skill_id) {
                    template.required_skill_value =
                        self.resolved_player_skill_value_like_cpp(skill_id)?;
                }
                Some(template)
            });
        Some(ApplyEnchantmentSocketContext::prismatic(
            prismatic_enchantment,
            gem_requirement,
        ))
    }
    /// C++ `Player::ApplyEnchantment(item, slot, apply, ...)` bridge for a
    /// represented inventory item owned by the current player.
    ///
    /// The item runtime lives in the session inventory while the player state
    /// lives in the canonical map. This temporarily moves the item out, runs the
    /// entity-level plan against the canonical player when available, then puts
    /// the item back without clearing or setting the enchantment field itself.
    pub(crate) fn apply_current_player_item_enchantment_plan_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        mut args: ApplyEnchantmentArgs,
    ) -> Option<ApplyEnchantmentPlan> {
        let enchantment_id = self
            .resolved_inventory_item_object_like_cpp(item_guid)?
            .data()
            .enchantments[slot as usize]
            .id;
        let condition_fits = u32::try_from(enchantment_id)
            .ok()
            .and_then(|id| {
                self.spell_catalogs
                    .spell_item_enchantment_store
                    .as_ref()?
                    .get(id)
            })
            .is_none_or(|entry| {
                self.enchantment_fits_requirements_like_cpp(u32::from(entry.condition_id), None)
            });
        if args.socket_context.is_none() {
            args.socket_context =
                self.current_item_enchantment_socket_context_like_cpp(item_guid, slot);
        }
        let mut item = self.remove_inventory_item_object(item_guid)?;
        let mut template = self.apply_enchantment_template_ref(enchantment_id, 0, condition_fits);
        if let Some(template) = &mut template {
            if let Ok(skill_id) = u16::try_from(template.required_skill_id) {
                template.required_skill_value =
                    self.resolved_player_skill_value_like_cpp(skill_id)?;
            }
        }

        let plan = self.mutate_canonical_player_like_cpp(|player| {
            player.apply_enchantment_plan(Some(&mut item), slot, template, args)
        });
        self.insert_inventory_item_object(item);
        plan
    }
    pub(in crate::session) fn apply_loaded_enchantment_spell_action_like_cpp(
        &mut self,
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        match action {
            ApplyEnchantmentEffectAction::CastEquipSpell {
                spell_id,
                item_guid,
            } => {
                let Ok(spell_id) = i32::try_from(spell_id) else {
                    return false;
                };
                let Some(spell_info) = self
                    .spell_store()
                    .and_then(|store| store.get(spell_id))
                    .cloned()
                else {
                    return false;
                };
                let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
                if effect_mask == 0 {
                    return false;
                }
                self.apply_aura_with_effect_mask_without_update_like_cpp(
                    spell_id,
                    item_guid,
                    0,
                    AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                    effect_mask,
                )
                .is_ok()
            }
            ApplyEnchantmentEffectAction::RemoveEquipSpellAura {
                spell_id,
                item_guid,
            } => {
                let Ok(spell_id) = i32::try_from(spell_id) else {
                    return false;
                };
                let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
                    return false;
                };
                let slots = visible_auras
                    .values()
                    .filter_map(|aura| {
                        (aura.spell_id == spell_id && aura.caster_guid == item_guid)
                            .then_some(aura.slot)
                    })
                    .collect::<Vec<_>>();
                let removed = !slots.is_empty();
                for slot in slots {
                    let _ = self.remove_aura(slot);
                }
                removed
            }
            _ => false,
        }
    }
    pub fn send_item_enchant_time_update_plan(
        &self,
        owner_guid: ObjectGuid,
        update: &PlayerEnchantTimeUpdate,
    ) {
        self.send_packet(&ItemEnchantTimeUpdate {
            owner_guid,
            item_guid: update.item_guid,
            duration_left: update.duration_secs,
            slot: update.slot as u32,
        });
    }
    pub fn send_item_enchant_time_update_plans(
        &self,
        owner_guid: ObjectGuid,
        updates: &[PlayerEnchantTimeUpdate],
    ) {
        for update in updates {
            self.send_item_enchant_time_update_plan(owner_guid, update);
        }
    }
    /// C++ `_StoreItem` merge branch calls `AddEnchantmentDurations(pItem2)`
    /// without calling `AddItemDurations` for the existing destination stack.
    pub(crate) fn refresh_inventory_item_enchantment_duration_refs_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) {
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        let Some((owner_guid, enchantment_updates)) =
            self.mutate_canonical_player_like_cpp(|player| {
                (player.guid(), player.add_enchantment_durations(&mut item))
            })
        else {
            return;
        };

        self.insert_inventory_item_object(item);
        self.send_item_enchant_time_update_plans(owner_guid, &enchantment_updates);
    }
    /// C++ `Player::RemoveItem` clears main-hand-only enchantments when the
    /// main-hand item leaves that slot. Return both the post-remove DB value
    /// and the runtime slots to clear, without mutating live state before the
    /// caller's transaction commits.
    pub(crate) fn inventory_remove_enchantment_persistence_like_cpp(
        &self,
        item_guid: ObjectGuid,
        clear_mainhand_only: bool,
    ) -> Option<(String, Vec<EnchantmentSlot>)> {
        let item = self.resolved_inventory_item_object_like_cpp(item_guid)?;
        let current_durations = self
            .canonical_player_snapshot_like_cpp(|player| {
                player
                    .enchant_durations()
                    .iter()
                    .filter(|duration| duration.item_guid == item_guid)
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut cleared = Vec::new();
        let mut persisted = String::new();

        for (index, enchantment) in item.data().enchantments.iter().enumerate() {
            let Some(slot) = <EnchantmentSlot as num_traits::FromPrimitive>::from_usize(index)
            else {
                continue;
            };
            let enchantment_entry = u32::try_from(enchantment.id)
                .ok()
                .and_then(|id| {
                    self.spell_catalogs
                        .spell_item_enchantment_store
                        .as_ref()?
                        .get(id)
                })
                .copied();
            let clear_mainhand = clear_mainhand_only
                && enchantment_entry.is_some_and(|entry| {
                    entry
                        .flags
                        .contains(SpellItemEnchantmentFlags::MAINHAND_ONLY)
                });
            if clear_mainhand {
                cleared.push(slot);
            }
            if clear_mainhand
                || enchantment_entry.is_none_or(|entry| {
                    entry
                        .flags
                        .contains(SpellItemEnchantmentFlags::DO_NOT_SAVE_TO_DB)
                })
            {
                persisted.push_str("0 0 0 ");
            } else {
                let duration = current_durations
                    .iter()
                    .find(|duration| duration.slot == slot)
                    .map_or(enchantment.duration, |duration| duration.left_duration_ms);
                persisted.push_str(&format!(
                    "{} {} {} ",
                    enchantment.id, duration, enchantment.charges
                ));
            }
        }

        Some((persisted, cleared))
    }
    pub(crate) fn resolved_enchanting_skill_like_cpp(&self) -> Option<u16> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.enchanting_skill_value_like_cpp(SKILL_ENCHANTING_LIKE_CPP)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_enchanting_skill);
        }
        canonical
    }
    /// C++ `Player::_LoadInventory` finishes by `_ApplyAllItemMods`, which in
    /// turn calls `ApplyEnchantment(m_items[i], true)` for equipped items.
    pub(crate) fn apply_loaded_equipped_item_enchantments_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
        };
        // `_ApplyAllItemMods` visits the broad top-level range, but C++
        // `ApplyEnchantment` immediately returns unless `Item::IsEquipped`.
        if !item.is_equipped() {
            return LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
        }
        if !self.initial_loaded_item_mods_can_apply_like_cpp(item_guid) {
            return LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
        }
        let slots = item
            .data()
            .enchantments
            .iter()
            .enumerate()
            .filter_map(|(slot_index, enchantment)| {
                if enchantment.id == 0 {
                    return None;
                }
                <EnchantmentSlot as num_traits::FromPrimitive>::from_usize(slot_index)
            })
            .collect::<Vec<_>>();

        let mut outcome = LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
        for slot in slots {
            if let Some(plan) = self.apply_current_player_item_enchantment_plan_like_cpp(
                item_guid,
                slot,
                ApplyEnchantmentArgs::apply(),
            ) {
                if let ApplyEnchantmentResult::Applied {
                    enchantment_id,
                    apply,
                    effects_allowed,
                    update_permanent_visible_item,
                    duration_action,
                    ..
                } = plan.result
                {
                    if let Some(ApplyEnchantmentDurationAction::Added(duration_update)) =
                        duration_action
                    {
                        outcome.duration_updates.push(duration_update);
                    }
                    if effects_allowed {
                        let (changed_stats, mut effect_actions, mut unrepresented_effect_actions) =
                            self.apply_loaded_equipped_item_enchantment_effects_like_cpp(
                                item_guid,
                                slot,
                                enchantment_id,
                                apply,
                            );
                        outcome.send_stat_update |= changed_stats;
                        outcome.effect_actions.append(&mut effect_actions);
                        outcome
                            .unrepresented_effect_actions
                            .append(&mut unrepresented_effect_actions);
                    }
                    if update_permanent_visible_item
                        && let Some(visible_item_update) =
                            self.loaded_inventory_item_visible_update_like_cpp(item_guid)
                    {
                        outcome.visible_item_changes.push(visible_item_update);
                    }
                }
                outcome.plans.push(plan);
            }
        }
        outcome
    }
    pub(crate) fn send_loaded_equipped_item_enchantment_updates_like_cpp(
        &self,
        outcome: &LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
    ) {
        if let Some(owner_guid) = self.player_guid() {
            self.send_item_enchant_time_update_plans(owner_guid, &outcome.duration_updates);
        }
        if !outcome.visible_item_changes.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                &[],
                &outcome.visible_item_changes,
                &[],
                &[],
                None,
            );
        }
    }
    fn apply_loaded_equipped_item_enchantment_effects_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        enchantment_id: i32,
        apply: bool,
    ) -> (
        bool,
        Vec<RepresentedItemBonusActionLikeCpp>,
        Vec<RepresentedItemBonusActionLikeCpp>,
    ) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return (false, Vec::new(), Vec::new());
        };
        let Some(effects) = u32::try_from(enchantment_id)
            .ok()
            .and_then(|enchantment_id| self.apply_enchantment_effect_refs(enchantment_id))
        else {
            return (false, Vec::new(), Vec::new());
        };
        let item_template = self.item_storage_template(item.object().entry());
        let random_suffix =
            self.apply_enchantment_random_suffix_ref(item.data().random_properties_id);
        let actions = self
            .mutate_canonical_player_like_cpp(|player| {
                player.apply_enchantment_effect_actions_for_enchantment(
                    &item,
                    item_template.as_ref(),
                    slot,
                    enchantment_id,
                    random_suffix,
                    apply,
                    &effects,
                )
            })
            .unwrap_or_default();

        let mut changed_stats = false;
        let mut represented_actions = Vec::new();
        let mut unrepresented_actions = Vec::new();
        for action in actions {
            if matches!(action, ApplyEnchantmentEffectAction::Noop) {
                continue;
            }
            changed_stats |=
                crate::session_rules::represented_item_bonus_action_updates_stats_like_cpp(action);
            let represented_action = RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: slot as u8,
                action,
            };
            #[cfg(test)]
            self.represented_item_bonus_actions_like_cpp
                .push(represented_action.clone());
            let spell_action_applied = self.apply_loaded_enchantment_spell_action_like_cpp(action);
            if !spell_action_applied
                && crate::session_rules::loaded_enchantment_effect_action_is_unrepresented_like_cpp(
                    action,
                )
            {
                unrepresented_actions.push(represented_action.clone());
            }
            represented_actions.push(represented_action);
            self.apply_represented_item_bonus_action_state_like_cpp(action);
        }
        (changed_stats, represented_actions, unrepresented_actions)
    }
}
