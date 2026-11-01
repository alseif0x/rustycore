// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void storage: `CMSG_SWAP_VOID_ITEM` (#1363 slice 4c).

use super::*;

pub async fn handle_void_storage_swap_item<H: VoidStorageSessionHostLikeCpp>(
    host: &mut H,
    mut pkt: WorldPacket,
) {
    let Ok(swap) = SwapVoidItem::read(&mut pkt) else {
        return;
    };
    if host
        .hub_ref()
        .represented_npc_can_interact_with_like_cpp(swap.npc, NPCFlags1::VAULT_KEEPER.bits(), 0)
        .is_none()
        || !host.void_storage_is_unlocked_like_cpp()
        || represented_void_storage_loaded_like_cpp(host) != Some(true)
    {
        return;
    }

    let Some((old_slot, source_item)) =
        represented_void_storage_item_by_id_like_cpp(host, swap.void_item_guid.counter() as u64)
    else {
        return;
    };
    let new_slot = void_storage_swap_destination_slot_like_cpp(swap.dst_slot);
    let destination_item = represented_void_storage_item_at_like_cpp(host, new_slot);
    if old_slot == new_slot
        || usize::from(new_slot)
            >= wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP
    {
        send_void_storage_transfer_result_like_cpp(host, VoidTransferErrorLikeCpp::InternalError1);
        return;
    }

    let Some(player_guid) = host.player_guid() else {
        return;
    };
    let Some(port) =
        BattlePetSessionHostLikeCpp::lifecycle(host).void_storage_persistence_port_like_cpp()
    else {
        return;
    };
    let Some(money_persistence) = host
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
    else {
        return;
    };
    let Some(money) = host.resolved_player_money_like_cpp() else {
        return;
    };
    let request = wow_persistence::VoidStorageSwapWriteRequestLikeCpp {
        player_guid: player_guid.counter() as u64,
        money_before: money,
        money_after: money,
        old_slot,
        new_slot,
        source_item: void_storage_item_write_like_cpp(&source_item),
        destination_item: destination_item
            .as_ref()
            .map(void_storage_item_write_like_cpp),
    };
    let Some(money_persistence) = host
        .await_exclusive_player_money_transaction_outcome_like_cpp(
            money_persistence,
            port.persist_void_storage_swap_like_cpp(request),
            money,
            money,
            "void-storage slot swap",
        )
        .await
    else {
        send_void_storage_transfer_result_like_cpp(host, VoidTransferErrorLikeCpp::InternalError1);
        return;
    };
    let swapped = {
        let (s, mut h) = host.split_inventory_mut();
        s.swap_represented_void_storage_item_like_cpp(&mut h, old_slot, new_slot)
    };
    debug_assert!(swapped);
    drop(money_persistence);

    host.send_packet(&VoidItemSwapResponse {
        void_item_a: swap.void_item_guid,
        void_item_b: destination_item
            .as_ref()
            .map_or(wow_core::ObjectGuid::EMPTY, |item| {
                wow_core::ObjectGuid::create_item(host.core().realm_id(), item.item_id as i64)
            }),
        void_item_slot_a: u32::from(new_slot),
        void_item_slot_b: destination_item.as_ref().map_or(0, |_| u32::from(old_slot)),
    });
}
