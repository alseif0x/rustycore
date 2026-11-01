// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory packet handlers for swap, equip and bag storage.

use tracing::{debug, warn};
use wow_constants::{InventoryResult, ItemFieldFlags};
use wow_entities::{
    INVENTORY_SLOT_BAG_0, NULL_BAG, NULL_SLOT, SwapItemPreflightResult, is_bank_pos,
    is_equipment_pos, is_inventory_pos,
};
use wow_packet::packets::item::{AutoEquipItem, AutoEquipItemSlot, SwapInvItem};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use super::ItemSessionHostLikeCpp;
use super::moves::{
    execute_inventory_equip_to_empty_like_cpp, execute_inventory_swap_positions_like_cpp,
};
use crate::{InventoryStorageQuestChecksLikeCpp, InventoryStorageTargetLikeCpp};

/// Handle CMSG_SWAP_INV_ITEM: drag-and-drop item between two inventory slots.
pub async fn handle_swap_inv_item_with_generator_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    swap: SwapInvItem,
) {
    if swap.inv_update.items.len() != 2 {
        warn!(
            "HandleSwapInvItemOpcode - Invalid itemCount ({})",
            swap.inv_update.items.len()
        );
        return;
    }

    if host.player_guid().is_none() || swap.src_slot == swap.dst_slot {
        return;
    }
    if !({
        let (state, hub) = host.split_inventory_ref();
        state.is_valid_inventory_pos_like_cpp(hub, INVENTORY_SLOT_BAG_0, swap.src_slot, true)
    }) {
        host.core()
            .send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
        return;
    }
    if !({
        let (state, hub) = host.split_inventory_ref();
        state.is_valid_inventory_pos_like_cpp(hub, INVENTORY_SLOT_BAG_0, swap.dst_slot, true)
    }) {
        host.core()
            .send_equip_error(InventoryResult::WrongSlot, None, None, 0, 0);
        return;
    }
    if (is_bank_pos(INVENTORY_SLOT_BAG_0, swap.src_slot)
        || is_bank_pos(INVENTORY_SLOT_BAG_0, swap.dst_slot))
        && !crate::can_use_current_bank_with_access_like_cpp(
            host.interaction(),
            &host.hub_ref().trainer_npc_interaction_access_like_cpp(),
            &host.core().owned_inventory_access_like_cpp(),
        )
    {
        return;
    }
    execute_inventory_swap_positions_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, swap.src_slot),
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, swap.dst_slot),
    )
    .await;
}

/// Handle CMSG_AUTO_EQUIP_ITEM: right-click to auto-equip/unequip an item.
pub async fn handle_auto_equip_item_with_generator_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    equip: AutoEquipItem,
) {
    if equip.inv_update.items.len() != 1 {
        warn!(
            "HandleAutoEquipItemOpcode - Invalid itemCount ({})",
            equip.inv_update.items.len()
        );
        return;
    }

    if host.player_guid().is_none() {
        return;
    }
    // Audited legacy 3.4.3 and current upstream TrinityCore intentionally
    // do not call WorldSession::CanUseBank in AutoEquipItem. The explicit
    // bank-interaction gates belong to HandleSwapInvItem/HandleSwapItem;
    // preserve the observable C++ handler contract here.
    let Some(source) = ({
        let (state, hub) = host.split_inventory_ref();
        state.get_inventory_item_by_pos(hub, equip.pack_slot, equip.slot)
    }) else {
        return;
    };
    let source_is_bag = host
        .hub_ref()
        .catalogs
        .item_storage_template(source.entry_id)
        .is_some_and(|template| template.container_slots > 0);
    let Some((result, destination)) = host.plan_equip_existing_inventory_item_like_cpp(
        equip.pack_slot,
        equip.slot,
        NULL_SLOT,
        !source_is_bag,
    ) else {
        return;
    };
    if result != InventoryResult::Ok {
        host.core()
            .send_equip_error(result, Some(source.guid), None, 0, 0);
        return;
    }
    let source_pos = wow_entities::make_item_pos(equip.pack_slot, equip.slot);
    if source_pos == destination {
        return;
    }

    // C++ HandleAutoEquipItemOpcode first tries to place the displaced
    // equipment item back at the source position, then in the source bag,
    // and finally anywhere. Player::SwapItem only covers the first case,
    // so preserve the two fallback searches before executing the equip.
    let [destination_bag, destination_slot] = destination.to_be_bytes();
    let Some(displaced) = ({
        let (state, hub) = host.split_inventory_ref();
        state.get_inventory_item_by_pos(hub, destination_bag, destination_slot)
    }) else {
        execute_inventory_swap_positions_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            source_pos,
            destination,
        )
        .await;
        return;
    };
    let displaced_is_child = ({
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, displaced.guid)
    })
    .is_some_and(|item| item.has_item_flag(ItemFieldFlags::CHILD));
    if displaced_is_child {
        execute_inventory_swap_positions_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            source_pos,
            destination,
        )
        .await;
        return;
    }

    let Some(preflight) = ({
        let (state, hub) = host.split_inventory_ref();
        state.plan_inventory_swap_preflight_like_cpp(hub, source_pos, destination)
    }) else {
        return;
    };
    match preflight.result {
        SwapItemPreflightResult::NoSource => return,
        SwapItemPreflightResult::ChildRedirect { .. } => {
            execute_inventory_swap_positions_like_cpp(
                host,
                item_guid_generator,
                creature_spawn_catalogs,
                source_pos,
                destination,
            )
            .await;
            return;
        }
        SwapItemPreflightResult::Error(result) => {
            host.core()
                .send_equip_error(result, Some(source.guid), Some(displaced.guid), 0, 0);
            return;
        }
        SwapItemPreflightResult::Continue => {}
    }

    let exact_displaced_result = host
        .validate_inventory_swap_target_like_cpp(
            destination_bag,
            destination_slot,
            equip.pack_slot,
            equip.slot,
            true,
            true,
        )
        .map_or(InventoryResult::CantSwap, |(result, _)| result);
    if exact_displaced_result == InventoryResult::Ok {
        execute_inventory_swap_positions_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            source_pos,
            destination,
        )
        .await;
        return;
    }

    let mut fallback_error = exact_displaced_result;
    let fallback = if is_inventory_pos(equip.pack_slot, equip.slot) {
        let mut fallback = None;
        for (bag, slot) in [(equip.pack_slot, NULL_SLOT), (NULL_BAG, NULL_SLOT)] {
            let Some((result, _, _)) = host.plan_store_existing_inventory_item_at_like_cpp(
                destination_bag,
                destination_slot,
                bag,
                slot,
                true,
            ) else {
                continue;
            };
            fallback_error = result;
            if result == InventoryResult::Ok {
                fallback = Some((bag, slot, InventoryStorageTargetLikeCpp::Inventory));
                break;
            }
        }
        fallback
    } else if is_bank_pos(equip.pack_slot, equip.slot) {
        let mut fallback = None;
        for (bag, slot) in [(equip.pack_slot, NULL_SLOT), (NULL_BAG, NULL_SLOT)] {
            let Some((result, _)) = host.plan_bank_existing_inventory_item_at_like_cpp(
                destination_bag,
                destination_slot,
                bag,
                slot,
                true,
            ) else {
                continue;
            };
            fallback_error = result;
            if result == InventoryResult::Ok {
                fallback = Some((bag, slot, InventoryStorageTargetLikeCpp::Bank));
                break;
            }
        }
        fallback
    } else {
        None
    };
    let Some((fallback_bag, fallback_slot, fallback_target)) = fallback else {
        host.core().send_equip_error(
            fallback_error,
            Some(displaced.guid),
            Some(source.guid),
            0,
            0,
        );
        return;
    };

    host.execute_inventory_storage_move_like_cpp(
        item_guid_generator,
        creature_spawn_catalogs,
        destination_bag,
        destination_slot,
        fallback_bag,
        fallback_slot,
        fallback_target,
        InventoryStorageQuestChecksLikeCpp::None,
        None,
    )
    .await;
    if ({
        let (state, hub) = host.split_inventory_ref();
        state.get_inventory_item_by_pos(hub, destination_bag, destination_slot)
    })
    .is_some()
    {
        return;
    }
    execute_inventory_equip_to_empty_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
        equip.pack_slot,
        equip.slot,
        destination,
    )
    .await;
}

/// Handle CMSG_AUTO_EQUIP_ITEM_SLOT.
///
/// C++ treats this as an explicit GUID + destination equipment-slot swap:
/// it requires exactly one `InvUpdate` source position, verifies that the
/// GUID still lives at that source position, rejects src==dst, then calls
/// `Player::SwapItem` with the packed source position.
pub async fn handle_auto_equip_item_slot_with_generator_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    equip: AutoEquipItemSlot,
) {
    if host.player_guid().is_none() {
        return;
    }

    if equip.inv_update.items.len() != 1
        || !is_equipment_pos(INVENTORY_SLOT_BAG_0, equip.item_dst_slot)
    {
        return;
    }

    let (container_slot, src_slot) = equip.inv_update.items[0];
    let Some((actual_bag, actual_slot, _item)) = ({
        let (state, hub) = host.split_inventory_ref();
        state.get_inventory_item_by_guid_like_cpp(hub, equip.item)
    }) else {
        return;
    };

    if actual_bag != container_slot || actual_slot != src_slot {
        return;
    }

    let source = wow_entities::make_item_pos(container_slot, src_slot);
    let destination = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, equip.item_dst_slot);
    if source == destination {
        return;
    }
    execute_inventory_swap_positions_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
        source,
        destination,
    )
    .await;
}

/// Handle CMSG_SWAP_ITEM: C++ container-aware swap between two positions.
/// C++ ref: `WorldSession::HandleSwapItem`
/// (`Handlers/ItemHandler.cpp:130-173`).
pub async fn handle_swap_item_with_generator_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    swap: wow_packet::packets::item::SwapItem,
) {
    if swap.inv_update.items.len() != 2 {
        warn!(
            "HandleSwapItem - Invalid itemCount ({})",
            swap.inv_update.items.len()
        );
        return;
    }

    if host.player_guid().is_none() {
        return;
    }
    let source = wow_entities::make_item_pos(swap.container_slot_a, swap.slot_a);
    let destination = wow_entities::make_item_pos(swap.container_slot_b, swap.slot_b);
    if source == destination {
        return;
    }
    if !({
        let (state, hub) = host.split_inventory_ref();
        state.is_valid_inventory_pos_like_cpp(hub, swap.container_slot_a, swap.slot_a, true)
    }) {
        host.core()
            .send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
        return;
    }
    if !({
        let (state, hub) = host.split_inventory_ref();
        state.is_valid_inventory_pos_like_cpp(hub, swap.container_slot_b, swap.slot_b, true)
    }) {
        host.core()
            .send_equip_error(InventoryResult::WrongSlot, None, None, 0, 0);
        return;
    }
    if (is_bank_pos(swap.container_slot_a, swap.slot_a)
        || is_bank_pos(swap.container_slot_b, swap.slot_b))
        && !crate::can_use_current_bank_with_access_like_cpp(
            host.interaction(),
            &host.hub_ref().trainer_npc_interaction_access_like_cpp(),
            &host.core().owned_inventory_access_like_cpp(),
        )
    {
        return;
    }
    execute_inventory_swap_positions_like_cpp(
        host,
        item_guid_generator,
        creature_spawn_catalogs,
        source,
        destination,
    )
    .await;
}

/// Handle CMSG_AUTO_STORE_BAG_ITEM: right-click to store item in bag/backpack.
///
/// C++ ref: `WorldSession::HandleAutoStoreBagItemOpcode`
/// (`Handlers/ItemHandler.cpp:699-743`).
pub async fn handle_auto_store_bag_item_with_generator_like_cpp<H: ItemSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    store: wow_packet::packets::item::AutoStoreBagItem,
) {
    let player_guid = match host.player_guid() {
        Some(g) => g,
        None => return,
    };

    if !store.inv_update.items.is_empty() {
        warn!(
            "HandleAutoStoreBagItemOpcode - Invalid itemCount ({})",
            store.inv_update.items.len()
        );
        return;
    }

    debug!(
        "AutoStoreBagItem: src container={} slot={} dst container={} for {:?}",
        store.container_slot_a, store.slot_a, store.container_slot_b, player_guid
    );

    // Audited legacy 3.4.3 and current upstream TrinityCore likewise do
    // not call WorldSession::CanUseBank in AutoStoreBagItem. Do not add a
    // Rust-only rejection that would diverge from the cited handler.
    let Some(source) = ({
        let (state, hub) = host.split_inventory_ref();
        state.get_inventory_item_by_pos(hub, store.container_slot_a, store.slot_a)
    }) else {
        return;
    };

    if !({
        let (state, hub) = host.split_inventory_ref();
        state.is_valid_inventory_pos_like_cpp(hub, store.container_slot_b, NULL_SLOT, false)
    }) {
        host.core()
            .send_equip_error(InventoryResult::WrongSlot, Some(source.guid), None, 0, 0);
        return;
    }

    let runtime_item = {
        let (state, hub) = host.split_inventory_ref();
        state.resolved_inventory_item_object_like_cpp(hub, source.guid)
    };
    let proto = host
        .hub_ref()
        .catalogs
        .item_storage_template(source.entry_id);
    let source_pos = wow_entities::make_item_pos(store.container_slot_a, store.slot_a);
    if is_equipment_pos(store.container_slot_a, store.slot_a)
        || wow_entities::is_bag_pos(source_pos)
    {
        let result = {
            let (state, hub) = host.split_inventory_ref();
            state.can_unequip_inventory_item_at_like_cpp(
                hub,
                store.container_slot_a,
                store.slot_a,
                !wow_entities::is_bag_pos(source_pos),
                runtime_item.as_ref(),
                proto.as_ref(),
                {
                    let (state, hub) = host.split_inventory_ref();
                    state.direct_item_contains_items(hub, source.guid)
                },
            )
        };
        if result != InventoryResult::Ok {
            host.core()
                .send_equip_error(result, Some(source.guid), None, 0, 0);
            return;
        }
    }

    host.execute_inventory_storage_move_like_cpp(
        item_guid_generator,
        creature_spawn_catalogs,
        store.container_slot_a,
        store.slot_a,
        store.container_slot_b,
        NULL_SLOT,
        InventoryStorageTargetLikeCpp::Inventory,
        InventoryStorageQuestChecksLikeCpp::None,
        None,
    )
    .await;
}
