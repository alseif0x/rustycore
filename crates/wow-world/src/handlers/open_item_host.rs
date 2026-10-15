// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the item-container bodies (`CMSG_OPEN_ITEM`, the
//! wrapped gift and the item loot template) that moved to `wow_world_loot_ops`
//! (#1363 slice 4c). Each method forwards to the same-named inherent
//! `WorldSession` operation; the wrapped-gift row application keeps its
//! inventory split here because the scenarios also call it.

use wow_core::ObjectGuid;
use wow_entities::PlayerInventoryItem as InventoryItem;
use wow_world_loot_ops::OpenItemHostLikeCpp;

use crate::session::WorldSession;

impl OpenItemHostLikeCpp for WorldSession {
    fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<InventoryItem> {
        WorldSession::get_inventory_item_by_pos(self, bag, slot)
    }
    fn apply_wrapped_gift_row_to_runtime_item_like_cpp(
        &mut self,
        bag: u8,
        item_guid: ObjectGuid,
        slot: u8,
        entry: u32,
        flags: u32,
    ) -> Option<u32> {
        WorldSession::apply_wrapped_gift_row_to_runtime_item_like_cpp(
            self, bag, item_guid, slot, entry, flags,
        )
    }
    fn remove_fully_looted_runtime_item(&mut self, bag: u8, slot: u8, item_guid: ObjectGuid) {
        WorldSession::remove_fully_looted_runtime_item(self, bag, slot, item_guid)
    }
    fn send_stat_update(&mut self) -> bool {
        WorldSession::send_stat_update(self)
    }
}

impl WorldSession {
    pub(crate) fn apply_wrapped_gift_row_to_runtime_item_like_cpp(
        &mut self,
        bag: u8,
        item_guid: ObjectGuid,
        slot: u8,
        entry: u32,
        flags: u32,
    ) -> Option<u32> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_wrapped_gift_row_to_runtime_item_like_cpp(
            &mut hub, bag, item_guid, slot, entry, flags,
        )
    }
}
