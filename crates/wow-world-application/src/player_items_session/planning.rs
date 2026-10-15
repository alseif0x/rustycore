// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Storage, bank and equip planning and the swap-target validation over the
//! session's player-condition projection (#1363 slice 12b1b; moved verbatim
//! from the World `session/player_items/{persistence,equipment,bank}.rs` and
//! `handlers/character/items/host.rs`).

use wow_constants::InventoryResult;
use wow_entities::{Item, ItemPosCount, NULL_BAG, NULL_SLOT};
use wow_world_inventory::DirectInventoryStorageOverlayLikeCpp;
use wow_world_quest::QuestSessionHostLikeCpp;

use crate::InventorySwapTargetLikeCpp;

pub fn validate_inventory_swap_target_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    source_bag: u8,
    source_slot: u8,
    destination_bag: u8,
    destination_slot: u8,
    swap: bool,
    require_exact_destination: bool,
) -> Option<(InventoryResult, InventorySwapTargetLikeCpp)> {
    let conditions = host.player_condition_projection_cx_like_cpp();
    crate::InventoryMovePlanningCxLikeCpp::new(&conditions).validate_inventory_swap_target_like_cpp(
        source_bag,
        source_slot,
        destination_bag,
        destination_slot,
        swap,
        require_exact_destination,
    )
}

/// C++ `Player::CanEquipItem` for a represented runtime item.
pub fn plan_equip_existing_inventory_item_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    source_bag: u8,
    source_slot: u8,
    requested_slot: u8,
    swap: bool,
) -> Option<(InventoryResult, u16)> {
    let conditions = host.player_condition_projection_cx_like_cpp();
    crate::InventoryMovePlanningCxLikeCpp::new(&conditions)
        .plan_equip_existing_inventory_item_like_cpp(source_bag, source_slot, requested_slot, swap)
}

pub fn plan_bank_existing_inventory_item_at_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    source_bag: u8,
    source_slot: u8,
    destination_bag: u8,
    destination_slot: u8,
    swap: bool,
) -> Option<(InventoryResult, Vec<ItemPosCount>)> {
    let conditions = host.player_condition_projection_cx_like_cpp();
    crate::InventoryMovePlanningCxLikeCpp::new(&conditions)
        .plan_bank_existing_inventory_item_at_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            swap,
        )
}

pub fn plan_store_existing_inventory_item_at_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    source_bag: u8,
    source_slot: u8,
    destination_bag: u8,
    destination_slot: u8,
    swap: bool,
) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
    let conditions = host.player_condition_projection_cx_like_cpp();
    crate::InventoryMovePlanningCxLikeCpp::new(&conditions)
        .plan_store_existing_inventory_item_at_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            swap,
        )
}

pub fn plan_store_new_direct_inventory_item<H: QuestSessionHostLikeCpp>(
    host: &H,
    entry_id: u32,
    count: u32,
) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
    plan_store_new_direct_inventory_item_at(host, entry_id, count, NULL_BAG, NULL_SLOT)
}

pub fn plan_store_new_direct_inventory_item_at<H: QuestSessionHostLikeCpp>(
    host: &H,
    entry_id: u32,
    count: u32,
    bag: u8,
    slot: u8,
) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
    plan_store_direct_inventory_item_like_cpp(
        host,
        entry_id,
        count,
        bag,
        slot,
        None,
        false,
        &[],
        &[],
    )
}

pub fn plan_store_new_direct_inventory_item_with_overlays_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    entry_id: u32,
    count: u32,
    overlays: &[DirectInventoryStorageOverlayLikeCpp],
    vacated_positions: &[(u8, u8)],
) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
    plan_store_direct_inventory_item_like_cpp(
        host,
        entry_id,
        count,
        NULL_BAG,
        NULL_SLOT,
        None,
        false,
        overlays,
        vacated_positions,
    )
}

fn plan_store_direct_inventory_item_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    entry_id: u32,
    count: u32,
    bag: u8,
    slot: u8,
    source_item: Option<&Item>,
    swap: bool,
    overlays: &[DirectInventoryStorageOverlayLikeCpp],
    vacated_positions: &[(u8, u8)],
) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
    host.player_condition_projection_cx_like_cpp()
        .plan_store_direct_inventory_item_like_cpp(
            host.core()
                .inventory_valuation_access_like_cpp()
                .realm_id_like_cpp(),
            entry_id,
            count,
            bag,
            slot,
            source_item,
            swap,
            overlays,
            vacated_positions,
        )
}
