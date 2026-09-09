//! Quest scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn push_quest_to_party_inactive_pooled_quest_records_not_daily_before_group_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    let quest_store = store_with_daily_sharable_quests(&[7106, 7107]);
    let quest_pool_store =
        quest_pool_store_with_active_saved(&quest_store, 77, &[7106, 7107], &[7107]);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7106);
    session.group_guid = Some(99);

    run_push_quest_to_party(&mut session, 7106).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7106,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotDaily,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: false,
            receiver_fanout_unrepresented: false,
        }]
    );
    assert_eq!(
        recv_push_quest_result_response(&send_rx),
        (
            sender_guid.expect("test session has player guid"),
            quest_push_reason::NOT_DAILY,
            String::new()
        )
    );
}
#[tokio::test]
async fn push_quest_to_party_active_pooled_quest_passes_pool_check_to_not_in_party_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    let quest_store = store_with_daily_sharable_quests(&[7108, 7109]);
    let quest_pool_store =
        quest_pool_store_with_active_saved(&quest_store, 78, &[7108, 7109], &[7108]);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7108);

    run_push_quest_to_party(&mut session, 7108).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7108,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotInParty,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: false,
            receiver_fanout_unrepresented: false,
        }]
    );
    assert_eq!(
        recv_push_quest_result_response(&send_rx),
        (
            sender_guid.expect("test session has player guid"),
            quest_push_reason::NOT_IN_PARTY,
            String::new()
        )
    );
}
#[tokio::test]
async fn push_quest_to_party_non_pooled_quest_passes_pool_check_to_group_boundary_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    let quest_store = store_with_sharable_quest(7110);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7110);
    session.group_guid = Some(99);

    run_push_quest_to_party(&mut session, 7110).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7110,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: true,
            receiver_fanout_unrepresented: true,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}
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
    receiver_session.sync_player_registry_state_like_cpp();

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
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.sync_player_registry_state_like_cpp();

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
    receiver_session.sync_player_registry_state_like_cpp();

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
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_represented_daily_quest_completed_like_cpp_for_test(shared_quest_id, true);
    receiver_session.sync_player_registry_state_like_cpp();

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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_represented_df_quest_like_cpp_for_test(shared_quest_id, true);
    receiver_session.sync_player_registry_state_like_cpp();

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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_represented_daily_quest_completed_like_cpp_for_test(9001, true);
    receiver_session.sync_player_registry_state_like_cpp();

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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
            outcome.reason,
            RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
        ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
    );
}
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
    receiver_session.set_player_level_like_cpp(4);
    receiver_session.sync_player_registry_state_like_cpp();

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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_player_level_like_cpp(80);
    receiver_session.sync_player_registry_state_like_cpp();

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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_player_level_like_cpp(80);
    receiver_session.sync_player_registry_state_like_cpp();

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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();
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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();
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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();

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
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();

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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.sync_player_registry_state_like_cpp();
    set_canonical_party_reputation_like_cpp(
        receiver_session
            .canonical_map_manager
            .as_ref()
            .expect("canonical map manager"),
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
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
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
    receiver_session.sync_player_registry_state_like_cpp();
    set_canonical_party_reputation_like_cpp(
        receiver_session
            .canonical_map_manager
            .as_ref()
            .expect("canonical map manager"),
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
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)));
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
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)));
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
    receiver_session.sync_player_registry_state_like_cpp();

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
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
}
