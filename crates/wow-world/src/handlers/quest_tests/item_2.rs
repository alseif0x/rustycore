//! Item scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn quest_giver_choose_reward_package_primary_inventory_failure_sends_equip_error_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7010;
    let reward_item_id = 19_024;
    let package_id = 80;
    let limit_category = 46;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9909);
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
            .with_limit_category(limit_category)
            .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_choose_reward_package_fallback_inventory_failure_sends_equip_error_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7011;
    let reward_item_id = 19_025;
    let package_id = 81;
    let limit_category = 47;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9910);
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
            .with_limit_category(limit_category)
            .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_start_quest_no_grant_adds_local_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 90);
    let quest_id = 7012;
    let source_item_id = 9000;
    let source_spell_id = 12_344;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        2,
        source_spell_id,
    )));
    install_source_item_template_with_start_quest(
        &mut session,
        source_item_id,
        20,
        0,
        quest_id as i32,
    );
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(session.player_quests.contains_key(&quest_id));
    assert_eq!(
        session
            .represented_inventory_item_counts_like_cpp()
            .expect("fixture canonical inventory owner")
            .get(&source_item_id)
            .copied()
            .unwrap_or(0),
        0
    );
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStartQuestNoGrant,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::Ok),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: Some(source_spell_id),
            represented_source_spell_self_casts: 2,
        }]
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_with_space_stores_and_pushes_item_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 91);
    let quest_id = 7013;
    let source_item_id = 9001;
    let source_spell_id = 12_346;
    let quest_log_item_id = 9101;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, source_spell_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: quest_log_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.cache_item_template_addon_quest_log_item_id_like_cpp(source_item_id, quest_log_item_id);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("source-item quest should still add local quest state")
            .status,
        QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("source-item quest should still add local quest state")
            .objective_counts,
        vec![2]
    );
    let stored_source_item_count: u32 = session
        .inventory_items_like_cpp()
        .values()
        .filter(|item| item.entry_id == source_item_id)
        .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
        .map(|item| item.count())
        .sum();
    let stored_source_item_slot = session
        .inventory_items_like_cpp()
        .iter()
        .find_map(|(&slot, item)| (item.entry_id == source_item_id).then_some(slot))
        .expect("source item should have a direct inventory slot");
    assert_eq!(stored_source_item_count, 2);
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStoredNewItem,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::Ok),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: Some(source_spell_id),
            represented_source_spell_self_casts: 2,
        }]
    );
    assert_complete_status_update_like_cpp(&session, quest_id, false);
    let mut sent = Vec::new();
    while let Ok(packet) = send_rx.try_recv() {
        sent.push(packet);
    }
    assert!(
        sent.len() >= 3,
        "StoreNewItem(update=true) should create item/update player and SendNewItem"
    );
    let mut saw_item_push = false;
    for bytes in &sent {
        let mut packet = WorldPacket::from_bytes(bytes);
        if packet.read_uint16().unwrap() != wow_constants::ServerOpcodes::ItemPushResult as u16 {
            continue;
        }
        saw_item_push = true;
        assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
        assert_eq!(
            packet.read_uint8().unwrap(),
            u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
        );
        assert_eq!(
            packet.read_int32().unwrap(),
            i32::from(stored_source_item_slot)
        );
        assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 2);
    }
    assert!(
        saw_item_push,
        "source item grant should send ItemPushResult"
    );
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_full_backpack_stores_in_represented_bag_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 98);
    let quest_id = 7020;
    let source_item_id = 9007;
    let bag_item_id = 9107;
    let filler_item_id = 9108;
    let bag_guid = ObjectGuid::create_item(1, 9_107);

    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        3,
        0,
    )));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: source_item_id,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: bag_item_id,
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
        ItemRecord {
            id: filler_item_id,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    let sparse = |inventory_type: InventoryType, stackable: i32, container_slots: u8| {
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable,
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
            container_slots,
            inventory_type: inventory_type as i8,
        }
    };
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (source_item_id, sparse(InventoryType::NonEquip, 20, 0)),
        (bag_item_id, sparse(InventoryType::Bag, 1, 4)),
        (filler_item_id, sparse(InventoryType::NonEquip, 1, 0)),
    ])));

    session.insert_inventory_item_like_cpp(
        wow_entities::INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: bag_item_id,
            db_guid: 9_107,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        bag_item_id,
        receiver_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag_item);
    for slot_offset in 0..wow_entities::INVENTORY_DEFAULT_SIZE {
        insert_direct_inventory_item(
            &mut session,
            receiver_guid,
            wow_entities::INVENTORY_SLOT_ITEM_START + slot_offset,
            filler_item_id,
            1,
            9_200 + u64::from(slot_offset),
        );
    }

    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert!(session.player_quests.contains_key(&quest_id));
    assert!(
        session
            .inventory_items_like_cpp()
            .values()
            .all(|item| item.entry_id != source_item_id)
    );
    let child = session
        .inventory_item_objects_like_cpp()
        .values()
        .find(|item| item.object().entry() == source_item_id)
        .expect("source item should be created inside represented bag");
    assert_eq!(child.container_guid(), bag_guid);
    assert_eq!(child.bag_slot(), wow_entities::INVENTORY_SLOT_BAG_START);
    assert_eq!(child.slot(), 0);
    assert_eq!(child.count(), 3);
    assert_eq!(
        session
            .represented_inventory_item_counts_like_cpp()
            .expect("fixture canonical inventory owner")
            .get(&source_item_id)
            .copied(),
        Some(3)
    );

    let mut saw_item_push = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        if packet.read_uint16().unwrap() != wow_constants::ServerOpcodes::ItemPushResult as u16 {
            continue;
        }
        saw_item_push = true;
        assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
        assert_eq!(
            packet.read_uint8().unwrap(),
            wow_entities::INVENTORY_SLOT_BAG_START
        );
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 3);
        assert_eq!(packet.read_int32().unwrap(), 3);
    }
    assert!(saw_item_push);
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_merges_existing_stack_inside_represented_bag_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 99);
    let quest_id = 7021;
    let source_item_id = 9008;
    let bag_item_id = 9109;
    let filler_item_id = 9110;
    let bag_guid = ObjectGuid::create_item(1, 9_109);
    let child_guid = ObjectGuid::create_item(1, 9_110);

    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        2,
        0,
    )));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: source_item_id,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: bag_item_id,
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
        ItemRecord {
            id: filler_item_id,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    let sparse = |inventory_type: InventoryType, stackable: i32, container_slots: u8| {
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable,
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
            container_slots,
            inventory_type: inventory_type as i8,
        }
    };
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (source_item_id, sparse(InventoryType::NonEquip, 20, 0)),
        (bag_item_id, sparse(InventoryType::Bag, 1, 4)),
        (filler_item_id, sparse(InventoryType::NonEquip, 1, 0)),
    ])));

    session.insert_inventory_item_like_cpp(
        wow_entities::INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: bag_item_id,
            db_guid: 9_109,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        bag_item_id,
        receiver_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag_item);
    let mut child = session.make_inventory_item_object(
        child_guid,
        source_item_id,
        receiver_guid,
        18,
        0,
        ItemContext::None,
        0,
    );
    child.set_container_guid_and_slot(bag_guid, wow_entities::INVENTORY_SLOT_BAG_START);
    session.insert_inventory_item_object(child);
    for slot_offset in 0..wow_entities::INVENTORY_DEFAULT_SIZE {
        insert_direct_inventory_item(
            &mut session,
            receiver_guid,
            wow_entities::INVENTORY_SLOT_ITEM_START + slot_offset,
            filler_item_id,
            1,
            9_300 + u64::from(slot_offset),
        );
    }

    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let child = session
        .inventory_item_objects_like_cpp()
        .get(&child_guid)
        .expect("existing represented bag stack should remain");
    assert_eq!(child.count(), 20);
    assert_eq!(child.container_guid(), bag_guid);
    assert_eq!(child.bag_slot(), wow_entities::INVENTORY_SLOT_BAG_START);
    assert_eq!(child.slot(), 0);
    assert_eq!(
        session
            .represented_inventory_item_counts_like_cpp()
            .expect("fixture canonical inventory owner")
            .get(&source_item_id)
            .copied(),
        Some(20)
    );

    let mut saw_item_push = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        if packet.read_uint16().unwrap() != wow_constants::ServerOpcodes::ItemPushResult as u16 {
            continue;
        }
        saw_item_push = true;
        assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
        assert_eq!(
            packet.read_uint8().unwrap(),
            wow_entities::INVENTORY_SLOT_BAG_START
        );
        assert_eq!(packet.read_int32().unwrap(), -1);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 20);
    }
    assert!(saw_item_push);
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_binds_on_acquire_like_cpp_store_item() {
    let (mut session, _send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 203);
    let quest_id = 7122;
    let source_item_id = 9210;
    let quest = quest_template_with_source_item(quest_id, source_item_id, 1, 0);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template_with_start_quest_limit_category_flags3_and_bonding(
        &mut session,
        source_item_id,
        20,
        0,
        0,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let stored = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == source_item_id)
        .and_then(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
        .expect("source item should be stored as runtime item object");
    assert_eq!(stored.bonding(), ItemBondingType::OnAcquire);
    assert!(stored.is_soul_bound());
    assert_eq!(
        stored.item_flags_bits() & ItemFieldFlags::SOULBOUND.bits(),
        ItemFieldFlags::SOULBOUND.bits()
    );
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_bound_objective_updates_quest_without_creating_item_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 191);
    let quest_id = 7113;
    let source_item_id = 9201;
    let source_spell_id = 12_347;
    let quest_log_item_id = 9301;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, source_spell_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: quest_log_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.cache_item_template_addon_quest_log_item_id_like_cpp(source_item_id, quest_log_item_id);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    let status = session
        .player_quests
        .get(&quest_id)
        .expect("bound source-item quest should still add local quest state");
    assert_eq!(status.status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(status.objective_counts, vec![2]);
    let stored_source_item_count: u32 = session
        .inventory_items_like_cpp()
        .values()
        .filter(|item| item.entry_id == source_item_id)
        .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
        .map(|item| item.count())
        .sum();
    assert_eq!(stored_source_item_count, 0);
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemBoundObjectiveNoGrant,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::Ok),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: Some(source_spell_id),
            represented_source_spell_self_casts: 2,
        }]
    );
    assert_complete_status_update_like_cpp(&session, quest_id, false);

    let sent = send_rx.try_recv().expect("bound item update packet");
    assert!(send_rx.try_recv().is_err());
    let mut packet = WorldPacket::from_bytes(&sent);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
    assert_eq!(
        packet.read_uint8().unwrap(),
        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
    );
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_uint32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert!(!packet.read_bit().unwrap());
    assert!(!packet.read_bit().unwrap());
    assert_eq!(packet.read_bits(3).unwrap(), 3);
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_tracking_event_source_item_objective_auto_rewards_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 194);
    let quest_id = 7119;
    let source_item_id = 9204;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 1, 0);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP;
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: source_item_id as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_complete_status_update_like_cpp(&session, quest_id, false);

    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        if let Some(opcode) = wow_packet::WorldPacket::from_bytes(&bytes).server_opcode() {
            opcodes.push(opcode);
        }
    }
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::QuestGiverQuestComplete));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::QuestUpdateComplete));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::ItemPushResult));
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
