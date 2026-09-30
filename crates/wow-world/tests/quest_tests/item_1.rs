//! Item scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn quest_giver_choose_reward_rejects_missing_reward_item_template_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7003;
    let reward_item_id = 19019;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (reward_item_id, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    set_player_gold_for_test(&mut session, 5);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
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
        player_quest_status_for_test(&session, quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(
        !contains_rewarded_quest_for_test(&session, quest_id)
    );
    assert_eq!(player_gold_for_test(&session), 5);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_choose_reward_removes_item_objective_before_rewards_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7015;
    let required_item_id = 19_028;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: required_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    set_player_gold_for_test(&mut session, 5);
    install_source_item_template(&mut session, required_item_id, 20, 0);
    insert_direct_inventory_item(&mut session, player_guid, 23, required_item_id, 5, 9911);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
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
            0,
        ))
        .await;

    assert!(
        !contains_player_quest_status_for_test(&session, quest_id)
    );
    assert!(
        contains_rewarded_quest_for_test(&session, quest_id)
    );
    assert_eq!(player_gold_for_test(&session), 42);
    let item = inventory_items_for_test(&session)
        .values()
        .find(|item| item.entry_id == required_item_id)
        .expect("partial objective item stack should remain");
    assert_eq!(
        inventory_item_objects_for_test(&session)
            .get(&item.guid)
            .map(|item| item.count()),
        Some(3)
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_direct_choice_inventory_failure_sends_quest_failed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7008;
    let reward_item_id = 19_022;
    let limit_category = 44;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (reward_item_id, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    set_player_gold_for_test(&mut session, 5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9907);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
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
        player_quest_status_for_test(&session, quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(
        !contains_rewarded_quest_for_test(&session, quest_id)
    );
    assert_eq!(player_gold_for_test(&session), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QuestGiverQuestFailed {
            quest_id,
            reason: InventoryResult::ItemMaxLimitCategoryCountExceededIs as u32,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_choose_reward_fixed_reward_inventory_failure_sends_quest_failed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7009;
    let reward_item_id = 19_023;
    let limit_category = 45;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_items[0] = reward_item_id;
    quest.reward_amounts[0] = 1;
    set_player_gold_for_test(&mut session, 5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9908);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
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
            0,
        ))
        .await;

    assert_eq!(
        player_quest_status_for_test(&session, quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(
        !contains_rewarded_quest_for_test(&session, quest_id)
    );
    assert_eq!(player_gold_for_test(&session), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QuestGiverQuestFailed {
            quest_id,
            reason: InventoryResult::ItemMaxLimitCategoryCountExceededIs as u32,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn quest_giver_choose_reward_fixed_reward_stores_and_pushes_item_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7012;
    let reward_item_id = 19_026;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_items[0] = reward_item_id;
    quest.reward_amounts[0] = 2;
    set_player_gold_for_test(&mut session, 5);
    install_source_item_template(&mut session, reward_item_id, 20, 0);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
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
            0,
        ))
        .await;

    assert!(
        !contains_player_quest_status_for_test(&session, quest_id)
    );
    assert!(
        contains_rewarded_quest_for_test(&session, quest_id)
    );
    assert_eq!(player_gold_for_test(&session), 42);
    let reward_item = inventory_items_for_test(&session)
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("fixed reward item should be in direct inventory");
    assert_eq!(
        inventory_item_objects_for_test(&session)
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(2)
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
                assert_eq!(packet.read_int32().unwrap(), 2);
                assert_eq!(packet.read_int32().unwrap(), 2);
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
async fn quest_reward_item_definite_and_unknown_commit_fail_closed_before_publication_like_cpp() {
    for outcome in [
        wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "fixture rollback".into(),
        },
        wow_persistence::PlayerQuestRewardCommitOutcomeLikeCpp::CommitOutcomeUnknown {
            reason: "fixture unknown commit".into(),
            witness: wow_persistence::PlayerQuestRewardCommitWitnessLikeCpp::Money {
                observed_money: None,
            },
        },
    ] {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 70_120;
        let reward_item_id = 19_126;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_items[0] = reward_item_id;
        quest.reward_amounts[0] = 2;
        set_player_gold_for_test(&mut session, 5);
        install_source_item_template(&mut session, reward_item_id, 20, 0);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        insert_player_quest_status_for_test(&mut session,
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
        // The reward no longer commits each grant: it accumulates them and
        // closes with one character transaction, so the outcome under test is
        // that transaction's, not an individual inventory write's.
        let fixture = Arc::new(player_quest_reward_persistence_fixture_with_outcome_for_test(
            outcome,
        ));
        session.set_player_quest_reward_persistence_port_like_cpp(fixture.clone());

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            player_quest_status_for_test(&session, quest_id)
                .map(|status| status.status),
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
        );
        assert!(
            !contains_rewarded_quest_for_test(&session, quest_id)
        );
        assert_eq!(player_gold_for_test(&session), 5);
        // The operation mutated the in-memory inventory while it planned, as
        // C++ `StoreNewItem` does before its save. Nothing durable changed, so
        // the session is quarantined instead of being left showing an item the
        // database does not have.
        assert_eq!(
            session.state(),
            wow_world::session::SessionState::Disconnecting,
            "a reward that did not commit must quarantine the session"
        );
        let requests = player_quest_reward_requests_for_test(&fixture);
        assert_eq!(
            requests.len(),
            1,
            "a failed reward must have attempted exactly one transaction"
        );
        assert!(matches!(
            requests[0].inventory_mutations.as_slice(),
            [
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestTurnIn(_),
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestItemGrant(_),
            ]
        ));
    }
}
#[tokio::test]
async fn quest_giver_choose_reward_chosen_item_stores_and_pushes_item_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7013;
    let reward_item_id = 19_027;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (reward_item_id, 3);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    set_player_gold_for_test(&mut session, 5);
    install_source_item_template(&mut session, reward_item_id, 20, 0);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
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

    assert!(
        !contains_player_quest_status_for_test(&session, quest_id)
    );
    assert!(
        contains_rewarded_quest_for_test(&session, quest_id)
    );
    assert_eq!(player_gold_for_test(&session), 42);
    let reward_item = inventory_items_for_test(&session)
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("chosen reward item should be in direct inventory");
    assert_eq!(
        inventory_item_objects_for_test(&session)
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(3)
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
                assert_eq!(packet.read_int32().unwrap(), 3);
                assert_eq!(packet.read_int32().unwrap(), 3);
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
