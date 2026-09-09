//! Quest scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn push_quest_to_party_receiver_positive_exclusive_group_active_peer_emits_invalid_pair_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 610);
    let shared_quest_id = 76091;
    let peer_quest_id = 76092;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    shared_quest.exclusive_group = 609;
    let mut peer_quest = quest_template(peer_quest_id);
    peer_quest.exclusive_group = 609;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, peer_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_active_quest(&mut receiver_session, peer_quest_id);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_INVALID_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
            "Quest 76091".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid
                ))
    );
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| outcome.target_guid == Some(receiver_guid) && matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
}
#[tokio::test]
async fn push_quest_to_party_prerequisite_precedes_expansion_gate_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 250);
    let shared_quest_id = 7123;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.prev_quest_id = 9001;
    quest.expansion = 2;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.unregister_from_player_registry();
    receiver_session.expansion = 1;
    receiver_session.register_in_player_registry();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7123".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
            ))
    );
}
#[tokio::test]
async fn push_quest_to_party_grouped_receiver_busy_emits_sender_only_busy_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 45);
    let quest_store = store_with_sharable_quest(7113);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7113);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session
        .set_represented_pending_quest_sharing_like_cpp(ObjectGuid::create_player(1, 77), 9000);

    run_push_quest_to_party(&mut session, 7113).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_BUSY_LIKE_CPP,
            String::new()
        )
    );
    assert!(receiver_rx.try_recv().is_err());
}
#[tokio::test]
async fn push_quest_to_party_grouped_receiver_dead_emits_dead_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 46);
    let quest_store = store_with_sharable_quest(7114);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7114);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_player_alive_like_cpp(false);

    run_push_quest_to_party(&mut session, 7114).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_DEAD_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
            "Quest 7114".to_string()
        )
    );
}
#[tokio::test]
async fn push_quest_to_party_grouped_receiver_dead_observes_runtime_under_map_sync_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 146);
    let quest_store = store_with_sharable_quest(7114);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7114);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_player_health_like_cpp(1_000, 1_000);
    let mut movement_info = wow_packet::packets::movement::MovementInfo::default();
    movement_info.position.z = -501.0;

    let event = receiver_session.handle_under_map_like_cpp(&movement_info);

    assert!(event.is_some());
    assert!(!receiver_session.player_is_alive_like_cpp());
    assert!(
        !player_registry
            .group_presence(receiver_guid)
            .expect("receiver registry snapshot")
            .is_alive
    );
    let health_update = receiver_rx.try_recv().expect("void health update");
    assert_eq!(
        u16::from_le_bytes([health_update[0], health_update[1]]),
        wow_constants::ServerOpcodes::HealthUpdate as u16
    );
    let damage_log = receiver_rx
        .try_recv()
        .expect("void environmental damage log");
    assert_eq!(
        u16::from_le_bytes([damage_log[0], damage_log[1]]),
        wow_constants::ServerOpcodes::EnvironmentalDamageLog as u16
    );

    run_push_quest_to_party(&mut session, 7114).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_DEAD_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response_after_death_sync(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
            "Quest 7114".to_string()
        )
    );
}
#[tokio::test]
async fn push_quest_to_party_missing_group_registry_keeps_explicit_blocker_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid();
    let quest_store = store_with_sharable_quest(7115);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7115);
    session.group_guid = Some(1234);

    run_push_quest_to_party(&mut session, 7115).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7115,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: true,
            receiver_fanout_unrepresented: true,
        }]
    );
    assert!(sender_rx.try_recv().is_err());
}
#[test]
fn push_quest_to_party_registration_and_dispatch_are_wired_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::PushQuestToParty)
        .expect("PushQuestToParty handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_push_quest_to_party");
    assert!(
        QUEST_HANDLER_REGISTRATIONS.contains("session.handle_push_quest_to_party(pkt).await"),
        "the PushQuestToParty registration must carry the call itself"
    );
}
#[test]
fn quest_packet_registration_and_dispatch_are_wired_like_cpp() {
    let cases = [
        (
            ClientOpcodes::QuestGiverQueryQuest,
            "handle_quest_giver_query_quest",
            "session.handle_quest_giver_query_quest(pkt).await",
        ),
        (
            ClientOpcodes::QuestGiverAcceptQuest,
            "handle_quest_giver_accept_quest",
            ".handle_quest_giver_accept_quest_with_generator_like_cpp(",
        ),
        (
            ClientOpcodes::QuestGiverRequestReward,
            "handle_quest_giver_request_reward",
            ".handle_quest_giver_request_reward_with_generator_like_cpp(",
        ),
        (
            ClientOpcodes::QuestGiverCompleteQuest,
            "handle_quest_giver_complete_quest",
            "session.handle_quest_giver_complete_quest(pkt).await",
        ),
        (
            ClientOpcodes::QuestGiverChooseReward,
            "handle_quest_giver_choose_reward",
            ".handle_quest_giver_choose_reward_with_generator_like_cpp(",
        ),
        (
            ClientOpcodes::QueryQuestInfo,
            "handle_query_quest_info",
            "session.handle_query_quest_info(pkt).await",
        ),
    ];
    for (opcode, handler_name, call) in cases {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == opcode)
            .unwrap_or_else(|| panic!("{opcode:?} handler registration"));

        assert_eq!(entry.status, SessionStatus::LoggedIn, "{opcode:?}");
        assert_eq!(entry.processing, PacketProcessing::Inplace, "{opcode:?}");
        assert_eq!(entry.handler_name, handler_name, "{opcode:?}");
        assert!(
            QUEST_HANDLER_REGISTRATIONS.contains(call),
            "{opcode:?} must reach its handler from its own registration"
        );
    }
}
#[tokio::test]
async fn request_world_quest_update_empty_payload_sends_empty_response_like_cpp() {
    let (mut session, send_rx) = make_session();

    run_request_world_quest_update(&mut session).await;

    assert_eq!(recv_world_quest_update_count(&send_rx), 0);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn request_world_quest_update_with_payload_ignores_bytes_and_sends_empty_response_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(1);

    session.handle_request_world_quest_update(pkt).await;

    assert_eq!(recv_world_quest_update_count(&send_rx), 0);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_status_query_missing_noncanonical_guid_sends_no_packet_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[1001])));
    attach_map_manager(&mut session, wow_map::MapManager::default());

    run_status_query(&mut session, creature_guid(9001, 1)).await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_status_query_uses_configured_low_level_hide_diff_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_low_level_hide_diff_like_cpp(5);
    let mut quest = quest_template(1013);
    quest.quest_level = 75;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9013).or_default().push(1013);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9013, 13);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9013);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
}
#[tokio::test]
async fn quest_giver_status_query_uses_configured_high_level_hide_diff_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_high_level_hide_diff_like_cpp(2);
    let mut quest = quest_template(1014);
    quest.quest_level = 85;
    quest.min_level = 83;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9014).or_default().push(1014);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9014, 14);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9014);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
}
#[tokio::test]
async fn quest_giver_status_query_starter_respects_quest_available_conditions_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1008;
    let mut quest = quest_template(quest_id);
    quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9008).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id as i32,
            condition_type: ConditionType::Level,
            condition_value1: 90,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));
    let guid = creature_guid(9008, 8);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9008);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
}
#[tokio::test]
async fn quest_giver_status_query_starter_allows_passing_quest_available_conditions_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1009;
    let mut quest = quest_template(quest_id);
    quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9009).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id as i32,
            condition_type: ConditionType::Level,
            condition_value1: 80,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));
    let guid = creature_guid(9009, 9);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9009);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
}
#[tokio::test]
async fn quest_giver_status_query_starter_allows_objective_progress_condition_like_cpp() {
    let (mut session, send_rx) = make_session();
    let active_quest_id = 1015;
    let starter_quest_id = 1016;
    let active_quest = quest_template_with_objective_count(active_quest_id, 1);
    let objective_id = active_quest.objectives[0].id;
    let mut starter_quest = quest_template(starter_quest_id);
    starter_quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([active_quest, starter_quest]);
    store
        .starter_quests
        .entry(9016)
        .or_default()
        .push(starter_quest_id);
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        active_quest_id,
        PlayerQuestStatus {
            quest_id: active_quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![2],
            slot: 0,
        },
    );
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: starter_quest_id as i32,
            condition_type: ConditionType::QuestObjectiveProgress,
            condition_value1: objective_id,
            condition_value3: 2,
            ..Condition::default()
        }]),
    ));
    let guid = creature_guid(9016, 16);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9016);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
}
#[tokio::test]
async fn quest_giver_status_query_starter_rejects_objective_progress_mismatch_like_cpp() {
    let (mut session, send_rx) = make_session();
    let active_quest_id = 1017;
    let starter_quest_id = 1018;
    let active_quest = quest_template_with_objective_count(active_quest_id, 1);
    let objective_id = active_quest.objectives[0].id;
    let mut starter_quest = quest_template(starter_quest_id);
    starter_quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([active_quest, starter_quest]);
    store
        .starter_quests
        .entry(9018)
        .or_default()
        .push(starter_quest_id);
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        active_quest_id,
        PlayerQuestStatus {
            quest_id: active_quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![2],
            slot: 0,
        },
    );
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: starter_quest_id as i32,
            condition_type: ConditionType::QuestObjectiveProgress,
            condition_value1: objective_id,
            condition_value3: 3,
            ..Condition::default()
        }]),
    ));
    let guid = creature_guid(9018, 18);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9018);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
}
#[tokio::test]
async fn quest_giver_status_query_important_starter_uses_quest_info_modifiers_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1010;
    let mut quest = quest_template(quest_id);
    quest.quest_level = 80;
    quest.quest_info_id = 710;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9010).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
        quest_info_entry_like_cpp(710, 2, 0x400),
    ])));
    let guid = creature_guid(9010, 10);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9010);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (guid, quest_giver_status::IMPORTANT_QUEST)
    );
}
#[tokio::test]
async fn quest_giver_status_query_important_low_level_uses_future_important_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1011;
    let mut quest = quest_template(quest_id);
    quest.quest_level = 85;
    quest.min_level = 85;
    quest.quest_info_id = 711;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9011).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
        quest_info_entry_like_cpp(711, 2, 0x400),
    ])));
    let guid = creature_guid(9011, 11);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9011);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (guid, quest_giver_status::FUTURE_IMPORTANT_QUEST)
    );
}
#[tokio::test]
async fn quest_giver_status_query_covenant_completed_ender_uses_quest_info_tag_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1012;
    let mut quest = quest_template(quest_id);
    quest.quest_info_id = 712;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.ender_quests.entry(9012).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
        quest_info_entry_like_cpp(712, 15, 0),
    ])));
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
    let guid = creature_guid(9012, 12);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9012);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (
            guid,
            quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_POI
        )
    );
}
#[tokio::test]
async fn quest_giver_status_multiple_empty_visible_set_sends_zero_count_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[2001])));
    attach_map_manager(&mut session, wow_map::MapManager::default());

    session.handle_quest_giver_status_multiple_query().await;

    assert!(recv_status_multiple(&send_rx).is_empty());
}
#[tokio::test]
async fn quest_giver_close_active_existing_template_records_acknowledge_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[5901])));
    add_active_quest(&mut session, 5901);

    run_close_quest(&mut session, 5901).await;

    assert_eq!(
        session.represented_auto_accept_acknowledged_quests_like_cpp,
        vec![5901]
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_close_missing_active_quest_records_no_acknowledge_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[5902])));

    run_close_quest(&mut session, 5902).await;

    assert!(
        session
            .represented_auto_accept_acknowledged_quests_like_cpp
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_close_missing_template_records_no_acknowledge_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[5904])));
    add_active_quest(&mut session, 5903);

    run_close_quest(&mut session, 5903).await;

    assert!(
        session
            .represented_auto_accept_acknowledged_quests_like_cpp
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_close_short_packet_records_no_acknowledge_and_sends_no_packet_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[5905])));
    add_active_quest(&mut session, 5905);

    session
        .handle_quest_giver_close_quest(WorldPacket::from_bytes(&[0x05, 0x17]))
        .await;

    assert!(
        session
            .represented_auto_accept_acknowledged_quests_like_cpp
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_log_remove_short_packet_does_not_remove_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5911, 0);

    session
        .handle_quest_log_remove_quest(WorldPacket::from_bytes(&[]))
        .await;

    assert!(session.player_quests.contains_key(&5911));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(0), Some(5911));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_log_remove_slot_outside_max_does_not_remove_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5912, 0);

    run_remove_quest_slot(&mut session, 25).await;

    assert!(session.player_quests.contains_key(&5912));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(0), Some(5912));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_log_remove_valid_slot_removes_only_that_slot_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 880_001, 7);
    add_active_quest_in_slot(&mut session, 17, 3);

    run_remove_quest_slot(&mut session, 7).await;

    assert!(!session.player_quests.contains_key(&880_001));
    assert!(session.player_quests.contains_key(&17));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(7), None);
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(3), Some(17));

    let update = send_rx
        .try_recv()
        .expect("C++ SetQuestSlot(slot, 0) must become an immediate player UpdateObject");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
    assert!(
        !update
            .windows(std::mem::size_of::<u32>())
            .any(|window| window == 880_001_u32.to_le_bytes()),
        "quest-log abandon UpdateObject should clear the removed QuestID"
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_log_remove_empty_valid_slot_does_not_remove_other_quest_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5914, 4);

    run_remove_quest_slot(&mut session, 3).await;

    assert!(session.player_quests.contains_key(&5914));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(4), Some(5914));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(3), None);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_log_remove_duplicate_slot_fails_closed_and_removes_none_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5915, 2);
    add_active_quest_in_slot(&mut session, 5916, 2);

    run_remove_quest_slot(&mut session, 2).await;

    assert!(session.player_quests.contains_key(&5915));
    assert!(session.player_quests.contains_key(&5916));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(2), None);
    assert_eq!(session.first_free_quest_slot_like_cpp(), Some(0));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_log_create_entries_preserve_explicit_slot_holes_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5916, 9);
    add_active_quest_in_slot(&mut session, 5915, 2);

    let entries = session.quest_log_create_entries_like_cpp();

    assert_eq!(entries.len(), MAX_QUEST_LOG_SIZE_LIKE_CPP as usize);
    assert_eq!(entries[0], (0, 0, 0, [0; 24]));
    assert_eq!(entries[2].0, 5915);
    assert_eq!(entries[9].0, 5916);
}
#[test]
fn quest_log_create_entries_preserve_end_time_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5917, 4);
    session
        .player_quests
        .get_mut(&5917)
        .expect("active quest")
        .end_time_secs = 123_456;

    let entries = session.quest_log_create_entries_like_cpp();

    assert_eq!(entries[4].2, 123_456);
}
#[test]
fn save_to_db_quest_status_list_includes_active_quests_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot_with_status(&mut session, 5920, 2, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    add_active_quest_in_slot_with_status(&mut session, 5919, 3, QUEST_STATUS_COMPLETE_LIKE_CPP);

    assert_eq!(
        session.represented_quest_statuses_for_save_like_cpp(),
        vec![
            (5919, QUEST_STATUS_COMPLETE_LIKE_CPP),
            (5920, QUEST_STATUS_INCOMPLETE_LIKE_CPP)
        ],
        "C++ Player::SaveToDB reaches _SaveQuestStatus for represented active quests"
    );
}
