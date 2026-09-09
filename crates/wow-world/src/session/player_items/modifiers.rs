//! Represented item bonuses, modifiers and item-set effects.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the C++ ItemLimitCategory.db2 store for this session.
    pub fn set_item_limit_category_store(&mut self, store: Arc<ItemLimitCategoryStore>) {
        self.item_limit_category_store = Some(store);
    }
    /// Set the C++ ItemLimitCategoryCondition.db2 store for this session.
    pub fn set_item_limit_category_condition_store(
        &mut self,
        store: Arc<ItemLimitCategoryConditionStore>,
    ) {
        self.item_limit_category_condition_store = Some(store);
    }
    /// C++ `sItemLimitCategoryStore.LookupEntry(limitCategory)`.
    pub(crate) fn item_limit_category_template_like_cpp(
        &self,
        limit_category_id: u32,
    ) -> Option<ItemLimitCategoryTemplate> {
        if limit_category_id == 0 {
            return None;
        }

        let entry = self
            .item_limit_category_store
            .as_ref()
            .and_then(|store| store.get(limit_category_id))?;

        let mut quantity = entry.quantity;
        if let Some(condition_store) = self.item_limit_category_condition_store.as_ref() {
            let context_holder = self.represented_player_condition_context_like_cpp()?;
            let context = context_holder.as_context(self)?;
            for condition in condition_store.conditions_for_parent_like_cpp(entry.id) {
                let player_condition = self
                    .player_condition_store
                    .as_ref()
                    .and_then(|store| store.get(condition.player_condition_id));
                if player_condition.is_none_or(|condition| {
                    is_player_meeting_condition_like_cpp(condition, &context)
                }) {
                    quantity = (i16::from(quantity) + i16::from(condition.add_quantity)) as u8;
                }
            }
        }

        Some(ItemLimitCategoryTemplate {
            id: entry.id,
            quantity,
            flags: entry.flags,
        })
    }
    /// Set the item stats store for this session.
    pub fn set_item_bonus_db2_store(&mut self, store: Arc<ItemBonusDb2Store>) {
        self.item_bonus_db2_store = Some(store);
    }
    pub fn set_item_set_store(&mut self, store: Arc<ItemSetStore>) {
        self.item_set_store = Some(store);
    }
    pub fn set_item_set_spell_store(&mut self, store: Arc<ItemSetSpellStore>) {
        self.spell_catalogs.item_set_spell_store = Some(store);
    }
    pub(crate) fn item_set_for_item_id_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<&wow_data::ItemSetEntry> {
        self.item_set_store
            .as_ref()
            .and_then(|store| store.item_set_for_item_id_like_cpp(item_id))
    }
    pub(crate) fn item_set_spells_like_cpp(
        &self,
        item_set_id: u32,
    ) -> Vec<&wow_data::ItemSetSpellEntry> {
        self.spell_catalogs
            .item_set_spell_store
            .as_ref()
            .map(|store| store.item_set_spells_like_cpp(item_set_id))
            .unwrap_or_default()
    }
    pub(in crate::session) fn player_item_modifier_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerItemModifierRuntimeStateLikeCpp> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.gameplay_state().item_modifiers.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerItemModifierRuntimeStateLikeCpp {
                bonuses: self.represented_item_bonus_state_like_cpp.clone(),
                item_set_effects: self.represented_item_set_effects_like_cpp.clone(),
                item_level_caps: self.represented_item_level_caps_like_cpp,
            });
        }
        canonical
    }
    pub(in crate::session) fn mutate_player_item_modifier_runtime_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerItemModifierRuntimeStateLikeCpp) -> R,
    ) -> Option<R> {
        let mut mutate = Some(mutate);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut state = self.player_item_modifier_runtime_snapshot_like_cpp()?;
            let result =
                mutate
                    .take()
                    .expect("test item-modifier mutation executes once")(&mut state);
            self.represented_item_bonus_state_like_cpp = state.bonuses;
            self.represented_item_set_effects_like_cpp = state.item_set_effects;
            self.represented_item_level_caps_like_cpp = state.item_level_caps;
            return Some(result);
        }
        self.with_owned_player_mut_like_cpp(|player| {
            mutate
                .take()
                .expect("Player item-modifier mutation executes once")(
                &mut player.gameplay_state_mut().item_modifiers,
            )
        })
    }
    pub(in crate::session) fn record_represented_all_item_mods_like_cpp(
        &mut self,
        targets: &[(u8, ObjectGuid)],
        apply: bool,
    ) {
        for (slot, item_guid) in targets {
            self.record_represented_item_mods_like_cpp(*item_guid, *slot, apply);
        }
    }
    pub(in crate::session) fn record_represented_item_mods_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: u8,
        apply: bool,
    ) -> usize {
        #[cfg(test)]
        {
            self.represented_item_mod_reapply_events_like_cpp.push(
                RepresentedItemModsReapplyEventLikeCpp {
                    item_guid,
                    slot,
                    apply,
                },
            );
        }

        let Some(item_entry) = self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .map(|item| item.object().entry())
        else {
            return 0;
        };
        let Some(item_stats_store) = self.item_stats_store.as_ref().cloned() else {
            return 0;
        };
        let mut planned_actions = Vec::new();

        let scaling_context = self.represented_scaling_stat_context_like_cpp(item_entry);
        if let Some(context) = scaling_context {
            planned_actions.extend(
                item_scaling_stat_bonus_actions_like_cpp(
                    &context.stat_id,
                    &context.bonus,
                    context.ssd_multiplier,
                    apply,
                )
                .into_iter()
                .map(|action| RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action,
                }),
            );
            if context.spell_bonus > 0 {
                planned_actions.push(RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action: ApplyEnchantmentEffectAction::SpellPowerBonus {
                        amount: context.spell_bonus as u32,
                        apply,
                    },
                });
            } else if context.spell_bonus < 0 {
                planned_actions.push(RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action: ApplyEnchantmentEffectAction::UnhandledStatModifier {
                        item_mod: wow_constants::ItemModType::SpellPower,
                        amount: context.spell_bonus.unsigned_abs(),
                        apply,
                    },
                });
            }
        } else if let Some(stat_entry) = item_stats_store.get(item_entry) {
            planned_actions.extend(
                item_stat_bonus_actions_like_cpp(&stat_entry.stats, apply)
                    .into_iter()
                    .map(|action| RepresentedItemBonusActionLikeCpp {
                        item_guid,
                        slot,
                        action,
                    }),
            );
        }

        if let Some(stat_entry) = item_stats_store.get(item_entry) {
            let resistances = self.represented_resistances_with_scaling_armor_like_cpp(
                &stat_entry.resistances,
                scaling_context,
            );
            planned_actions.extend(
                item_resistance_bonus_actions_like_cpp(&resistances, apply)
                    .into_iter()
                    .map(|action| RepresentedItemBonusActionLikeCpp {
                        item_guid,
                        slot,
                        action,
                    }),
            );
        }

        if let Some(action) =
            self.item_shield_block_value_like_cpp(item_entry)
                .and_then(|shield_block_value| {
                    item_shield_block_bonus_action_like_cpp(shield_block_value, true, apply)
                })
        {
            planned_actions.push(RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot,
                action,
            });
        }

        if let (Some(weapon), Some(inventory_type)) = (
            item_stats_store.weapon_template(item_entry),
            self.represented_item_inventory_type_like_cpp(item_entry, item_guid),
        ) {
            let (min_damage, max_damage) =
                self.represented_weapon_damage_bounds_like_cpp(item_entry, weapon);
            planned_actions.extend(
                item_weapon_damage_actions_like_cpp(
                    slot,
                    inventory_type,
                    min_damage,
                    max_damage,
                    weapon.item_delay,
                    apply,
                    false,
                    true,
                    false,
                    true,
                )
                .into_iter()
                .map(|action| RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action,
                }),
            );
        }

        #[cfg(test)]
        self.represented_item_bonus_actions_like_cpp
            .extend(planned_actions.iter().cloned());
        let action_count = planned_actions.len();
        for planned in planned_actions {
            self.apply_represented_item_bonus_action_state_like_cpp(planned.action);
        }
        action_count
    }
    pub(in crate::session) fn represented_item_set_spell_exists_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        self.spell_catalogs
            .spell_store
            .as_ref()
            .is_none_or(|store| store.get(spell_id).is_some())
    }
    pub(in crate::session) fn represented_heirloom_item_set_bonus_over_level_cap_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(item_entry) = self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .map(|item| item.object().entry())
        else {
            return false;
        };
        if !self
            .heirloom_store
            .as_ref()
            .is_some_and(|store| store.get_by_item_id_like_cpp(item_entry).is_some())
        {
            return false;
        }

        let Some(template) = self
            .item_stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_entry))
        else {
            return false;
        };
        let curve_id = template.player_level_to_item_level_curve_id_like_cpp();
        if curve_id == 0 {
            return false;
        }

        let Some((curve_store, curve_point_store)) = self
            .curve_store
            .as_ref()
            .zip(self.curve_point_store.as_ref())
        else {
            return false;
        };
        let Some((_min_level, max_level)) =
            curve_store.curve_x_axis_range_like_cpp(curve_point_store, curve_id)
        else {
            return false;
        };
        if !max_level.is_finite() || max_level < 0.0 {
            return false;
        }
        let mut max_level = max_level as u32;

        if let Some(content_tuning) = self.content_tuning_store.as_ref().and_then(|store| {
            store
                .content_tuning_data_like_cpp(template.scaling_stat_content_tuning_like_cpp(), true)
        }) {
            max_level = max_level.min(u32::try_from(content_tuning.max_level).unwrap_or(0));
        }

        u32::from(self.player_level_like_cpp()) > max_level
    }
    pub(crate) fn record_represented_update_item_set_auras_like_cpp(
        &mut self,
        form_change: bool,
    ) -> usize {
        let events = self.plan_represented_update_item_set_auras_like_cpp(form_change);
        #[cfg(test)]
        self.represented_item_set_aura_refresh_events_like_cpp
            .extend(events.iter().cloned());
        events.len()
    }
    fn plan_represented_update_item_set_auras_like_cpp(
        &self,
        form_change: bool,
    ) -> Vec<RepresentedItemSetAuraRefreshEventLikeCpp> {
        let mut events = Vec::new();
        let primary_spec = self.represented_primary_specialization_id_like_cpp();
        let Some(active_effects) = self
            .player_item_modifier_runtime_snapshot_like_cpp()
            .map(|state| state.item_set_effects.values().cloned().collect::<Vec<_>>())
        else {
            return events;
        };

        for effect in active_effects {
            let active_bonus_ids = effect.set_bonuses.clone();
            let spells: Vec<_> = self
                .item_set_spells_like_cpp(effect.item_set_id)
                .into_iter()
                .filter(|spell| active_bonus_ids.contains(&spell.id))
                .cloned()
                .collect();

            for item_set_spell in spells {
                if item_set_spell.chr_spec_id != 0
                    && Some(u32::from(item_set_spell.chr_spec_id)) != primary_spec
                {
                    events.push(RepresentedItemSetAuraRefreshEventLikeCpp {
                        item_set_id: effect.item_set_id,
                        spell_entry_id: item_set_spell.id,
                        spell_id: item_set_spell.spell_id,
                        apply: false,
                        form_change: false,
                    });
                    continue;
                }

                let fits_shapeshift =
                    self.represented_equip_spell_fits_shapeshift_like_cpp(item_set_spell.spell_id);
                if !form_change || !fits_shapeshift {
                    events.push(RepresentedItemSetAuraRefreshEventLikeCpp {
                        item_set_id: effect.item_set_id,
                        spell_entry_id: item_set_spell.id,
                        spell_id: item_set_spell.spell_id,
                        apply: false,
                        form_change,
                    });
                }
                if fits_shapeshift {
                    events.push(RepresentedItemSetAuraRefreshEventLikeCpp {
                        item_set_id: effect.item_set_id,
                        spell_entry_id: item_set_spell.id,
                        spell_id: item_set_spell.spell_id,
                        apply: true,
                        form_change,
                    });
                }
            }
        }

        events
    }
    pub(in crate::session) fn apply_initial_item_set_auras_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let events = self.record_represented_items_set_item_events_like_cpp(item_guid, true);
        let mut applied = 0usize;
        for event in events {
            if !event.apply {
                continue;
            }
            let Ok(spell_id) = i32::try_from(event.spell_id) else {
                continue;
            };
            if self.player_has_visible_aura_spell_like_cpp(spell_id) != Some(false) {
                continue;
            }
            let effect_mask = self
                .spell_store()
                .and_then(|store| store.get(spell_id))
                .map(unit_owned_apply_aura_effect_mask_like_cpp)
                .unwrap_or(0x0000_0001)
                .max(0x0000_0001);
            if self
                .apply_aura_with_effect_mask_like_cpp(
                    spell_id,
                    player_guid,
                    0,
                    AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                    effect_mask,
                )
                .is_ok()
            {
                applied += 1;
            }
        }
        applied
    }
    /// Replays the aura-producing part of C++ `Player::_ApplyAllItemMods` after
    /// `Player::_LoadAuras`. C++ walks equipment slots and, for each item,
    /// applies its item-set effect, regular equip spell, and enchantments before
    /// advancing to the next slot; preserve that exact order for aura slots.
    pub(crate) fn apply_initial_loaded_item_mods_like_cpp(
        &mut self,
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> InitialLoadedItemModsOutcomeLikeCpp {
        // This is the initial C++ `_ApplyAllItemMods` replay for a newly
        // constructed Player. Start from the same empty modifier state even
        // after a failed/retried login that did not reach normal teardown.
        self.reset_represented_item_bonus_runtime_like_cpp();

        let mut equipped = loaded_equipped_item_guids
            .iter()
            .filter_map(|&item_guid| {
                self.resolved_inventory_item_object_like_cpp(item_guid)
                    .map(|item| (item.slot(), item_guid))
            })
            .collect::<Vec<_>>();
        equipped.sort_by_key(|(slot, guid)| (*slot, guid.counter()));

        let mut item_set_auras = 0usize;
        let mut item_equip_auras = 0usize;
        let mut enchantments = LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
        for (_slot, item_guid) in equipped {
            item_set_auras += self.apply_initial_item_set_auras_like_cpp(item_guid);
            item_equip_auras += self.apply_initial_item_equip_auras_like_cpp(item_guid);
            enchantments.append(self.apply_loaded_equipped_item_enchantments_like_cpp(item_guid));
        }

        InitialLoadedItemModsOutcomeLikeCpp {
            item_set_auras,
            item_equip_auras,
            enchantments,
        }
    }
    pub(crate) fn apply_represented_item_set_aura_refresh_events_like_cpp(
        &mut self,
        form_change: bool,
    ) -> usize {
        let events = self.plan_represented_update_item_set_auras_like_cpp(form_change);
        #[cfg(test)]
        self.represented_item_set_aura_refresh_events_like_cpp
            .extend(events.iter().cloned());
        let recorded = events.len();
        let Some(player_guid) = self.player_guid() else {
            return recorded;
        };

        for event in events {
            let Ok(spell_id) = i32::try_from(event.spell_id) else {
                continue;
            };
            if event.apply {
                if event.form_change
                    && self.player_has_visible_aura_spell_like_cpp(spell_id) != Some(false)
                {
                    continue;
                }
                let effect_mask = self
                    .spell_store()
                    .and_then(|store| store.get(spell_id))
                    .map(unit_owned_apply_aura_effect_mask_like_cpp)
                    .unwrap_or(0x0000_0001)
                    .max(0x0000_0001);
                let _ = self.apply_aura_with_effect_mask_like_cpp(
                    spell_id,
                    player_guid,
                    0,
                    AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                    effect_mask,
                );
            } else {
                let _ = self.remove_represented_auras_due_to_spell_like_cpp(spell_id);
            }
        }

        recorded
    }
    fn represented_item_bonus_player_stat_update_object_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        let player_guid = self.player_guid()?;
        let bonuses = self.resolved_item_bonus_state_like_cpp()?;
        Some(
            wow_packet::packets::update::UpdateObject::player_stat_update(
                player_guid,
                self.player_map_id_like_cpp(),
                represented_player_stat_changes_like_cpp(&bonuses),
            ),
        )
    }
    pub(crate) fn send_represented_item_bonus_player_stat_update_like_cpp(&self) -> bool {
        let Some(update) = self.represented_item_bonus_player_stat_update_object_like_cpp() else {
            return false;
        };
        self.send_packet(&update);
        true
    }
    pub(in crate::session) fn represented_item_bonus_action_updates_stats_like_cpp(
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        matches!(
            action,
            ApplyEnchantmentEffectAction::UnitModifier { .. }
                | ApplyEnchantmentEffectAction::UpdateStatBuffMod(_)
                | ApplyEnchantmentEffectAction::RatingModifier { .. }
                | ApplyEnchantmentEffectAction::ManaRegenBonus { .. }
                | ApplyEnchantmentEffectAction::SpellPowerBonus { .. }
                | ApplyEnchantmentEffectAction::HealthRegenBonus { .. }
                | ApplyEnchantmentEffectAction::SpellPenetrationBonus { .. }
                | ApplyEnchantmentEffectAction::BaseModFlatValue { .. }
                | ApplyEnchantmentEffectAction::SetShieldBlockValue { .. }
                | ApplyEnchantmentEffectAction::SetBaseWeaponDamage { .. }
                | ApplyEnchantmentEffectAction::SetBaseAttackTime { .. }
                | ApplyEnchantmentEffectAction::UpdateDamagePhysical { .. }
        )
    }
    pub(in crate::session) fn apply_represented_item_bonus_action_state_like_cpp(
        &mut self,
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        self.mutate_player_item_modifier_runtime_like_cpp(|runtime| {
            Self::apply_represented_item_bonus_action_to_state_like_cpp(
                &mut runtime.bonuses,
                action,
            );
        })
        .is_some()
    }
    fn apply_represented_item_bonus_action_to_state_like_cpp(
        state: &mut RepresentedItemBonusStateLikeCpp,
        action: ApplyEnchantmentEffectAction,
    ) {
        match action {
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod,
                modifier,
                amount,
                apply,
            } => Self::apply_represented_unit_modifier_like_cpp(
                state, unit_mod, modifier, amount, apply,
            ),
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(stat) => {
                state.stat_buff_updates.push(stat)
            }
            ApplyEnchantmentEffectAction::RatingModifier {
                rating,
                amount,
                apply,
            } => {
                if let Some(index) = represented_combat_rating_index_like_cpp(rating) {
                    apply_represented_i32_delta_like_cpp(
                        &mut state.combat_ratings[index],
                        amount,
                        apply,
                    );
                }
            }
            ApplyEnchantmentEffectAction::ManaRegenBonus { amount, apply } => {
                apply_represented_i32_delta_like_cpp(&mut state.mana_regen_bonus, amount, apply);
            }
            ApplyEnchantmentEffectAction::SpellPowerBonus { amount, apply } => {
                apply_represented_i32_delta_like_cpp(&mut state.spell_power_bonus, amount, apply);
            }
            ApplyEnchantmentEffectAction::HealthRegenBonus { amount, apply } => {
                apply_represented_i32_delta_like_cpp(&mut state.health_regen_bonus, amount, apply);
            }
            ApplyEnchantmentEffectAction::SpellPenetrationBonus { amount, apply } => {
                apply_represented_i32_delta_like_cpp(
                    &mut state.spell_penetration_bonus,
                    amount,
                    apply,
                );
            }
            ApplyEnchantmentEffectAction::BaseModFlatValue {
                base_mod: wow_entities::ApplyEnchantmentBaseMod::ShieldBlockValue,
                amount,
                apply,
            } => {
                apply_represented_i32_delta_like_cpp(
                    &mut state.shield_block_base_mod,
                    amount,
                    apply,
                );
            }
            ApplyEnchantmentEffectAction::SetShieldBlockValue { amount } => {
                state.shield_block_value = amount;
            }
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type,
                bound,
                amount_bits,
            } => {
                let attack = attack_type as usize;
                if attack < state.weapon_damage.len() {
                    let bound = match bound {
                        wow_entities::WeaponDamageBoundLikeCpp::Min => 0,
                        wow_entities::WeaponDamageBoundLikeCpp::Max => 1,
                    };
                    state.weapon_damage[attack][bound] = f32::from_bits(amount_bits);
                }
            }
            ApplyEnchantmentEffectAction::SetBaseAttackTime {
                attack_type,
                time_ms,
            } => {
                let attack = attack_type as usize;
                if attack < state.base_attack_time.len() {
                    state.base_attack_time[attack] = time_ms;
                }
            }
            ApplyEnchantmentEffectAction::UpdateDamagePhysical { attack_type } => {
                state.damage_physical_updates.push(attack_type)
            }
            ApplyEnchantmentEffectAction::Noop
            | ApplyEnchantmentEffectAction::DeferredCombatSpell
            | ApplyEnchantmentEffectAction::DeferredUseSpell
            | ApplyEnchantmentEffectAction::UpdateDamageDoneMods { .. }
            | ApplyEnchantmentEffectAction::CastEquipSpell { .. }
            | ApplyEnchantmentEffectAction::RemoveEquipSpellAura { .. }
            | ApplyEnchantmentEffectAction::UnhandledStatModifier { .. }
            | ApplyEnchantmentEffectAction::MissingItemTemplateForAttack { .. }
            | ApplyEnchantmentEffectAction::Unknown { .. } => {}
        }
    }
    pub(in crate::session) fn reset_represented_item_bonus_runtime_like_cpp(&mut self) {
        // C++ WorldSession::HandlePlayerLogin constructs a fresh Player, so
        // item modifiers from a previous character cannot survive into the
        // next login on the same session.
        #[cfg(test)]
        self.represented_item_bonus_actions_like_cpp.clear();
        let _ = self.mutate_player_item_modifier_runtime_like_cpp(|runtime| {
            runtime.bonuses = RepresentedItemBonusStateLikeCpp::default();
        });
    }
    pub(in crate::session) fn initial_loaded_item_mods_can_apply_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return false;
        };
        // C++ `_ApplyAllItemMods` skips broken items before both
        // `ApplyItemEquipSpell` and `ApplyEnchantment`.
        if item.is_broken() {
            return false;
        }
        let inventory_type = self
            .item_storage_template(item.object().entry())
            .map(|template| template.inventory_type);
        // C++ `Player::GetAttackBySlot` has cases only for MAINHAND and
        // OFFHAND. In particular, legacy `EQUIPMENT_SLOT_RANGED` deliberately
        // falls through to `MAX_ATTACK`; ranged inventory types map to
        // `RANGED_ATTACK` only when stored in MAINHAND.
        let attack_type = match item.slot() {
            EQUIPMENT_SLOT_MAINHAND
                if matches!(
                    inventory_type,
                    Some(InventoryType::Ranged | InventoryType::RangedRight)
                ) =>
            {
                WeaponAttackType::RangedAttack
            }
            EQUIPMENT_SLOT_MAINHAND => WeaponAttackType::BaseAttack,
            EQUIPMENT_SLOT_OFFHAND => WeaponAttackType::OffAttack,
            _ => WeaponAttackType::Max,
        };
        let can_use_attack_type = match attack_type {
            WeaponAttackType::BaseAttack => self
                .canonical_player_snapshot_like_cpp(|player| {
                    !player
                        .unit()
                        .unit_flags_like_cpp()
                        .contains(UnitFlags::DISARMED)
                })
                .unwrap_or(false),
            WeaponAttackType::OffAttack => self
                .canonical_player_snapshot_like_cpp(|player| {
                    !player
                        .unit()
                        .unit_flags2_like_cpp()
                        .contains(UnitFlags2::DISARM_OFFHAND)
                })
                .unwrap_or(false),
            WeaponAttackType::RangedAttack => self
                .canonical_player_snapshot_like_cpp(|player| {
                    !player
                        .unit()
                        .unit_flags2_like_cpp()
                        .contains(UnitFlags2::DISARM_RANGED)
                })
                .unwrap_or(false),
            WeaponAttackType::Max => true,
        };
        can_use_attack_type
    }
    #[cfg(test)]
    pub(crate) fn represented_item_bonus_actions_like_cpp(
        &self,
    ) -> &[RepresentedItemBonusActionLikeCpp] {
        &self.represented_item_bonus_actions_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_item_set_spell_events_like_cpp(
        &self,
    ) -> &[RepresentedItemSetSpellEventLikeCpp] {
        &self.represented_item_set_spell_events_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_item_set_aura_refresh_events_like_cpp(
        &self,
    ) -> &[RepresentedItemSetAuraRefreshEventLikeCpp] {
        &self.represented_item_set_aura_refresh_events_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_item_set_effect_like_cpp(
        &self,
        item_set_id: u32,
    ) -> Option<RepresentedItemSetEffectLikeCpp> {
        self.player_item_modifier_runtime_snapshot_like_cpp()?
            .item_set_effects
            .get(&item_set_id)
            .cloned()
    }
    #[cfg(test)]
    pub(crate) fn represented_item_bonus_state_like_cpp(&self) -> RepresentedItemBonusStateLikeCpp {
        self.resolved_item_bonus_state_like_cpp()
            .expect("test Player item-bonus owner must resolve")
    }
    pub(crate) fn resolved_item_bonus_state_like_cpp(
        &self,
    ) -> Option<RepresentedItemBonusStateLikeCpp> {
        Some(
            self.player_item_modifier_runtime_snapshot_like_cpp()?
                .bonuses,
        )
    }
}
