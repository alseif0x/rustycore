// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;
use wow_core::ObjectGuid;
use wow_entities::INVENTORY_SLOT_BAG_END;

pub fn plan_item_set_aura_refresh_with_access_like_cpp(
    inventory: &wow_world_inventory::InventoryState,
    modifiers: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
    item_sets: &wow_world_core::session::OwnedItemSetAccessLikeCpp<'_>,
    spell_store: Option<&wow_data::SpellStore>,
    form_store: Option<&wow_data::SpellShapeshiftFormStore>,
    form_change: bool, consumer_test: bool,
    #[cfg(any(test, feature = "test-fixtures"))] fixture_form: &u32,
) -> Vec<wow_world_inventory::RepresentedItemSetAuraRefreshEventLikeCpp> {
        use wow_world_inventory::RepresentedItemSetAuraRefreshEventLikeCpp;
        let mut events = Vec::new();
        let primary_spec = item_sets.primary_specialization_id_like_cpp(consumer_test);
        let Some(active_effects) = inventory
            .player_item_modifier_runtime_snapshot_with_access_like_cpp(modifiers)
            .map(|state| state.item_set_effects_like_cpp().values().cloned().collect::<Vec<_>>())
        else { return events; };
        for effect in active_effects {
            let active_bonus_ids = effect.set_bonuses.clone();
            let spells: Vec<_> = item_sets.item_set_spells_like_cpp(effect.item_set_id)
                .into_iter().filter(|spell| active_bonus_ids.contains(&spell.id)).cloned().collect();
            for item_set_spell in spells {
                if item_set_spell.chr_spec_id != 0
                    && Some(u32::from(item_set_spell.chr_spec_id)) != primary_spec
                {
                    events.push(RepresentedItemSetAuraRefreshEventLikeCpp {
                        item_set_id: effect.item_set_id, spell_entry_id: item_set_spell.id,
                        spell_id: item_set_spell.spell_id, apply: false, form_change: false,
                    });
                    continue;
                }
                let fits_shapeshift = inventory.represented_equip_spell_fits_shapeshift_with_access_like_cpp(
                    modifiers, spell_store, form_store, item_set_spell.spell_id,
                    #[cfg(any(test, feature = "test-fixtures"))] fixture_form,
                );
                if !form_change || !fits_shapeshift {
                    events.push(RepresentedItemSetAuraRefreshEventLikeCpp {
                        item_set_id: effect.item_set_id, spell_entry_id: item_set_spell.id,
                        spell_id: item_set_spell.spell_id, apply: false, form_change,
                    });
                }
                if fits_shapeshift {
                    events.push(RepresentedItemSetAuraRefreshEventLikeCpp {
                        item_set_id: effect.item_set_id, spell_entry_id: item_set_spell.id,
                        spell_id: item_set_spell.spell_id, apply: true, form_change,
                    });
                }
            }
        }
        events
}

impl AuraRemovalCxLikeCpp<'_> {
    fn plan_item_set_aura_refresh_like_cpp(
        &self, form_change: bool,
    ) -> Vec<wow_world_inventory::RepresentedItemSetAuraRefreshEventLikeCpp> {
        plan_item_set_aura_refresh_with_access_like_cpp(
            self.inventory, &self.player.item_modifiers_access_like_cpp(), &self.item_sets,
            self.spell_store.map(|store| store.as_ref()), self.form_store,
            form_change, self.consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))] &*self.shapeshift_form,
        )
    }

    fn remove_auras_due_to_spell_like_cpp(&mut self, spell_id: i32) -> usize {
        let Some(visible_auras) = self.player.visible_auras_snapshot_like_cpp() else { return 0; };
        let slots = visible_auras.values()
            .filter_map(|aura| (aura.spell_id == spell_id).then_some(aura.slot))
            .collect::<Vec<_>>();
        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura_like_cpp(slot);
        }
        removed
    }

    pub fn apply_item_set_aura_refresh_events_like_cpp(&mut self, form_change: bool) -> usize {
        let events = self.plan_item_set_aura_refresh_like_cpp(form_change);
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.consumer_test {
            self.inventory.record_represented_item_set_aura_refresh_events_for_test_like_cpp(&events);
        }
        let recorded = events.len();
        let Some(player_guid) = self.player.player_guid_like_cpp() else { return recorded; };
        for event in events {
            let Ok(spell_id) = i32::try_from(event.spell_id) else { continue; };
            if event.apply {
                if event.form_change && self.player.player_has_visible_aura_spell_like_cpp(spell_id) != Some(false)
                { continue; }
                let effect_mask = self.spell_store.and_then(|store| store.get(spell_id))
                    .map(wow_world_spell::unit_owned_apply_aura_effect_mask_like_cpp)
                    .unwrap_or(0x0000_0001).max(0x0000_0001);
                let _ = self.apply_aura_with_effect_mask_like_cpp(
                    spell_id, player_guid, 0,
                    wow_world_core::session::AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                    effect_mask,
                );
            } else {
                let _ = self.remove_auras_due_to_spell_like_cpp(spell_id);
            }
        }
        recorded
    }

    fn equip_spell_fits_shapeshift_like_cpp(&self, spell_id: u32) -> bool {
        self.inventory.represented_equip_spell_fits_shapeshift_with_access_like_cpp(
            &self.player.item_modifiers_access_like_cpp(), self.spell_store.map(|store| store.as_ref()),
            self.form_store, spell_id,
            #[cfg(any(test, feature = "test-fixtures"))] &*self.shapeshift_form,
        )
    }

    pub fn refresh_item_effects_at_form_change_like_cpp(&mut self) -> usize {
        let Some(equipped) = self.inventory
            .resolved_inventory_item_objects_with_access_like_cpp(&self.player.inventory_access_like_cpp())
            .map(|items| {
                items.values()
                    .filter(|item| item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END)
                    .map(|item| (item.slot(), item.object().guid()))
                    .collect::<Vec<_>>()
            })
        else { return 0; };
        let item_guids: Vec<ObjectGuid> = equipped.iter().map(|(_, guid)| *guid).collect();
        let mut stale_slots: Vec<u8> = self.player.visible_auras_snapshot_like_cpp()
            .unwrap_or_default().values()
            .filter(|aura| item_guids.contains(&aura.caster_guid)
                && !self.equip_spell_fits_shapeshift_like_cpp(aura.spell_id as u32))
            .map(|aura| aura.slot).collect();
        stale_slots.sort_unstable();
        let mut changed = 0usize;
        for slot in stale_slots {
            if self.remove_aura_like_cpp(slot).is_ok() { changed += 1; }
        }
        let mut equipped = equipped;
        equipped.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        for (_slot, item_guid) in equipped {
            changed += self.apply_initial_item_equip_auras_like_cpp(item_guid);
        }
        self.apply_item_set_aura_refresh_events_like_cpp(true);
        changed
    }

    pub fn apply_initial_item_equip_auras_like_cpp(&mut self, item_guid: ObjectGuid) -> usize {
        let Some(_player_guid) = self.player.player_guid_like_cpp() else { return 0; };
        let Some(item_effect_store) = self.item_effects.cloned() else { return 0; };
        let Some(item_entry) = self.inventory
            .resolved_player_inventory_item_object_with_access_like_cpp(&self.player.inventory_access_like_cpp(), item_guid)
            .filter(|item| item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END)
            .map(|item| item.object().entry())
        else { return 0; };
        if !self.inventory.initial_loaded_item_mods_can_apply_with_access_like_cpp(
            &self.player.inventory_access_like_cpp(), &self.player.item_modifiers_access_like_cpp(),
            self.item_store, self.item_stats, item_guid,
        ) { return 0; }
        let primary_spec = self.item_sets.primary_specialization_id_like_cpp(self.consumer_test);
        let mut applied = 0usize;
        if self.item_stats.and_then(|store| store.sparse_template(item_entry))
            .is_some_and(|template| template.item_flags().contains(wow_constants::ItemFlags::LEGACY))
        { return 0; }
        let effects: Vec<_> = item_effect_store.item_effects_for_item_id_like_cpp(item_entry)
            .into_iter().cloned().collect();
        for effect in effects {
            if effect.trigger_type != 1 { continue; }
            if effect.spell_id <= 0 { continue; }
            if effect.chr_specialization_id != 0
                && Some(u32::from(effect.chr_specialization_id)) != primary_spec
            { continue; }
            if !self.equip_spell_fits_shapeshift_like_cpp(effect.spell_id as u32) { continue; }
            let Some(spell_info) = self.spell_store.and_then(|store| store.get(effect.spell_id)).cloned()
            else { continue; };
            let effect_mask = wow_world_spell::unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
            if effect_mask == 0 { continue; }
            if self.apply_aura_with_effect_mask_like_cpp(
                effect.spell_id, item_guid, 0,
                wow_world_core::session::AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                effect_mask,
            ).is_ok() { applied += 1; }
        }
        applied
    }
}
