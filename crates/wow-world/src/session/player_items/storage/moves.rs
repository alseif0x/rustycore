//! moves for the existing storage owner.

use super::*;

impl WorldSession {
    pub(crate) fn move_represented_direct_inventory_item_like_cpp(
        &mut self,
        src: u8,
        dst: u8,
    ) -> bool {
        if src == dst {
            return true;
        }

        let src_item = self.resolved_inventory_item_like_cpp(src);
        let dst_item = self.resolved_inventory_item_like_cpp(dst);
        let Some(src_item) = src_item else {
            return false;
        };

        self.insert_inventory_item_like_cpp(dst, src_item.clone());
        let player_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            src_item.guid,
            &[
                ItemObjectUpdateLikeCpp::SetContainedIn(player_guid),
                ItemObjectUpdateLikeCpp::SetSlot(dst),
            ],
        );

        if let Some(dst_item) = dst_item {
            self.insert_inventory_item_like_cpp(src, dst_item.clone());
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                dst_item.guid,
                &[
                    ItemObjectUpdateLikeCpp::SetContainedIn(player_guid),
                    ItemObjectUpdateLikeCpp::SetSlot(src),
                ],
            );
        } else {
            self.remove_inventory_item_like_cpp(src);
        }

        true
    }
    pub(crate) fn move_represented_direct_inventory_item_with_item_mods_like_cpp(
        &mut self,
        src: u8,
        dst: u8,
    ) -> Option<bool> {
        if src == dst {
            return Some(false);
        }

        let src_item = self.resolved_inventory_item_like_cpp(src)?;
        let dst_item = self.resolved_inventory_item_like_cpp(dst);
        let mut item_mods_changed = false;

        if src < INVENTORY_SLOT_BAG_END {
            let _ = self.record_direct_inventory_item_set_remove_like_cpp(
                INVENTORY_SLOT_BAG_0,
                src,
                src_item.guid,
            );
        }

        if src < INVENTORY_SLOT_BAG_END
            && self
                .resolved_inventory_item_object_like_cpp(src_item.guid)
                .is_some_and(|item| !item.is_broken())
        {
            self.record_represented_item_mods_like_cpp(src_item.guid, src, false);
            item_mods_changed = true;
        }

        if dst < INVENTORY_SLOT_BAG_END
            && let Some(dst_item) = dst_item.as_ref()
        {
            let _ = self.record_direct_inventory_item_set_remove_like_cpp(
                INVENTORY_SLOT_BAG_0,
                dst,
                dst_item.guid,
            );
        }

        if dst < INVENTORY_SLOT_BAG_END
            && dst_item.as_ref().is_some_and(|item| {
                self.resolved_inventory_item_object_like_cpp(item.guid)
                    .is_some_and(|item_object| !item_object.is_broken())
            })
        {
            let dst_item = dst_item.as_ref().expect("checked Some above");
            self.record_represented_item_mods_like_cpp(dst_item.guid, dst, false);
            item_mods_changed = true;
        }

        if !self.move_represented_direct_inventory_item_like_cpp(src, dst) {
            return None;
        }

        if dst < INVENTORY_SLOT_BAG_END {
            let _ = self.record_represented_items_set_item_like_cpp(src_item.guid, true);
        }

        if dst < INVENTORY_SLOT_BAG_END
            && self
                .resolved_inventory_item_object_like_cpp(src_item.guid)
                .is_some_and(|item| !item.is_broken())
        {
            self.record_represented_item_mods_like_cpp(src_item.guid, dst, true);
            item_mods_changed = true;
        }

        if src < INVENTORY_SLOT_BAG_END
            && let Some(dst_item) = dst_item.as_ref()
        {
            let _ = self.record_represented_items_set_item_like_cpp(dst_item.guid, true);
        }

        if src < INVENTORY_SLOT_BAG_END
            && dst_item.as_ref().is_some_and(|item| {
                self.resolved_inventory_item_object_like_cpp(item.guid)
                    .is_some_and(|item_object| !item_object.is_broken())
            })
        {
            let dst_item = dst_item.as_ref().expect("checked Some above");
            self.record_represented_item_mods_like_cpp(dst_item.guid, src, true);
            item_mods_changed = true;
        }

        Some(item_mods_changed)
    }
    pub(in crate::session) fn sync_canonical_direct_inventory_move_like_cpp(
        &mut self,
        src: u8,
        dst_bag: u8,
        dst_slot: u8,
        item_guid: ObjectGuid,
    ) {
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            let _ = player.remove_top_level_item(src);
            if dst_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.store_top_level_item(dst_slot, item_guid);
            } else {
                let _ = player.store_bag_item(dst_bag, dst_slot, item_guid);
            }
        });
    }
    pub(in crate::session) fn sync_canonical_direct_inventory_remove_like_cpp(&mut self, src: u8) {
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            let _ = player.remove_top_level_item(src);
        });
    }
    pub(crate) fn record_destroyed_inventory_item_mod_remove_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }
        if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| item.is_broken())
        {
            return false;
        }

        self.record_represented_item_mods_like_cpp(item_guid, slot, false) != 0
    }
    pub(crate) fn record_direct_inventory_item_set_remove_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }

        self.record_represented_items_set_item_like_cpp(item_guid, false)
    }
    pub(in crate::session) fn represented_item_inventory_type_like_cpp(
        &self,
        item_entry: u32,
        item_guid: ObjectGuid,
    ) -> Option<InventoryType> {
        self.resolved_inventory_items_like_cpp()?
            .values()
            .find(|item| item.guid == item_guid)
            .and_then(|item| item.inventory_type)
            .and_then(<InventoryType as num_traits::FromPrimitive>::from_u8)
            .or_else(|| {
                self.items
                    .store
                    .as_ref()
                    .and_then(|store| store.get(item_entry))
                    .and_then(|record| {
                        <InventoryType as num_traits::FromPrimitive>::from_i8(record.inventory_type)
                    })
            })
    }
}
