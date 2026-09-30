//! Quest-share receiver gate order and synchronized eligibility facts.

use super::*;

#[tokio::test]
async fn push_quest_to_party_reputation_precedes_previous_prerequisite_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 263);
    let shared_quest_id = 7136;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.required_min_rep_faction = 72;
    quest.required_min_rep_value = 100;
    quest.prev_quest_id = 9005;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    sync_player_registry_state_for_test(&receiver_session);
    set_canonical_party_reputation_like_cpp(
        canonical_map_manager_for_test(&receiver_session).expect("canonical map manager"),
        receiver_guid,
        72,
        99,
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
            "Quest 7136".to_string()
        )
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_class_precedes_reputation_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 258);
    let shared_quest_id = 7131;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.allowable_classes = 1 << (2 - 1);
    quest.required_min_rep_faction = 72;
    quest.required_min_rep_value = 100;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_loaded_player_identity_like_cpp(&mut receiver_session, 571, 1, 1, 80, 0);
    sync_player_registry_state_for_test(&receiver_session);
    set_canonical_party_reputation_like_cpp(
        canonical_map_manager_for_test(&receiver_session).expect("canonical map manager"),
        receiver_guid,
        72,
        -42000,
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_CLASS_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
            "Quest 7131".to_string()
        )
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass)));
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
}

#[tokio::test]
async fn push_quest_to_party_daily_precedes_low_level_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 251);
    let shared_quest_id = 7124;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.min_level = 80;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_player_level_for_test(&mut receiver_session, 1);
    set_represented_daily_quest_completed_for_test(&mut receiver_session, shared_quest_id, true);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7124".to_string()
        )
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_receiver_level_snapshot_syncs_from_world_session_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 252);
    let shared_quest_id = 7125;
    let quest_store = store_with_sharable_quest_levels(shared_quest_id, 20, 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    assert_eq!(
        player_registry
            .group_presence(receiver_guid)
            .map(|presence| presence.level),
        Some(80)
    );
    set_player_level_for_test(&mut receiver_session, 19);
    sync_player_registry_state_for_test(&receiver_session);
    assert_eq!(
        player_registry
            .group_presence(receiver_guid)
            .map(|presence| presence.level),
        Some(19)
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP,
            "Quest 7125".to_string()
        )
    );
}

#[tokio::test]
async fn push_quest_to_party_log_full_precedes_daily_completed_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 247);
    let shared_quest_id = 7120;
    let quest_store = store_with_daily_sharable_quests(&[shared_quest_id]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    for slot in 0..MAX_QUEST_LOG_SIZE_LIKE_CPP {
        add_active_quest_in_slot(&mut receiver_session, 8100 + u32::from(slot), slot);
    }
    set_represented_daily_quest_completed_for_test(&mut receiver_session, shared_quest_id, true);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP,
            "Quest 7120".to_string()
        )
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull
            ))
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_low_receiver_expansion_emits_expansion_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 248);
    let shared_quest_id = 7121;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.expansion = 2;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    unregister_from_player_registry_for_test(&receiver_session);
    receiver_session.expansion = 1;
    register_in_player_registry_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_EXPANSION_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP,
            "Quest 7121".to_string()
        )
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
            ))
    );
}
