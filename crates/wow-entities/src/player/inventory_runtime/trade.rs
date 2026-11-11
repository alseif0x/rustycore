use super::PlayerInventoryRuntime;
use crate::Item;
use crate::player::{INVENTORY_SLOT_BAG_0, PlayerInventoryStorage};
use wow_core::ObjectGuid;

impl PlayerInventoryRuntime {
    /// Resolve a trade item using the runtime slot/object maps, with native
    /// bag storage as the same fallback used by the canonical Player query.
    pub fn trade_item_position_like_cpp(
        &self,
        inventory: &PlayerInventoryStorage,
        item_guid: ObjectGuid,
    ) -> Option<(u8, u8)> {
        if let Some((&slot, _)) = self
            .inventory_items()
            .iter()
            .find(|(slot, record)| record.guid == item_guid && !crate::is_buyback_slot(**slot))
        {
            return Some((INVENTORY_SLOT_BAG_0, slot));
        }
        let item = self.item_objects().get(&item_guid)?;
        let container = item.container_guid();
        if container.is_empty() {
            return None;
        }
        let bag = self
            .inventory_items()
            .iter()
            .find(|(slot, record)| record.guid == container && !crate::is_buyback_slot(**slot))
            .map(|(&slot, _)| slot)
            .or_else(|| {
                inventory
                    .bags
                    .iter()
                    .enumerate()
                    .find_map(|(slot, storage)| {
                        storage
                            .as_ref()
                            .filter(|storage| storage.bag_guid == container)
                            .map(|_| slot as u8)
                    })
            })?;
        Some((bag, item.slot()))
    }

    /// Resolve a bag item from the runtime first, then the native registry.
    pub fn trade_bag_guid_like_cpp(
        &self,
        inventory: &PlayerInventoryStorage,
        bag: u8,
    ) -> Option<ObjectGuid> {
        self.inventory_items()
            .get(&bag)
            .map(|record| record.guid)
            .or_else(|| {
                inventory
                    .bags
                    .get(bag as usize)
                    .and_then(Option::as_ref)
                    .map(|storage| storage.bag_guid)
            })
    }

    /// Resolve an item at a storage position through the runtime objects.
    pub fn trade_item_at_like_cpp<'a>(
        &'a self,
        inventory: &PlayerInventoryStorage,
        bag: u8,
        slot: u8,
    ) -> Option<&'a Item> {
        if bag == INVENTORY_SLOT_BAG_0 {
            let guid = self.inventory_items().get(&slot)?.guid;
            return self.item_objects().get(&guid);
        }
        let bag_guid = self.trade_bag_guid_like_cpp(inventory, bag)?;
        self.item_objects()
            .values()
            .find(|item| item.container_guid() == bag_guid && item.slot() == slot)
    }
}
