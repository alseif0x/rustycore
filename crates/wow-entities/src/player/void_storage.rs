//! Canonical void-storage ownership and transitions.
//!
//! C++ keeps `_voidStorageItems` on `Player` and exposes the complete
//! transition surface through `Player::AddVoidStorageItem`,
//! `DeleteVoidStorageItem` and `SwapVoidStorageItem` (`Player.cpp:28025-28100`).
//! The session owns admission, item templates and packets; this module owns
//! the fixed-slot state and its invariants.

use super::*;

impl Player {
    fn ensure_void_storage_slots_like_cpp(&mut self) {
        if self.gameplay_state().void_storage_items.len() != PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP
        {
            self.gameplay_state_mut().void_storage_items =
                vec![None; PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP];
        }
    }

    /// C++ `_voidStorageItems`, normalized to the fixed slot count.
    #[must_use]
    pub fn void_storage_items_like_cpp(&self) -> &[Option<PlayerVoidStorageItemLikeCpp>] {
        &self.gameplay_state().void_storage_items
    }

    /// C++'s coherent void-storage load marker.
    #[must_use]
    pub fn void_storage_loaded_like_cpp(&self) -> bool {
        self.gameplay_state().void_storage_loaded
    }

    /// Clear the owner before `Player::_LoadVoidStorage` consumes a new result.
    pub fn clear_void_storage_like_cpp(&mut self) {
        self.ensure_void_storage_slots_like_cpp();
        self.gameplay_state_mut().void_storage_items.fill(None);
        self.gameplay_state_mut().void_storage_loaded = false;
    }

    /// Mark the void-storage query as consumed, including a valid empty result.
    pub fn mark_void_storage_loaded_like_cpp(&mut self) {
        self.ensure_void_storage_slots_like_cpp();
        self.gameplay_state_mut().void_storage_loaded = true;
    }

    /// Install one validated persisted item while preserving C++ duplicate and
    /// occupied-slot rejection. Item-template validation remains with Session.
    pub fn load_void_storage_item_like_cpp(
        &mut self,
        slot: u8,
        item: PlayerVoidStorageItemLikeCpp,
    ) -> bool {
        self.ensure_void_storage_slots_like_cpp();
        let slot = usize::from(slot);
        if item.item_id == 0 || slot >= PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP {
            return false;
        }
        let state = self.gameplay_state_mut();
        if state.void_storage_items[slot].is_some()
            || state
                .void_storage_items
                .iter()
                .flatten()
                .any(|loaded| loaded.item_id == item.item_id)
        {
            return false;
        }
        state.void_storage_items[slot] = Some(item);
        true
    }

    /// C++ `Player::GetNextVoidStorageFreeSlot`.
    #[must_use]
    pub fn next_void_storage_free_slot_like_cpp(&self) -> Option<u8> {
        self.void_storage_items_like_cpp()
            .iter()
            .position(Option::is_none)
            .and_then(|slot| u8::try_from(slot).ok())
    }

    /// C++ `Player::GetNumOfVoidStorageFreeSlots`.
    #[must_use]
    pub fn void_storage_free_slots_like_cpp(&self) -> usize {
        self.void_storage_items_like_cpp()
            .iter()
            .filter(|item| item.is_none())
            .count()
    }

    /// C++ `Player::GetVoidStorageItem(uint64, uint8&)` without a borrowed
    /// mutable item escaping the owner.
    #[must_use]
    pub fn void_storage_item_by_id_like_cpp(
        &self,
        item_id: u64,
    ) -> Option<(u8, PlayerVoidStorageItemLikeCpp)> {
        self.void_storage_items_like_cpp()
            .iter()
            .enumerate()
            .find_map(|(slot, item)| {
                let item = item.as_ref()?;
                (item.item_id == item_id).then(|| {
                    (
                        u8::try_from(slot).expect("void-storage slot fits u8"),
                        item.clone(),
                    )
                })
            })
    }

    /// C++ `Player::GetVoidStorageItem(uint8)` as an owned read result.
    #[must_use]
    pub fn void_storage_item_at_like_cpp(&self, slot: u8) -> Option<PlayerVoidStorageItemLikeCpp> {
        self.void_storage_items_like_cpp()
            .get(usize::from(slot))
            .cloned()
            .flatten()
    }

    /// C++ `Player::AddVoidStorageItem` returns the allocated slot.
    pub fn add_void_storage_item_like_cpp(
        &mut self,
        item: PlayerVoidStorageItemLikeCpp,
    ) -> Option<u8> {
        let slot = self.next_void_storage_free_slot_like_cpp()?;
        self.ensure_void_storage_slots_like_cpp();
        self.gameplay_state_mut().void_storage_items[usize::from(slot)] = Some(item);
        Some(slot)
    }

    /// C++ `Player::DeleteVoidStorageItem` returns the removed value for the
    /// application operation that persists the deletion.
    pub fn delete_void_storage_item_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<PlayerVoidStorageItemLikeCpp> {
        self.ensure_void_storage_slots_like_cpp();
        self.gameplay_state_mut()
            .void_storage_items
            .get_mut(usize::from(slot))
            .and_then(Option::take)
    }

    /// C++ `Player::SwapVoidStorageItem` rejects out-of-range and identical
    /// slots before swapping the two owner entries.
    pub fn swap_void_storage_item_like_cpp(&mut self, old_slot: u8, new_slot: u8) -> bool {
        let old_slot = usize::from(old_slot);
        let new_slot = usize::from(new_slot);
        if old_slot >= PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP
            || new_slot >= PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP
            || old_slot == new_slot
        {
            return false;
        }
        self.ensure_void_storage_slots_like_cpp();
        self.gameplay_state_mut()
            .void_storage_items
            .swap(old_slot, new_slot);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(item_id: u64) -> PlayerVoidStorageItemLikeCpp {
        PlayerVoidStorageItemLikeCpp {
            item_id,
            item_entry: 100,
            creator_guid: ObjectGuid::EMPTY,
            fixed_scaling_level: 0,
            random_properties_id: 0,
            random_properties_seed: 0,
            context: 0,
        }
    }

    #[test]
    fn owner_transitions_preserve_slots_and_reject_duplicate_loads() {
        let mut player = Player::new(Some(1), false);
        assert_eq!(player.void_storage_free_slots_like_cpp(), 160);
        assert!(!player.void_storage_loaded_like_cpp());
        player.mark_void_storage_loaded_like_cpp();
        assert!(player.void_storage_loaded_like_cpp());

        assert_eq!(player.add_void_storage_item_like_cpp(item(10)), Some(0));
        assert_eq!(player.add_void_storage_item_like_cpp(item(20)), Some(1));
        assert!(!player.load_void_storage_item_like_cpp(2, item(10)));
        assert!(!player.load_void_storage_item_like_cpp(1, item(30)));
        assert!(player.swap_void_storage_item_like_cpp(0, 1));
        assert_eq!(player.void_storage_item_at_like_cpp(0).unwrap().item_id, 20);
        assert_eq!(
            player.delete_void_storage_item_like_cpp(1).unwrap().item_id,
            10
        );
        assert_eq!(player.next_void_storage_free_slot_like_cpp(), Some(1));

        player.clear_void_storage_like_cpp();
        assert_eq!(player.void_storage_free_slots_like_cpp(), 160);
        assert!(!player.void_storage_loaded_like_cpp());
    }

    #[test]
    fn owner_bounds_match_fixed_cpp_slot_surface() {
        let mut player = Player::new(Some(1), false);
        assert!(!player.load_void_storage_item_like_cpp(160, item(1)));
        assert!(!player.swap_void_storage_item_like_cpp(0, 160));
        assert!(!player.swap_void_storage_item_like_cpp(2, 2));
        assert!(player.delete_void_storage_item_like_cpp(160).is_none());
    }
}
