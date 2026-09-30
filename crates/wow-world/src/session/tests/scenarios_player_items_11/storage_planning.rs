use super::*;

#[test]
fn direct_inventory_store_plan_counts_represented_bag_contents_for_limit_category_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 800);
    let child_guid = ObjectGuid::create_item(1, 801);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            600,
            InventoryType::Bag,
            ItemClass::Container,
            0,
        ),
        represented_test_item_record_like_cpp(
            700,
            InventoryType::NonEquip,
            ItemClass::Consumable,
            0,
        ),
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (
            600,
            ItemSparseTemplateEntry {
                container_slots: 4,
                ..inventory_sparse_template_for_test(InventoryType::Bag as i8)
            },
        ),
        (
            700,
            ItemSparseTemplateEntry {
                stackable: 20,
                limit_category: 44,
                ..inventory_sparse_template_for_test(InventoryType::NonEquip as i8)
            },
        ),
    ])));
    session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
        ItemLimitCategoryEntry {
            id: 44,
            name: "Have one".into(),
            quantity: 1,
            flags: wow_entities::ITEM_LIMIT_CATEGORY_MODE_HAVE,
        },
    ])));

    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            INVENTORY_SLOT_BAG_START,
            InventoryItem {
                guid: bag_guid,
                entry_id: 600,
                db_guid: 800,
                inventory_type: Some(InventoryType::Bag as u8),
            },
        );
    let bag = session.make_inventory_item_object(
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag);
    let mut child = session.make_inventory_item_object(
        child_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    child.set_container_guid_and_slot(bag_guid, 0);
    session.insert_inventory_item_object(child);

    let (result, dest, no_space) = session
        .plan_store_new_direct_inventory_item(700, 1)
        .expect("player snapshot should exist");

    assert_eq!(result, InventoryResult::ItemMaxLimitCategoryCountExceededIs);
    assert!(dest.is_empty());
    assert_eq!(no_space, Some(1));
}
#[test]
fn direct_inventory_store_plan_allocates_represented_bag_slot_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 850);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            600,
            InventoryType::Bag,
            ItemClass::Container,
            0,
        ),
        represented_test_item_record_like_cpp(
            700,
            InventoryType::NonEquip,
            ItemClass::Consumable,
            0,
        ),
        represented_test_item_record_like_cpp(
            701,
            InventoryType::NonEquip,
            ItemClass::Consumable,
            0,
        ),
    ])));
    let sparse = |inventory_type: InventoryType,
                  stackable: i32,
                  container_slots: u8|
     -> ItemSparseTemplateEntry {
        ItemSparseTemplateEntry {
            stackable: stackable,
            container_slots: container_slots,
            ..inventory_sparse_template_for_test(inventory_type as i8)
        }
    };
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (600, sparse(InventoryType::Bag, 1, 4)),
        (700, sparse(InventoryType::NonEquip, 20, 0)),
        (701, sparse(InventoryType::NonEquip, 20, 0)),
    ])));

    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            INVENTORY_SLOT_BAG_START,
            InventoryItem {
                guid: bag_guid,
                entry_id: 600,
                db_guid: 850,
                inventory_type: Some(InventoryType::Bag as u8),
            },
        );
    let bag = session.make_inventory_item_object(
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag);

    for slot_offset in 0..INVENTORY_DEFAULT_SIZE {
        let slot = INVENTORY_SLOT_ITEM_START + slot_offset;
        let db_guid = 900 + u64::from(slot_offset);
        let guid = ObjectGuid::create_item(1, db_guid as i64);
        session
            .player_item_test_fixture_like_cpp
            .inventory_items
            .insert(
                slot,
                InventoryItem {
                    guid,
                    entry_id: 701,
                    db_guid,
                    inventory_type: None,
                },
            );
        let item = session.make_inventory_item_object(
            guid,
            701,
            player_guid,
            1,
            0,
            ItemContext::None,
            slot,
        );
        session.insert_inventory_item_object(item);
    }

    let (result, dest, no_space) = session
        .plan_store_new_direct_inventory_item(700, 3)
        .expect("player snapshot should exist");

    assert_eq!(result, InventoryResult::Ok);
    assert_eq!(no_space, None);
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            (u16::from(INVENTORY_SLOT_BAG_START) << 8) | 0,
            3,
        )]
    );
}
