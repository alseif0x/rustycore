//! Quest scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn quest_status_projection_persists_only_nonzero_storage_objectives_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 5925;
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: 45,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 2,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONEY_LIKE_CPP_LOCAL,
        order: 2,
        storage_index: -1,
        object_id: 0,
        amount: 20,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: true,
            accept_time_secs: 12,
            end_time_secs: 34,
            objective_counts: vec![3, 0, 9],
            slot: 0,
        },
    );

    let projected = represented_quest_status_persistence_for_test(
        &session,
        &player_quest_status_for_test(&session, quest_id).unwrap(),
    );
    assert_eq!(projected.quest_id, quest_id);
    assert_eq!(projected.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert!(projected.explored);
    assert_eq!(projected.accept_time_secs, 12);
    assert_eq!(projected.end_time_secs, 34);
    assert_eq!(
        projected.objectives,
        vec![wow_persistence::QuestObjectiveCountPersistenceLikeCpp {
            objective_index: 0,
            count: 3,
        }]
    );
}
#[tokio::test]
async fn quest_status_save_uses_the_sqlx_free_player_quest_port_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 5928;
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    insert_player_quest_status_for_test(&mut session,
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: true,
            accept_time_secs: 12,
            end_time_secs: 34,
            objective_counts: vec![5],
            slot: 0,
        },
    );
    let fixture = Arc::new(PlayerQuestPersistencePortFixtureLikeCpp::default());
    session.set_player_quest_persistence_port_like_cpp(fixture.clone());

    save_quest_to_db_for_test(&session, quest_id, QUEST_STATUS_COMPLETE_LIKE_CPP).await;

    assert_eq!(
        player_quest_status_requests_for_test(&fixture).as_slice(),
        &[PlayerQuestStatusPersistenceRequestLikeCpp::Save {
            owner_guid: 42,
            status: wow_persistence::QuestStatusPersistenceLikeCpp {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: true,
                accept_time_secs: 12,
                end_time_secs: 34,
                objectives: vec![wow_persistence::QuestObjectiveCountPersistenceLikeCpp {
                    objective_index: 0,
                    count: 5,
                }],
            },
        }]
    );
}
#[tokio::test]
async fn quest_load_keeps_the_seven_stage_order_behind_the_typed_port_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let active_id = 5930;
    let rewarded_id = 5931;
    let daily_id = 5932;
    let weekly_id = 5933;
    let monthly_id = 5934;
    let mut active = quest_template(active_id);
    active.objectives.push(QuestObjective {
        id: active_id * 10,
        quest_id: active_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let rewarded = quest_template(rewarded_id);
    let mut daily = quest_template(daily_id);
    daily.flags |= QUEST_FLAGS_DAILY_LIKE_CPP;
    let mut weekly = quest_template(weekly_id);
    weekly.flags |= QUEST_FLAGS_WEEKLY_LIKE_CPP;
    let mut monthly = quest_template(monthly_id);
    monthly.special_flags |= QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        active, rewarded, daily, weekly, monthly,
    ])));

    let fixture = player_quest_persistence_port_with_load_rows_for_test(
        vec![PlayerQuestActivePersistenceRowLikeCpp {
            quest_id: Some(active_id),
            status: Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
            explored: Some(1),
            accept_time_secs: Some(12),
            end_time_secs: Some(34),
        }],
        vec![PlayerQuestObjectivePersistenceRowLikeCpp {
            quest_id: Some(active_id),
            storage_index: Some(0),
            count: Some(3),
        }],
        vec![PlayerQuestIdPersistenceRowLikeCpp {
            quest_id: Some(rewarded_id),
        }],
        vec![PlayerQuestDailyPersistenceRowLikeCpp {
            quest_id: Some(daily_id),
            completed_time: Some(45),
        }],
        vec![PlayerQuestIdPersistenceRowLikeCpp {
            quest_id: Some(weekly_id),
        }],
        vec![PlayerQuestIdPersistenceRowLikeCpp {
            quest_id: Some(monthly_id),
        }],
    );
    session.set_player_quest_persistence_port_like_cpp(fixture.clone());

    load_player_quests_for_test(&mut session).await;

    assert_eq!(
        player_quest_load_stages_for_test(&fixture).as_slice(),
        &[
            PlayerQuestLoadStageFixtureLikeCpp::Active,
            PlayerQuestLoadStageFixtureLikeCpp::Objectives,
            PlayerQuestLoadStageFixtureLikeCpp::Rewarded,
            PlayerQuestLoadStageFixtureLikeCpp::Daily,
            PlayerQuestLoadStageFixtureLikeCpp::Weekly,
            PlayerQuestLoadStageFixtureLikeCpp::Monthly,
            PlayerQuestLoadStageFixtureLikeCpp::Seasonal,
        ]
    );
    assert_eq!(
        player_quest_status_for_test(&session, active_id)
            .expect("active quest status should exist")
            .objective_counts,
        vec![3]
    );
    assert!(
        contains_rewarded_quest_for_test(&session, rewarded_id)
    );
    assert!(
        contains_daily_quest_completed_for_test(&session, daily_id)
    );
    assert!(
        contains_weekly_quest_completed_for_test(&session, weekly_id)
    );
    assert!(
        contains_monthly_quest_completed_for_test(&session, monthly_id)
    );
}
#[test]
fn save_to_db_quest_status_list_skips_rewarded_non_repeatable_active_duplicate_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let active_rewarded_quest_id = 5921;
    let active_quest_id = 5922;
    let quest_store = QuestStore::from_quests_like_cpp([
        quest_template(active_rewarded_quest_id),
        quest_template(active_quest_id),
    ]);
    set_quest_store_for_test(&mut session, Arc::new(quest_store));
    add_active_quest_in_slot_with_status(
        &mut session,
        active_rewarded_quest_id,
        0,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
    );
    add_active_quest_in_slot_with_status(
        &mut session,
        active_quest_id,
        1,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    insert_rewarded_quest_for_test(&mut session, active_rewarded_quest_id);

    assert_eq!(
        represented_quest_statuses_for_save_for_test(&session),
        vec![(active_quest_id, QUEST_STATUS_INCOMPLETE_LIKE_CPP)],
        "C++ reward save deletes active quest status and stores rewarded separately"
    );
}
#[test]
fn quest_load_removes_active_rewarded_duplicate_and_compacts_slots_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let duplicate_quest_id = 5923;
    let active_quest_id = 5924;
    let quest_store = QuestStore::from_quests_like_cpp([
        quest_template(duplicate_quest_id),
        quest_template(active_quest_id),
    ]);
    set_quest_store_for_test(&mut session, Arc::new(quest_store));
    add_active_quest_in_slot_with_status(
        &mut session,
        duplicate_quest_id,
        0,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
    );
    add_active_quest_in_slot_with_status(
        &mut session,
        active_quest_id,
        3,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    insert_rewarded_quest_for_test(&mut session, duplicate_quest_id);

    assert_eq!(
        remove_represented_active_rewarded_duplicates_for_test(&mut session),
        vec![duplicate_quest_id]
    );
    assert!(
        !contains_player_quest_status_for_test(&session, duplicate_quest_id)
    );
    assert_eq!(
        player_quest_status_for_test(&session, active_quest_id)
            .map(|status| status.slot),
        Some(0)
    );
}
#[test]
fn save_to_db_quest_status_list_is_empty_without_active_quests_like_cpp() {
    let (session, _send_rx) = make_session();

    assert!(
        represented_quest_statuses_for_save_for_test(&session)
            .is_empty()
    );
}
#[test]
fn quest_log_create_entries_store_flag_objectives_in_state_flags_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 5918;
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: 10,
        order: 1,
        storage_index: 1,
        object_id: 45,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot(&mut session, quest_id, 3);
    with_player_quest_status_mut_for_test(&mut session, quest_id, |status| {
        status.objective_counts = vec![3, 1];
    })
    .expect("active quest");

    let entries = quest_log_create_entries_for_test(&session);

    assert_eq!(entries[3].1, 256 << 1);
    assert_eq!(entries[3].3[0], 3);
    assert_eq!(
        entries[3].3[1], 0,
        "C++ stores flag objectives in QuestLog.StateFlags, not ObjectiveProgress"
    );
}
#[test]
fn quest_log_create_entries_preserve_failed_state_flag_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot_with_status(&mut session, 5919, 5, QUEST_STATUS_FAILED_LIKE_CPP);

    let entries = quest_log_create_entries_for_test(&session);

    assert_eq!(entries[5].1, 2);
}
#[test]
fn quest_log_create_entries_duplicate_slot_is_empty_fail_closed_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5915, 2);
    add_active_quest_in_slot(&mut session, 5916, 2);

    let entries = quest_log_create_entries_for_test(&session);

    assert_eq!(entries.len(), MAX_QUEST_LOG_SIZE_LIKE_CPP as usize);
    assert_eq!(entries[2], (0, 0, 0, [0; 24]));
    assert!(
        contains_player_quest_status_for_test(&session, 5915)
    );
    assert!(
        contains_player_quest_status_for_test(&session, 5916)
    );
}
