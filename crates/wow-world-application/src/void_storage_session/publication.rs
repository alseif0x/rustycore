// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void storage: publication operations (#1363 slice 4c).

use super::*;

pub fn void_storage_withdrawal_container_db_guid_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
    bag: u8,
    planned_container_db_guids: &HashMap<u8, u64>,
) -> Option<u64> {
    let (state, hub) = host.split_inventory_ref();
    state.void_storage_withdrawal_container_db_guid_like_cpp(hub, bag, planned_container_db_guids)
}

pub fn void_storage_withdrawal_container_item_guid_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
    bag: u8,
    planned_container_item_guids: &HashMap<u8, wow_core::ObjectGuid>,
) -> Option<wow_core::ObjectGuid> {
    let (state, hub) = host.split_inventory_ref();
    state.void_storage_withdrawal_container_item_guid_like_cpp(
        hub,
        bag,
        planned_container_item_guids,
    )
}

pub fn new_void_withdrawal_item_publication_like_cpp(
    create_item: &wow_entities::Item,
    post_store_item: &wow_entities::Item,
    create_dynamic_flags: u32,
    container_slots: u32,
    map_id: u16,
) -> VoidWithdrawalItemPublicationLikeCpp {
    let post_store = wow_world_core::entity_update_bridge::item_values_update_to_update_object(
        post_store_item.object().guid(),
        map_id,
        &post_store_item.values_update(),
    );
    VoidWithdrawalItemPublicationLikeCpp::New {
        create: void_withdrawal_item_create_data_like_cpp(
            create_item,
            create_dynamic_flags,
            container_slots,
        ),
        post_store,
    }
}

pub fn merged_void_withdrawal_item_publications_like_cpp(
    item_guid: wow_core::ObjectGuid,
    store_stack_count: u32,
    store_dynamic_flags: Option<u32>,
    post_store_item: &wow_entities::Item,
    map_id: u16,
) -> Vec<VoidWithdrawalItemPublicationLikeCpp> {
    let store_update = if let Some(dynamic_flags) = store_dynamic_flags {
        UpdateObject::item_stack_count_and_flags_update(
            item_guid,
            map_id,
            store_stack_count,
            dynamic_flags,
        )
    } else {
        UpdateObject::item_stack_count_update(item_guid, map_id, store_stack_count)
    };
    let mut publications = vec![VoidWithdrawalItemPublicationLikeCpp::Values(store_update)];
    publications.extend(
        wow_world_core::entity_update_bridge::item_values_update_to_update_object(
            item_guid,
            map_id,
            &post_store_item.values_update(),
        )
        .map(VoidWithdrawalItemPublicationLikeCpp::Values),
    );
    publications
}

pub(crate) fn apply_void_withdrawal_post_store_state_like_cpp(
    item: &mut wow_entities::Item,
    creator_guid: wow_core::ObjectGuid,
    random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
) {
    item.set_creator(creator_guid);
    // C++ `StoreNewItem` calls `SetItemRandomProperties` after
    // `StoreItem` and applies it to the item returned by `_StoreItem`.
    // On a merge that return value is the destination stack, so the later
    // void item's random properties intentionally replace it.
    apply_effective_void_storage_random_properties_like_cpp(item, random_properties);
    item.set_binding(true);
}

pub(crate) fn send_void_storage_transfer_result_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
    result: VoidTransferErrorLikeCpp,
) {
    host.send_packet(&VoidTransferResult { result });
}

/// Publish the committed item-object lifecycle in the same order as C++
/// `HandleVoidStorageTransfer`: every deposit `DestroyItem(..., true)` and
/// its cleared inventory slot precede withdrawal `StoreNewItem(..., true)`
/// creates, including when a withdrawal reuses a deposited slot.
pub fn publish_void_storage_item_lifecycle_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
    map_id: u16,
    destroyed_deposit_items: Vec<((u8, u8), Vec<wow_core::ObjectGuid>)>,
    withdrawal_item_publications: Vec<VoidWithdrawalItemPublicationLikeCpp>,
) {
    for ((bag, slot), destroyed_guids) in destroyed_deposit_items {
        host.send_packet(&UpdateObject::destroy_objects(destroyed_guids, map_id));
        if bag == INVENTORY_SLOT_BAG_0 {
            let visible = (slot < 19)
                .then_some((slot, 0, 0, 0))
                .into_iter()
                .collect::<Vec<_>>();
            let virtual_item = ((15..=17).contains(&slot))
                .then_some((slot - 15, 0, 0, 0))
                .into_iter()
                .collect::<Vec<_>>();
            BattlePetPurchaseHostLikeCpp::send_player_values_update_from_entity_bridge(
                host,
                &[(slot, wow_core::ObjectGuid::EMPTY)],
                &visible,
                &virtual_item,
                &[],
                None,
            );
        } else {
            host.send_bag_slot_values_update_like_cpp(bag, slot);
        }
    }

    // C++ calls `StoreNewItem(..., true)` once per withdrawal. Preserve
    // that request order rather than batching creates: a first unit can
    // create a new object and a later unit can then publish a VALUES
    // update when it merges into that just-created stack.
    for publication in withdrawal_item_publications {
        match publication {
            VoidWithdrawalItemPublicationLikeCpp::New { create, post_store } => {
                host.send_packet(&UpdateObject::create_stored_items(vec![create], map_id));
                if let Some(post_store) = post_store {
                    host.send_packet(&post_store);
                }
            }
            VoidWithdrawalItemPublicationLikeCpp::Values(update) => {
                host.send_packet(&update);
            }
        }
    }
}
