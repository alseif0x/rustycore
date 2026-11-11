use crate::MAX_BAG_SIZE;
use crate::player::{
    PlayerBagStorage, PlayerInventoryStorage, PlayerStorageError, is_bag_storage_slot,
};
use wow_core::ObjectGuid;

impl PlayerInventoryStorage {
    pub fn register_bag_storage(
        &mut self,
        bag_slot: u8,
        bag_guid: ObjectGuid,
        bag_size: u8,
    ) -> Result<(), PlayerStorageError> {
        if !is_bag_storage_slot(bag_slot) {
            return Err(PlayerStorageError::InvalidBagSlot(bag_slot));
        }
        if bag_size as usize > MAX_BAG_SIZE {
            return Err(PlayerStorageError::InvalidBagItemSlot(bag_size));
        }

        self.bags[bag_slot as usize] = Some(PlayerBagStorage::boxed(bag_guid, bag_size));
        Ok(())
    }

    pub fn store_bag_item(
        &mut self,
        bag: u8,
        slot: u8,
        guid: ObjectGuid,
    ) -> Result<(), PlayerStorageError> {
        let bag_storage = self
            .bags
            .get_mut(bag as usize)
            .and_then(Option::as_mut)
            .ok_or(PlayerStorageError::UnknownBag(bag))?;
        if slot as usize >= MAX_BAG_SIZE || slot >= bag_storage.bag_size {
            return Err(PlayerStorageError::InvalidBagItemSlot(slot));
        }

        bag_storage.set_item(slot, Some(guid));
        Ok(())
    }

    pub fn remove_bag_item(
        &mut self,
        bag: u8,
        slot: u8,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let bag_storage = self
            .bags
            .get_mut(bag as usize)
            .and_then(Option::as_mut)
            .ok_or(PlayerStorageError::UnknownBag(bag))?;
        if slot as usize >= MAX_BAG_SIZE || slot >= bag_storage.bag_size {
            return Err(PlayerStorageError::InvalidBagItemSlot(slot));
        }

        let removed = bag_storage.item_by_pos(slot);
        bag_storage.set_item(slot, None);
        Ok(removed)
    }
}
