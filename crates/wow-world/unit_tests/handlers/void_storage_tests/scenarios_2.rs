//! Void-storage regressions, part 2 of 2.
//!
//! Moved out of the void_storage_tests.rs root under #683; every test is unchanged.

use super::*;

#[tokio::test]
async fn withdrawal_store_plan_merges_before_empty_slots_with_atomic_overlays_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    install_void_test_item_template_with_stack(&mut session, 19019, 20);
    let player_guid = ObjectGuid::create_player(1, 42);
    let existing_guid = ObjectGuid::create_item(1, 501);
    let mut existing_item = session.make_inventory_item_object(
        existing_guid,
        19019,
        player_guid,
        19,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    existing_item.set_count(19);
    session.insert_inventory_item_object(existing_item);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: existing_guid,
            entry_id: 19019,
            db_guid: 501,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );

    let (result, destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(19019, 1, &[], &[])
        .expect("represented inventory planner");
    assert_eq!(result, wow_constants::InventoryResult::Ok);
    assert_eq!(
        destinations,
        vec![wow_entities::ItemPosCount::new(
            (u16::from(INVENTORY_SLOT_BAG_0) << 8) | u16::from(INVENTORY_SLOT_ITEM_START),
            1,
        )]
    );

    let overlays = [
        DirectInventoryStorageOverlayLikeCpp {
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
            entry_id: 19019,
            count: 20,
        },
        DirectInventoryStorageOverlayLikeCpp {
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START + 1,
            entry_id: 19019,
            count: 1,
        },
    ];
    let (result, destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(19019, 1, &overlays, &[])
        .expect("represented inventory planner with detached reservations");
    assert_eq!(result, wow_constants::InventoryResult::Ok);
    assert_eq!(
        destinations,
        vec![wow_entities::ItemPosCount::new(
            (u16::from(INVENTORY_SLOT_BAG_0) << 8) | u16::from(INVENTORY_SLOT_ITEM_START + 1),
            1,
        )]
    );

    let mut merged_item = session
        .inventory_item_objects_like_cpp()
        .get(&existing_guid)
        .cloned()
        .expect("existing merge target");
    merged_item.set_count(20);
    merged_item.set_creator(ObjectGuid::create_player(1, 7));
    merged_item.set_binding(true);
    let enchantments = WorldSession::void_storage_enchantments_db_string_like_cpp(&[0; 13]);
    let write = session.void_storage_merged_item_write_like_cpp(
        &InventoryItem {
            guid: existing_guid,
            entry_id: 19019,
            db_guid: 501,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
        &merged_item,
        &enchantments,
    );
    assert_eq!(write.count, 20);
    assert_eq!(write.enchantments, enchantments);
    assert_eq!(write.item_db_guid, 501);

    session.update_inventory_item_object_like_cpp(existing_guid, |item| item.set_count(20));
    let (_, destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(19019, 1, &[], &[])
        .expect("full stack must force the next empty slot");
    assert_eq!(
        destinations[0].pos,
        (u16::from(INVENTORY_SLOT_BAG_0) << 8) | u16::from(INVENTORY_SLOT_ITEM_START + 1)
    );
    let (_, destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(
            19019,
            1,
            &[],
            &[(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
        )
        .expect("a stack planned for deposit must be absent from withdrawal planning");
    assert_eq!(
        destinations[0].pos,
        (u16::from(INVENTORY_SLOT_BAG_0) << 8) | u16::from(INVENTORY_SLOT_ITEM_START),
        "C++ destroys deposits before CanStoreNewItem scans withdrawal destinations"
    );
}

#[test]
fn withdrawal_planner_excludes_slots_from_a_deposited_equipped_bag_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_entry = 21841;
    let item_entry = 19019;
    install_void_test_bag_and_child_templates(&mut session, bag_entry, item_entry);

    for offset in 0..INVENTORY_DEFAULT_SIZE {
        let slot = INVENTORY_SLOT_ITEM_START + offset;
        let guid = ObjectGuid::create_item(1, 600 + i64::from(offset));
        let item = session.make_inventory_item_object(
            guid,
            item_entry,
            player_guid,
            1,
            0,
            ItemContext::None,
            slot,
        );
        session.insert_inventory_item_object(item);
        session.insert_inventory_item_like_cpp(
            slot,
            InventoryItem {
                guid,
                entry_id: item_entry,
                db_guid: guid.counter() as u64,
                inventory_type: Some(InventoryType::NonEquip as u8),
            },
        );
    }

    let bag_slot = wow_entities::INVENTORY_SLOT_BAG_START;
    let bag_guid = ObjectGuid::create_item(1, 700);
    let bag_inventory = InventoryItem {
        guid: bag_guid,
        entry_id: bag_entry,
        db_guid: 700,
        inventory_type: Some(InventoryType::Bag as u8),
    };
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        bag_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        bag_slot,
    );
    session.insert_inventory_item_object(bag_item);
    session.insert_inventory_item_like_cpp(bag_slot, bag_inventory.clone());

    let child_guid = ObjectGuid::create_item(1, 701);
    let mut child_item = session.make_inventory_item_object(
        child_guid,
        item_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        5,
    );
    child_item.set_container_guid_and_slot(bag_guid, bag_slot);
    session.insert_inventory_item_object(child_item);

    let (_, child_only_destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(
            item_entry,
            1,
            &[],
            &[(bag_slot, 5)],
        )
        .expect("child-only snapshot still has the equipped bag");
    let [child_only_bag, _] = child_only_destinations[0].pos.to_be_bytes();
    assert_eq!(
        child_only_bag, bag_slot,
        "the adversarial fixture must expose the orphan-container risk"
    );

    let destroyed = session
        .plan_void_storage_destroyed_items_like_cpp(
            INVENTORY_SLOT_BAG_0,
            bag_slot,
            bag_inventory,
            Vec::new(),
        )
        .expect("fixture canonical inventory owner");
    let vacated_positions = destroyed
        .iter()
        .map(|destroyed| (destroyed.bag, destroyed.slot))
        .collect::<Vec<_>>();
    assert_eq!(
        vacated_positions,
        vec![(bag_slot, 5), (INVENTORY_SLOT_BAG_0, bag_slot)]
    );

    let (result, destinations, no_space_count) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(
            item_entry,
            1,
            &[],
            &vacated_positions,
        )
        .expect("represented inventory planner");
    assert_ne!(result, wow_constants::InventoryResult::Ok);
    assert!(destinations.is_empty());
    assert_eq!(no_space_count, Some(1));
}

#[tokio::test]
async fn nonempty_bag_deposit_plan_destroys_children_before_parent_atomically() {
    let (mut session, _, _) = make_void_storage_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_entry = 21841;
    let child_entry = 19019;
    install_void_test_bag_and_child_templates(&mut session, bag_entry, child_entry);

    let bag_guid = ObjectGuid::create_item(1, 501);
    let bag_slot = wow_entities::INVENTORY_SLOT_BAG_START;
    let bag_inventory = InventoryItem {
        guid: bag_guid,
        entry_id: bag_entry,
        db_guid: 501,
        inventory_type: Some(InventoryType::Bag as u8),
    };
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        bag_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        bag_slot,
    );
    session.insert_inventory_item_object(bag_item);
    session.insert_inventory_item_like_cpp(bag_slot, bag_inventory.clone());

    let child_guid = ObjectGuid::create_item(1, 502);
    let mut child_item = session.make_inventory_item_object(
        child_guid,
        child_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        5,
    );
    child_item.set_container_guid_and_slot(bag_guid, bag_slot);
    session.insert_inventory_item_object(child_item);

    let destroyed = session
        .plan_void_storage_destroyed_items_like_cpp(
            INVENTORY_SLOT_BAG_0,
            bag_slot,
            bag_inventory,
            Vec::new(),
        )
        .expect("fixture canonical inventory owner");
    assert_eq!(
        destroyed
            .iter()
            .map(|item| item.inventory_item.guid)
            .collect::<Vec<_>>(),
        vec![child_guid, bag_guid]
    );

    assert_eq!(
        destroyed
            .iter()
            .map(|item| item.inventory_item.db_guid)
            .collect::<Vec<_>>(),
        vec![502, 501]
    );

    let (destroyed_guids, changed_quest_ids) = session
        .apply_committed_void_storage_destroyed_items_like_cpp(&destroyed)
        .expect("fixture canonical inventory owner");
    assert_eq!(destroyed_guids, vec![child_guid, bag_guid]);
    assert!(changed_quest_ids.is_empty());
    assert!(session.get_inventory_item_by_pos(bag_slot, 5).is_none());
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, bag_slot)
            .is_none()
    );
    assert!(
        !session
            .inventory_item_objects_like_cpp()
            .contains_key(&child_guid)
    );
    assert!(
        !session
            .inventory_item_objects_like_cpp()
            .contains_key(&bag_guid)
    );
}

#[tokio::test]
async fn swap_definite_rollback_keeps_void_slots_unchanged() {
    let (mut session, send_rx, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    let item = represented_void_item(77, 19019);
    assert_eq!(
        session.add_represented_void_storage_item_like_cpp(item.clone()),
        Some(0)
    );

    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "write failed".to_string(),
        },
    ));
    session.set_void_storage_persistence_port_like_cpp(port.clone());

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_packed_guid(&ObjectGuid::create_item(1, 77));
    packet.write_uint32(4);
    session.handle_void_storage_swap_item(packet).await;

    assert_eq!(
        session.represented_void_storage_item_at_like_cpp(0),
        Some(item)
    );
    assert!(
        session
            .represented_void_storage_item_at_like_cpp(4)
            .is_none()
    );
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::VoidTransferResult]
    );
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .begin_like_cpp()
            .is_ok(),
        "definite rollback must reopen payout/save admission"
    );
    assert_eq!(port.swaps.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn swap_unknown_commit_with_unchanged_money_quarantines_session() {
    let (mut session, _, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    let item = represented_void_item(77, 19019);
    assert_eq!(
        session.add_represented_void_storage_item_like_cpp(item.clone()),
        Some(0)
    );
    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
            reason: "commit reply lost".to_string(),
            observed_money: Some(0),
        },
    ));
    session.set_void_storage_persistence_port_like_cpp(port);

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_packed_guid(&ObjectGuid::create_item(1, 77));
    packet.write_uint32(4);
    session.handle_void_storage_swap_item(packet).await;

    assert_eq!(session.state(), SessionState::Disconnecting);
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
    assert_eq!(
        session.represented_void_storage_item_at_like_cpp(0),
        Some(item)
    );
    assert!(
        session
            .represented_void_storage_item_at_like_cpp(4)
            .is_none()
    );
}

#[tokio::test]
async fn deposit_definite_rollback_keeps_money_inventory_and_void_state_unchanged() {
    let (mut session, send_rx, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    install_void_test_item_template(&mut session, 19019);
    session.set_player_gold_like_cpp(500_000);
    let item_guid = ObjectGuid::create_item(1, 501);
    let item = session.make_inventory_item_object(
        item_guid,
        19019,
        ObjectGuid::create_player(1, 42),
        1,
        0,
        ItemContext::None,
        35,
    );
    session.insert_inventory_item_object(item);
    session.insert_inventory_item_like_cpp(
        35,
        InventoryItem {
            guid: item_guid,
            entry_id: 19019,
            db_guid: 501,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );

    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "write failed".to_string(),
        },
    ));
    session.set_void_storage_persistence_port_like_cpp(port.clone());

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_uint32(1);
    packet.write_uint32(0);
    packet.write_packed_guid(&item_guid);
    session.handle_void_storage_transfer(packet).await;

    assert_eq!(session.player_gold_like_cpp(), 500_000);
    let (bag, slot, inventory_item) = session
        .get_inventory_item_by_guid_like_cpp(item_guid)
        .expect("rolled-back deposit must remain in inventory");
    assert_eq!((bag, slot), (INVENTORY_SLOT_BAG_0, 35));
    assert_eq!(inventory_item.guid, item_guid);
    assert_eq!(inventory_item.entry_id, 19019);
    assert_eq!(inventory_item.db_guid, 501);
    assert_eq!(
        session.represented_void_storage_free_slots_like_cpp(),
        Some(160)
    );
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::VoidTransferResult]
    );
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .begin_like_cpp()
            .is_ok(),
        "definite rollback must reopen payout/save admission"
    );
    let requests = port.transfers.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].money_before, 500_000);
    assert_eq!(requests[0].money_after, 400_000);
    assert_eq!(requests[0].deposits.len(), 1);
    assert_eq!(requests[0].deposits[0].destroyed_items[0].item_db_guid, 501);
}

#[tokio::test]
async fn deposit_commits_typed_plan_before_runtime_publication_like_cpp() {
    let (session, send_rx, port, item_guid) = run_one_void_deposit_with_outcome_like_cpp(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed,
    )
    .await;

    assert_eq!(session.player_gold_like_cpp(), 400_000);
    assert!(
        session
            .get_inventory_item_by_guid_like_cpp(item_guid)
            .is_none()
    );
    assert_eq!(
        session
            .represented_void_storage_item_at_like_cpp(0)
            .map(|item| item.item_entry),
        Some(19019)
    );
    assert_eq!(port.transfers.lock().unwrap().len(), 1);
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .filter(|opcode| {
                matches!(
                    opcode,
                    ServerOpcodes::VoidStorageTransferChanges | ServerOpcodes::VoidTransferResult
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::VoidStorageTransferChanges,
            ServerOpcodes::VoidTransferResult,
        ]
    );
}

#[tokio::test]
async fn deposit_unknown_commit_reconciles_from_durable_money_like_cpp() {
    let (session, _, _, item_guid) = run_one_void_deposit_with_outcome_like_cpp(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
            reason: "commit reply lost".to_string(),
            observed_money: Some(400_000),
        },
    )
    .await;

    assert_eq!(session.player_gold_like_cpp(), 400_000);
    assert!(
        session
            .get_inventory_item_by_guid_like_cpp(item_guid)
            .is_none()
    );
    assert_ne!(session.state(), SessionState::Disconnecting);
}

#[tokio::test]
async fn deposit_indeterminate_commit_quarantines_without_runtime_publication_like_cpp() {
    let (session, _, _, item_guid) = run_one_void_deposit_with_outcome_like_cpp(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
            reason: "commit reply lost".to_string(),
            observed_money: None,
        },
    )
    .await;

    assert_eq!(session.player_gold_like_cpp(), 500_000);
    assert!(
        session
            .get_inventory_item_by_guid_like_cpp(item_guid)
            .is_some()
    );
    assert_eq!(
        session.represented_void_storage_free_slots_like_cpp(),
        Some(160)
    );
    assert_eq!(session.state(), SessionState::Disconnecting);
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
}

#[tokio::test]
async fn deposit_definite_rollback_retains_active_item_loot_view_atomically() {
    let (mut session, send_rx, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    install_void_test_item_template_with_stack_and_flags(
        &mut session,
        19019,
        1,
        ItemFlags::HAS_LOOT,
    );
    session.set_player_gold_like_cpp(500_000);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 501);
    let item = session.make_inventory_item_object(
        item_guid,
        19019,
        player_guid,
        1,
        0,
        ItemContext::None,
        35,
    );
    session.insert_inventory_item_object(item);
    session.insert_inventory_item_like_cpp(
        35,
        InventoryItem {
            guid: item_guid,
            entry_id: 19019,
            db_guid: 501,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    session.loot_table.insert(
        item_guid,
        CreatureLoot {
            loot_guid: item_guid,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 1,
                item_id: 19019,
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
    session.set_active_loot_guid(item_guid);

    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "write failed".to_string(),
        },
    ));
    session.set_void_storage_persistence_port_like_cpp(port);

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_uint32(1);
    packet.write_uint32(0);
    packet.write_packed_guid(&item_guid);
    session.handle_void_storage_transfer(packet).await;

    assert!(session.has_active_loot_views_like_cpp());
    assert!(session.loot_table.contains_key(&item_guid));
    assert!(
        session
            .get_inventory_item_by_guid_like_cpp(item_guid)
            .is_some(),
        "the release runs before planning, while definite DB rollback still preserves inventory"
    );
    assert_eq!(
        session.represented_void_storage_free_slots_like_cpp(),
        Some(160)
    );
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::VoidTransferResult]
    );
}

#[test]
fn committed_void_deposit_retires_only_its_destroyed_item_loot_like_cpp() {
    let (mut session, send_rx, _) = make_void_storage_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let destroyed_item = ObjectGuid::create_item(1, 501);
    let unrelated_item = ObjectGuid::create_item(1, 502);
    for item_guid in [destroyed_item, unrelated_item] {
        session.loot_table.insert(
            item_guid,
            CreatureLoot {
                loot_guid: item_guid,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: vec![player_guid],
                items: Vec::new(),
                looted_by_player: false,
            },
        );
        session.add_active_loot_view_owner_like_cpp(item_guid);
    }

    session.retire_committed_destroyed_item_loot_like_cpp(destroyed_item, player_guid);

    assert!(!session.active_loot_view_owners.contains(&destroyed_item));
    assert!(!session.loot_table.contains_key(&destroyed_item));
    assert!(session.active_loot_view_owners.contains(&unrelated_item));
    assert!(session.loot_table.contains_key(&unrelated_item));
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::LootRelease]
    );
}

#[tokio::test]
async fn mixed_transfer_validation_failure_publishes_no_partial_deposit() {
    let (mut session, send_rx, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    install_void_test_item_template(&mut session, 19019);
    session.set_player_gold_like_cpp(500_000);

    let deposit_guid = ObjectGuid::create_item(1, 501);
    let deposit_item = session.make_inventory_item_object(
        deposit_guid,
        19019,
        ObjectGuid::create_player(1, 42),
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(deposit_item);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: deposit_guid,
            entry_id: 19019,
            db_guid: 501,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );

    let unstoreable_void_item = represented_void_item(77, 99999);
    assert_eq!(
        session.add_represented_void_storage_item_like_cpp(unstoreable_void_item.clone()),
        Some(0)
    );
    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed,
    ));
    session.set_void_storage_persistence_port_like_cpp(port.clone());

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_uint32(1);
    packet.write_uint32(1);
    packet.write_packed_guid(&deposit_guid);
    packet.write_packed_guid(&ObjectGuid::create_item(1, 77));
    session.handle_void_storage_transfer(packet).await;

    assert_eq!(session.player_gold_like_cpp(), 500_000);
    assert!(
        session
            .get_inventory_item_by_guid_like_cpp(deposit_guid)
            .is_some(),
        "the deposit remains visible when a later withdrawal cannot be planned"
    );
    assert_eq!(
        session.represented_void_storage_item_at_like_cpp(0),
        Some(unstoreable_void_item)
    );
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::VoidTransferResult]
    );
    assert!(port.transfers.lock().unwrap().is_empty());
}
