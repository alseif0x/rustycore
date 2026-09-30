//! Quest-share previous and breadcrumb prerequisite publication.

use super::*;

#[tokio::test]
async fn push_quest_to_party_positive_prev_rewarded_passes_to_unrepresented_boundary_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 260);
    let shared_quest_id = 7133;
    let quest_store = store_with_sharable_quest_previous(shared_quest_id, 9002);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, 9002);
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
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_negative_prev_missing_active_incomplete_emits_prerequisite_pair_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 261);
    let shared_quest_id = 7134;
    let quest_store = store_with_sharable_quest_previous(shared_quest_id, -9003);
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
            "Quest 7134".to_string()
        )
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_negative_prev_active_incomplete_passes_to_unrepresented_boundary_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 262);
    let shared_quest_id = 7135;
    let quest_store = store_with_sharable_quest_previous(shared_quest_id, -9004);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_active_quest(&mut receiver_session, 9004);
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
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_dependent_previous_missing_rewarded_emits_prerequisite_pair_like_cpp()
{
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 470);
    let shared_quest_id = 7607;
    let prev_id = 9607;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut previous_quest = quest_template(prev_id);
    previous_quest.next_quest_id = shared_quest_id;
    previous_quest.exclusive_group = 0;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, previous_quest]);
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
            "Quest 7607".to_string()
        )
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_dependent_previous_rewarded_nonnegative_group_passes_to_unrepresented_boundary_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 471);
    let shared_quest_id = 7608;
    let prev_id = 9608;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut previous_quest = quest_template(prev_id);
    previous_quest.next_quest_id = shared_quest_id;
    previous_quest.exclusive_group = 0;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, previous_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, prev_id);
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
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_dependent_previous_negative_exclusive_group_requires_all_other_members_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 472);
    let shared_quest_id = 7609;
    let prev_id = 9609;
    let sibling_id = 9610;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut previous_quest = quest_template(prev_id);
    previous_quest.next_quest_id = shared_quest_id;
    previous_quest.exclusive_group = -90;
    let mut sibling_quest = quest_template(sibling_id);
    sibling_quest.exclusive_group = -90;
    let quest_store =
        QuestStore::from_quests_like_cpp([shared_quest, previous_quest, sibling_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, prev_id);
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
            "Quest 7609".to_string()
        )
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));

    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut previous_quest = quest_template(prev_id);
    previous_quest.next_quest_id = shared_quest_id;
    previous_quest.exclusive_group = -90;
    let mut sibling_quest = quest_template(sibling_id);
    sibling_quest.exclusive_group = -90;
    let quest_store =
        QuestStore::from_quests_like_cpp([shared_quest, previous_quest, sibling_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, prev_id);
    add_rewarded_quest(&mut receiver_session, sibling_id);
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
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_dependent_breadcrumb_active_status_emits_prerequisite_and_absent_passes_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 473);
    let shared_quest_id = 7610;
    let breadcrumb_id = 9611;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut breadcrumb_quest = quest_template(breadcrumb_id);
    breadcrumb_quest.breadcrumb_for_quest_id = shared_quest_id as i32;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, breadcrumb_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_active_quest_in_slot_with_status(
        &mut receiver_session,
        breadcrumb_id,
        2,
        QUEST_STATUS_FAILED_LIKE_CPP,
    );
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
            "Quest 7610".to_string()
        )
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite)));

    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut breadcrumb_quest = quest_template(breadcrumb_id);
    breadcrumb_quest.breadcrumb_for_quest_id = shared_quest_id as i32;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, breadcrumb_quest]);
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
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_breadcrumb_for_quest_remains_unrepresented_without_prerequisite_pair_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 474);
    let shared_quest_id = 7611;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    shared_quest.breadcrumb_for_quest_id = 9991;
    let target_quest = quest_template(9991);
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, target_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    sync_player_registry_state_for_test(&receiver_session);

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert!(sender_rx.try_recv().is_err());
    assert!(receiver_rx.try_recv().is_err());
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
            ))
    );
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_negative_previous_rejects_rewarded_complete_or_failed_status() {
    for status in [QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP] {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 391);
        let shared_quest_id = 7191;
        let previous_id = 9191;
        let quest_store = store_with_sharable_quest_previous(shared_quest_id, -9191);
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::new(quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        add_rewarded_quest(&mut receiver_session, previous_id);
        add_active_quest_in_slot_with_status(&mut receiver_session, previous_id, 2, status);
        sync_player_registry_state_for_test(&receiver_session);
        let snapshot = player_registry
            .quest_sharing_snapshot(
                receiver_guid,
                canonical_map_manager_for_test(&receiver_session),
            )
            .expect("registered canonical receiver snapshot");
        assert!(snapshot.rewarded_quests.contains(&previous_id));
        assert_eq!(
            snapshot.active_quest_statuses.get(&previous_id).copied(),
            Some(status)
        );

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
                "Quest 7191".to_string()
            )
        );
        assert!(drain_session_commands_for_test(&receiver_session).is_empty());
        assert!(represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite
            )));
        assert!(
            !represented_push_quest_to_party_outcomes_for_test(&session)
                .iter()
                .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
        );
        assert!(sender_rx.try_recv().is_err());
        assert!(receiver_rx.try_recv().is_err());
    }
}
