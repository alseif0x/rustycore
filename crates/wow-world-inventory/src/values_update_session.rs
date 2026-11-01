// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The player values-update snapshot and the entity-bridge values
//! publication at the session boundary (#1363 slice 12b7), moved from the
//! World `session/publication/operations.rs`. Both read the inventory state
//! through the owned inventory and packet-publication accesses.

use wow_core::ObjectGuid;

use crate::BattlePetSessionHostLikeCpp;

pub fn player_values_update_snapshot<H: BattlePetSessionHostLikeCpp>(
    host: &H,
) -> Option<wow_world_core::session::InventoryPlayerProjectionLikeCpp> {
    let access = host.core().owned_inventory_access_like_cpp();
    let publication = host.core().packet_publication_access_like_cpp();
    host.split_inventory_ref()
        .0
        .player_values_update_snapshot_with_access_like_cpp(
            &access,
            &publication,
            host.hub_ref().catalogs.items.store.as_ref(),
            host.hub_ref().catalogs.items.stats_store.as_ref(),
        )
}

pub fn send_player_values_update_from_entity_bridge<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    inv_slot_changes: &[(u8, ObjectGuid)],
    visible_item_changes: &[(u8, i32, u16, u16)],
    virtual_item_changes: &[(u8, i32, u16, u16)],
    buyback_changes: &[(u8, u32, i64)],
    coinage: Option<u64>,
) -> bool {
    let access = host.core().owned_inventory_access_like_cpp();
    let publication = host.core().packet_publication_access_like_cpp();
    host.split_inventory_ref()
        .0
        .send_player_values_update_from_entity_bridge_with_access_like_cpp(
            &access,
            &publication,
            host.hub_ref().catalogs.items.store.as_ref(),
            host.hub_ref().catalogs.items.stats_store.as_ref(),
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
            buyback_changes,
            coinage,
        )
}
