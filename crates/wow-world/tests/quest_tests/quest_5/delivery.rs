//! Quest-share command delivery, queue failures and receiver status.

use super::*;

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
    sync_player_registry_state_for_test(&receiver_session);

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
    let commands = drain_session_commands_for_test(&receiver_session);
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
    process_represented_session_commands_like_cpp(&mut receiver_session).await;
    assert_eq!(
        represented_pending_quest_sharing_for_test(&receiver_session),
        Some(RepresentedPendingQuestSharingLikeCpp {
            sender_guid,
            quest_id: shared_quest_id,
        })
    );
    recv_quest_giver_quest_details_contains_quest_id(&receiver_rx, shared_quest_id);
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                )
                && !outcome.receiver_fanout_unrepresented)
    );
    assert!(
        !represented_push_quest_to_party_outcomes_for_test(&session)
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
    sync_player_registry_state_for_test(&receiver_session);

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
    assert_eq!(
        drain_session_commands_for_test(&receiver_session).len(),
        256
    );
    assert!(represented_push_quest_to_party_outcomes_for_test(&session).iter().any(
        |outcome| outcome.target_guid == Some(receiver_guid)
            && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed
            )
            && outcome.receiver_fanout_unrepresented
    ));
    assert!(
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| outcome.target_guid == Some(sender_guid)
                && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
            ) && outcome.receiver_fanout_unrepresented)
    );
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(
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
    sync_player_registry_state_for_test(&receiver_session);

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
        represented_push_quest_to_party_outcomes_for_test(&session)
            .iter()
            .any(|outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid
                ))
    );
    assert!(!represented_push_quest_to_party_outcomes_for_test(&session).iter().any(|outcome| outcome.target_guid == Some(receiver_guid) && matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
}

#[tokio::test]
async fn push_quest_to_party_preserves_partial_exclusive_peer_policy_after_expansion() {
    for (case, allowed) in [
        ("stored_none", true),
        ("weekly", true),
        ("seasonal", true),
        ("df_daily", false),
    ] {
        let (mut session, sender_rx) = make_session();
        let sender_guid = session.player_guid().expect("test sender guid");
        let receiver_guid = ObjectGuid::create_player(1, 613);
        let shared_quest_id = 76130;
        let peer_quest_id = 76131;
        let mut shared_quest = quest_template(shared_quest_id);
        shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
        shared_quest.exclusive_group = 610;
        let mut peer_quest = quest_template(peer_quest_id);
        peer_quest.exclusive_group = 610;
        match case {
            "stored_none" => {}
            "weekly" => peer_quest.flags |= QUEST_FLAGS_WEEKLY_LIKE_CPP,
            "seasonal" => {
                peer_quest.quest_sort_id = -376;
                peer_quest.event_id_for_quest = 9;
            }
            "df_daily" => {
                peer_quest.flags |= QUEST_FLAGS_DAILY_LIKE_CPP;
                peer_quest.special_flags |= QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
            }
            _ => unreachable!(),
        }
        let quest_store = Arc::new(QuestStore::from_quests_like_cpp([shared_quest, peer_quest]));
        let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
        session.set_quest_store(Arc::clone(&quest_store));
        session.set_quest_pool_store(Arc::new(quest_pool_store));
        add_active_quest(&mut session, shared_quest_id);
        let (_player_registry, mut receiver_session, receiver_rx) =
            install_represented_party(&mut session, sender_guid, receiver_guid);
        receiver_session.set_quest_store(Arc::clone(&quest_store));
        if case == "stored_none" {
            add_active_quest_in_slot_with_status(
                &mut receiver_session,
                peer_quest_id,
                2,
                wow_constants::quest::QUEST_STATUS_NONE_LIKE_CPP,
            );
        } else {
            mutate_player_quest_gameplay_for_test(&mut receiver_session, |state| match case {
                "weekly" => state.set_weekly_like_cpp(peer_quest_id, true),
                "seasonal" => state.set_seasonal_like_cpp(9, peer_quest_id, 100),
                "df_daily" => state.set_daily_like_cpp(peer_quest_id, true),
                _ => unreachable!(),
            })
            .expect("registered canonical receiver quest owner");
        }

        // Full local eligibility blocks all four inputs. The represented final
        // sharing operation intentionally retains its narrower policy.
        assert!(
            !receiver_session
                .can_take_quest(quest_store.get(shared_quest_id).expect("shared quest row"),),
            "local exclusive rule, case={case}"
        );
        sync_player_registry_state_for_test(&receiver_session);

        run_push_quest_to_party(&mut session, shared_quest_id).await;

        if allowed {
            assert_success_command_queued_like_cpp(
                &sender_rx,
                &receiver_rx,
                &mut receiver_session,
                receiver_guid,
                sender_guid,
                shared_quest_id,
            );
        } else {
            assert_eq!(
                recv_push_quest_result_response(&sender_rx),
                (
                    receiver_guid,
                    QUEST_PUSH_REASON_INVALID_LIKE_CPP,
                    String::new()
                ),
            );
            assert_eq!(
                recv_push_quest_result_response(&receiver_rx),
                (
                    sender_guid,
                    QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
                    "Quest 76130".to_string(),
                ),
            );
            assert!(drain_session_commands_for_test(&receiver_session).is_empty());
            assert!(!represented_push_quest_to_party_outcomes_for_test(&session)
                .iter()
                .any(|outcome| matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                )));
        }
    }
}
