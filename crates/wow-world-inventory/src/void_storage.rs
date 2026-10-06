// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::VoidStorageItemIdGeneratorLikeCpp;
use wow_entities::PlayerVoidStorageItemLikeCpp as RepresentedVoidStorageItemLikeCpp;
use wow_world_core::session::{HubMut, HubRef};

impl crate::InventoryState {
    pub fn with_owned_void_storage_like_cpp<R>(
        &self,
        hub: HubRef<'_>,
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
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some(f(
                &self.represented_void_storage_items_like_cpp,
                self.represented_void_storage_loaded_like_cpp,
            ));
        }
        None
    }

    pub fn clear_represented_void_storage_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        if hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.clear_void_storage_like_cpp())
            .is_some()
        {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.represented_void_storage_items_like_cpp.fill(None);
            self.represented_void_storage_loaded_like_cpp = false;
        }
    }

    pub fn represented_void_storage_free_slots_like_cpp(&self, hub: HubRef<'_>) -> Option<usize> {
        self.with_owned_void_storage_like_cpp(hub, |items, _| {
            items.iter().filter(|item| item.is_none()).count()
        })
    }

    pub(crate) fn represented_void_storage_next_free_slot_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u8> {
        self.with_owned_void_storage_like_cpp(hub, |items, _| {
            items
                .iter()
                .position(Option::is_none)
                .and_then(|slot| u8::try_from(slot).ok())
        })?
    }

    pub fn represented_void_storage_item_by_id_like_cpp(
        &self,
        hub: HubRef<'_>,
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

    pub fn represented_void_storage_item_at_like_cpp(
        &self,
        hub: HubRef<'_>,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        self.with_owned_void_storage_like_cpp(hub, |items, _| {
            items.get(usize::from(slot)).cloned().flatten()
        })?
    }

    pub fn add_represented_void_storage_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item: RepresentedVoidStorageItemLikeCpp,
    ) -> Option<u8> {
        if let Some(slot) = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.add_void_storage_item_like_cpp(item.clone())
        }) {
            return slot;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            let slot = self.represented_void_storage_next_free_slot_like_cpp(hub.shared())?;
            self.represented_void_storage_items_like_cpp[usize::from(slot)] = Some(item);
            return Some(slot);
        }
        None
    }

    pub fn delete_represented_void_storage_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        if let Some(item) = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.delete_void_storage_item_like_cpp(slot))
        {
            return item;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return self
                .represented_void_storage_items_like_cpp
                .get_mut(usize::from(slot))
                .and_then(Option::take);
        }
        None
    }

    pub fn swap_represented_void_storage_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
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
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.represented_void_storage_items_like_cpp
                .swap(usize::from(old_slot), usize::from(new_slot));
            return true;
        }
        false
    }

    pub fn next_represented_void_storage_item_id_with_generator_like_cpp(
        &self,
        generator: &VoidStorageItemIdGeneratorLikeCpp,
    ) -> u64 {
        generator.generate()
    }

    pub fn represented_void_storage_contents_like_cpp(
        &self,
        hub: HubRef<'_>,
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
