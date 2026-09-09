//! Misc scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn game_event_world_catalog_keeps_world_character_world_order() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let world = RecordingGameEventWorldCatalogLikeCpp {
        calls: Arc::clone(&calls),
        fail_prefix: false,
        fail_suffix: false,
    };
    let character = RecordingGameEventConditionSaveLikeCpp {
        calls: Arc::clone(&calls),
        fail: false,
    };
    let mut events = GameEventDataStoreLikeCpp::default();
    let mut report = CanonicalSpawnStoreLoadReport::default();

    load_game_event_world_prefix_like_cpp(&world).await.unwrap();
    load_game_event_condition_saves_then_world_suffix_like_cpp(
        &character,
        &world,
        &mut events,
        &mut report,
    )
    .await
    .unwrap();

    assert_eq!(
        *calls.lock().unwrap(),
        ["world-prefix", "character-saves", "world-suffix"]
    );
}
#[tokio::test]
async fn game_event_character_failure_stops_before_world_suffix() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let world = RecordingGameEventWorldCatalogLikeCpp {
        calls: Arc::clone(&calls),
        fail_prefix: false,
        fail_suffix: false,
    };
    let character = RecordingGameEventConditionSaveLikeCpp {
        calls: Arc::clone(&calls),
        fail: true,
    };
    let mut events = GameEventDataStoreLikeCpp::default();
    let mut report = CanonicalSpawnStoreLoadReport::default();

    load_game_event_world_prefix_like_cpp(&world).await.unwrap();
    let error = load_game_event_condition_saves_then_world_suffix_like_cpp(
        &character,
        &world,
        &mut events,
        &mut report,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("condition-save"));
    assert_eq!(*calls.lock().unwrap(), ["world-prefix", "character-saves"]);
}
#[tokio::test]
async fn world_state_startup_capability_keeps_success_and_failure_distinct() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let success = RecordingWorldStateStartupLikeCpp {
        calls: Arc::clone(&calls),
        fail: false,
    };
    let maps = map_store(&[]);
    let areas = wow_data::AreaTableStore::default();

    load_world_state_mgr_like_cpp(&success, &maps, &areas)
        .await
        .unwrap();
    assert_eq!(*calls.lock().unwrap(), ["world-then-characters"]);

    calls.lock().unwrap().clear();
    let failure = RecordingWorldStateStartupLikeCpp {
        calls: Arc::clone(&calls),
        fail: true,
    };
    let error = load_world_state_mgr_like_cpp(&failure, &maps, &areas)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("WorldState"));
    assert_eq!(*calls.lock().unwrap(), ["world-then-characters"]);
}
#[test]
fn game_event_is_active_event_checks_membership_like_cpp() {
    let mut active = GameEventActiveSetLikeCpp::new();
    active.add_active_event_like_cpp(3);

    assert!(active.is_active_event_like_cpp(3));
    assert!(!active.is_active_event_like_cpp(4));
}
#[test]
fn game_event_is_holiday_active_matches_cpp_and_reports_missing_active_event_like_cpp() {
    let store = game_event_store([
        event_with_holiday(event(1, GameEventStateLikeCpp::Normal, 0, 0, 0, 0), 141),
        event_with_holiday(event(2, GameEventStateLikeCpp::Normal, 0, 0, 0, 0), 142),
    ]);
    let mut active = GameEventActiveSetLikeCpp::new();

    assert_eq!(
        active.is_holiday_active_like_cpp(&store, 0),
        GameEventHolidayActiveOutcomeLikeCpp::Active(false)
    );

    active.add_active_event_like_cpp(2);
    assert_eq!(
        active.is_holiday_active_like_cpp(&store, 142),
        GameEventHolidayActiveOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        active.is_holiday_active_like_cpp(&store, 141),
        GameEventHolidayActiveOutcomeLikeCpp::Active(false)
    );

    active.add_active_event_like_cpp(99);
    assert_eq!(
        active.is_holiday_active_like_cpp(&store, 141),
        GameEventHolidayActiveOutcomeLikeCpp::MissingActiveEvent { event_id: 99 }
    );
}
#[test]
fn game_event_active_set_lives_with_canonical_metadata_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new());

    assert!(
        !metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(4)
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(4);
    assert!(
        metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(4)
    );
}
#[test]
fn game_event_world_state_update_evidence_orders_conditions_done_then_max_like_cpp() {
    let event = event_with_condition(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            20,
            GameEventConditionLikeCpp {
                req_num: 9.8,
                done: 4.2,
                max_world_state: 220,
                done_world_state: 221,
            },
        ),
        10,
        GameEventConditionLikeCpp {
            req_num: 7.0,
            done: 3.0,
            max_world_state: 120,
            done_world_state: 121,
        },
    );
    let events = game_event_store([event]);

    assert_eq!(
        events.send_world_state_update_evidence_like_cpp(1),
        GameEventWorldStateUpdateOutcomeLikeCpp::Updates {
            event_id: 1,
            updates: vec![
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 121,
                    value: 3,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 120,
                    value: 7,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 221,
                    value: 4,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 220,
                    value: 9,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                },
            ],
            skipped: Vec::new(),
        }
    );
}
#[test]
fn game_event_world_state_update_skips_zero_worldstate_ids_like_cpp() {
    let events = game_event_store([event_with_condition(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            GameEventConditionLikeCpp {
                req_num: 2.0,
                done: 1.0,
                max_world_state: 0,
                done_world_state: 88,
            },
        ),
        20,
        GameEventConditionLikeCpp {
            req_num: 4.0,
            done: 3.0,
            max_world_state: 77,
            done_world_state: 0,
        },
    )]);

    assert_eq!(
        events.send_world_state_update_evidence_like_cpp(1),
        GameEventWorldStateUpdateOutcomeLikeCpp::Updates {
            event_id: 1,
            updates: vec![
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 88,
                    value: 1,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 77,
                    value: 4,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                },
            ],
            skipped: Vec::new(),
        }
    );
}
#[test]
fn game_event_world_state_update_skips_invalid_numeric_values_like_cpp() {
    let events = game_event_store([event_with_condition(
        event_with_condition(
            event_with_condition(
                event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
                10,
                GameEventConditionLikeCpp {
                    req_num: f32::INFINITY,
                    done: -1.0,
                    max_world_state: 110,
                    done_world_state: 111,
                },
            ),
            20,
            GameEventConditionLikeCpp {
                req_num: 2_147_483_648.0,
                done: 2.0,
                max_world_state: 220,
                done_world_state: 221,
            },
        ),
        30,
        GameEventConditionLikeCpp {
            req_num: 3.0,
            done: f32::NAN,
            max_world_state: 330,
            done_world_state: 331,
        },
    )]);

    assert_eq!(
        events.send_world_state_update_evidence_like_cpp(1),
        GameEventWorldStateUpdateOutcomeLikeCpp::Updates {
            event_id: 1,
            updates: vec![
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 221,
                    value: 2,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 30,
                    variable_id: 330,
                    value: 3,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                },
            ],
            skipped: vec![
                GameEventWorldStateUpdateSkipLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 111,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                    reason: GameEventWorldStateValueSkipReasonLikeCpp::Negative,
                },
                GameEventWorldStateUpdateSkipLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 110,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                    reason: GameEventWorldStateValueSkipReasonLikeCpp::NonFinite,
                },
                GameEventWorldStateUpdateSkipLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 220,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                    reason: GameEventWorldStateValueSkipReasonLikeCpp::OutOfI32Range,
                },
                GameEventWorldStateUpdateSkipLikeCpp {
                    event_id: 1,
                    condition_id: 30,
                    variable_id: 331,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                    reason: GameEventWorldStateValueSkipReasonLikeCpp::NonFinite,
                },
            ],
        }
    );
}
#[test]
fn game_event_check_one_conditions_empty_loop_completes_and_preserves_next_start_like_cpp() {
    let mut events = game_event_store([event_with_next_start(
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
        999,
    )]);

    assert_eq!(
        events.check_one_game_event_conditions_like_cpp(1, 100),
        GameEventConditionCheckOutcomeLikeCpp::Completed(GameEventConditionCheckSummaryLikeCpp {
            event_id: 1,
            condition_count: 0,
            state_before_raw: GameEventStateLikeCpp::WorldConditions as u8,
            state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            next_start_before: 999,
            next_start_after: 999,
        })
    );
}
#[test]
fn game_event_check_one_conditions_blocks_until_all_done_like_cpp() {
    let mut events = game_event_store([event_with_condition(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            20,
            condition(2.0, 2.0),
        ),
        10,
        condition(3.0, 1.0),
    )]);

    assert_eq!(
        events.check_one_game_event_conditions_like_cpp(1, 100),
        GameEventConditionCheckOutcomeLikeCpp::NotCompleted {
            event_id: 1,
            blocking_condition_id: 10,
        }
    );
    assert_eq!(
        events.event_like_cpp(1).unwrap().state_raw,
        GameEventStateLikeCpp::WorldConditions as u8
    );
}
#[test]
fn game_event_condition_progress_early_returns_do_not_mutate_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        )]));

    assert_eq!(
        metadata.represented_update_game_event_condition_progress_like_cpp(1, 10, 1.0, 100),
        GameEventConditionProgressOutcomeLikeCpp::InactiveEvent { event_id: 1 }
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);
    assert_eq!(
        metadata.represented_update_game_event_condition_progress_like_cpp(1, 99, 1.0, 100),
        GameEventConditionProgressOutcomeLikeCpp::MissingCondition {
            event_id: 1,
            condition_id: 99,
        }
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}
#[test]
fn game_event_start_normal_internal_adds_active_apply_only_like_cpp() {
    for state in [
        GameEventStateLikeCpp::Normal,
        GameEventStateLikeCpp::Internal,
    ] {
        let mut metadata =
            CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
                .with_game_events_like_cpp(game_event_store([event(1, state, 100, 1_000, 10, 2)]));

        assert_eq!(
            metadata.start_game_event_like_cpp(1, false, 500, true),
            GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
                event_id: 1,
                state_before_raw: state as u8,
                state_after_raw: state as u8,
                active_added: true,
                active_was_present: false,
                apply_new_event_requested: true,
                save_world_event_state_requested: false,
                force_game_event_update_requested: false,
                completed: false,
            })
        );
        assert!(
            metadata
                .game_event_active_set_like_cpp()
                .is_active_event_like_cpp(1)
        );
        assert_eq!(metadata.game_event_like_cpp(1).unwrap().start, 100);
    }
}
#[test]
fn game_event_start_normal_overwrite_repairs_end_without_minutes_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event(
            1,
            GameEventStateLikeCpp::Normal,
            100,
            400,
            10,
            7,
        )]));

    let outcome = metadata.start_game_event_like_cpp(1, true, 500, false);

    assert!(matches!(
        outcome,
        GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
            completed: false,
            save_world_event_state_requested: false,
            ..
        })
    ));
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.start, 500);
    assert_eq!(event.end, 507);
}
#[test]
fn game_event_start_unknown_raw_state_is_serverwide_no_panic_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_raw_state(1, 99, 0, 0, 0, 3)]));

    assert_eq!(
        metadata.start_game_event_like_cpp(1, false, 100, true),
        GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
            event_id: 1,
            state_before_raw: 99,
            state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            active_added: true,
            active_was_present: false,
            apply_new_event_requested: true,
            save_world_event_state_requested: true,
            force_game_event_update_requested: false,
            completed: true,
        })
    );
    assert_eq!(metadata.game_event_like_cpp(1).unwrap().next_start, 280);
}
#[test]
fn game_event_stop_normal_overwrite_removes_active_and_repairs_without_minutes_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event(
            1,
            GameEventStateLikeCpp::Normal,
            0,
            70,
            10,
            7,
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.stop_game_event_like_cpp(1, true, 500),
        GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: GameEventStateLikeCpp::Normal as u8,
            state_after_raw: GameEventStateLikeCpp::Normal as u8,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: false,
            condition_reset_requested: false,
            delete_world_event_state_requested: false,
            delete_condition_saves_requested: false,
        })
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.start, 80);
    assert_eq!(event.end, 87);
    assert!(
        !metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}
#[test]
fn game_event_stop_serverwide_non_finished_resets_and_reports_deletes_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_next_start(
            event(1, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 7),
            777,
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.stop_game_event_like_cpp(1, false, 500),
        GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            state_after_raw: GameEventStateLikeCpp::WorldInactive as u8,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: true,
            condition_reset_requested: true,
            delete_world_event_state_requested: true,
            delete_condition_saves_requested: true,
        })
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldInactive as u8);
    assert_eq!(event.next_start, 0);
}
#[test]
fn game_event_stop_world_finished_without_overwrite_keeps_state_but_unapplies_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_next_start(
            event(1, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 7),
            777,
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.stop_game_event_like_cpp(1, false, 500),
        GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: GameEventStateLikeCpp::WorldFinished as u8,
            state_after_raw: GameEventStateLikeCpp::WorldFinished as u8,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: true,
            condition_reset_requested: false,
            delete_world_event_state_requested: false,
            delete_condition_saves_requested: false,
        })
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldFinished as u8);
    assert_eq!(event.next_start, 777);
    assert!(
        !metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}
#[test]
fn game_event_start_stop_missing_event_do_not_mutate_active_set_or_events_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event(
            1,
            GameEventStateLikeCpp::Normal,
            100,
            200,
            10,
            7,
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);
    let before = metadata.game_event_like_cpp(1).unwrap().clone();
    let active_before = metadata
        .game_event_active_set_like_cpp()
        .active_event_ids_like_cpp()
        .collect::<Vec<_>>();

    assert_eq!(
        metadata.start_game_event_like_cpp(99, true, 500, true),
        GameEventStartOutcomeLikeCpp::MissingEvent { event_id: 99 }
    );
    assert_eq!(
        metadata.stop_game_event_like_cpp(99, true, 500),
        GameEventStopOutcomeLikeCpp::MissingEvent { event_id: 99 }
    );
    assert_eq!(metadata.game_event_like_cpp(1).unwrap(), &before);
    assert_eq!(
        metadata
            .game_event_active_set_like_cpp()
            .active_event_ids_like_cpp()
            .collect::<Vec<_>>(),
        active_before
    );
}
#[test]
fn game_event_update_queues_starts_before_stops_sorted_and_updates_active_set_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store_with_max(
            3,
            [
                event(1, GameEventStateLikeCpp::Normal, 200, 1_000, 10, 2),
                event(2, GameEventStateLikeCpp::Normal, 0, 1_000, 10, 2),
                event(3, GameEventStateLikeCpp::Normal, 0, 1_000, 10, 2),
            ],
        ));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(3);
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(2);

    let outcome = metadata.update_game_events_like_cpp(250, true, |_| false);

    assert_eq!(outcome.scanned_event_ids, vec![1, 2, 3]);
    assert_eq!(outcome.queued_activation_event_ids, vec![1]);
    assert_eq!(outcome.queued_deactivation_event_ids, vec![2, 3]);
    assert!(matches!(
        outcome.start_outcomes.as_slice(),
        [GameEventStartOutcomeLikeCpp::Started(
            GameEventStartSummaryLikeCpp {
                event_id: 1,
                active_added: true,
                ..
            }
        )]
    ));
    assert!(matches!(
        outcome.stop_outcomes.as_slice(),
        [
            GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
                event_id: 2,
                active_removed: true,
                ..
            }),
            GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
                event_id: 3,
                active_removed: true,
                ..
            })
        ]
    ));
    assert_eq!(
        metadata
            .game_event_active_set_like_cpp()
            .active_event_ids_like_cpp()
            .collect::<Vec<_>>(),
        vec![1]
    );
}
#[test]
fn game_event_update_invalid_zero_occurrence_surfaces_without_fake_start_or_stop_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store_with_max(
            1,
            [event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 0, 2)],
        ));

    let outcome = metadata.update_game_events_like_cpp(200, false, |_| false);

    assert_eq!(
        outcome.invalid_check_outcomes,
        vec![GameEventCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { event_id: 1 }]
    );
    assert!(outcome.start_outcomes.is_empty());
    assert!(outcome.stop_outcomes.is_empty());
    assert!(outcome.queued_activation_event_ids.is_empty());
    assert!(outcome.queued_deactivation_event_ids.is_empty());
    assert!(outcome.negative_spawn_event_ids.is_empty());
    assert_eq!(
        outcome.next_event_delay_secs_before_padding,
        MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP
    );
    assert_eq!(
        metadata
            .game_event_active_set_like_cpp()
            .active_event_ids_like_cpp()
            .collect::<Vec<_>>(),
        Vec::<u16>::new()
    );
}
#[test]
fn game_event_seasonal_last_start_time_normal_event_like_cpp() {
    let store = game_event_store([event(1, GameEventStateLikeCpp::Normal, 100, 2_000, 10, 2)]);

    assert_eq!(store.last_start_time_like_cpp(1, 1_350), 1_300);
}
#[test]
fn game_event_seasonal_last_start_time_non_normal_out_of_range_and_zero_occurrence_like_cpp() {
    let store = game_event_store_with_max(
        2,
        [
            event(1, GameEventStateLikeCpp::WorldInactive, 100, 2_000, 10, 2),
            event(2, GameEventStateLikeCpp::Normal, 100, 2_000, 0, 2),
        ],
    );

    assert_eq!(store.last_start_time_like_cpp(1, 1_350), 0);
    assert_eq!(store.last_start_time_like_cpp(3, 1_350), 0);
    assert_eq!(store.last_start_time_like_cpp(2, 1_350), 0);
}
#[test]
fn game_event_check_normal_window_and_strict_start_end_like_cpp() {
    let store = game_event_store([event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2)]);

    assert_eq!(
        store.check_one_game_event_like_cpp(1, 100),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 101),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 221),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 1_000),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
}
#[test]
fn game_event_check_unknown_raw_state_uses_normal_default_like_cpp() {
    let store = game_event_store([event_with_raw_state(1, 99, 100, 1_000, 10, 2)]);

    assert_eq!(
        store.check_one_game_event_like_cpp(1, 101),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 221),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
}
#[test]
fn game_event_check_world_state_branches_like_cpp() {
    let store = game_event_store([
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 0),
        event(2, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
        event(3, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 0),
        event(4, GameEventStateLikeCpp::Internal, 0, 0, 0, 0),
    ]);

    assert_eq!(
        store.check_one_game_event_like_cpp(1, 500),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(2, 500),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(3, 500),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(4, 500),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
}
#[test]
fn game_event_check_inactive_prerequisites_like_cpp() {
    let base_events = [
        event(1, GameEventStateLikeCpp::WorldInactive, 0, 0, 0, 0),
        event_with_next_start(
            event(2, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
            400,
        ),
        event_with_next_start(
            event(3, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 0),
            500,
        ),
        event_with_next_start(
            event(4, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
            700,
        ),
        event(5, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2),
    ];
    let store = game_event_store(base_events.clone());

    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );

    let store = game_event_store([
        event_with_prerequisites(base_events[0].clone(), [2, 3]),
        base_events[1].clone(),
        base_events[2].clone(),
        base_events[3].clone(),
        base_events[4].clone(),
    ]);
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );

    let store = game_event_store([
        event_with_prerequisites(base_events[0].clone(), [5]),
        base_events[1].clone(),
        base_events[2].clone(),
        base_events[3].clone(),
        base_events[4].clone(),
    ]);
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );

    let store = game_event_store([
        event_with_prerequisites(base_events[0].clone(), [4]),
        base_events[1].clone(),
        base_events[2].clone(),
        base_events[3].clone(),
        base_events[4].clone(),
    ]);
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );

    let store = game_event_store([
        event_with_prerequisites(base_events[0].clone(), [9]),
        base_events[1].clone(),
        base_events[2].clone(),
        base_events[3].clone(),
        base_events[4].clone(),
    ]);
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::MissingPrerequisite { event_id: 9 }
    );
}
