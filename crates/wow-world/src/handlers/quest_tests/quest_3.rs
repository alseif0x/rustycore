//! Quest scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn quest_giver_choose_reward_accepts_quest_package_primary_everyone_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7005;
    let reward_item_id = 19_019;
    let package_id = 77;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_test_item_template_with_flags2_like_cpp(&mut session, reward_item_id, 0);
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
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

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let reward_item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("primary package reward item should be in direct inventory");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(1)
    );

    let mut saw_item_push = false;
    let mut saw_quest_complete = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        match packet.read_uint16().unwrap() {
            opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                saw_item_push = true;
                assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                assert_eq!(
                    packet.read_uint8().unwrap(),
                    u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                );
                let slot_in_bag = packet.read_int32().unwrap();
                assert!(slot_in_bag >= 0);
                assert_eq!(packet.read_int32().unwrap(), 0);
                assert_eq!(packet.read_int32().unwrap(), 1);
                assert_eq!(packet.read_int32().unwrap(), 1);
            }
            opcode if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 => {
                saw_quest_complete = true;
            }
            _ => {}
        }
    }
    assert!(saw_item_push);
    assert!(saw_quest_complete);
}
#[tokio::test]
async fn quest_giver_choose_reward_accepts_quest_package_fallback_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7006;
    let reward_item_id = 19_020;
    let package_id = 78;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_test_item_template_with_flags2_like_cpp(&mut session, reward_item_id, 0);
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
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

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let reward_item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("fallback package reward item should be in direct inventory");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(1)
    );

    let mut saw_item_push = false;
    let mut saw_quest_complete = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        match packet.read_uint16().unwrap() {
            opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                saw_item_push = true;
                assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                assert_eq!(
                    packet.read_uint8().unwrap(),
                    u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                );
                let slot_in_bag = packet.read_int32().unwrap();
                assert!(slot_in_bag >= 0);
                assert_eq!(packet.read_int32().unwrap(), 0);
                assert_eq!(packet.read_int32().unwrap(), 1);
                assert_eq!(packet.read_int32().unwrap(), 1);
            }
            opcode if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 => {
                saw_quest_complete = true;
            }
            _ => {}
        }
    }
    assert!(saw_item_push);
    assert!(saw_quest_complete);
}
#[tokio::test]
async fn quest_giver_choose_reward_rejects_quest_package_wrong_faction_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7007;
    let reward_item_id = 19_021;
    let package_id = 79;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_test_item_template_with_flags2_like_cpp(
        &mut session,
        reward_item_id,
        ItemFlags2::FactionHorde as u32,
    );
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
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

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_request_reward_completes_ready_quest_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().expect("player guid");
    let quest_id = 9021;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest(&mut session, quest_id);

    session
        .handle_quest_giver_request_reward(quest_giver_request_reward_packet_like_cpp(
            player_guid,
            quest_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("quest should still be active before choose-reward")
            .status,
        QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert_complete_status_update_like_cpp(&session, quest_id, false);
    recv_quest_giver_offer_reward_contains_quest_id(&send_rx, quest_id);
}
#[tokio::test]
async fn quest_confirm_accept_short_packet_does_not_clear_pending_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 81);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7001);

    session
        .handle_quest_confirm_accept(WorldPacket::from_bytes(&[0x59, 0x1B, 0x00]))
        .await;

    assert_eq!(
        session.represented_pending_quest_sharing_like_cpp(),
        Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
            sender_guid,
            quest_id: 7001,
        })
    );
    assert!(
        session
            .represented_quest_confirm_accepts_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_no_pending_valid_packet_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7002])));

    run_quest_confirm_accept(&mut session, 7002).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(
        session
            .represented_quest_confirm_accepts_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_mismatch_preserves_pending_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 82);
    session.set_quest_store(Arc::new(store_with_quests(&[7003])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7003);

    run_quest_confirm_accept(&mut session, 7004).await;

    assert_eq!(
        session.represented_pending_quest_sharing_like_cpp(),
        Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
            sender_guid,
            quest_id: 7003,
        })
    );
    assert!(
        session
            .represented_quest_confirm_accepts_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_match_missing_template_clears_without_evidence_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 83);
    session.set_quest_store(Arc::new(store_with_quests(&[7005])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7006);

    run_quest_confirm_accept(&mut session, 7006).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(
        session
            .represented_quest_confirm_accepts_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_match_template_records_original_player_missing_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 84);
    let receiver_guid = ObjectGuid::create_player(1, 42);
    session.set_quest_store(Arc::new(store_with_quests(&[7007])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7007);

    run_quest_confirm_accept(&mut session, 7007).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(receiver_guid),
        sender_guid,
        7007,
        7007,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_negative_raw_id_compares_as_u32_bit_pattern_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 85);
    let quest_id = u32::MAX;
    session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);

    run_quest_confirm_accept(&mut session, -1).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        quest_id,
        -1,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_sender_exists_not_same_group_records_not_in_same_raid_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 86);
    session.set_quest_store(Arc::new(store_with_quests(&[7008])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7008);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        7008,
        false,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, 7008).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        7008,
        7008,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_same_group_sender_not_active_records_original_not_active_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 87);
    session.set_quest_store(Arc::new(store_with_quests(&[7009])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7009);
    let (_sender_session, sender_rx) =
        install_confirm_accept_sender_snapshot(&mut session, sender_guid, 7009, true, None);

    run_quest_confirm_accept(&mut session, 7009).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        7009,
        7009,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerNotActiveQuest,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_same_group_sender_active_can_take_failed_records_receiver_gate_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 88);
    let quest_id = 7010;
    session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
    session.rewarded_quests.insert(quest_id);
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
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanTakeQuestFailed,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_can_take_ok_log_full_records_can_add_log_full_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 89);
    let quest_id = 7011;
    session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
    for slot in 0..MAX_QUEST_LOG_SIZE_LIKE_CPP {
        add_active_quest_in_slot_with_status(
            &mut session,
            80_000 + u32::from(slot),
            slot,
            QUEST_STATUS_COMPLETE_LIKE_CPP,
        );
    }
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_no_source_side_effects_adds_local_quest_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 90);
    let quest_id = 7012;
    session.set_quest_store(Arc::new(store_with_sharable_timed_quest_objectives(
        quest_id, 3, 600,
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

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    let status = session
        .player_quests
        .get(&quest_id)
        .expect("receiver quest log should receive bounded local AddQuest state");
    assert_eq!(status.quest_id, quest_id);
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert!(!status.explored);
    assert!(status.accept_time_secs > 0);
    assert_eq!(status.end_time_secs, status.accept_time_secs + 600);
    assert_eq!(status.objective_counts, vec![0, 0, 0]);
    assert_eq!(status.slot, 0);
    let registry = session.player_registry().expect("test installs registry");
    let snapshot = registry
        .loot_player_context(receiver_guid)
        .expect("receiver canonical state should sync after quest insertion");
    assert_eq!(
        snapshot.active_quest_statuses.get(&quest_id),
        Some(&QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    );
    assert_eq!(
        snapshot.active_quest_objective_counts.get(&quest_id),
        Some(&vec![0, 0, 0])
    );
    assert_confirm_accept_outcome(
        &session,
        Some(receiver_guid),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_first_free_slot_skips_occupied_slot_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 190);
    let occupied_quest_id = 8000;
    let quest_id = 70120;
    session.set_quest_store(Arc::new(store_with_sharable_quest_objectives(quest_id, 1)));
    add_active_quest_in_slot_with_status(
        &mut session,
        occupied_quest_id,
        0,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let occupied_status = session
        .player_quests
        .get(&occupied_quest_id)
        .expect("pre-existing quest should remain in slot 0");
    assert_eq!(occupied_status.slot, 0);
    let status = session
        .player_quests
        .get(&quest_id)
        .expect("accepted quest should be inserted into first free slot");
    assert_eq!(status.slot, 1);
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let registry = session.player_registry().expect("test installs registry");
    let snapshot = registry
        .loot_player_context(receiver_guid)
        .expect("receiver canonical state should sync after quest insertion");
    assert_eq!(
        snapshot.active_quest_statuses.get(&quest_id),
        Some(&QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    );
    assert_confirm_accept_outcome(
        &session,
        Some(receiver_guid),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_confirm_accept_tracking_event_auto_rewards_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 201);
    let quest_id = 7121;
    let mut quest = quest_template(quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
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
    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_complete_status_update_like_cpp(&session, quest_id, false);
    let slot_update = send_rx
        .try_recv()
        .expect("tracking event auto reward should clear quest log slot");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&slot_update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
    let complete = send_rx
        .try_recv()
        .expect("tracking event auto reward should send quest complete");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&complete).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete)
    );
    let update = send_rx
        .try_recv()
        .expect("tracking event auto reward should send quest update complete");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestUpdateComplete)
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_push_short_packet_does_not_clear_pending_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 77);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7001);

    session
        .handle_quest_push_result(WorldPacket::from_bytes(&[0x00]))
        .await;

    assert_eq!(
        session.represented_pending_quest_sharing_like_cpp(),
        Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
            sender_guid,
            quest_id: 7001,
        })
    );
    assert!(
        session
            .represented_quest_push_result_responses_like_cpp()
            .is_empty()
    );
    assert_eq!(
        session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
        0
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_push_no_pending_valid_packet_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 78);

    run_quest_push_result(&mut session, sender_guid, 7002, 3).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(
        session
            .represented_quest_push_result_responses_like_cpp()
            .is_empty()
    );
    assert_eq!(
        session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
        0
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_push_pending_sender_match_clears_and_records_response_evidence_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 79);
    let receiver_guid = session.player_guid().unwrap();
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7003);

    run_quest_push_result(&mut session, sender_guid, 8003, 6).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_eq!(
        session.represented_quest_push_result_responses_like_cpp(),
        &[RepresentedQuestPushResultResponseLikeCpp {
            receiver_guid,
            sender_guid,
            parsed_quest_id: 8003,
            pending_quest_id: 7003,
            result: 6,
        }]
    );
    assert_eq!(
        session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
        0
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_push_pending_sender_mismatch_clears_without_response_evidence_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pending_sender_guid = ObjectGuid::create_player(1, 80);
    let packet_sender_guid = ObjectGuid::create_player(1, 81);
    session.set_represented_pending_quest_sharing_like_cpp(pending_sender_guid, 7004);

    run_quest_push_result(&mut session, packet_sender_guid, 7004, 4).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(
        session
            .represented_quest_push_result_responses_like_cpp()
            .is_empty()
    );
    assert_eq!(
        session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
        1
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn quest_push_packet_parser_reads_sender_quest_id_result_in_cpp_order() {
    let sender_guid = ObjectGuid::create_player(1, 82);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&sender_guid);
    pkt.write_uint32(7005);
    pkt.write_uint8(9);

    let parsed = QuestPushResult::read(&mut pkt).expect("valid QuestPushResult");

    assert_eq!(parsed.sender_guid, sender_guid);
    assert_eq!(parsed.quest_id, 7005);
    assert_eq!(parsed.result, 9);
}
#[tokio::test]
async fn push_quest_to_party_malformed_packet_records_no_evidence_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_sharable_quest(7101)));
    add_active_quest(&mut session, 7101);

    session
        .handle_push_quest_to_party(WorldPacket::from_bytes(&[0x9F, 0x34, 0x00]))
        .await;

    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .is_empty()
    );
    assert!(
        session
            .represented_pending_quest_sharing_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn push_quest_to_party_missing_quest_template_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7102])));

    run_push_quest_to_party(&mut session, 7103).await;

    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn push_quest_to_party_unshareable_or_not_in_log_records_not_allowed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    session.set_quest_store(Arc::new(store_with_sharable_quest(7104)));

    run_push_quest_to_party(&mut session, 7104).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7104,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotAllowed,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: false,
            receiver_fanout_unrepresented: false,
        }]
    );
    assert_eq!(
        recv_push_quest_result_response(&send_rx),
        (
            sender_guid.expect("test session has player guid"),
            quest_push_reason::NOT_ALLOWED,
            String::new()
        )
    );
}
#[tokio::test]
async fn push_quest_to_party_shareable_sender_without_pool_store_still_blocks_before_group_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    session.set_quest_store(Arc::new(store_with_sharable_quest(7105)));
    add_active_quest(&mut session, 7105);

    run_push_quest_to_party(&mut session, 7105).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7105,
            target_guid: sender_guid,
            reason:
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::QuestPoolActiveCheckUnrepresented,
            quest_pool_active_check_unrepresented: true,
            group_runtime_unrepresented: false,
            receiver_fanout_unrepresented: false,
        }]
    );
    assert!(
        session
            .represented_pending_quest_sharing_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}
