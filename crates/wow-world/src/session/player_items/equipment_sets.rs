//! Represented equipment sets and outfits.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
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
    /// Set `ItemChildEquipment.db2`, used by C++ `CanEquipChildItem` and
    /// `EquipChildItem` to move a linked child into its visible equipment slot.
    pub fn set_item_child_equipment_store(&mut self, store: Arc<ItemChildEquipmentStore>) {
        self.catalogs.items.child_equipment_store = Some(store);
    }
    pub fn set_spell_equipped_items_store(&mut self, store: Arc<SpellEquippedItemsStore>) {
        self.catalogs.spell_catalogs.spell_equipped_items_store = Some(store);
    }
}

impl
    wow_world_inventory::EquipmentSetUseHandlerHostLikeCpp<
        crate::session::SessionHandlerCatalogsLikeCpp,
    > for WorldSession
{
    fn equipment_set_use_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a crate::session::SessionHandlerCatalogsLikeCpp,
    ) -> wow_world_inventory::EquipmentSetUseContextLikeCpp<'a> {
        wow_world_inventory::build_equipment_set_use_context_like_cpp(self)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/equipment_sets/f3_shims.rs"]
mod f3_shims;
