use super::*;

#[test]
fn open_item_wrapped_gift_row_helper_updates_runtime_and_top_level_metadata_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gift_creator = ObjectGuid::create_player(1, 77);
    let item_guid = ObjectGuid::create_item(1, 904);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([represented_test_item_record_like_cpp(
        200,
        InventoryType::Weapon,
        ItemClass::Weapon,
        0,
    )])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        200,
        ItemSparseTemplateEntry {
            max_durability: 40,
            ..inventory_sparse_template_for_test(InventoryType::Weapon as i8)
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
    let inventory_item = session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .get(&23)
        .unwrap();
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
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
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
