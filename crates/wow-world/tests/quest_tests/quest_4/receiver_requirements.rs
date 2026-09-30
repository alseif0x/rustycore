//! Receiver level, identity, reputation and previous-quest requirements.

use super::*;

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_low_level_emits_low_level_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 248);
    let shared_quest_id = 7121;
    let quest_store = store_with_sharable_quest_levels(shared_quest_id, 10, 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_player_level_for_test(&mut receiver_session, 4);
    sync_player_registry_state_for_test(&receiver_session);

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
            "Quest 7121".to_string()
        )
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel
            ))
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_high_level_emits_high_level_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 249);
    let shared_quest_id = 7122;
    let quest_store = store_with_sharable_quest_levels(shared_quest_id, 1, 40);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_player_level_for_test(&mut receiver_session, 80);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP,
            "Quest 7122".to_string()
        )
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel
            ))
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_receiver_max_level_zero_does_not_block_high_level_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 250);
    let shared_quest_id = 7123;
    let quest_store = store_with_sharable_quest_levels(shared_quest_id, 1, 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_player_level_for_test(&mut receiver_session, 80);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_wrong_class_emits_class_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 251);
    let shared_quest_id = 7124;
    let quest_store = store_with_sharable_quest_class_race(shared_quest_id, 1 << (2 - 1), 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_loaded_player_identity_like_cpp(&mut receiver_session, 571, 1, 1, 80, 0);
    sync_player_registry_state_for_test(&receiver_session);
    assert_eq!(
        player_registry
            .quest_sharing_snapshot(receiver_guid, None)
            .expect("receiver snapshot")
            .class,
        1
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
            "Quest 7124".to_string()
        )
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_wrong_race_emits_race_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 252);
    let shared_quest_id = 7125;
    let quest_store = store_with_sharable_quest_class_race(shared_quest_id, 0, 1 << (2 - 1));
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_loaded_player_identity_like_cpp(&mut receiver_session, 571, 1, 1, 80, 0);
    sync_player_registry_state_for_test(&receiver_session);
    assert_eq!(
        player_registry
            .quest_sharing_snapshot(receiver_guid, None)
            .expect("receiver snapshot")
            .race,
        1
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_RACE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7125".to_string()
        )
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_receiver_class_precedes_race_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 253);
    let shared_quest_id = 7126;
    let quest_store =
        store_with_sharable_quest_class_race(shared_quest_id, 1 << (2 - 1), 1 << (2 - 1));
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_loaded_player_identity_like_cpp(&mut receiver_session, 571, 1, 1, 80, 0);
    sync_player_registry_state_for_test(&receiver_session);

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
            "Quest 7126".to_string()
        )
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_zero_class_and_race_masks_do_not_block_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 254);
    let shared_quest_id = 7127;
    let quest_store = store_with_sharable_quest_class_race(shared_quest_id, 0, 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_loaded_player_identity_like_cpp(&mut receiver_session, 571, 1, 1, 80, 0);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass
                    | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_low_min_reputation_emits_low_faction_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 255);
    let shared_quest_id = 7128;
    let quest_store = store_with_sharable_quest_reputation(shared_quest_id, 72, 100, 0, 0);
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
            "Quest 7128".to_string()
        )
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_equal_max_reputation_emits_low_faction_pair_like_cpp()
{
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 256);
    let shared_quest_id = 7129;
    let quest_store = store_with_sharable_quest_reputation(shared_quest_id, 0, 0, 72, 100);
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
        100,
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
            "Quest 7129".to_string()
        )
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)));
}

#[tokio::test]
async fn push_quest_to_party_zero_reputation_factions_do_not_block_with_missing_snapshot_like_cpp()
{
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 257);
    let shared_quest_id = 7130;
    let quest_store = store_with_sharable_quest_reputation(shared_quest_id, 0, 999, 0, -1);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)));
}

#[tokio::test]
async fn push_quest_to_party_positive_prev_missing_rewarded_emits_prerequisite_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 259);
    let shared_quest_id = 7132;
    let quest_store = store_with_sharable_quest_previous(shared_quest_id, 9001);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    sync_player_registry_state_for_test(&receiver_session);

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
            "Quest 7132".to_string()
        )
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
}
