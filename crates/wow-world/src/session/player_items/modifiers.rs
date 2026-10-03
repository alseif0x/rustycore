//! Represented item bonuses, modifiers and item-set effects.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the C++ ItemLimitCategory.db2 store for this session.
    pub fn set_item_limit_category_store(&mut self, store: Arc<ItemLimitCategoryStore>) {
        self.catalogs.items.limit_category_store = Some(store);
    }
    /// Set the C++ ItemLimitCategoryCondition.db2 store for this session.
    pub fn set_item_limit_category_condition_store(
        &mut self,
        store: Arc<ItemLimitCategoryConditionStore>,
    ) {
        self.catalogs.items.limit_category_condition_store = Some(store);
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
            .catalogs
            .items
            .limit_category_store
            .as_ref()
            .and_then(|store| store.get(limit_category_id))?;

        let mut quantity = entry.quantity;
        if let Some(condition_store) = self.catalogs.items.limit_category_condition_store.as_ref() {
            let context_holder = self.represented_player_condition_context_like_cpp()?;
            let context = context_holder.as_context(self)?;
            for condition in condition_store.conditions_for_parent_like_cpp(entry.id) {
                let player_condition = self
                    .catalogs
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
        self.catalogs.items.bonus_db2_store = Some(store);
    }
    pub fn set_item_set_store(&mut self, store: Arc<ItemSetStore>) {
        self.catalogs.items.set_store = Some(store);
    }
    pub fn set_item_set_spell_store(&mut self, store: Arc<ItemSetSpellStore>) {
        self.catalogs.spell_catalogs.item_set_spell_store = Some(store);
    }
    pub(in crate::session) fn player_item_modifier_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerItemModifierRuntimeStateLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.player_item_modifier_runtime_snapshot_like_cpp(hub)
    }

    pub(in crate::session) fn add_player_item_set_item_like_cpp(
        &mut self,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<usize> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_player_item_set_item_like_cpp(&mut hub, item_set_id, item_guid)
    }

    pub(in crate::session) fn add_player_item_set_bonus_like_cpp(
        &mut self,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_player_item_set_bonus_like_cpp(&mut hub, item_set_id, spell_entry_id)
    }

    pub(in crate::session) fn set_player_item_level_caps_like_cpp(
        &mut self,
        caps: wow_entities::PlayerItemLevelCapsLikeCpp,
    ) -> bool {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_item_level_caps_like_cpp(caps);
        });
        if canonical.is_some() {
            return true;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.inventory
                .set_player_item_level_caps_for_test_like_cpp(caps);
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
        let inventory_access = self.core.owned_inventory_access_like_cpp();
        let modifier_access = self.core.owned_item_modifiers_access_like_cpp();
        let catalogs = wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
            self.catalogs.items.store.as_ref(),
            self.catalogs.items.stats_store.as_ref(),
            self.catalogs.scaling_stat_distribution_store.as_ref(),
            self.catalogs.scaling_stat_values_store.as_ref(),
            self.catalogs.shield_block_regular_game_table.as_ref(),
            self.catalogs.spell_catalogs.spell_shapeshift_form_store(),
        );
        self.inventory.record_represented_item_mods_with_access_like_cpp(
            &inventory_access,
            &modifier_access,
            catalogs,
            item_guid,
            slot,
            apply,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.auras.represented_shapeshift_form_like_cpp,
            cfg!(test),
        )
    }
    pub(in crate::session) fn represented_heirloom_item_set_bonus_over_level_cap_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_heirloom_item_set_bonus_over_level_cap_like_cpp(hub, item_guid)
    }
    pub(crate) fn record_represented_update_item_set_auras_like_cpp(
        &mut self,
        form_change: bool,
    ) -> usize {
        let events = self.plan_represented_update_item_set_auras_like_cpp(form_change);
        #[cfg(test)]
        self.inventory
            .record_represented_item_set_aura_refresh_events_for_test_like_cpp(&events);
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
                .catalogs
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
            if crate::session::hub_ref(self).player_has_visible_aura_spell_like_cpp(spell_id)
                != Some(false)
            {
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
        {
            let (s, mut h) = crate::session::split_inventory_mut(self);
            s.reset_represented_item_bonus_runtime_like_cpp(&mut h)
        };

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
        self.inventory
            .record_represented_item_set_aura_refresh_events_for_test_like_cpp(&events);
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
                    && crate::session::hub_ref(self)
                        .player_has_visible_aura_spell_like_cpp(spell_id)
                        != Some(false)
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
        let Some(update) = ({
            let (s, h) = crate::session::split_inventory_ref(self);
            s.represented_item_bonus_player_stat_update_object_like_cpp(h)
        }) else {
            return false;
        };
        self.send_packet(&update);
        true
    }
    pub(in crate::session) fn apply_represented_item_bonus_action_state_like_cpp(
        &mut self,
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_represented_item_bonus_action_state_like_cpp(&mut hub, action)
    }
    pub(in crate::session) fn initial_loaded_item_mods_can_apply_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.initial_loaded_item_mods_can_apply_like_cpp(hub, item_guid)
    }

    pub(in crate::session) fn represented_can_use_attack_type_like_cpp(
        &self,
        slot: u8,
        inventory_type: Option<InventoryType>,
    ) -> Option<bool> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_can_use_attack_type_like_cpp(hub, slot, inventory_type)
    }

    pub(crate) fn represented_item_reputation_rank_like_cpp(
        &self,
        required_reputation_faction: u32,
    ) -> Option<u32> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_item_reputation_rank_like_cpp(hub, required_reputation_faction)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/modifiers/f3_shims.rs"]
mod f3_shims;
