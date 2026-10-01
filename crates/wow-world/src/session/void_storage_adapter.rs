// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Void storage adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;
use super::{RepresentedVoidStorageItemLikeCpp, Rng, VoidStorageItemIdGeneratorLikeCpp};

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

impl crate::session::state::InventoryState {
    pub(in crate::session) fn with_owned_void_storage_like_cpp<R>(
        &self,
        hub: crate::session::HubRef<'_>,
        mut f: impl FnMut(&[Option<RepresentedVoidStorageItemLikeCpp>], bool) -> R,
    ) -> Option<R> {
        let canonical = hub.core.with_owned_player_like_cpp(|player| {
            f(
                player.void_storage_items_like_cpp(),
                player.void_storage_loaded_like_cpp(),
            )
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some(f(
                &self.represented_void_storage_items_like_cpp,
                self.represented_void_storage_loaded_like_cpp,
            ));
        }
        None
    }

    pub(crate) fn clear_represented_void_storage_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
    ) {
        if hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.clear_void_storage_like_cpp())
            .is_some()
        {
            return;
        }
        #[cfg(test)]
        if hub.core.player_handle_like_cpp.is_none() {
            self.represented_void_storage_items_like_cpp.fill(None);
            self.represented_void_storage_loaded_like_cpp = false;
        }
    }

    pub(crate) fn represented_void_storage_free_slots_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Option<usize> {
        self.with_owned_void_storage_like_cpp(hub, |items, _| {
            items.iter().filter(|item| item.is_none()).count()
        })
    }

    pub(crate) fn represented_void_storage_next_free_slot_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Option<u8> {
        self.with_owned_void_storage_like_cpp(hub, |items, _| {
            items
                .iter()
                .position(Option::is_none)
                .and_then(|slot| u8::try_from(slot).ok())
        })?
    }

    pub(crate) fn represented_void_storage_item_by_id_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        item_id: u64,
    ) -> Option<(u8, RepresentedVoidStorageItemLikeCpp)> {
        self.with_owned_void_storage_like_cpp(hub, |items, _| {
            items.iter().enumerate().find_map(|(slot, item)| {
                let item = item.as_ref()?;
                (item.item_id == item_id).then(|| {
                    (
                        u8::try_from(slot).expect("void-storage slot fits u8"),
                        item.clone(),
                    )
                })
            })
        })?
    }

    pub(crate) fn represented_void_storage_item_at_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        self.with_owned_void_storage_like_cpp(hub, |items, _| {
            items.get(usize::from(slot)).cloned().flatten()
        })?
    }

    pub(crate) fn add_represented_void_storage_item_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        item: RepresentedVoidStorageItemLikeCpp,
    ) -> Option<u8> {
        if let Some(slot) = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.add_void_storage_item_like_cpp(item.clone())
        }) {
            return slot;
        }
        #[cfg(test)]
        if hub.core.player_handle_like_cpp.is_none() {
            let slot = self.represented_void_storage_next_free_slot_like_cpp(hub.shared())?;
            self.represented_void_storage_items_like_cpp[usize::from(slot)] = Some(item);
            return Some(slot);
        }
        None
    }

    pub(crate) fn delete_represented_void_storage_item_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        if let Some(item) = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.delete_void_storage_item_like_cpp(slot))
        {
            return item;
        }
        #[cfg(test)]
        if hub.core.player_handle_like_cpp.is_none() {
            return self
                .represented_void_storage_items_like_cpp
                .get_mut(usize::from(slot))
                .and_then(Option::take);
        }
        None
    }

    pub(crate) fn swap_represented_void_storage_item_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        old_slot: u8,
        new_slot: u8,
    ) -> bool {
        if usize::from(old_slot)
            >= wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP
            || usize::from(new_slot)
                >= wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP
            || old_slot == new_slot
        {
            return false;
        }
        if let Some(swapped) = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.swap_void_storage_item_like_cpp(old_slot, new_slot)
        }) {
            return swapped;
        }
        #[cfg(test)]
        if hub.core.player_handle_like_cpp.is_none() {
            self.represented_void_storage_items_like_cpp
                .swap(usize::from(old_slot), usize::from(new_slot));
            return true;
        }
        false
    }

    pub(crate) fn next_represented_void_storage_item_id_with_generator_like_cpp(
        &self,
        generator: &VoidStorageItemIdGeneratorLikeCpp,
    ) -> u64 {
        generator.generate()
    }

    pub(crate) fn represented_void_storage_contents_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Option<wow_packet::packets::void_storage::VoidStorageContents> {
        self.with_owned_void_storage_like_cpp(hub, |stored, _| {
            let items = stored
                .iter()
                .enumerate()
                .filter_map(|(slot, item)| {
                    let item = item.as_ref()?;
                    Some(
                        hub.core
                            .represented_void_storage_item_packet_like_cpp(slot as u8, item),
                    )
                })
                .collect();
            wow_packet::packets::void_storage::VoidStorageContents { items }
        })
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/void_storage_adapter/f3_shims.rs"]
mod f3_shims;
