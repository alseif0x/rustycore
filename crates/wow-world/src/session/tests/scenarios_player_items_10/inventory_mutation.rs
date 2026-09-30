use super::*;

#[test]
fn get_inventory_item_by_guid_finds_nested_bag_item_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (bag_guid, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, INVENTORY_SLOT_BAG_START, 5);

    let direct = session
        .get_inventory_item_by_guid_like_cpp(bag_guid)
        .expect("direct bag item");
    assert_eq!(direct.0, INVENTORY_SLOT_BAG_0);
    assert_eq!(direct.1, INVENTORY_SLOT_BAG_START);
    assert_eq!(direct.2.guid, bag_guid);

    let nested = session
        .get_inventory_item_by_guid_like_cpp(child_guid)
        .expect("nested child item");
    assert_eq!(nested.0, INVENTORY_SLOT_BAG_START);
    assert_eq!(nested.1, 5);
    assert_eq!(nested.2.guid, child_guid);
    assert_eq!(nested.2.entry_id, 700);
}
#[test]
fn direct_destroy_uses_cpp_can_unequip_gate_for_equipment_and_bags() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            100,
            InventoryType::Chest,
            ItemClass::Armor,
            0,
        ),
        represented_test_item_record_like_cpp(
            101,
            InventoryType::Bag,
            ItemClass::Container,
            0,
        ),
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (
            100,
            ItemSparseTemplateEntry {
                bonding: 0,
                ..inventory_sparse_template_for_test(InventoryType::Chest as i8)
            },
        ),
        (
            101,
            ItemSparseTemplateEntry {
                bonding: 0,
                container_slots: 16,
                ..inventory_sparse_template_for_test(InventoryType::Bag as i8)
            },
        ),
    ])));

    let chest_guid = ObjectGuid::create_item(1, 1000);
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            EQUIPMENT_SLOT_CHEST,
            InventoryItem {
                guid: chest_guid,
                entry_id: 100,
                db_guid: 1000,
                inventory_type: Some(InventoryType::Chest as u8),
            },
        );
    let chest_item = session.make_inventory_item_object(
        chest_guid,
        100,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    session.insert_inventory_item_object(chest_item);
    let chest_proto = session.item_storage_template(100);
    session.in_combat = true;
    assert_eq!(
        session.can_destroy_direct_item_like_cpp(
            EQUIPMENT_SLOT_CHEST,
            session.inventory_item_objects.get(&chest_guid),
            chest_proto.as_ref(),
            false,
        ),
        InventoryResult::NotInCombat
    );
    session.in_combat = false;

    let bag_guid = ObjectGuid::create_item(1, 1001);
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            INVENTORY_SLOT_BAG_START,
            InventoryItem {
                guid: bag_guid,
                entry_id: 101,
                db_guid: 1001,
                inventory_type: Some(InventoryType::Bag as u8),
            },
        );
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        101,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag_item);
    let child_guid = ObjectGuid::create_item(1, 1002);
    let mut child = session.make_inventory_item_object(
        child_guid,
        100,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    child.set_container_guid_and_slot(bag_guid, 0);
    session.insert_inventory_item_object(child);

    let bag_proto = session.item_storage_template(101);
    assert!(session.direct_item_contains_items(bag_guid));
    assert_eq!(
        session.can_destroy_direct_item_like_cpp(
            INVENTORY_SLOT_BAG_START,
            session.inventory_item_objects.get(&bag_guid),
            bag_proto.as_ref(),
            session.direct_item_contains_items(bag_guid),
        ),
        InventoryResult::DestroyNonemptyBag
    );
}
#[test]
fn moved_bag_detects_active_child_item_loot_like_cpp_swap_item() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 1);
    let (bag_guid, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, INVENTORY_SLOT_BAG_START, 0);
    let other_child = ObjectGuid::create_item(1, 1003);

    assert!(!session.represented_bag_contains_active_item_loot_like_cpp(bag_guid));

    session.set_active_loot_guid(other_child);
    assert!(!session.represented_bag_contains_active_item_loot_like_cpp(bag_guid));

    session.set_active_loot_guid(child_guid);
    assert!(!session.represented_bag_contains_active_item_loot_like_cpp(bag_guid));

    session.loot_table.insert(
        child_guid,
        CreatureLoot {
            loot_guid: child_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 1,
                item_id: 700,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: ItemContext::None as u8,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    assert!(session.represented_bag_contains_active_item_loot_like_cpp(bag_guid));
}
