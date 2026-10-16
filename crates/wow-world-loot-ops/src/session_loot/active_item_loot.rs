// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Whether a bag holds an item whose loot is still being viewed (#1363 slice
//! 12b7; moved from the World `session/player_items/storage_bags.rs`). The
//! inventory swap checks read it through the item host.

use super::*;

pub fn represented_bag_contains_active_item_loot_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    bag_guid: ObjectGuid,
) -> bool {
    if host.loot().active_loot_view_owners_is_empty_like_cpp() {
        return false;
    }

    ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_objects_like_cpp(hub)
    })
    .is_some_and(|items| {
        items.values().any(|item| {
            item.container_guid() == bag_guid
                && host
                    .loot()
                    .has_active_loot_view_owner_like_cpp(item.object().guid())
                && host
                    .loot()
                    .cached_loot_contains_owner_like_cpp(item.object().guid())
        })
    })
}
