//! equip spells for the existing equipment owner.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_equip_spell_fits_shapeshift_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let Some(spell_store) = self.spell_catalogs.spell_store.as_ref() else {
            return true;
        };
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };

        let Some(form_id) = self.represented_shapeshift_form_like_cpp() else {
            return false;
        };
        spell_store
            .check_shapeshift_like_cpp(spell_id, form_id, |form| {
                self.spell_catalogs
                    .spell_shapeshift_form_store
                    .as_ref()
                    .and_then(|store| store.get(form))
            })
            .unwrap_or(SpellCastResult::Success)
            == SpellCastResult::Success
    }
    /// C++ `Player::UpdateEquipSpellsAtFormChange` (`Player.cpp:22093-22094`,
    /// reached from `InitDataForForm`): drop the equipped items' spell auras the
    /// old form allowed and the new one rejects (`ApplyItemEquipSpell(item,
    /// false, true)`), re-apply every item's now-fitting equip spells
    /// (`ApplyItemEquipSpell(item, true, true)`) and replay the item-set auras
    /// under the new form. Returns the number of applied or removed effects.
    pub(crate) fn refresh_represented_item_effects_at_form_change_like_cpp(&mut self) -> usize {
        let Some(equipped) = self
            .resolved_inventory_item_objects_like_cpp()
            .map(|items| {
                items
                    .values()
                    .filter(|item| {
                        item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END
                    })
                    .map(|item| (item.slot(), item.object().guid()))
                    .collect::<Vec<_>>()
            })
        else {
            return 0;
        };
        let item_guids: Vec<ObjectGuid> = equipped.iter().map(|(_, guid)| *guid).collect();
        let mut stale_slots: Vec<u8> = self
            .resolved_player_visible_auras_like_cpp()
            .unwrap_or_default()
            .values()
            .filter(|aura| {
                item_guids.contains(&aura.caster_guid)
                    && !self.represented_equip_spell_fits_shapeshift_like_cpp(aura.spell_id as u32)
            })
            .map(|aura| aura.slot)
            .collect();
        stale_slots.sort_unstable();
        let mut changed = 0usize;
        for slot in stale_slots {
            if self.remove_aura(slot).is_ok() {
                changed += 1;
            }
        }
        let mut equipped = equipped;
        equipped.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        for (_slot, item_guid) in equipped {
            changed += self.apply_initial_item_equip_auras_like_cpp(item_guid);
        }
        self.apply_represented_item_set_aura_refresh_events_like_cpp(true);
        changed
    }
    pub(crate) fn apply_initial_equipped_item_equip_auras_like_cpp(&mut self) -> Option<usize> {
        let mut equipped: Vec<_> = self
            .resolved_inventory_item_objects_like_cpp()?
            .values()
            .filter(|item| item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END)
            .map(|item| (item.slot(), item.object().guid()))
            .collect();
        equipped.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        Some(
            equipped
                .into_iter()
                .map(|(_slot, item_guid)| self.apply_initial_item_equip_auras_like_cpp(item_guid))
                .sum(),
        )
    }
    pub(in crate::session) fn apply_initial_item_equip_auras_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> usize {
        let Some(_player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(item_effect_store) = self.items.effect_store.as_ref().cloned() else {
            return 0;
        };
        let Some(item_entry) = self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .filter(|item| item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END)
            .map(|item| item.object().entry())
        else {
            return 0;
        };
        if !self.initial_loaded_item_mods_can_apply_like_cpp(item_guid) {
            return 0;
        }

        let primary_spec = self.represented_primary_specialization_id_like_cpp();
        let mut applied = 0usize;
        if self
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_entry))
            .is_some_and(|template| template.item_flags().contains(ItemFlags::LEGACY))
        {
            return 0;
        }

        let effects: Vec<_> = item_effect_store
            .item_effects_for_item_id_like_cpp(item_entry)
            .into_iter()
            .cloned()
            .collect();
        for effect in effects {
            if effect.trigger_type != 1 {
                continue;
            }
            if effect.spell_id <= 0 {
                continue;
            }
            if effect.chr_specialization_id != 0
                && Some(u32::from(effect.chr_specialization_id)) != primary_spec
            {
                continue;
            }
            if !self.represented_equip_spell_fits_shapeshift_like_cpp(effect.spell_id as u32) {
                continue;
            }

            let Some(spell_info) = self
                .spell_store()
                .and_then(|store| store.get(effect.spell_id))
                .cloned()
            else {
                continue;
            };
            let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
            if effect_mask == 0 {
                continue;
            }
            if self
                .apply_aura_with_effect_mask_like_cpp(
                    effect.spell_id,
                    item_guid,
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
}
