//! Quest pool admission before party resolution.

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
    set_group_guid_for_test_like_cpp(&mut session, Some(99));

    run_push_quest_to_party(&mut session, 7106).await;

    assert_eq!(
        represented_push_quest_to_party_outcomes_for_test(&session),
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
        represented_push_quest_to_party_outcomes_for_test(&session),
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
    set_group_guid_for_test_like_cpp(&mut session, Some(99));

    run_push_quest_to_party(&mut session, 7110).await;

    assert_eq!(
        represented_push_quest_to_party_outcomes_for_test(&session),
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
