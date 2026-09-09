//! Item scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn quest_confirm_accept_source_item_bound_objective_dont_report_flag_sends_direct_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 195);
    let other_guid = ObjectGuid::create_player(1, 196);
    let quest_id = 7116;
    let source_item_id = 9204;
    let quest_log_item_id = 9304;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
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
    install_source_item_template_with_flags3(
        &mut session,
        source_item_id,
        20,
        0,
        ItemFlags3::DontReportLootLogToParty as u32,
    );
    session.cache_item_template_addon_quest_log_item_id_like_cpp(source_item_id, quest_log_item_id);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_loaded_player_name_like_cpp("Receiver".to_string());
    session.register_in_player_registry();

    let (mut sender_session, sender_rx) = make_session();
    sender_session.set_player_guid(Some(sender_guid));
    sender_session.set_loaded_player_name_like_cpp("Sender".to_string());
    sender_session.set_player_registry(Arc::clone(&player_registry));
    sender_session.register_in_player_registry();
    assert!(sender_session.adopt_registered_canonical_player_fixture_like_cpp());
    add_active_quest_in_slot_with_status(
        &mut sender_session,
        quest_id,
        0,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    sender_session.sync_player_registry_state_like_cpp();

    let (mut other_session, other_rx) = make_session();
    other_session.set_player_guid(Some(other_guid));
    other_session.set_loaded_player_name_like_cpp("Other".to_string());
    other_session.set_player_registry(player_registry);
    other_session.register_in_player_registry();

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender_guid);
    group.add_member(receiver_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let sent = send_rx.try_recv().expect("direct bound item update packet");
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
    assert!(other_rx.try_recv().is_err());
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
}
#[tokio::test]
async fn quest_confirm_accept_source_item_multiple_bound_objectives_stops_after_first_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 192);
    let quest_id = 7114;
    let source_item_id = 9202;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: source_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: source_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
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

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-item quest should add local quest state");
    assert_eq!(status.objective_counts, vec![2, 0]);
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
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    let sent: Vec<_> = std::iter::from_fn(|| send_rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|bytes| {
            let mut packet = WorldPacket::from_bytes(bytes);
            packet.read_uint16().ok() == Some(wow_constants::ServerOpcodes::ItemPushResult as u16)
        }),
        "C++ sends the bound-objective ItemPushResult without materializing an inventory Item"
    );
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_sequenced_objective_waits_for_previous_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 197);
    let quest_id = 7117;
    let source_item_id = 9205;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 9901,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: source_item_id as i32,
        amount: 2,
        flags: QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
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

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-item quest should add local quest state");
    assert_eq!(status.objective_counts, vec![0, 0]);
    let stored_source_item_count: u32 = session
        .inventory_items_like_cpp()
        .values()
        .filter(|item| item.entry_id == source_item_id)
        .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
        .map(|item| item.count())
        .sum();
    assert_eq!(stored_source_item_count, 2);
    let sent: Vec<_> = std::iter::from_fn(|| send_rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|bytes| {
            let mut packet = WorldPacket::from_bytes(bytes);
            packet.read_uint16().ok() == Some(wow_constants::ServerOpcodes::ItemPushResult as u16)
        }),
        "source item is still granted; only sequenced objective progress is blocked"
    );
    assert!(sender_rx.try_recv().is_err());
    assert_eq!(session.player_guid(), Some(receiver_guid));
}
#[tokio::test]
async fn quest_confirm_accept_source_item_optional_previous_allows_sequenced_objective_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 198);
    let quest_id = 7118;
    let source_item_id = 9206;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 9902,
        amount: 1,
        flags: QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: source_item_id as i32,
        amount: 2,
        flags: QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
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

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-item quest should add local quest state");
    assert_eq!(status.objective_counts, vec![0, 2]);
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_progress_bar_part_objective_progresses_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 199);
    let quest_id = 7119;
    let source_item_id = 9207;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: source_item_id as i32,
        amount: 2,
        flags: QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL,
        flags2: 0,
        progress_bar_weight: 50.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: 0,
        amount: 100,
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

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-item quest should add local quest state");
    assert_eq!(status.objective_counts, vec![2, 0]);
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_zero_count_normalizes_to_one_and_fails_full_inventory_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 96);
    let quest_id = 7018;
    let source_item_id = 9005;
    let filler_item_id = 9105;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        0,
        0,
    )));
    install_source_item_template(&mut session, source_item_id, 1, 0);
    for slot in 35..59 {
        insert_direct_inventory_item(
            &mut session,
            receiver_guid,
            slot,
            filler_item_id,
            1,
            91_000 + u64::from(slot),
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

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(!session.player_quests.contains_key(&quest_id));
    let outcomes = session.represented_quest_confirm_accepts_like_cpp();
    assert_eq!(outcomes.len(), 1);
    let outcome = &outcomes[0];
    assert_eq!(outcome.receiver_guid, Some(receiver_guid));
    assert_eq!(outcome.sender_guid_before_clear, sender_guid);
    assert_eq!(outcome.quest_id, quest_id);
    assert_eq!(outcome.raw_quest_id, quest_id as i32);
    assert_eq!(
        outcome.reason,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed
    );
    assert!(!outcome.add_quest_runtime_unrepresented);
    let source_item_result = outcome
        .can_add_source_item_result
        .expect("zero ProvidedItemCount must normalize to one and reach planner failure");
    assert_ne!(source_item_result, InventoryResult::Ok);
    assert_ne!(source_item_result, InventoryResult::ItemMaxCount);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(source_item_result).to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_at_max_count_allows_can_add_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 92);
    let quest_id = 7014;
    let source_item_id = 9002;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        1,
        0,
    )));
    install_source_item_template(&mut session, source_item_id, 20, 1);
    insert_direct_inventory_item(&mut session, receiver_guid, 23, source_item_id, 1, 9002);
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
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemMaxCountNoGrant,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::ItemMaxCount),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_missing_source_item_proto_fails_can_add_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 93);
    let quest_id = 7015;
    let source_item_id = 9003;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        1,
        0,
    )));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason:
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::ItemNotFound),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::ItemNotFound).to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_limit_category_missing_db2_entry_fails_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 95);
    let quest_id = 7017;
    let source_item_id = 9004;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        1,
        0,
    )));
    install_source_item_template_with_limit_category(&mut session, source_item_id, 20, 0, 44);
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
    assert!(!session.player_quests.contains_key(&quest_id));
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason:
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::NotEquippable),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::NotEquippable).to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_source_item_start_quest_still_respects_limit_category_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 97);
    let quest_id = 7019;
    let source_item_id = 9006;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        1,
        0,
    )));
    install_source_item_template_with_start_quest_and_limit_category(
        &mut session,
        source_item_id,
        20,
        0,
        quest_id as i32,
        44,
    );
    session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
        ItemLimitCategoryEntry {
            id: 44,
            name: "Quest Source Have Limit".into(),
            quantity: 1,
            flags: ITEM_LIMIT_CATEGORY_MODE_HAVE,
        },
    ])));
    insert_direct_inventory_item(&mut session, receiver_guid, 23, source_item_id, 1, 9906);
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
    assert!(!session.player_quests.contains_key(&quest_id));
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason:
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(
                InventoryResult::ItemMaxLimitCategoryCountExceededIs
            ),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
            .with_limit_category(44)
            .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_without_source_item_does_not_overclaim_source_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 94);
    let quest_id = 7016;
    session.set_quest_store(Arc::new(store_with_source_item_quest(quest_id, 0, 0, 0)));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
    );
    assert!(session.player_quests.contains_key(&quest_id));
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("no-objective shared quest should be locally tracked")
            .status,
        QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert_complete_status_update_like_cpp(&session, quest_id, false);
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[test]
fn quest_push_inventory_registration_and_dispatcher_contract_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::QuestPushResult)
        .expect("QuestPushResult handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_quest_push_result");
    assert!(
        QUEST_HANDLER_REGISTRATIONS.contains("session.handle_quest_push_result(pkt).await"),
        "the QuestPushResult registration must carry the call itself"
    );
}
#[tokio::test]
async fn push_quest_to_party_repeatable_turn_in_success_prompts_request_items_without_pending_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 611);
    let shared_quest_id = 76110;
    let mut quest = quest_template(shared_quest_id);
    quest.quest_type = 0;
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.special_flags |= 0x0000_0001;
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id: shared_quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 49211,
        amount: 3,
        flags: 0xA5,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let quest_for_assertion = quest.clone();
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
            String::new()
        )
    );
    assert!(receiver_rx.try_recv().is_err());
    let commands = receiver_session.drain_session_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        SessionCommand::SendRepeatableTurnInRequestItemsLikeCpp(command) => {
            assert_eq!(command.sender_guid, sender_guid);
            assert_eq!(command.quest.id, shared_quest_id);
        }
        other => panic!("unexpected session command: {other:?}"),
    }
    receiver_session
        .session_command_tx()
        .try_send(commands.into_iter().next().expect("command"))
        .expect("requeue command for processing");
    receiver_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        receiver_session.represented_pending_quest_sharing_like_cpp(),
        None
    );
    let (collect, auto_launched) =
        recv_quest_giver_request_items_like_cpp(&receiver_rx, shared_quest_id);
    assert_eq!(collect, vec![(49211, 3, 0xA5)]);
    assert!(auto_launched);
    assert!(
        !receiver_session
            .can_complete_repeatable_quest_represented_bounded_like_cpp(&quest_for_assertion)
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
        |outcome| outcome.target_guid == Some(receiver_guid)
            && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPrompted
            )
            && !outcome.receiver_fanout_unrepresented
    ));
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
}
#[test]
fn request_world_quest_update_inventory_entry_matches_cpp_status_and_processing() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::RequestWorldQuestUpdate)
        .expect("RequestWorldQuestUpdate handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_request_world_quest_update");
}
#[tokio::test]
async fn quest_giver_status_query_unsupported_player_or_item_guid_sends_no_packet_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[1001])));
    attach_map_manager(&mut session, wow_map::MapManager::default());

    run_status_query(&mut session, ObjectGuid::create_player(1, 99)).await;
    run_status_query(&mut session, ObjectGuid::create_item(1, 100)).await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_status_multiple_skips_missing_player_item_and_non_questgiver_go_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[2004]);
    store.starter_quests.entry(9204).or_default().push(2004);
    assert!(store.insert_gameobject_starter_relation_like_cpp(9204, 2004));
    session.set_quest_store(Arc::new(store));
    let accepted_guid = creature_guid(9204, 204);
    let missing_guid = creature_guid(9204, 205);
    let player_guid = ObjectGuid::create_player(1, 204);
    let item_guid = ObjectGuid::create_item(1, 204);
    let non_questgiver_go = gameobject_guid(9204, 206);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, accepted_guid, 9204);
    insert_gameobject(&mut manager, non_questgiver_go, 9204);
    attach_map_manager(&mut session, manager);
    for guid in [
        accepted_guid,
        missing_guid,
        player_guid,
        item_guid,
        non_questgiver_go,
    ] {
        mark_visible(&mut session, guid);
    }
    let mut state = crate::session::RepresentedGameObjectUseState::default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
    session
        .represented_gameobject_use_states
        .insert(non_questgiver_go, state);

    session.handle_quest_giver_status_multiple_query().await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(accepted_guid, quest_giver_status::TRIVIAL)]
    );
}
#[test]
fn quest_giver_close_inventory_registration_matches_dispatch_contract_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::QuestGiverCloseQuest)
        .expect("QuestGiverCloseQuest handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_quest_giver_close_quest");
}
