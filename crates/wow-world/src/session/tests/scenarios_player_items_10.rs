//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn open_item_wrapped_gift_row_helper_updates_runtime_and_top_level_metadata_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gift_creator = ObjectGuid::create_player(1, 77);
    let item_guid = ObjectGuid::create_item(1, 904);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 200,
        class_id: ItemClass::Weapon as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::Weapon as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        200,
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 1,
            max_count: 0,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 40,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::Weapon as i8,
        },
    )])));
    insert_open_item_top_level(&mut session, player_guid, 23, item_guid, 100, true);
    {
        let item = session.inventory_item_objects.get_mut(&item_guid).unwrap();
        item.set_gift_creator(gift_creator);
        item.set_item_flag(ItemFieldFlags::WRAPPED);
        item.set_durability(55);
        item.force_state(ItemUpdateState::Unchanged);
    }

    let durability = session
        .apply_wrapped_gift_row_to_runtime_item_like_cpp(
            INVENTORY_SLOT_BAG_0,
            item_guid,
            23,
            200,
            ItemFieldFlags::SOULBOUND.bits(),
        )
        .unwrap();

    let item = session.inventory_item_objects.get(&item_guid).unwrap();
    assert_eq!(durability, 55);
    assert_eq!(item.object().entry(), 200);
    assert_eq!(item.data().gift_creator, ObjectGuid::EMPTY);
    assert_eq!(item.item_flags_bits(), ItemFieldFlags::SOULBOUND.bits());
    assert_eq!(item.data().max_durability, 40);
    assert_eq!(item.data().durability, 55);
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert!(!item.is_wrapped());
    let inventory_item = session.inventory_items.get(&23).unwrap();
    assert_eq!(inventory_item.entry_id, 200);
    assert_eq!(
        inventory_item.inventory_type,
        Some(InventoryType::Weapon as u8)
    );
}
#[tokio::test]
async fn open_item_wrapped_locked_template_returns_item_locked_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 905);
    session.set_player_guid(Some(player_guid));
    install_open_item_template_with_flags(&mut session, 700, ItemFlags::empty(), 123);
    insert_open_item_top_level(&mut session, player_guid, 23, item_guid, 700, false);
    {
        let item = session.inventory_item_objects.get_mut(&item_guid).unwrap();
        item.set_item_flag(ItemFieldFlags::WRAPPED);
        item.set_durability(17);
        item.force_state(ItemUpdateState::Unchanged);
    }

    session
        .handle_open_item(WorldPacket::from_bytes(&[INVENTORY_SLOT_BAG_0, 23]))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        sent,
        InventoryChangeFailure::new(InventoryResult::ItemLocked, item_guid, ObjectGuid::EMPTY)
            .to_bytes()
    );
    assert!(!session.loot_table.contains_key(&item_guid));
    let item = session.inventory_item_objects.get(&item_guid).unwrap();
    assert_eq!(item.object().entry(), 700);
    assert_eq!(item.data().durability, 17);
    assert_eq!(item.update_state(), ItemUpdateState::Unchanged);
    assert!(item.is_wrapped());
}
#[tokio::test]
async fn open_item_locked_container_returns_item_locked_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 900);
    session.set_player_guid(Some(player_guid));
    install_open_item_has_loot_template_with_lock(&mut session, 700, 123);
    install_lock_store(&mut session, 123);
    insert_open_item_top_level(&mut session, player_guid, 23, item_guid, 700, false);

    session
        .handle_open_item(WorldPacket::from_bytes(&[INVENTORY_SLOT_BAG_0, 23]))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        sent,
        InventoryChangeFailure::new(InventoryResult::ItemLocked, item_guid, ObjectGuid::EMPTY)
            .to_bytes()
    );
    assert!(!session.loot_table.contains_key(&item_guid));
    assert!(
        session
            .inventory_item_objects
            .get(&item_guid)
            .is_some_and(|item| !item.loot_generated())
    );
}
#[tokio::test]
async fn open_item_unlocked_locked_template_continues_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 901);
    session.set_player_guid(Some(player_guid));
    install_open_item_has_loot_template_with_lock(&mut session, 700, 123);
    install_lock_store(&mut session, 123);
    insert_open_item_top_level(&mut session, player_guid, 23, item_guid, 700, true);

    session
        .handle_open_item(WorldPacket::from_bytes(&[INVENTORY_SLOT_BAG_0, 23]))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::LootResponse as u16);
    assert!(session.loot_table.contains_key(&item_guid));
    assert!(
        session
            .inventory_item_objects
            .get(&item_guid)
            .is_some_and(|item| item.loot_generated())
    );
}
#[tokio::test]
async fn open_item_unknown_lock_id_returns_item_locked_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 903);
    session.set_player_guid(Some(player_guid));
    install_open_item_has_loot_template_with_lock(&mut session, 700, 123);
    session.set_lock_store(Arc::new(LockStore::from_entries([])));
    insert_open_item_top_level(&mut session, player_guid, 23, item_guid, 700, true);

    session
        .handle_open_item(WorldPacket::from_bytes(&[INVENTORY_SLOT_BAG_0, 23]))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        sent,
        InventoryChangeFailure::new(InventoryResult::ItemLocked, item_guid, ObjectGuid::EMPTY)
            .to_bytes()
    );
    assert!(!session.loot_table.contains_key(&item_guid));
}
#[tokio::test]
async fn open_item_missing_runtime_object_fails_closed_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 902);
    session.set_player_guid(Some(player_guid));
    install_open_item_has_loot_template_with_lock(&mut session, 700, 123);
    session.inventory_items.insert(
        23,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: None,
        },
    );
    assert!(!session.inventory_item_objects.contains_key(&item_guid));

    session
        .handle_open_item(WorldPacket::from_bytes(&[INVENTORY_SLOT_BAG_0, 23]))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        sent,
        InventoryChangeFailure::new(InventoryResult::ItemLocked, item_guid, ObjectGuid::EMPTY)
            .to_bytes()
    );
    assert!(!session.loot_table.contains_key(&item_guid));
}
#[test]
fn open_item_release_destroy_nested_item_leaves_container_in_place() {
    assert_open_item_release_destroy_nested_item_leaves_container_in_place(
        INVENTORY_SLOT_BAG_START,
    );
}
#[test]
fn open_item_release_destroy_nested_bank_bag_item_leaves_container_in_place() {
    assert_open_item_release_destroy_nested_item_leaves_container_in_place(BANK_SLOT_BAG_START);
}
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
        ItemRecord {
            id: 100,
            class_id: ItemClass::Armor as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 101,
            class_id: ItemClass::Container as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::Bag as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (
            100,
            ItemSparseTemplateEntry {
                flags: [0, 0, 0, 0],
                bag_family: 0,
                start_quest_id: 0,
                stackable: 1,
                max_count: 0,
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 0,
                buy_price: 0,
                vendor_stack_count: 1,
                price_variance: 1.0,
                price_random_value: 1.0,
                max_durability: 0,
                other_faction_item_id: 0,
                content_tuning_id: 0,
                player_level_to_item_level_curve_id: 0,
                limit_category: 0,
                instance_bound: 0,
                zone_bound: [0, 0],
                required_reputation_faction: 0,
                allowable_class: -1,
                required_expansion: 0,
                bonding: 0,
                container_slots: 0,
                inventory_type: InventoryType::Chest as i8,
            },
        ),
        (
            101,
            ItemSparseTemplateEntry {
                flags: [0, 0, 0, 0],
                bag_family: 0,
                start_quest_id: 0,
                stackable: 1,
                max_count: 0,
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 0,
                buy_price: 0,
                vendor_stack_count: 1,
                price_variance: 1.0,
                price_random_value: 1.0,
                max_durability: 0,
                other_faction_item_id: 0,
                content_tuning_id: 0,
                player_level_to_item_level_curve_id: 0,
                limit_category: 0,
                instance_bound: 0,
                zone_bound: [0, 0],
                required_reputation_faction: 0,
                allowable_class: -1,
                required_expansion: 0,
                bonding: 0,
                container_slots: 16,
                inventory_type: InventoryType::Bag as i8,
            },
        ),
    ])));

    let chest_guid = ObjectGuid::create_item(1, 1000);
    session.inventory_items.insert(
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
    session.inventory_items.insert(
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
#[test]
fn inventory_item_object_uses_template_durability_and_runtime_fields() {
    let (mut session, _, _) = make_session();
    let owner_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 900);
    session.total_played_time = 123;
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        700,
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 1,
            max_count: 0,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 55,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: 0,
            container_slots: 0,
            inventory_type: 0,
        },
    )])));

    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        owner_guid,
        3,
        44,
        ItemContext::Vendor,
        35,
    );
    item.set_state(ItemUpdateState::Unchanged);
    session.insert_inventory_item_object(item);

    let stored = session.inventory_item_objects.get(&item_guid).unwrap();
    assert_eq!(stored.object().entry(), 700);
    assert_eq!(stored.data().owner, owner_guid);
    assert_eq!(stored.data().contained_in, owner_guid);
    assert_eq!(stored.data().stack_count, 3);
    assert_eq!(stored.data().max_durability, 55);
    assert_eq!(stored.data().durability, 44);
    assert_eq!(stored.data().context, ItemContext::Vendor as i32);
    assert_eq!(stored.slot(), 35);
    assert_eq!(stored.update_state(), ItemUpdateState::Unchanged);

    session.set_inventory_item_object_slot(item_guid, 36);
    assert_eq!(
        session
            .inventory_item_objects
            .get(&item_guid)
            .unwrap()
            .slot(),
        36
    );
    assert!(session.remove_inventory_item_object(item_guid).is_some());
    assert!(!session.inventory_item_objects.contains_key(&item_guid));
}
#[test]
fn canonical_player_logout_cleanup_removes_player_before_session_inventory_like_cpp() {
    run_canonical_player_owner_test(|| {
        let (mut session, _, _) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 46);
        let item_guid = ObjectGuid::create_item(1, 901);

        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_player_guid(Some(player_guid));
        session.player_name = Some("LogoutMap".into());
        session.player_position = Some(Position::new(1.0, 2.0, 3.0, 0.0));
        session.current_map_id = 571;
        session.inventory_items.insert(
            23,
            InventoryItem {
                guid: item_guid,
                entry_id: 700,
                db_guid: 901,
                inventory_type: None,
            },
        );
        let item = session.make_inventory_item_object(
            item_guid,
            700,
            player_guid,
            1,
            0,
            ItemContext::None,
            23,
        );
        session.insert_inventory_item_object(item);

        insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
        assert!(session.adopt_registered_canonical_player_fixture_like_cpp());

        assert!(
            canonical
                .lock()
                .unwrap()
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_player(player_guid)
                .is_some()
        );

        session.cleanup_shared_runtime_state();

        assert!(
            canonical
                .lock()
                .unwrap()
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_player(player_guid)
                .is_none()
        );
        assert!(session.inventory_items.is_empty());
        assert!(session.inventory_item_objects.is_empty());
    });
}
#[test]
fn direct_inventory_store_plan_uses_cpp_can_store_merge_then_empty_order() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 900);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 700,
        class_id: ItemClass::Consumable as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        700,
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 20,
            max_count: 0,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 0,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));

    session.inventory_items.insert(
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
        session.inventory_items.insert(
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
