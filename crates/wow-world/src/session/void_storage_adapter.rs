// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void storage adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;
use super::{RepresentedVoidStorageItemLikeCpp, VoidStorageItemIdGeneratorLikeCpp};

impl WorldSession {
    pub(crate) fn clear_represented_void_storage_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.clear_represented_void_storage_like_cpp(&mut hub)
    }

    pub(crate) fn represented_void_storage_free_slots_like_cpp(&self) -> Option<usize> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_void_storage_free_slots_like_cpp(hub)
    }

    pub(crate) fn represented_void_storage_item_by_id_like_cpp(
        &self,
        item_id: u64,
    ) -> Option<(u8, RepresentedVoidStorageItemLikeCpp)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_void_storage_item_by_id_like_cpp(hub, item_id)
    }

    pub(crate) fn represented_void_storage_item_at_like_cpp(
        &self,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_void_storage_item_at_like_cpp(hub, slot)
    }

    pub(crate) fn add_represented_void_storage_item_like_cpp(
        &mut self,
        item: RepresentedVoidStorageItemLikeCpp,
    ) -> Option<u8> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_represented_void_storage_item_like_cpp(&mut hub, item)
    }

    pub(crate) fn delete_represented_void_storage_item_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.delete_represented_void_storage_item_like_cpp(&mut hub, slot)
    }

    pub(crate) fn next_represented_void_storage_item_id_with_generator_like_cpp(
        &self,
        generator: &VoidStorageItemIdGeneratorLikeCpp,
    ) -> u64 {
        self.inventory
            .next_represented_void_storage_item_id_with_generator_like_cpp(generator)
    }

    #[cfg(test)]
    pub(crate) fn next_represented_void_storage_item_id_like_cpp(&self) -> Option<u64> {
        self.core
            .void_storage_item_id_generator_like_cpp
            .as_deref()
            .map(|generator| {
                self.inventory
                    .next_represented_void_storage_item_id_with_generator_like_cpp(generator)
            })
    }

    pub(crate) fn represented_void_storage_contents_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::void_storage::VoidStorageContents> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_void_storage_contents_like_cpp(hub)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/void_storage_adapter/f3_shims.rs"]
mod f3_shims;
