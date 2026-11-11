use crate::player::{PlayerInventoryStorage, is_bag_storage_slot};
use wow_core::ObjectGuid;

mod limits;
mod slots;
mod store;

/// Read-only storage planning over one Player's borrowed inventory state.
pub struct ItemStorageView<'a> {
    owner_guid: ObjectGuid,
    inventory: &'a PlayerInventoryStorage,
    backpack_slots: u8,
}

impl<'a> ItemStorageView<'a> {
    /// Creates a view without copying or taking ownership of Player storage.
    pub fn new(
        owner_guid: ObjectGuid,
        inventory: &'a PlayerInventoryStorage,
        backpack_slots: u8,
    ) -> Self {
        Self {
            owner_guid,
            inventory,
            backpack_slots,
        }
    }

    fn get_bag_by_pos(&self, bag: u8) -> Option<ObjectGuid> {
        if is_bag_storage_slot(bag) {
            self.inventory.bag_at(bag).map(|bag| bag.bag_guid)
        } else {
            None
        }
    }
}
