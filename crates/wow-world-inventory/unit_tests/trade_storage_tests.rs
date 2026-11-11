//! Regressions for represented trade storage planning and snapshot isolation.

use super::*;
use wow_constants::{EnchantmentSlot, ItemClass, ItemSubClassContainer};
use wow_entities::{
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START, MAX_BAG_SIZE,
    PlayerInventoryItem,
};

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

fn bag_template(entry: u32) -> Option<ItemStorageTemplate> {
    let mut template = template(entry)?;
    if entry == 900 {
        template.class_id = ItemClass::Container;
        template.subclass_id = ItemSubClassContainer::Container as u32;
        template.container_slots = 2;
        template.max_stack_size = 1;
    }
    Some(template)
}

fn runtime_bag(receiver: &mut Player, counter: i64) -> ObjectGuid {
    let mut bag = item(counter, 900, 1);
    bag.set_owner_guid(receiver.guid());
    bag.set_slot(INVENTORY_SLOT_BAG_START);
    bag.set_expiration(600);
    bag.set_enchantment(EnchantmentSlot::EnhancementTemporary, 500, 5_000, 0);
    receiver.add_item_durations(&bag);
    receiver.add_enchantment_durations(&mut bag);
    let guid = bag.object().guid();
    receiver
        .inventory_runtime_mut_like_cpp()
        .store_item_in_slot_like_cpp(
            INVENTORY_SLOT_BAG_START,
            PlayerInventoryItem {
                guid,
                entry_id: 900,
                db_guid: counter as u64,
                inventory_type: None,
            },
        );
    receiver
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(bag);
    guid
}

#[test]
fn snapshot_registers_runtime_only_bags_without_changing_canonical_state() {
    let mut receiver = player(0);
    runtime_bag(&mut receiver, 700);
    let native_before = receiver.inventory().clone();
    let runtime_before = receiver.inventory_runtime_like_cpp().clone();
    let active_before = receiver.active_data().clone();
    let player_mask_before = receiver.player_data_changes_mask().clone();
    let active_mask_before = receiver.active_player_data_changes_mask().clone();
    let durations_before = receiver.item_durations().to_vec();
    let enchants_before = receiver.enchant_durations().to_vec();
    assert!(!durations_before.is_empty());
    assert!(!enchants_before.is_empty());
    let catalogs = TradeStoreCatalogsLikeCpp {
        template: &bag_template,
        limit_category: &no_limit,
    };

    let plans = plan_trade_receive_snapshot_like_cpp(
        TradeStorageSnapshot::capture(&receiver),
        &[],
        &[item(701, 25, 3)],
        &catalogs,
    )
    .unwrap();
    assert_eq!(
        plans[0].actions,
        vec![TradeStoreActionLikeCpp::Store {
            bag: INVENTORY_SLOT_BAG_START,
            slot: 0,
            count: 3,
        }]
    );
    assert_eq!(receiver.inventory(), &native_before);
    assert_eq!(receiver.inventory_runtime_like_cpp(), &runtime_before);
    assert_eq!(receiver.active_data(), &active_before);
    assert_eq!(receiver.player_data_changes_mask(), &player_mask_before);
    assert_eq!(
        receiver.active_player_data_changes_mask(),
        &active_mask_before
    );
    assert_eq!(receiver.item_durations(), durations_before);
    assert_eq!(receiver.enchant_durations(), enchants_before);
}

#[test]
fn snapshot_frees_bag_slots_and_later_incoming_sees_partial_merge_and_store() {
    let mut receiver = player(0);
    let bag_guid = runtime_bag(&mut receiver, 710);
    receiver
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 2)
        .unwrap();
    let existing = item(711, 25, 18);
    let existing_guid = existing.object().guid();
    let outgoing = item(712, 30, 1);
    let outgoing_guid = outgoing.object().guid();
    assert!(receiver.trade_store_item_like_cpp(existing, 711, None, INVENTORY_SLOT_BAG_START, 0));
    assert!(receiver.trade_store_item_like_cpp(outgoing, 712, None, INVENTORY_SLOT_BAG_START, 1));
    let incoming = [item(713, 25, 5), item(714, 25, 4)];
    let runtime_before = receiver.inventory_runtime_like_cpp().clone();
    let catalogs = TradeStoreCatalogsLikeCpp {
        template: &bag_template,
        limit_category: &no_limit,
    };
    let plans = plan_trade_receive_snapshot_like_cpp(
        TradeStorageSnapshot::capture(&receiver),
        &[outgoing_guid],
        &incoming,
        &catalogs,
    )
    .unwrap();
    assert_eq!(
        plans,
        vec![
            TradeStorePlanLikeCpp {
                item_guid: incoming[0].object().guid(),
                actions: vec![
                    TradeStoreActionLikeCpp::Merge {
                        target_guid: existing_guid,
                        added: 2,
                        new_count: 20
                    },
                    TradeStoreActionLikeCpp::Store {
                        bag: INVENTORY_SLOT_BAG_START,
                        slot: 1,
                        count: 3
                    },
                ]
            },
            TradeStorePlanLikeCpp {
                item_guid: incoming[1].object().guid(),
                actions: vec![TradeStoreActionLikeCpp::Merge {
                    target_guid: incoming[0].object().guid(),
                    added: 4,
                    new_count: 7
                },]
            },
        ]
    );
    assert_eq!(receiver.inventory_runtime_like_cpp(), &runtime_before);
}

#[test]
fn outgoing_bag_clears_native_fallback_before_later_outgoing_lookup() {
    fn limited(entry: u32) -> Option<ItemStorageTemplate> {
        let mut template = bag_template(entry)?;
        if entry == 25 {
            template.max_count = 1;
        }
        Some(template)
    }
    let catalogs = TradeStoreCatalogsLikeCpp {
        template: &limited,
        limit_category: &no_limit,
    };
    for native_top_level_matches in [true, false] {
        let mut receiver = player(1);
        let bag_guid = runtime_bag(&mut receiver, 720);
        receiver
            .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 2)
            .unwrap();
        if native_top_level_matches {
            receiver
                .store_top_level_item(INVENTORY_SLOT_BAG_START, bag_guid)
                .unwrap();
        }
        let child = item(721, 25, 1);
        let child_guid = child.object().guid();
        assert!(receiver.trade_store_item_like_cpp(child, 721, None, INVENTORY_SLOT_BAG_START, 0));
        let result = plan_trade_receive_snapshot_like_cpp(
            TradeStorageSnapshot::capture(&receiver),
            &[bag_guid, child_guid],
            &[item(722, 25, 1)],
            &catalogs,
        );
        if native_top_level_matches {
            // Removing the matching native bag also removes its registry.
            // The later child lookup no longer resolves it; its original count remains.
            assert_eq!(result, Err((InventoryResult::ItemMaxCount, 25)));
        } else {
            assert_eq!(
                result.unwrap()[0].actions,
                vec![TradeStoreActionLikeCpp::Store {
                    bag: INVENTORY_SLOT_BAG_0,
                    slot: INVENTORY_SLOT_ITEM_START,
                    count: 1,
                }]
            );
        }
    }
}

#[test]
fn snapshot_reports_binding_and_missing_template_in_incoming_order() {
    fn missing(entry: u32) -> Option<ItemStorageTemplate> {
        (entry != 404).then(|| ItemStorageTemplate::regular_item(entry, 20))
    }
    let receiver = player(2);
    let mut bound = item(730, 25, 1);
    bound.set_owner_guid(ObjectGuid::create_player(1, 99));
    bound.set_binding(true);
    let unknown = item(731, 404, 1);
    let catalogs = TradeStoreCatalogsLikeCpp {
        template: &missing,
        limit_category: &no_limit,
    };
    assert_eq!(
        plan_trade_receive_snapshot_like_cpp(
            TradeStorageSnapshot::capture(&receiver),
            &[],
            &[bound.clone(), unknown.clone()],
            &catalogs,
        ),
        Err((InventoryResult::NotOwner, 25))
    );
    assert_eq!(
        plan_trade_receive_snapshot_like_cpp(
            TradeStorageSnapshot::capture(&receiver),
            &[],
            &[unknown, bound],
            &catalogs,
        ),
        Err((InventoryResult::ItemNotFound, 404))
    );
}

#[test]
fn snapshot_rejects_multiple_empty_destinations_for_one_source_item() {
    let receiver = player(2);
    assert_eq!(
        plan_trade_receive_snapshot_like_cpp(
            TradeStorageSnapshot::capture(&receiver),
            &[],
            &[item(740, 25, 21)],
            &catalogs(),
        ),
        Err((InventoryResult::InternalBagError, 25))
    );
}

#[test]
fn snapshot_does_not_register_oversized_runtime_bag() {
    fn oversized(entry: u32) -> Option<ItemStorageTemplate> {
        let mut template = bag_template(entry)?;
        if entry == 900 {
            template.container_slots = MAX_BAG_SIZE as u8 + 1;
        }
        Some(template)
    }
    let mut receiver = player(0);
    runtime_bag(&mut receiver, 750);
    let catalogs = TradeStoreCatalogsLikeCpp {
        template: &oversized,
        limit_category: &no_limit,
    };
    assert_eq!(
        plan_trade_receive_snapshot_like_cpp(
            TradeStorageSnapshot::capture(&receiver),
            &[],
            &[item(751, 25, 1)],
            &catalogs,
        ),
        Err((InventoryResult::InvFull, 25))
    );
}
