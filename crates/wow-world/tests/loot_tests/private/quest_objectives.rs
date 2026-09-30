//! Original loot quest application cases use the existing Quest fixtures.
use super::quest_support::*;
use super::support::*;
use wow_loot::LootConditionRowLikeCpp;

#[test]
fn banked_quest_item_recomputes_objective_and_reopens_quest_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 1, 0);
    wow_world::test_fixtures::enable_ownerless_inventory_snapshots_for_test(&mut session);
    let quest_id = 8_338;
    let item_id = 20_484;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 3,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    set_quest_store_for_test(&mut session, Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_gameplay_status_for_test(&mut session, 
        quest_id,
        wow_world::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: wow_world::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![3],
            slot: 0,
        },
    );

    let planned = plan_bank_item_quest_persistence_for_test(&session, item_id, 0, true, 0, 0);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![0]);
    assert_eq!(
        planned[0].status,
        wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    assert_eq!(
        remove_loot_item_objectives_for_test(&mut session, item_id),
        Some(vec![quest_id])
    );
    let quest_statuses = quest_compatibility_statuses_for_test(&session);
    let status = quest_statuses
        .get(&quest_id)
        .expect("active quest");
    assert_eq!(
        status.status,
        wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
    );
    assert_eq!(status.objective_counts, vec![0]);
    let update = send_rx
        .try_recv()
        .expect("banking a quest item should update the quest-log slot");
    assert_eq!(
        WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
}

#[tokio::test]
async fn loot_item_added_progresses_incomplete_quest_item_objective_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let quest_id = 8_336;
    let item_id = 20_482;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 6,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    set_quest_store_for_test(&mut session, Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_gameplay_status_for_test(&mut session, 
        quest_id,
        wow_world::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    assert!(loot_item_quest_allowed_for_test(&session, item_id, true, 0, 0, None));

    let changed_quest_ids = advance_loot_item_objectives_for_test(&mut session, item_id, 0, 3)
        .await;

    assert_eq!(changed_quest_ids, vec![quest_id]);
    assert_eq!(
        quest_compatibility_statuses_for_test(&session)
            .get(&quest_id)
            .expect("quest progress should remain active")
            .objective_counts,
        vec![3]
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ UpdateQuestObjectiveProgress suppresses generic credit packets for ITEM objectives"
    );
}

#[tokio::test]
async fn loot_item_eligibility_does_not_treat_complete_quest_as_incomplete_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    let quest_id = 8_337;
    let item_id = 20_483;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    set_quest_store_for_test(&mut session, Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_gameplay_status_for_test(&mut session, 
        quest_id,
        wow_world::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: wow_world::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    assert!(!incomplete_loot_item_objective_for_test(&session, item_id));
    assert!(!loot_item_quest_allowed_for_test(&session, item_id, true, 0, 0, None));

    let changed_quest_ids = advance_loot_item_objectives_for_test(&mut session, item_id, 0, 1)
        .await;

    assert!(changed_quest_ids.is_empty());
    assert_eq!(
        quest_compatibility_statuses_for_test(&session)
            .get(&quest_id)
            .expect("complete quest should not progress as incomplete")
            .objective_counts,
        vec![0]
    );
}

#[tokio::test]
async fn withdrawn_banked_item_restores_bound_objective_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let quest_id = 8_339;
    let item_id = 20_485;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: item_id as i32,
        amount: 1,
        flags: 0,
        flags2: 1,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    set_quest_store_for_test(&mut session, Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_gameplay_status_for_test(&mut session, 
        quest_id,
        wow_world::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: wow_world::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    let planned = plan_bank_item_quest_persistence_for_test(&session, item_id, 0, false, 0, 1);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![1]);
    assert_eq!(
        planned[0].status,
        wow_world::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    let changed_quest_ids = apply_quest_item_added_objective_progress_for_test(&mut session, item_id, 0, 1)
        .await;

    assert_eq!(changed_quest_ids, vec![quest_id]);
    let quest_statuses = quest_compatibility_statuses_for_test(&session);
    let status = quest_statuses
        .get(&quest_id)
        .expect("active quest");
    assert_eq!(status.objective_counts, vec![1]);
    assert_eq!(
        status.status,
        wow_world::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
}

