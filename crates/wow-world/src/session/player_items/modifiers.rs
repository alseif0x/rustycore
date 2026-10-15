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

    pub(in crate::session) fn apply_initial_item_set_auras_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let events = wow_world_inventory::record_represented_items_set_item_events_like_cpp(
            self, item_guid, true,
        );
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
                wow_world_inventory::record_represented_item_mods_like_cpp(
                    self, item_guid, slot, true,
                );
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
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/modifiers/f3_shims.rs"]
mod f3_shims;
