//! Represented equipment sets and outfits.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

fn ignored_equipment_set_item_guid_like_cpp() -> ObjectGuid {
    ObjectGuid::new(0x0C00_0400_0000_0000_i64, -1_i64)
}

impl WorldSession {
    pub(crate) fn mark_represented_equipment_sets_loaded_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.mark_represented_equipment_sets_loaded_like_cpp(&mut hub)
    }
    pub(crate) fn load_represented_equipment_set_row_like_cpp(
        &mut self,
        guid: u64,
        set_id: u32,
        set_name: String,
        set_icon: String,
        ignore_mask: u32,
        assigned_spec_index: i32,
        pieces: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.load_represented_equipment_set_row_like_cpp(
            &mut hub,
            guid,
            set_id,
            set_name,
            set_icon,
            ignore_mask,
            assigned_spec_index,
            pieces,
        )
    }
    pub(crate) fn represented_load_equipment_set_packet_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::LoadEquipmentSet> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_load_equipment_set_packet_like_cpp(hub)
    }
    pub(crate) fn delete_represented_equipment_set_like_cpp(&mut self, id: u64) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.delete_represented_equipment_set_like_cpp(&mut hub, id)
    }
    pub(crate) fn use_represented_equipment_set_like_cpp(
        &mut self,
        request: &wow_packet::packets::misc::UseEquipmentSet,
    ) -> bool {
        let ignored_guid = ignored_equipment_set_item_guid_like_cpp();
        let mut changed_equipment = false;
        let mut represented_item_mods_changed = false;

        for (slot_index, set_item) in request.items.iter().enumerate() {
            let dst = slot_index as u8;
            if set_item.item == ignored_guid {
                continue;
            }

            if crate::session::hub_ref(self).resolved_in_combat_like_cpp() != Some(false)
                && dst != EQUIPMENT_SLOT_MAINHAND
                && dst != EQUIPMENT_SLOT_OFFHAND
            {
                continue;
            }

            if let Some((src, _item)) =
                self.represented_direct_inventory_slot_by_guid_like_cpp(set_item.item)
            {
                if src == dst {
                    continue;
                }
                if let Some(item_mods_changed) =
                    self.move_represented_direct_inventory_item_with_item_mods_like_cpp(src, dst)
                {
                    represented_item_mods_changed |= item_mods_changed;
                    changed_equipment |= dst < EQUIPMENT_SLOT_END;
                    changed_equipment |= src < EQUIPMENT_SLOT_END;
                }
                continue;
            }

            let Some(_equipped_item) = self.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, dst)
            else {
                continue;
            };
            let Some(backpack_slot) = ({
                let (s, h) = crate::session::split_inventory_ref(self);
                s.find_free_backpack_slot_like_cpp(h)
            }) else {
                continue;
            };
            if let Some(item_mods_changed) = self
                .move_represented_direct_inventory_item_with_item_mods_like_cpp(dst, backpack_slot)
            {
                represented_item_mods_changed |= item_mods_changed;
                changed_equipment = true;
            }
        }

        if changed_equipment {
            self.sync_player_registry_state_like_cpp();
        }

        represented_item_mods_changed
    }
    /// Set `ItemChildEquipment.db2`, used by C++ `CanEquipChildItem` and
    /// `EquipChildItem` to move a linked child into its visible equipment slot.
    pub fn set_item_child_equipment_store(&mut self, store: Arc<ItemChildEquipmentStore>) {
        self.catalogs.items.child_equipment_store = Some(store);
    }
    pub(crate) fn apply_initial_equipped_item_set_auras_like_cpp(&mut self) -> Option<usize> {
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
                .map(|(_slot, item_guid)| self.apply_initial_item_set_auras_like_cpp(item_guid))
                .sum(),
        )
    }
    #[cfg(test)]
    pub fn set_creature_equipment_store_like_cpp(
        &mut self,
        store: Arc<CreatureEquipmentStoreLikeCpp>,
    ) {
        self.catalogs.creature_equipment_store_like_cpp = Some(store);
    }
    pub fn set_spell_equipped_items_store(&mut self, store: Arc<SpellEquippedItemsStore>) {
        self.catalogs.spell_catalogs.spell_equipped_items_store = Some(store);
    }
}



#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/equipment_sets/f3_shims.rs"]
mod f3_shims;
