//! Receiver quest status, log capacity and daily/DF cooldown admission.

use super::*;

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_on_quest_emits_on_quest_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 43);
    let quest_store = store_with_sharable_quest(7111);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7111);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_active_quest(&mut receiver_session, 7111);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, 7111).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP,
            "Quest 7111".to_string()
        )
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_rewarded_emits_already_done_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 44);
    let quest_store = store_with_sharable_quest(7112);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7112);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, 7112);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, 7112).await;

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
            "Quest 7112".to_string()
        )
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_log_full_emits_log_full_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 144);
    let shared_quest_id = 7116;
    let quest_store = store_with_sharable_quest(shared_quest_id);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    for slot in 0..MAX_QUEST_LOG_SIZE_LIKE_CPP {
        add_active_quest_in_slot(&mut receiver_session, 8000 + u32::from(slot), slot);
    }
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
            "Quest 7116".to_string()
        )
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_daily_completed_emits_already_done_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 244);
    let shared_quest_id = 7117;
    let quest_store = store_with_daily_sharable_quests(&[shared_quest_id]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
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
            "Quest 7117".to_string()
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
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_df_completed_emits_already_done_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 245);
    let shared_quest_id = 7118;
    let quest_store = store_with_df_sharable_quest(shared_quest_id);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_represented_df_quest_for_test(&mut receiver_session, shared_quest_id, true);
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
            "Quest 7118".to_string()
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
}

#[tokio::test]
async fn push_quest_to_party_non_daily_non_df_ignores_unrelated_daily_snapshot_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 246);
    let shared_quest_id = 7119;
    let quest_store = store_with_sharable_quest(shared_quest_id);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_represented_daily_quest_completed_for_test(&mut receiver_session, 9001, true);
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
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_df_daily_reads_df_bucket_before_registered_mailbox_delivery() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 390);
    let shared_quest_id = 7190;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.special_flags |= QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    set_represented_daily_quest_completed_for_test(&mut receiver_session, shared_quest_id, true);
    assert!(contains_daily_quest_completed_for_test(&receiver_session, shared_quest_id));
    assert!(!contains_df_quest_for_test(&receiver_session, shared_quest_id));
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
    assert!(represented_push_quest_to_party_outcomes_for_test(&session)
        .iter()
        .any(|outcome| matches!(
            outcome.reason,
            RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
        )));
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session)
        .iter()
        .any(|outcome| matches!(
            outcome.reason,
            RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
        )));
}
