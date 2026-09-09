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
    session.player_quests.insert(
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

    let projected = session.represented_quest_status_persistence_like_cpp(
        session.player_quests.get(&quest_id).unwrap(),
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
    session.player_quests.insert(
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
    let fixture = PlayerQuestPersistencePortFixtureLikeCpp::default();
    let requests = Arc::clone(&fixture.status_requests);
    session.set_player_quest_persistence_port_like_cpp(Arc::new(fixture));

    session
        .save_quest_to_db(quest_id, QUEST_STATUS_COMPLETE_LIKE_CPP)
        .await;

    assert_eq!(
        requests.lock().unwrap().as_slice(),
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

    let fixture = PlayerQuestPersistencePortFixtureLikeCpp {
        active: vec![PlayerQuestActivePersistenceRowLikeCpp {
            quest_id: Some(active_id),
            status: Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
            explored: Some(1),
            accept_time_secs: Some(12),
            end_time_secs: Some(34),
        }],
        objectives: vec![PlayerQuestObjectivePersistenceRowLikeCpp {
            quest_id: Some(active_id),
            storage_index: Some(0),
            count: Some(3),
        }],
        rewarded: vec![PlayerQuestIdPersistenceRowLikeCpp {
            quest_id: Some(rewarded_id),
        }],
        daily: vec![PlayerQuestDailyPersistenceRowLikeCpp {
            quest_id: Some(daily_id),
            completed_time: Some(45),
        }],
        weekly: vec![PlayerQuestIdPersistenceRowLikeCpp {
            quest_id: Some(weekly_id),
        }],
        monthly: vec![PlayerQuestIdPersistenceRowLikeCpp {
            quest_id: Some(monthly_id),
        }],
        ..Default::default()
    };
    let stages = Arc::clone(&fixture.stages);
    session.set_player_quest_persistence_port_like_cpp(Arc::new(fixture));

    session.load_player_quests().await;

    assert_eq!(
        stages.lock().unwrap().as_slice(),
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
    assert_eq!(session.player_quests[&active_id].objective_counts, vec![3]);
    assert!(session.rewarded_quests.contains(&rewarded_id));
    assert!(session.daily_quests_completed_like_cpp.contains(&daily_id));
    assert!(
        session
            .weekly_quests_completed_like_cpp
            .contains(&weekly_id)
    );
    assert!(
        session
            .monthly_quests_completed_like_cpp
            .contains(&monthly_id)
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
    session.quests.store = Some(Arc::new(quest_store));
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
    session.rewarded_quests.insert(active_rewarded_quest_id);

    assert_eq!(
        session.represented_quest_statuses_for_save_like_cpp(),
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
    session.quests.store = Some(Arc::new(quest_store));
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
    session.rewarded_quests.insert(duplicate_quest_id);

    assert_eq!(
        session.remove_represented_active_rewarded_duplicates_like_cpp(),
        vec![duplicate_quest_id]
    );
    assert!(!session.player_quests.contains_key(&duplicate_quest_id));
    assert_eq!(
        session
            .player_quests
            .get(&active_quest_id)
            .map(|status| status.slot),
        Some(0)
    );
}
#[test]
fn save_to_db_quest_status_list_is_empty_without_active_quests_like_cpp() {
    let (session, _send_rx) = make_session();

    assert!(
        session
            .represented_quest_statuses_for_save_like_cpp()
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
    session
        .player_quests
        .get_mut(&quest_id)
        .expect("active quest")
        .objective_counts = vec![3, 1];

    let entries = session.quest_log_create_entries_like_cpp();

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

    let entries = session.quest_log_create_entries_like_cpp();

    assert_eq!(entries[5].1, 2);
}
#[test]
fn quest_log_create_entries_duplicate_slot_is_empty_fail_closed_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5915, 2);
    add_active_quest_in_slot(&mut session, 5916, 2);

    let entries = session.quest_log_create_entries_like_cpp();

    assert_eq!(entries.len(), MAX_QUEST_LOG_SIZE_LIKE_CPP as usize);
    assert_eq!(entries[2], (0, 0, 0, [0; 24]));
    assert!(session.player_quests.contains_key(&5915));
    assert!(session.player_quests.contains_key(&5916));
}
#[test]
fn can_take_quest_blocks_when_quest_available_condition_not_met_like_cpp() {
    // NEGATIVA: nivel requerido 90, jugador nivel 80 → condición no se cumple.
    let (mut session, _send_rx) = make_session();
    let quest_id = 7570u32;
    let quest = quest_template(quest_id);
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id as i32,
            condition_type: ConditionType::Level,
            condition_value1: 90,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));

    // Sin la condición la quest sería aceptable (nivel 1, min_level 1, raza/clase sin filtro).
    // Con la condición de nivel 90, el jugador (80) no la cumple.
    assert!(!session.can_take_quest(&quest));

    // POSITIVA: nivel requerido 80 (alcanzable para el jugador en nivel 80).
    let quest_id2 = 7571u32;
    let quest2 = quest_template(quest_id2);
    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session.set_quest_store(Arc::new(store2));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id2 as i32,
            condition_type: ConditionType::Level,
            condition_value1: 80,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));

    assert!(session.can_take_quest(&quest2));
}
#[test]
fn can_take_quest_blocks_when_session_expansion_below_required_like_cpp() {
    // NEGATIVA: expansión de sesión 1 < expansión requerida 2 → rechaza.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(9900u32);
    quest.expansion = 2;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.expansion = 1;
    assert!(!session.can_take_quest(&quest));

    // POSITIVA límite: expansión de sesión == expansión requerida → acepta.
    let (mut session2, _send_rx2) = make_session();
    let mut quest2 = quest_template(9901u32);
    quest2.expansion = 2;
    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session2.set_quest_store(Arc::new(store2));
    session2.expansion = 2;
    assert!(session2.can_take_quest(&quest2));
}
#[test]
fn can_take_quest_blocks_daily_already_completed_like_cpp() {
    // NEGATIVE: a daily quest already in DailyQuestsCompleted is blocked
    // (C++ SatisfyQuestDay, Player.cpp:15393-15407).
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7600u32);
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.daily_quests_completed_like_cpp.insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_allows_daily_not_yet_completed_like_cpp() {
    // POSITIVE: a daily quest not yet completed today is acceptable.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7601u32);
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_blocks_df_quest_already_completed_like_cpp() {
    // NEGATIVE: a DF (dungeon-finder) quest already in DFQuests is blocked
    // (C++ SatisfyQuestDay DFQuest branch, Player.cpp:15393-15407).
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7602u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.df_quests_like_cpp.insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_blocks_weekly_already_completed_like_cpp() {
    // NEGATIVE: a weekly quest already in the weekly cooldown set is blocked
    // (C++ SatisfyQuestWeek, Player.cpp:15409-15418).
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7603u32);
    quest.flags = QUEST_FLAGS_WEEKLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.weekly_quests_completed_like_cpp.insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_allows_weekly_not_yet_completed_like_cpp() {
    // POSITIVE: a weekly quest not on cooldown is acceptable.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7604u32);
    quest.flags = QUEST_FLAGS_WEEKLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_blocks_monthly_already_completed_like_cpp() {
    // NEGATIVE: a monthly quest already in the monthly cooldown set is blocked
    // (C++ SatisfyQuestMonth, Player.cpp:15445-15454).
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7605u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.monthly_quests_completed_like_cpp.insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_allows_monthly_not_yet_completed_like_cpp() {
    // POSITIVE: a monthly quest not on cooldown is acceptable.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7606u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_exclusive_group_blocks_when_peer_rewarded_non_repeatable_like_cpp() {
    // NEGATIVA: peer del mismo exclusive_group (>0) ya rewarded (no repetible)
    // → Player.cpp:15379 segundo término: !(repeatable && repeatable) && rewarded → false.
    let (mut session, _send_rx) = make_session();

    let mut peer = quest_template(9910u32);
    peer.exclusive_group = 5;
    // is_repeatable() false por defecto (quest_type=2, special_flags=0)

    let mut quest = quest_template(9911u32);
    quest.exclusive_group = 5;

    let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
    session.set_quest_store(Arc::new(store));

    // El peer ya fue recompensado (no repetible).
    session.rewarded_quests.insert(peer.id);

    // La quest objetivo no está ni activa ni rewarded.
    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_exclusive_group_blocks_when_peer_active_like_cpp() {
    // NEGATIVA-2: peer del mismo exclusive_group activo en player_quests (status != NONE)
    // → Player.cpp:15379 primer término: GetQuestStatus(peer) != NONE → false.
    let (mut session, _send_rx) = make_session();

    let mut peer = quest_template(9912u32);
    peer.exclusive_group = 7;

    let mut quest = quest_template(9913u32);
    quest.exclusive_group = 7;

    let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
    session.set_quest_store(Arc::new(store));

    // El peer está activo (Incomplete).
    add_active_quest(&mut session, peer.id);

    // La quest objetivo aún no ha sido aceptada.
    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_exclusive_group_positive_no_conflicting_peer_allows_like_cpp() {
    // POSITIVA: exclusive_group > 0 pero ningún peer está activo ni rewarded → true.
    let (mut session, _send_rx) = make_session();

    let mut peer = quest_template(9914u32);
    peer.exclusive_group = 9;

    let mut quest = quest_template(9915u32);
    quest.exclusive_group = 9;

    let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
    session.set_quest_store(Arc::new(store));

    // Ningún peer activo ni rewarded → debe permitir.
    assert!(session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_exclusive_group_zero_never_blocks_like_cpp() {
    // POSITIVA: exclusive_group <= 0 → Player.cpp:15351 → siempre true.
    let (mut session, _send_rx) = make_session();

    let mut quest = quest_template(9916u32);
    quest.exclusive_group = 0;

    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));

    // También verifica group negativo.
    let (mut session2, _send_rx2) = make_session();
    let mut quest2 = quest_template(9917u32);
    quest2.exclusive_group = -3;

    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session2.set_quest_store(Arc::new(store2));

    assert!(session2.can_take_quest(&quest2));
}
#[test]
fn can_take_quest_dependent_previous_not_rewarded_blocks_like_cpp() {
    // NEGATIVA: quest objetivo con dependent_previous_quests=[prev] (exclusive_group=0 >= 0),
    // prev NO en rewarded_quests → Player.cpp:15121-15177 → false.
    let (mut session, _send_rx) = make_session();

    let quest_id = 9920u32;
    let prev_id = 9921u32;

    // Construir quest previa con next_quest_id → quest objetivo, exclusive_group=0 (>= 0).
    let mut prev_quest = quest_template(prev_id);
    prev_quest.next_quest_id = quest_id;
    prev_quest.exclusive_group = 0;

    // from_quests_like_cpp normaliza dependent_previous_quests automáticamente.
    let store = Arc::new(QuestStore::from_quests_like_cpp([
        quest_template(quest_id),
        prev_quest,
    ]));
    // Obtener la quest del store ya normalizado (con dependent_previous_quests populado).
    let quest = store.get(quest_id).expect("quest in store").clone();
    session.set_quest_store(Arc::clone(&store));

    // prev NO en rewarded_quests → el gate debe bloquear.
    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_dependent_previous_rewarded_allows_like_cpp() {
    // POSITIVA: mismo setup pero prev en rewarded_quests (exclusive_group=0 >= 0)
    // → Player.cpp:15134 → false (no blocked) → can_take_quest sigue adelante.
    let (mut session, _send_rx) = make_session();

    let quest_id = 9922u32;
    let prev_id = 9923u32;

    let mut prev_quest = quest_template(prev_id);
    prev_quest.next_quest_id = quest_id;
    prev_quest.exclusive_group = 0;

    let store = Arc::new(QuestStore::from_quests_like_cpp([
        quest_template(quest_id),
        prev_quest,
    ]));
    let quest = store.get(quest_id).expect("quest in store").clone();
    session.set_quest_store(Arc::clone(&store));

    // prev en rewarded_quests → el gate no bloquea.
    session.rewarded_quests.insert(prev_id);
    assert!(session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_reputation_blocks_when_below_min_rep_like_cpp() {
    // NEGATIVA min: required_min_rep_faction != 0, jugador con standing < required_min_rep_value.
    // → Player.cpp:15265 → false.
    let (mut session, _send_rx) = make_session();

    // Facción 76 con reputation_index 5.
    let rep_list_id: u32 = 5;
    let faction_id: u32 = 76;
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16),
    ])));
    // Jugador standing 100 — por debajo del mínimo requerido (500).
    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(rep_list_id)
        .expect("reputation state")
        .standing = 100;

    let mut quest = quest_template(9920u32);
    quest.required_min_rep_faction = faction_id;
    quest.required_min_rep_value = 500;
    // Aislar el gate: sin restricciones de raza/clase/level/conditions/expansion/exclusive_group.
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_reputation_blocks_when_at_or_above_max_rep_like_cpp() {
    // NEGATIVA max: required_max_rep_faction != 0, jugador con standing >= required_max_rep_value.
    // → Player.cpp:15277 → false.
    let (mut session, _send_rx) = make_session();

    let rep_list_id: u32 = 5;
    let faction_id: u32 = 76;
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16),
    ])));
    // Jugador standing 1000 — igual al máximo requerido (1000) → bloqueado.
    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(rep_list_id)
        .expect("reputation state")
        .standing = 1000;

    let mut quest = quest_template(9921u32);
    quest.required_max_rep_faction = faction_id;
    quest.required_max_rep_value = 1000;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_reputation_allows_when_in_valid_range_like_cpp() {
    // POSITIVA: standing >= min y < max → el gate de reputación no bloquea.
    let (mut session, _send_rx) = make_session();

    let rep_list_id: u32 = 5;
    let faction_id: u32 = 76;
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16),
    ])));
    // Standing 500 — exactamente en el mínimo (500) y por debajo del máximo (1000).
    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(rep_list_id)
        .expect("reputation state")
        .standing = 500;

    let mut quest = quest_template(9922u32);
    quest.required_min_rep_faction = faction_id;
    quest.required_min_rep_value = 500;
    quest.required_max_rep_faction = faction_id;
    quest.required_max_rep_value = 1000;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_skill_blocks_when_skill_below_required_like_cpp() {
    // NEGATIVA: required_skill_id != 0, skill del jugador < required_skill_points → false.
    // Player.cpp:15015-15037.
    let skill_id: u16 = 1_000;
    let required_points: u32 = 300;

    let (mut session, _send_rx) = make_session();
    // Jugador con skill 1000 en valor 299 — por debajo del requisito.
    session
        .set_player_skill_values_like_cpp(std::collections::HashMap::from([(skill_id, 299_u16)]));

    let mut quest = quest_template(9940u32);
    quest.required_skill_id = u32::from(skill_id);
    quest.required_skill_points = required_points;
    // Aislar el gate: sin restricciones de raza/clase/level/rep/conditions/expansion.
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(!session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_skill_allows_when_skill_at_or_above_required_like_cpp() {
    // POSITIVA: skill del jugador >= required_skill_points → el gate no bloquea.
    let skill_id: u16 = 1_000;
    let required_points: u32 = 300;

    let (mut session, _send_rx) = make_session();
    // Jugador con skill 1000 exactamente en 300.
    session
        .set_player_skill_values_like_cpp(std::collections::HashMap::from([(skill_id, 300_u16)]));

    let mut quest = quest_template(9941u32);
    quest.required_skill_id = u32::from(skill_id);
    quest.required_skill_points = required_points;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}
#[test]
fn can_take_quest_blocks_when_dependent_breadcrumb_in_log_like_cpp() {
    let breadcrumb_quest_id = 9930u32;
    let quest_id = 9931u32;

    // NEGATIVA: breadcrumb B (9930) está en player_quests con status INCOMPLETE
    // → Player.cpp:15203-15222 → false.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(quest_id);
    quest.dependent_breadcrumb_quests = vec![breadcrumb_quest_id];
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    add_active_quest(&mut session, breadcrumb_quest_id);
    assert!(!session.can_take_quest(&quest));

    // POSITIVA: breadcrumb no está en el log → no bloquea.
    let (mut session2, _send_rx2) = make_session();
    let mut quest2 = quest_template(quest_id);
    quest2.dependent_breadcrumb_quests = vec![breadcrumb_quest_id];
    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session2.set_quest_store(Arc::new(store2));
    // breadcrumb_quest_id no insertado en player_quests
    assert!(session2.can_take_quest(&quest2));
}
