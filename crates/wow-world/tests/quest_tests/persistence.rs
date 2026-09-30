//! Quest POI and status persistence at the handler owner.

use super::*;

#[tokio::test]
async fn quest_poi_cache_consumes_typed_port_rows_and_caches_the_result() {
    let (mut session, _) = make_session();
    session.set_quest_poi_persistence_port_like_cpp(Arc::new(QuestPoiPortFixtureLikeCpp(
        QuestPoiLoadOutcomeLikeCpp::Loaded {
            points: vec![QuestPoiPointLoadRowLikeCpp {
                quest_id: 77,
                idx1: 3,
                x: 10,
                y: 11,
                z: 12,
            }],
            blobs: vec![quest_poi_blob_row_like_cpp(77, 3)],
        },
    )));

    let first = quest_poi_store_for_test(&mut session).await;
    let second = quest_poi_store_for_test(&mut session).await;
    assert_eq!(first[&77].blobs.len(), 1);
    assert!(Arc::ptr_eq(&first, &second));
}

#[tokio::test]
async fn missing_or_failed_quest_poi_port_caches_the_existing_empty_result() {
    let (mut missing, _) = make_session();
    let missing_first = quest_poi_store_for_test(&mut missing).await;
    let missing_second = quest_poi_store_for_test(&mut missing).await;
    assert!(missing_first.is_empty());
    assert!(Arc::ptr_eq(&missing_first, &missing_second));

    let (mut failed, _) = make_session();
    failed.set_quest_poi_persistence_port_like_cpp(Arc::new(QuestPoiPortFixtureLikeCpp(
        QuestPoiLoadOutcomeLikeCpp::Failed {
            stage: QuestPoiLoadStageLikeCpp::Points,
            reason: "world DB unavailable".to_owned(),
        },
    )));
    let failed_store = quest_poi_store_for_test(&mut failed).await;
    assert!(failed_store.is_empty());
}

#[test]
fn save_to_db_quest_status_list_includes_active_quests_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot_with_status(&mut session, 5920, 2, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    add_active_quest_in_slot_with_status(&mut session, 5919, 3, QUEST_STATUS_COMPLETE_LIKE_CPP);

    assert_eq!(
        represented_quest_statuses_for_save_for_test(&session),
        vec![
            (5919, QUEST_STATUS_COMPLETE_LIKE_CPP),
            (5920, QUEST_STATUS_INCOMPLETE_LIKE_CPP)
        ],
        "C++ Player::SaveToDB reaches _SaveQuestStatus for represented active quests"
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
fn save_to_db_quest_status_list_is_empty_without_active_quests_like_cpp() {
    let (session, _send_rx) = make_session();

    assert!(
        represented_quest_statuses_for_save_for_test(&session)
            .is_empty()
    );
}

fn hydration_active_row(quest_id: u32) -> PlayerQuestActivePersistenceRowLikeCpp {
    PlayerQuestActivePersistenceRowLikeCpp {
        quest_id: Some(quest_id), status: Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        explored: Some(0), accept_time_secs: Some(12), end_time_secs: Some(34),
    }
}

#[tokio::test]
async fn quest_hydration_rejects_each_required_null_without_consuming_slots() {
    for missing_field in 0..5 {
        let (mut session, _) = make_session();
        let mut invalid = hydration_active_row(6000);
        match missing_field {
            0 => invalid.quest_id = None,
            1 => invalid.status = None,
            2 => invalid.explored = None,
            3 => invalid.accept_time_secs = None,
            _ => invalid.end_time_secs = None,
        }
        let fixture = player_quest_persistence_port_with_load_rows_for_test(
            vec![invalid, hydration_active_row(6001)], vec![], vec![], vec![], vec![], vec![],
        );
        session.set_player_quest_persistence_port_like_cpp(fixture.clone());
        load_player_quests_for_test(&mut session).await;
        let state = player_quest_gameplay_snapshot_for_test(&session).unwrap();
        assert!(!state.statuses_like_cpp().contains_key(&6000));
        assert_eq!(state.statuses_like_cpp()[&6001].slot, 0);
        assert!(state.statuses_like_cpp()[&6001].objective_counts.is_empty());
        assert!(!state.status_authority_complete_like_cpp());
        assert_eq!(player_quest_load_stages_for_test(&fixture).len(), 7);
    }
}

#[tokio::test]
async fn quest_hydration_preserves_sql_defaults_and_distinct_rewarded_memberships() {
    for null_rewarded_id in [false, true] {
        let (mut session, _) = make_session();
        let mut zero = quest_template(0);
        zero.flags |= QUEST_FLAGS_DAILY_LIKE_CPP;
        zero.objectives.push(QuestObjective {
            id: 1, quest_id: 0, obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL, order: 0,
            storage_index: 0, object_id: 44, amount: 5, flags: 0, flags2: 0,
            progress_bar_weight: 0.0, description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
            zero, quest_template(6100),
        ])));
        let fixture = player_quest_persistence_port_with_load_rows_for_test(
            vec![hydration_active_row(0)],
            vec![
                PlayerQuestObjectivePersistenceRowLikeCpp {
                    quest_id: Some(0), storage_index: Some(0), count: Some(5),
                },
                PlayerQuestObjectivePersistenceRowLikeCpp {
                    quest_id: None, storage_index: None, count: None,
                },
            ],
            vec![
                PlayerQuestIdPersistenceRowLikeCpp { quest_id: Some(0) },
                PlayerQuestIdPersistenceRowLikeCpp { quest_id: Some(6100) },
                PlayerQuestIdPersistenceRowLikeCpp { quest_id: Some(6101) },
                PlayerQuestIdPersistenceRowLikeCpp {
                    quest_id: if null_rewarded_id { None } else { Some(6100) },
                },
            ], vec![], vec![], vec![],
        );
        session.set_player_quest_persistence_port_like_cpp(fixture);
        load_player_quests_for_test(&mut session).await;
        let state = player_quest_gameplay_snapshot_for_test(&session).unwrap();
        assert_eq!(state.statuses_like_cpp()[&0].objective_counts, vec![0]);
        assert_eq!(state.rewarded_quest_rows_like_cpp().iter().copied().collect::<Vec<_>>(), vec![0, 6100, 6101]);
        assert_eq!(state.rewarded_quest_ids_like_cpp().iter().copied().collect::<Vec<_>>(), vec![6100]);
        assert_eq!(state.status_authority_complete_like_cpp(), !null_rewarded_id);
    }
}


struct QuestPoiPortFixtureLikeCpp(QuestPoiLoadOutcomeLikeCpp);

impl QuestPoiPersistencePortLikeCpp for QuestPoiPortFixtureLikeCpp {
    fn load_quest_poi_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestPoiLoadOutcomeLikeCpp> {
        let outcome = self.0.clone();
        Box::pin(async move { outcome })
    }
}


fn quest_poi_blob_row_like_cpp(quest_id: i32, idx1: i32) -> QuestPoiBlobLoadRowLikeCpp {
    QuestPoiBlobLoadRowLikeCpp {
        quest_id,
        blob_index: 1,
        idx1,
        objective_index: -1,
        quest_objective_id: 2,
        quest_object_id: 3,
        map_id: 571,
        ui_map_id: 486,
        priority: 4,
        flags: 5,
        world_effect_id: 6,
        player_condition_id: 7,
        navigation_player_condition_id: 8,
        spawn_tracking_id: 9,
        always_allow_merging_blobs: false,
    }
}

use wow_persistence::{PlayerQuestLoadOutcomeLikeCpp, PlayerQuestPersistencePortLikeCpp};
type HydrationLoad<'a, T> = PersistenceFutureLikeCpp<'a, PlayerQuestLoadOutcomeLikeCpp<T>>;

/// Override one query result while retaining the fixture's actual call trace.
struct HydrationFailurePort {
    base: PlayerQuestPersistencePortFixtureLikeCpp,
    failed_stage: PlayerQuestLoadStageFixtureLikeCpp,
}

impl HydrationFailurePort {
    fn outcome<'a, T: 'a>(
        &self, stage: PlayerQuestLoadStageFixtureLikeCpp, loaded: HydrationLoad<'a, T>,
    ) -> HydrationLoad<'a, T> {
        if stage == self.failed_stage {
            Box::pin(async { PlayerQuestLoadOutcomeLikeCpp::Failed { reason: "fixture failure".into() } })
        } else { loaded }
    }
}

impl PlayerQuestPersistencePortLikeCpp for HydrationFailurePort {
    fn load_active_statuses_like_cpp(&self, guid: u64) -> HydrationLoad<'_, PlayerQuestActivePersistenceRowLikeCpp> {
        self.outcome(PlayerQuestLoadStageFixtureLikeCpp::Active, self.base.load_active_statuses_like_cpp(guid))
    }
    fn load_objectives_like_cpp(&self, guid: u64) -> HydrationLoad<'_, PlayerQuestObjectivePersistenceRowLikeCpp> {
        self.outcome(PlayerQuestLoadStageFixtureLikeCpp::Objectives, self.base.load_objectives_like_cpp(guid))
    }
    fn load_rewarded_like_cpp(&self, guid: u64) -> HydrationLoad<'_, PlayerQuestIdPersistenceRowLikeCpp> {
        self.outcome(PlayerQuestLoadStageFixtureLikeCpp::Rewarded, self.base.load_rewarded_like_cpp(guid))
    }
    fn load_daily_like_cpp(&self, guid: u64) -> HydrationLoad<'_, PlayerQuestDailyPersistenceRowLikeCpp> {
        self.base.load_daily_like_cpp(guid)
    }
    fn load_weekly_like_cpp(&self, guid: u64) -> HydrationLoad<'_, PlayerQuestIdPersistenceRowLikeCpp> {
        self.base.load_weekly_like_cpp(guid)
    }
    fn load_monthly_like_cpp(&self, guid: u64) -> HydrationLoad<'_, PlayerQuestIdPersistenceRowLikeCpp> {
        self.base.load_monthly_like_cpp(guid)
    }
    fn load_seasonal_like_cpp(&self, guid: u64) -> HydrationLoad<'_, wow_persistence::PlayerQuestSeasonalPersistenceRowLikeCpp> {
        self.base.load_seasonal_like_cpp(guid)
    }
    fn persist_status_like_cpp(&self, request: PlayerQuestStatusPersistenceRequestLikeCpp) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        self.base.persist_status_like_cpp(request)
    }
    fn persist_lockout_like_cpp(&self, request: wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        self.base.persist_lockout_like_cpp(request)
    }
}

#[tokio::test]
async fn quest_hydration_query_failures_preserve_stages_install_and_authority() {
    for failed_stage in [PlayerQuestLoadStageFixtureLikeCpp::Active,
        PlayerQuestLoadStageFixtureLikeCpp::Objectives, PlayerQuestLoadStageFixtureLikeCpp::Rewarded]
    {
        let (mut session, _) = make_session();
        let mut quest = quest_template(6200);
        quest.objectives.push(QuestObjective {
            id: 1, quest_id: 6200, obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL, order: 0,
            storage_index: 0, object_id: 44, amount: 5, flags: 0, flags2: 0,
            progress_bar_weight: 0.0, description: String::new(),
        });
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
            quest, quest_template(6201),
        ])));
        add_active_quest_in_slot(&mut session, 6209, 4);
        mutate_player_quest_gameplay_for_test(&mut session, |state| {
            state.set_rewarded_like_cpp(6208, true);
            state.set_rewarded_row_like_cpp(6208, true);
            state.set_status_authority_complete_like_cpp(true);
        }).unwrap();
        let fixture = Arc::new(HydrationFailurePort {
            failed_stage,
            base: Arc::try_unwrap(player_quest_persistence_port_with_load_rows_for_test(
                vec![hydration_active_row(6200)],
                vec![PlayerQuestObjectivePersistenceRowLikeCpp {
                    quest_id: Some(6200), storage_index: Some(0), count: Some(9),
                }],
                vec![PlayerQuestIdPersistenceRowLikeCpp { quest_id: Some(6201) }],
                vec![], vec![], vec![],
            )).ok().expect("unique quest persistence fixture"),
        });
        session.set_player_quest_persistence_port_like_cpp(fixture.clone());
        load_player_quests_for_test(&mut session).await;
        let state = player_quest_gameplay_snapshot_for_test(&session).unwrap();
        let stages = player_quest_load_stages_for_test(&fixture.base);
        if failed_stage == PlayerQuestLoadStageFixtureLikeCpp::Active {
            assert_eq!(stages, vec![PlayerQuestLoadStageFixtureLikeCpp::Active]);
            assert!(state.statuses_like_cpp().contains_key(&6209));
            assert!(!state.statuses_like_cpp().contains_key(&6200));
            assert!(state.rewarded_quest_ids_like_cpp().contains(&6208));
            assert!(state.rewarded_quest_rows_like_cpp().is_empty());
        } else {
            assert_eq!(stages, vec![PlayerQuestLoadStageFixtureLikeCpp::Active,
                PlayerQuestLoadStageFixtureLikeCpp::Objectives, PlayerQuestLoadStageFixtureLikeCpp::Rewarded,
                PlayerQuestLoadStageFixtureLikeCpp::Daily, PlayerQuestLoadStageFixtureLikeCpp::Weekly,
                PlayerQuestLoadStageFixtureLikeCpp::Monthly, PlayerQuestLoadStageFixtureLikeCpp::Seasonal]);
            assert!(!state.statuses_like_cpp().contains_key(&6209));
            assert_eq!(state.statuses_like_cpp()[&6200].objective_counts,
                vec![if failed_stage == PlayerQuestLoadStageFixtureLikeCpp::Objectives { 0 } else { 9 }]);
            assert_eq!(state.rewarded_quest_rows_like_cpp().contains(&6201),
                failed_stage != PlayerQuestLoadStageFixtureLikeCpp::Rewarded);
        }
        assert_eq!(state.status_authority_complete_like_cpp(),
            failed_stage == PlayerQuestLoadStageFixtureLikeCpp::Objectives);
    }
}
