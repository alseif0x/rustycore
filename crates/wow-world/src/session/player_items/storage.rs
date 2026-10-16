//! Represented inventory storage: the complete store, remove and swap operations over the canonical Player item slots.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn can_use_inventory_item_represented_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
    ) -> InventoryResult {
        wow_world_player_conditions::can_use_inventory_item_represented_with_loading_like_cpp(
            &self.player_condition_projection_cx_like_cpp(),
            item,
            runtime_item,
            true,
        )
    }
}

impl crate::session::InventoryCxRef<'_> {
    pub(crate) fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item {
        let max_durability = self
            .hub
            .catalogs
            .item_template_max_durability(entry_id)
            .max(durability);
        wow_world_inventory::make_inventory_item_object_like_cpp(
            item_guid,
            entry_id,
            owner_guid,
            count,
            durability,
            max_durability,
            context,
            slot,
            self.lifecycle.total_played_time_like_cpp(),
        )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/storage/f3_shims.rs"]
mod f3_shims;
