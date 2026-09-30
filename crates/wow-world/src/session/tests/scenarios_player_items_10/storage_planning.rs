use super::*;

#[test]
fn direct_inventory_store_plan_uses_cpp_can_store_merge_then_empty_order() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 900);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([represented_test_item_record_like_cpp(
        700,
        InventoryType::NonEquip,
        ItemClass::Consumable,
        0,
    )])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        700,
        ItemSparseTemplateEntry {
            stackable: 20,
            ..inventory_sparse_template_for_test(InventoryType::NonEquip as i8)
        },
    )])));

    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            35,
            InventoryItem {
                guid: item_guid,
                entry_id: 700,
                db_guid: 900,
                inventory_type: None,
            },
        );
    let item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        18,
        0,
        ItemContext::None,
        35,
    );
    session.insert_inventory_item_object(item);

    let (result, dest, no_space) = session
        .plan_store_new_direct_inventory_item(700, 5)
        .expect("player snapshot should exist");

    assert_eq!(result, InventoryResult::Ok);
    assert_eq!(no_space, None);
    assert_eq!(dest.len(), 2);
    assert_eq!(
        dest[0],
        ItemPosCount::new((u16::from(INVENTORY_SLOT_BAG_0) << 8) | 35, 2)
    );
    assert_eq!(
        dest[1],
        ItemPosCount::new((u16::from(INVENTORY_SLOT_BAG_0) << 8) | 36, 3)
    );
}
#[test]
fn direct_inventory_store_plan_respects_cpp_explicit_empty_slot() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    install_stackable_test_item_template(&mut session, 700, 20);

    let (result, dest, no_space) = session
        .plan_store_new_direct_inventory_item_at(700, 5, INVENTORY_SLOT_BAG_0, 36)
        .expect("player snapshot should exist");

    assert_eq!(result, InventoryResult::Ok);
    assert_eq!(no_space, None);
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            (u16::from(INVENTORY_SLOT_BAG_0) << 8) | 36,
            5,
        )]
    );
}
#[test]
fn direct_inventory_store_plan_respects_cpp_explicit_stack_before_other_merge() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    install_stackable_test_item_template(&mut session, 700, 20);

    for (slot, db_guid) in [(35, 900_u64), (36, 901_u64)] {
        let item_guid = ObjectGuid::create_item(1, db_guid as i64);
        session
            .player_item_test_fixture_like_cpp
            .inventory_items
            .insert(
                slot,
                InventoryItem {
                    guid: item_guid,
                    entry_id: 700,
                    db_guid,
                    inventory_type: None,
                },
            );
        let item = session.make_inventory_item_object(
            item_guid,
            700,
            player_guid,
            18,
            0,
            ItemContext::None,
            slot,
        );
        session.insert_inventory_item_object(item);
    }

    let (result, dest, no_space) = session
        .plan_store_new_direct_inventory_item_at(700, 3, INVENTORY_SLOT_BAG_0, 36)
        .expect("player snapshot should exist");

    assert_eq!(result, InventoryResult::Ok);
    assert_eq!(no_space, None);
    assert_eq!(
        dest,
        vec![
            ItemPosCount::new((u16::from(INVENTORY_SLOT_BAG_0) << 8) | 36, 2),
            ItemPosCount::new((u16::from(INVENTORY_SLOT_BAG_0) << 8) | 35, 1),
        ]
    );
}
