//! Quest scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

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
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
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
            "Quest 7134".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
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
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
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
            "Quest 7607".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
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
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
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
            "Quest 7609".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));

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
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
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
            "Quest 7610".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite)));

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
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite)));
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
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert!(sender_rx.try_recv().is_err());
    assert!(receiver_rx.try_recv().is_err());
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
            ))
    );
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}
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
            "Quest 7136".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
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
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();
    set_canonical_party_reputation_like_cpp(
        receiver_session
            .canonical_map_manager
            .as_ref()
            .expect("canonical map manager"),
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
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
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
    receiver_session.set_player_level_like_cpp(1);
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
            "Quest 7124".to_string()
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
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
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
    receiver_session.set_player_level_like_cpp(19);
    receiver_session.sync_player_registry_state_like_cpp();
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
    receiver_session.set_represented_daily_quest_completed_like_cpp_for_test(shared_quest_id, true);
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
            "Quest 7120".to_string()
        )
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
    receiver_session.unregister_from_player_registry();
    receiver_session.expansion = 1;
    receiver_session.register_in_player_registry();

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
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
            ))
    );
}
#[tokio::test]
async fn push_quest_to_party_success_prompts_receiver_details_and_sets_pending_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 249);
    let shared_quest_id = 7122;
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
        SessionCommand::SetQuestSharingInfoAndSendDetails(command) => {
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
        Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
            sender_guid,
            quest_id: shared_quest_id,
        })
    );
    recv_quest_giver_quest_details_contains_quest_id(&receiver_rx, shared_quest_id);
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                )
                && !outcome.receiver_fanout_unrepresented)
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
                    | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
            ))
    );
}
#[tokio::test]
async fn push_quest_to_party_repeatable_turn_in_command_queue_failure_sends_no_success_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 612);
    let shared_quest_id = 76120;
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
        object_id: 49212,
        amount: 3,
        flags: 0xA5,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let quest_for_dummy_commands = quest.clone();
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    for _ in 0..256 {
        receiver_session
            .session_command_tx()
            .try_send(SessionCommand::SetQuestSharingInfoAndSendDetails(
                SetQuestSharingInfoAndSendDetailsCommand {
                    sender_guid,
                    quest: quest_for_dummy_commands.clone(),
                },
            ))
            .expect("fill receiver command queue fixture");
    }

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert!(sender_rx.try_recv().is_err());
    assert!(receiver_rx.try_recv().is_err());
    assert_eq!(receiver_session.drain_session_commands().len(), 256);
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
        |outcome| outcome.target_guid == Some(receiver_guid)
            && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed
            )
            && outcome.receiver_fanout_unrepresented
    ));
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| outcome.target_guid == Some(sender_guid)
                && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
            ) && outcome.receiver_fanout_unrepresented)
    );
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
        |outcome| outcome.target_guid == Some(receiver_guid)
            && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPrompted
            )
    ));
}
#[tokio::test]
async fn push_quest_to_party_receiver_unknown_status_after_expansion_emits_invalid_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 609);
    let shared_quest_id = 76090;
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
    add_active_quest_in_slot_with_status(&mut receiver_session, shared_quest_id, 2, 0xFE);
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
            "Quest 76090".to_string()
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
