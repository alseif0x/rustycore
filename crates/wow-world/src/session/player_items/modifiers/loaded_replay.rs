//! loaded replay for the existing modifiers owner.

use super::*;

impl WorldSession {
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
        self.player_item_test_fixture_like_cpp
            .represented_item_set_aura_refresh_events_like_cpp
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
}
