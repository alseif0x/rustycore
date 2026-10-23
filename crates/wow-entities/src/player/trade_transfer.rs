//! Canonical Player item transitions of a completed trade (#38).
//!
//! C++ `HandleAcceptTradeOpcode` takes every traded item out with
//! `Player::MoveItemFromInventory` (`PlayerStorage`, removing it from the
//! slot, its durations and the soulbound-tradeable list) and stores it in the
//! other Player with `MoveItemToInventory` (`TradeHandler.cpp:110-190`). The
//! inventory runtime (top-level records plus item objects) is the position
//! authority, as the live inventory handlers read it; the native slot arrays
//! are kept in step only where they are populated.

use super::{INVENTORY_SLOT_BAG_0, Player, PlayerInventoryItem};
use crate::Item;
use wow_core::ObjectGuid;

/// One item taken out of a Player for a trade.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeTakenItemLikeCpp {
    pub item: Item,
    pub record: Option<PlayerInventoryItem>,
    pub bag: u8,
    pub slot: u8,
}

impl Player {
    /// Where an item sits, resolved like `Player::GetItemByGuid` over the
    /// inventory runtime (the position authority in production): a top-level
    /// record (never buyback) or an object contained in a top-level bag.
    pub fn trade_item_position_like_cpp(&self, item_guid: ObjectGuid) -> Option<(u8, u8)> {
        let runtime = self.inventory_runtime_like_cpp();
        if let Some((&slot, _)) = runtime
            .inventory_items()
            .iter()
            .find(|(slot, record)| record.guid == item_guid && !crate::is_buyback_slot(**slot))
        {
            return Some((INVENTORY_SLOT_BAG_0, slot));
        }
        let item = runtime.item_objects().get(&item_guid)?;
        let container = item.container_guid();
        if container.is_empty() {
            return None;
        }
        let bag = runtime
            .inventory_items()
            .iter()
            .find(|(slot, record)| record.guid == container && !crate::is_buyback_slot(**slot))
            .map(|(&slot, _)| slot)
            .or_else(|| {
                self.inventory()
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

    /// The bag item at a bag slot: the runtime record (production), else the
    /// native bag storage.
    pub fn trade_bag_guid_like_cpp(&self, bag: u8) -> Option<ObjectGuid> {
        self.inventory_runtime_like_cpp()
            .inventory_items()
            .get(&bag)
            .map(|record| record.guid)
            .or_else(|| {
                self.inventory()
                    .bags
                    .get(bag as usize)
                    .and_then(Option::as_ref)
                    .map(|storage| storage.bag_guid)
            })
    }

    /// C++ `Player::MoveItemFromInventory(bag, slot, update)`: the item leaves
    /// its position and the Player's item objects.
    pub fn trade_take_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> Option<TradeTakenItemLikeCpp> {
        let (bag, slot) = self.trade_item_position_like_cpp(item_guid)?;
        let record = if bag == INVENTORY_SLOT_BAG_0 {
            if self.inventory().items.get(slot as usize).copied().flatten() == Some(item_guid) {
                let _ = self.remove_top_level_item(slot);
            }
            self.inventory_runtime_mut_like_cpp()
                .remove_item_from_slot_like_cpp(slot)
        } else {
            let native = self
                .inventory()
                .bags
                .get(bag as usize)
                .and_then(Option::as_ref)
                .and_then(|storage| storage.slots.get(slot as usize).copied().flatten());
            if native == Some(item_guid) {
                let _ = self.remove_bag_item(bag, slot);
            }
            None
        };
        let mut item = self
            .inventory_runtime_mut_like_cpp()
            .remove_item_object_like_cpp(item_guid)?;
        let _ = self.remove_item_durations(&item);
        let _ = self.remove_enchantment_durations(&mut item);
        self.remove_tradeable_item(&item);
        Some(TradeTakenItemLikeCpp {
            item,
            record,
            bag,
            slot,
        })
    }

    /// C++ `Player::MoveItemToInventory` storing a whole item at one empty
    /// position. `bag == INVENTORY_SLOT_BAG_0` stores at the top level;
    /// otherwise in that bag. Owner and container fields follow the new
    /// position.
    pub fn trade_store_item_like_cpp(
        &mut self,
        mut item: Item,
        db_guid: u64,
        inventory_type: Option<u8>,
        bag: u8,
        slot: u8,
    ) -> bool {
        if self.trade_item_at_like_cpp(bag, slot).is_some() {
            return false;
        }
        let owner = self.guid();
        let item_guid = item.object().guid();
        item.set_owner_guid(owner);
        if bag == INVENTORY_SLOT_BAG_0 {
            if self.store_top_level_item(slot, item_guid).is_err() {
                return false;
            }
            item.set_container_guid(ObjectGuid::EMPTY);
            item.set_contained_in(owner);
            item.set_slot(slot);
            let entry_id = item.object().entry();
            let _ = self
                .inventory_runtime_mut_like_cpp()
                .store_item_in_slot_like_cpp(
                    slot,
                    PlayerInventoryItem {
                        guid: item_guid,
                        entry_id,
                        db_guid,
                        inventory_type,
                    },
                );
        } else {
            let Some(bag_guid) = self.trade_bag_guid_like_cpp(bag) else {
                return false;
            };
            let has_native_bag = self
                .inventory()
                .bags
                .get(bag as usize)
                .is_some_and(Option::is_some);
            if has_native_bag && self.store_bag_item(bag, slot, item_guid).is_err() {
                return false;
            }
            item.set_container_guid_and_slot(bag_guid, bag);
            item.set_contained_in(bag_guid);
            item.set_slot(slot);
        }
        let _ = self.add_item_durations(&item);
        let _ = self.add_enchantment_durations(&mut item);
        if item.is_bop_tradeable() {
            self.add_tradeable_item(&item);
        }
        let _ = self
            .inventory_runtime_mut_like_cpp()
            .store_item_object_like_cpp(item);
        true
    }

    /// C++ `Player::_StoreItem` merging into an existing stack.
    pub fn trade_set_stack_count_like_cpp(&mut self, item_guid: ObjectGuid, count: u32) -> bool {
        let Some(item) = self
            .inventory_runtime_mut_like_cpp()
            .item_objects_mut()
            .get_mut(&item_guid)
        else {
            return false;
        };
        item.set_count(count);
        true
    }

    /// The item at a storage position, as `Player::GetItemByPos` resolves it
    /// over the inventory runtime.
    pub fn trade_item_at_like_cpp(&self, bag: u8, slot: u8) -> Option<&Item> {
        let runtime = self.inventory_runtime_like_cpp();
        if bag == INVENTORY_SLOT_BAG_0 {
            let guid = runtime.inventory_items().get(&slot)?.guid;
            return runtime.item_objects().get(&guid);
        }
        let bag_guid = self.trade_bag_guid_like_cpp(bag)?;
        runtime
            .item_objects()
            .values()
            .find(|item| item.container_guid() == bag_guid && item.slot() == slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player_with(guid: ObjectGuid) -> Player {
        let mut player = Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player
    }

    fn item(guid: ObjectGuid, entry: u32, count: u32) -> Item {
        let mut item = Item::default();
        item.object_mut().create(guid);
        item.object_mut().set_entry(entry);
        item.set_count(count);
        item
    }

    #[test]
    fn traded_item_leaves_one_player_and_lands_in_the_other_like_cpp() {
        let giver_guid = ObjectGuid::create_player(1, 1);
        let receiver_guid = ObjectGuid::create_player(1, 2);
        let item_guid = ObjectGuid::create_item(1, 50);
        let mut giver = player_with(giver_guid);
        let mut receiver = player_with(receiver_guid);
        assert!(giver.trade_store_item_like_cpp(item(item_guid, 25, 3), 50, None, 255, 23));
        assert_eq!(giver.trade_item_at_like_cpp(255, 23).unwrap().count(), 3);

        let taken = giver.trade_take_item_like_cpp(item_guid).expect("taken");
        assert_eq!((taken.bag, taken.slot), (255, 23));
        assert_eq!(taken.record.unwrap().db_guid, 50);
        assert!(giver.trade_item_at_like_cpp(255, 23).is_none());
        assert!(giver.inventory_runtime_like_cpp().item_objects().is_empty());
        assert!(giver.trade_take_item_like_cpp(item_guid).is_none());

        assert!(receiver.trade_store_item_like_cpp(taken.item, 50, None, 255, 24));
        let stored = receiver.trade_item_at_like_cpp(255, 24).unwrap();
        assert_eq!(stored.data().owner, receiver_guid);
        assert_eq!(stored.data().contained_in, receiver_guid);
        assert!(receiver.trade_set_stack_count_like_cpp(item_guid, 5));
        assert_eq!(receiver.trade_item_at_like_cpp(255, 24).unwrap().count(), 5);
    }

    #[test]
    fn traded_item_stores_inside_a_bag_like_cpp() {
        let receiver_guid = ObjectGuid::create_player(1, 2);
        let bag_guid = ObjectGuid::create_item(1, 9);
        let mut receiver = player_with(receiver_guid);
        receiver.register_bag_storage(30, bag_guid, 4).unwrap();
        let item_guid = ObjectGuid::create_item(1, 51);
        assert!(receiver.trade_store_item_like_cpp(item(item_guid, 25, 1), 51, None, 30, 2));
        let stored = receiver.trade_item_at_like_cpp(30, 2).unwrap();
        assert_eq!(stored.container_guid(), bag_guid);
        assert_eq!(stored.bag_slot(), 30);
        assert_eq!(stored.data().contained_in, bag_guid);
        assert!(!receiver.trade_store_item_like_cpp(item(item_guid, 25, 1), 51, None, 31, 0));
        let taken = receiver.trade_take_item_like_cpp(item_guid).unwrap();
        assert_eq!((taken.bag, taken.slot, taken.record), (30, 2, None));
    }

    /// Production logins fill only the inventory runtime (records plus
    /// objects); the native slot arrays stay empty. Trade must resolve, take
    /// and store items through that runtime, as the live handlers do.
    #[test]
    fn runtime_only_inventory_resolves_like_production_like_cpp() {
        let giver_guid = ObjectGuid::create_player(1, 1);
        let item_guid = ObjectGuid::create_item(1, 60);
        let bag_guid = ObjectGuid::create_item(1, 61);
        let nested_guid = ObjectGuid::create_item(1, 62);
        let mut giver = player_with(giver_guid);
        let runtime = giver.inventory_runtime_mut_like_cpp();
        runtime.store_item_in_slot_like_cpp(
            35,
            PlayerInventoryItem {
                guid: item_guid,
                entry_id: 25,
                db_guid: 60,
                inventory_type: None,
            },
        );
        runtime.store_item_in_slot_like_cpp(
            30,
            PlayerInventoryItem {
                guid: bag_guid,
                entry_id: 4500,
                db_guid: 61,
                inventory_type: Some(18),
            },
        );
        let mut top = item(item_guid, 25, 2);
        top.set_slot(35);
        runtime.store_item_object_like_cpp(top);
        runtime.store_item_object_like_cpp(item(bag_guid, 4500, 1));
        let mut nested = item(nested_guid, 26, 1);
        nested.set_container_guid_and_slot(bag_guid, 30);
        nested.set_slot(3);
        runtime.store_item_object_like_cpp(nested);
        assert!(giver.inventory().items.iter().all(Option::is_none));

        assert_eq!(
            giver
                .trade_item_by_guid_like_cpp(item_guid)
                .unwrap()
                .count(),
            2
        );
        assert_eq!(
            giver.trade_item_position_like_cpp(nested_guid),
            Some((30, 3))
        );
        assert_eq!(
            giver.trade_item_at_like_cpp(30, 3).unwrap().object().guid(),
            nested_guid
        );

        let taken = giver
            .trade_take_item_like_cpp(item_guid)
            .expect("top-level taken");
        assert_eq!(
            (taken.bag, taken.slot, taken.record.unwrap().db_guid),
            (255, 35, 60)
        );
        assert!(giver.trade_item_by_guid_like_cpp(item_guid).is_none());
        let taken = giver
            .trade_take_item_like_cpp(nested_guid)
            .expect("nested taken");
        assert_eq!((taken.bag, taken.slot), (30, 3));

        assert!(giver.trade_store_item_like_cpp(taken.item, 62, None, 30, 5));
        assert_eq!(
            giver.trade_item_position_like_cpp(nested_guid),
            Some((30, 5))
        );
        assert!(!giver.trade_store_item_like_cpp(item(item_guid, 25, 1), 60, None, 30, 5));
    }
}
