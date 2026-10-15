// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for `CMSG_ADD_TOY`, moved to `wow_world_inventory`
//! (#1363 slice 4d): the item-use admission read stays on the session.

use wow_constants::InventoryResult;
use wow_entities::{Item, PlayerInventoryItem as InventoryItem};

use crate::session::WorldSession;

impl wow_world_inventory::AddToyHostLikeCpp for WorldSession {
    fn can_use_inventory_item_represented_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
    ) -> InventoryResult {
        WorldSession::can_use_inventory_item_represented_like_cpp(self, item, runtime_item)
    }
}
