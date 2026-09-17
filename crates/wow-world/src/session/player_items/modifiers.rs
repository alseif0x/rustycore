//! Represented item bonuses, modifiers and item-set effects.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the C++ ItemLimitCategory.db2 store for this session.
    pub fn set_item_limit_category_store(&mut self, store: Arc<ItemLimitCategoryStore>) {
        self.items.limit_category_store = Some(store);
    }
    /// Set the C++ ItemLimitCategoryCondition.db2 store for this session.
    pub fn set_item_limit_category_condition_store(
        &mut self,
        store: Arc<ItemLimitCategoryConditionStore>,
    ) {
        self.items.limit_category_condition_store = Some(store);
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
            .items
            .limit_category_store
            .as_ref()
            .and_then(|store| store.get(limit_category_id))?;

        let mut quantity = entry.quantity;
        if let Some(condition_store) = self.items.limit_category_condition_store.as_ref() {
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
        self.items.bonus_db2_store = Some(store);
    }
    pub fn set_item_set_store(&mut self, store: Arc<ItemSetStore>) {
        self.items.set_store = Some(store);
    }
    pub fn set_item_set_spell_store(&mut self, store: Arc<ItemSetSpellStore>) {
        self.spell_catalogs.item_set_spell_store = Some(store);
    }
    pub(crate) fn item_set_for_item_id_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<&wow_data::ItemSetEntry> {
        self.items
            .set_store
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
            .with_owned_player_like_cpp(|player| player.item_modifier_runtime_snapshot_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_item_modifier_runtime_like_cpp.clone());
        }
        canonical
    }

    pub(in crate::session) fn add_player_item_set_item_like_cpp(
        &mut self,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<usize> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.add_item_set_item_like_cpp(item_set_id, item_guid)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.represented_item_modifier_runtime_like_cpp
                    .add_item_set_item_like_cpp(item_set_id, item_guid),
            );
        }
        None
    }

    pub(in crate::session) fn add_player_item_set_bonus_like_cpp(
        &mut self,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.add_item_set_bonus_like_cpp(item_set_id, spell_entry_id)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.represented_item_modifier_runtime_like_cpp
                    .add_item_set_bonus_like_cpp(item_set_id, spell_entry_id),
            );
        }
        None
    }

    pub(in crate::session) fn remove_player_item_set_item_like_cpp(
        &mut self,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<Option<usize>> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.remove_item_set_item_like_cpp(item_set_id, item_guid)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.represented_item_modifier_runtime_like_cpp
                    .remove_item_set_item_like_cpp(item_set_id, item_guid),
            );
        }
        None
    }

    pub(in crate::session) fn remove_player_item_set_bonus_like_cpp(
        &mut self,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.remove_item_set_bonus_like_cpp(item_set_id, spell_entry_id)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.represented_item_modifier_runtime_like_cpp
                    .remove_item_set_bonus_like_cpp(item_set_id, spell_entry_id),
            );
        }
        None
    }

    pub(in crate::session) fn drop_player_empty_item_set_effect_like_cpp(
        &mut self,
        item_set_id: u32,
    ) -> Option<bool> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.drop_empty_item_set_effect_like_cpp(item_set_id)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.represented_item_modifier_runtime_like_cpp
                    .drop_empty_item_set_effect_like_cpp(item_set_id),
            );
        }
        None
    }

    pub(in crate::session) fn set_player_item_level_caps_like_cpp(
        &mut self,
        caps: wow_entities::PlayerItemLevelCapsLikeCpp,
    ) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.set_item_level_caps_like_cpp(caps);
        });
        if canonical.is_some() {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_item_modifier_runtime_like_cpp
                .set_item_level_caps_like_cpp(caps);
            return true;
        }
        false
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
        let Some(item_stats_store) = self.items.stats_store.as_ref().cloned() else {
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
            // C++ `Player::_ApplyWeaponDamage` (`Player.cpp:7979-8020`) skips the
            // disarm gate in feral form and keeps the existing attack time while
            // the active form carries a `CombatRoundTime`.
            let is_in_feral_form = self
                .canonical_player_snapshot_like_cpp(|player| player.is_in_feral_form_like_cpp())
                .unwrap_or(false);
            // C++ reaches `_ApplyWeaponDamage` for any unit that is not
            // disarmed; an unavailable canonical owner is treated as unflagged.
            let can_use_attack_type = self
                .represented_can_use_attack_type_like_cpp(slot, Some(inventory_type))
                != Some(false);
            let has_shapeshift_combat_round_time = self
                .represented_shapeshift_combat_round_time_like_cpp()
                .is_some();
            planned_actions.extend(
                item_weapon_damage_actions_like_cpp(
                    slot,
                    inventory_type,
                    min_damage,
                    max_damage,
                    weapon.item_delay,
                    apply,
                    is_in_feral_form,
                    can_use_attack_type,
                    has_shapeshift_combat_round_time,
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
            .items
            .stats_store
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
        let Some(active_effects) =
            self.player_item_modifier_runtime_snapshot_like_cpp()
                .map(|state| {
                    state
                        .item_set_effects_like_cpp()
                        .values()
                        .cloned()
                        .collect::<Vec<_>>()
                })
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
        for (slot, item_guid) in equipped {
            // C++ `_ApplyAllItemMods` starts each equipped item with
            // `_ApplyItemBonuses`. Seed the canonical Player accumulator here
            // so login and a later equip/swap use exactly the same contribution
            // path. Broken items are rejected before both static/scaling
            // bonuses and aura/enchantment effects are considered.
            if self
                .resolved_inventory_item_object_like_cpp(item_guid)
                .is_some_and(|item| !item.is_broken())
            {
                self.record_represented_item_mods_like_cpp(item_guid, slot, true);
            }
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
    pub(crate) fn send_represented_item_bonus_player_stat_update_like_cpp(&mut self) -> bool {
        // Item changes alter derived stats, vital maxima and weapon ranges
        // together. Publish the same complete projection consumed by combat;
        // sending only the raw bonus accumulator would leave the canonical
        // snapshot stale and could make the client and server disagree after
        // repair or an equipment-set swap. A raw packet is retained only as a
        // fixture/early-login fallback when the complete projection cannot yet
        // be formed; no derived combat snapshot exists in that state.
        if self.send_stat_update() {
            return true;
        }
        let Some(update) = self.represented_item_bonus_player_stat_update_object_like_cpp() else {
            return false;
        };
        self.send_packet(&update);
        true
    }
    pub(in crate::session) fn apply_represented_item_bonus_action_state_like_cpp(
        &mut self,
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.apply_item_modifier_action_like_cpp(action);
        });
        if canonical.is_some() {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_item_modifier_runtime_like_cpp
                .apply_enchantment_effect_action_like_cpp(action);
            return true;
        }
        false
    }
    pub(in crate::session) fn reset_represented_item_bonus_runtime_like_cpp(&mut self) {
        // C++ WorldSession::HandlePlayerLogin constructs a fresh Player, so
        // item modifiers from a previous character cannot survive into the
        // next login on the same session.
        #[cfg(test)]
        self.represented_item_bonus_actions_like_cpp.clear();
        let canonical_missing = self
            .with_owned_player_mut_like_cpp(|player| {
                player.reset_item_modifier_bonuses_like_cpp();
            })
            .is_none();
        #[cfg(not(test))]
        let _ = canonical_missing;
        #[cfg(test)]
        if canonical_missing && self.player_handle_like_cpp.is_none() {
            self.represented_item_modifier_runtime_like_cpp
                .reset_bonuses_like_cpp();
        }
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
        self.represented_can_use_attack_type_like_cpp(item.slot(), inventory_type) == Some(true)
    }

    /// C++ `Player::CanUseAttackType` for the attack an equipment slot maps to:
    /// `BASE_ATTACK` requires no `UNIT_FLAG_DISARMED`, `OFF_ATTACK` no
    /// `UNIT_FLAG2_DISARM_OFFHAND`, `RANGED_ATTACK` no
    /// `UNIT_FLAG2_DISARM_RANGED`, and any other slot is unaffected.
    ///
    /// `None` when the canonical Player owner is unavailable, so a caller can
    /// choose whether an unknown disarm state is fail-closed (enchantments) or
    /// fail-open (the `_ApplyWeaponDamage` producer, which C++ reaches for an
    /// unflagged unit).
    ///
    /// C++ `Player::GetAttackBySlot` has cases only for MAINHAND and OFFHAND.
    /// In particular, legacy `EQUIPMENT_SLOT_RANGED` deliberately falls through
    /// to `MAX_ATTACK`; ranged inventory types map to `RANGED_ATTACK` only when
    /// stored in MAINHAND.
    pub(in crate::session) fn represented_can_use_attack_type_like_cpp(
        &self,
        slot: u8,
        inventory_type: Option<InventoryType>,
    ) -> Option<bool> {
        let attack_type = match slot {
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
        match attack_type {
            WeaponAttackType::BaseAttack => self.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags_like_cpp()
                    .contains(UnitFlags::DISARMED)
            }),
            WeaponAttackType::OffAttack => self.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags2_like_cpp()
                    .contains(UnitFlags2::DISARM_OFFHAND)
            }),
            WeaponAttackType::RangedAttack => self.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags2_like_cpp()
                    .contains(UnitFlags2::DISARM_RANGED)
            }),
            WeaponAttackType::Max => Some(true),
        }
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
            .item_set_effect_like_cpp(item_set_id)
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
                .bonuses_snapshot_like_cpp(),
        )
    }

    /// C++ `Player::CanUseItem(ItemTemplate const*)`'s reputation term
    /// (`Player.cpp:11106-11107`): the player's `GetReputationRank` for the
    /// required faction. Returns `None` when the faction exists but the
    /// reputation manager is not represented, so each caller keeps its own
    /// fail-closed policy; an unknown faction ranks zero like C++.
    pub(crate) fn represented_item_reputation_rank_like_cpp(
        &self,
        required_reputation_faction: u32,
    ) -> Option<u32> {
        if required_reputation_faction == 0 {
            return Some(0);
        }
        let Some(faction) = self
            .factions
            .store
            .as_ref()
            .and_then(|store| store.get(required_reputation_faction))
        else {
            return Some(0);
        };
        // The session identity accessors re-enter the canonical manager lock
        // held by `with_reputation_mgr_like_cpp`, so resolve them first.
        let player_race = self.player_race_like_cpp();
        let player_class = self.player_class_like_cpp();
        let standing = self.with_reputation_mgr_like_cpp(|mgr| {
            mgr.reputation_for_faction_like_cpp(faction, player_race, player_class)
        })?;
        Some(u32::from(
            reputation_to_rank_like_cpp(
                faction,
                standing,
                self.friendship_rep_reaction_store.as_deref(),
            )
            .as_u8(),
        ))
    }

    /// C++ `ItemTemplate::Effects` ordered by `ItemEffectEntry` slot. The
    /// `CanUseItem` learning-effect gate (`Player.cpp:11110-11113`) reads the
    /// first two entries.
    pub(crate) fn represented_item_effect_spell_ids_like_cpp(
        &self,
        item_id: u32,
    ) -> Vec<(u8, i32)> {
        let mut effects: Vec<(u8, i32)> = self
            .items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .values()
                    .filter(|effect| effect.parent_item_id == item_id)
                    .map(|effect| (effect.legacy_slot_index, effect.spell_id))
                    .collect()
            })
            .unwrap_or_default();
        effects.sort_by_key(|(slot, _)| *slot);
        effects
    }
}
