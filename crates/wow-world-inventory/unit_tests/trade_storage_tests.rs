//! `plan_trade_receive_like_cpp` against C++ `moveItems` storage order.

use super::*;
use wow_entities::INVENTORY_SLOT_ITEM_START;

fn template(entry: u32) -> Option<ItemStorageTemplate> {
    Some(ItemStorageTemplate::regular_item(entry, 20))
}

fn no_limit(_: u32) -> Option<ItemLimitCategoryTemplate> {
    None
}

fn catalogs() -> TradeStoreCatalogsLikeCpp<'static> {
    TradeStoreCatalogsLikeCpp {
        template: &template,
        limit_category: &no_limit,
    }
}

fn item(counter: i64, entry: u32, count: u32) -> Item {
    let mut item = Item::default();
    item.object_mut()
        .create(ObjectGuid::create_item(1, counter));
    item.object_mut().set_entry(entry);
    item.set_count(count);
    item
}

fn player(backpack: u8) -> Player {
    let mut player = Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, 7));
    player.set_inventory_slot_count(backpack);
    player
}

#[test]
fn incoming_item_takes_the_first_free_backpack_slot_like_cpp() {
    let receiver = player(2);
    let plans =
        plan_trade_receive_like_cpp(&receiver, &[], &[item(1, 25, 3)], &catalogs()).unwrap();
    assert_eq!(
        plans[0].actions,
        vec![TradeStoreActionLikeCpp::Store {
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
            count: 3
        }]
    );
    assert!(!plans[0].fully_merged());
}

#[test]
fn outgoing_items_free_their_slots_and_later_items_see_earlier_ones_like_cpp() {
    let mut receiver = player(1);
    let given = item(2, 30, 1);
    let given_guid = given.object().guid();
    assert!(receiver.trade_store_item_like_cpp(given, 2, None, 255, INVENTORY_SLOT_ITEM_START));

    // Full backpack, nothing leaves: the first incoming item does not fit.
    assert_eq!(
        plan_trade_receive_like_cpp(&receiver, &[], &[item(3, 25, 1)], &catalogs()),
        Err((InventoryResult::InvFull, 25))
    );
    // The traded-away item frees its slot for exactly one incoming item.
    let plans =
        plan_trade_receive_like_cpp(&receiver, &[given_guid], &[item(3, 25, 1)], &catalogs())
            .unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(
        plan_trade_receive_like_cpp(
            &receiver,
            &[given_guid],
            &[item(3, 25, 1), item(4, 26, 1)],
            &catalogs()
        ),
        Err((InventoryResult::InvFull, 26))
    );
}

#[test]
fn incoming_stack_merges_into_existing_stacks_like_cpp() {
    let mut receiver = player(1);
    let existing = item(5, 25, 15);
    let existing_guid = existing.object().guid();
    assert!(receiver.trade_store_item_like_cpp(existing, 5, None, 255, INVENTORY_SLOT_ITEM_START));

    let plans =
        plan_trade_receive_like_cpp(&receiver, &[], &[item(6, 25, 4)], &catalogs()).unwrap();
    assert_eq!(
        plans[0].actions,
        vec![TradeStoreActionLikeCpp::Merge {
            target_guid: existing_guid,
            added: 4,
            new_count: 19
        }]
    );
    assert!(plans[0].fully_merged());
}
