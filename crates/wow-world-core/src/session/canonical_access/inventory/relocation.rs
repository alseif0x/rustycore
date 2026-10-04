// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::OwnedInventoryAccessLikeCpp;
use wow_entities::INVENTORY_SLOT_BAG_0;

impl OwnedInventoryAccessLikeCpp<'_> {
    /// Final native placement; earlier projection writes retain separate scopes.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_committed_inventory_relocation_native_placement_like_cpp(
        &self, source_bag: u8, source_slot: u8, destination_bag: u8,
        destination_slot: u8, item_guid: wow_core::ObjectGuid,
        moved_bag_size: Option<u8>, moved_bag_children: &[(u8, wow_core::ObjectGuid)],
    ) {
        let _ = self.core.mutate_canonical_player_like_cpp(|player| {
            if source_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.remove_top_level_item(source_slot);
            } else {
                let _ = player.remove_bag_item(source_bag, source_slot);
            }
            if destination_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.store_top_level_item(destination_slot, item_guid);
                if wow_entities::is_bag_pos(wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, destination_slot))
                    && let Some(bag_size) = moved_bag_size
                    && player
                        .register_bag_storage(destination_slot, item_guid, bag_size)
                        .is_ok()
                {
                    for &(child_slot, child_guid) in moved_bag_children {
                        let _ = player.store_bag_item(destination_slot, child_slot, child_guid);
                    }
                }
            } else {
                let _ = player.store_bag_item(destination_bag, destination_slot, item_guid);
            }
        });
    }
}
