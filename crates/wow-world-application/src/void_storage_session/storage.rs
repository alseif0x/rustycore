// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void storage: represented void-storage reads and writes (moved from the World `session/void_storage_adapter.rs`) (#1363 slice 4c).

use super::*;

pub fn represented_void_storage_free_slots_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
) -> Option<usize> {
    let (state, hub) = host.split_inventory_ref();
    state.represented_void_storage_free_slots_like_cpp(hub)
}

pub fn represented_void_storage_item_by_id_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
    item_id: u64,
) -> Option<(u8, RepresentedVoidStorageItemLikeCpp)> {
    let (state, hub) = host.split_inventory_ref();
    state.represented_void_storage_item_by_id_like_cpp(hub, item_id)
}

pub fn represented_void_storage_item_at_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
    slot: u8,
) -> Option<RepresentedVoidStorageItemLikeCpp> {
    let (state, hub) = host.split_inventory_ref();
    state.represented_void_storage_item_at_like_cpp(hub, slot)
}

pub fn add_represented_void_storage_item_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &mut H,
    item: RepresentedVoidStorageItemLikeCpp,
) -> Option<u8> {
    let (state, mut hub) = host.split_inventory_mut();
    state.add_represented_void_storage_item_like_cpp(&mut hub, item)
}

pub fn delete_represented_void_storage_item_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &mut H,
    slot: u8,
) -> Option<RepresentedVoidStorageItemLikeCpp> {
    let (state, mut hub) = host.split_inventory_mut();
    state.delete_represented_void_storage_item_like_cpp(&mut hub, slot)
}

pub(crate) fn next_represented_void_storage_item_id_with_generator_like_cpp<
    H: VoidStorageSessionHostLikeCpp,
>(
    host: &H,
    generator: &VoidStorageItemIdGeneratorLikeCpp,
) -> u64 {
    host.split_inventory_ref()
        .0
        .next_represented_void_storage_item_id_with_generator_like_cpp(generator)
}

pub(crate) fn represented_void_storage_contents_like_cpp<H: VoidStorageSessionHostLikeCpp>(
    host: &H,
) -> Option<wow_packet::packets::void_storage::VoidStorageContents> {
    let (state, hub) = host.split_inventory_ref();
    state.represented_void_storage_contents_like_cpp(hub)
}
