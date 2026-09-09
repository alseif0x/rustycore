//! Persistence scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_world_state_load_inserts_realm_default_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(100, 7, "", "")],
        [],
        |_| false,
        |_| None,
    );

    assert_eq!(report.template_rows, 1);
    assert_eq!(report.templates_loaded, 1);
    assert_eq!(mgr.realm_value_like_cpp(100), 7);
    assert_eq!(
        mgr.template_like_cpp(100)
            .map(|template| template.area_ids.len()),
        Some(0)
    );
}
#[test]
fn game_event_world_state_saved_value_overlays_realm_default_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(101, 7, "", "")],
        [(101, 9)],
        |_| false,
        |_| None,
    );

    assert_eq!(report.saved_rows, 1);
    assert_eq!(report.saved_applied, 1);
    assert_eq!(mgr.realm_value_like_cpp(101), 9);
}
#[test]
fn game_event_world_state_map_defaults_and_saved_overlay_all_maps_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(102, 3, "1,2", "")],
        [(102, 11)],
        |map_id| matches!(map_id, 1 | 2),
        |_| None,
    );

    assert_eq!(report.templates_loaded, 1);
    assert_eq!(report.saved_applied, 1);
    assert_eq!(mgr.map_value_like_cpp(1, 102), 11);
    assert_eq!(mgr.map_value_like_cpp(2, 102), 11);
    assert_eq!(mgr.realm_value_like_cpp(102), 0);
}
#[test]
fn game_event_world_state_realm_row_with_area_ids_still_loads_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(107, 5, "", "20")],
        [],
        |_| false,
        |_| Some(1),
    );

    assert_eq!(report.realm_area_requirements_ignored, 1);
    assert_eq!(report.templates_loaded, 1);
    assert_eq!(mgr.realm_value_like_cpp(107), 5);
}
#[test]
fn game_event_world_state_unknown_saved_value_is_skipped_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(108, 5, "", "")],
        [(999, 12)],
        |_| false,
        |_| None,
    );

    assert_eq!(report.saved_rows, 1);
    assert_eq!(report.saved_skipped_unknown, 1);
    assert_eq!(report.saved_applied, 0);
    assert_eq!(mgr.realm_value_like_cpp(108), 5);
}
#[test]
fn game_event_condition_metadata_load_replaces_duplicate_and_skips_out_of_range_like_cpp() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventConditionLoadReportLikeCpp::default();

    apply_game_event_condition_row_like_cpp(
        GameEventConditionRowLikeCpp {
            event_id: 1,
            condition_id: 10,
            req_num: 3.5,
            max_world_state: 100,
            done_world_state: 101,
        },
        &mut events,
        &mut report,
    );
    apply_game_event_condition_row_like_cpp(
        GameEventConditionRowLikeCpp {
            event_id: 1,
            condition_id: 10,
            req_num: 7.0,
            max_world_state: 200,
            done_world_state: 201,
        },
        &mut events,
        &mut report,
    );
    apply_game_event_condition_row_like_cpp(
        GameEventConditionRowLikeCpp {
            event_id: 3,
            condition_id: 11,
            req_num: 1.0,
            max_world_state: 0,
            done_world_state: 0,
        },
        &mut events,
        &mut report,
    );

    let loaded = events
        .event_like_cpp(1)
        .unwrap()
        .conditions
        .get(&10)
        .unwrap();
    assert_eq!(loaded.req_num, 7.0);
    assert_eq!(loaded.done, 0.0);
    assert_eq!(loaded.max_world_state, 200);
    assert_eq!(loaded.done_world_state, 201);
    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 2);
    assert_eq!(report.skipped_out_of_range, 1);
}
#[test]
fn game_event_condition_save_applies_only_existing_event_condition_like_cpp() {
    let mut events = game_event_store([event_with_condition(
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
        10,
        condition(7.0, 0.0),
    )]);
    let mut report = GameEventConditionSaveLoadReportLikeCpp::default();

    apply_game_event_condition_save_row_like_cpp(
        GameEventConditionSaveRowLikeCpp {
            event_id: 1,
            condition_id: 10,
            done: 4.0,
        },
        &mut events,
        &mut report,
    );
    apply_game_event_condition_save_row_like_cpp(
        GameEventConditionSaveRowLikeCpp {
            event_id: 1,
            condition_id: 99,
            done: 6.0,
        },
        &mut events,
        &mut report,
    );
    apply_game_event_condition_save_row_like_cpp(
        GameEventConditionSaveRowLikeCpp {
            event_id: 99,
            condition_id: 10,
            done: 6.0,
        },
        &mut events,
        &mut report,
    );

    assert_eq!(
        events
            .event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        4.0
    );
    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_missing_condition, 1);
    assert_eq!(report.skipped_out_of_range_event, 1);
}
#[tokio::test]
async fn game_event_condition_save_loader_uses_typed_rows_and_preserves_validation_like_cpp() {
    let mut events = game_event_store([event_with_condition(
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
        10,
        condition(7.0, 0.0),
    )]);
    let persistence = FakeGameEventConditionSavePersistenceLikeCpp {
        outcome: wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Loaded(vec![
            wow_persistence::GameEventConditionSavePersistenceRowLikeCpp {
                event_id: 1,
                condition_id: 10,
                done: 4.0,
            },
            wow_persistence::GameEventConditionSavePersistenceRowLikeCpp {
                event_id: 1,
                condition_id: 99,
                done: 6.0,
            },
        ]),
    };
    let mut report = CanonicalSpawnStoreLoadReport::default();

    load_game_event_condition_saves_like_cpp(&persistence, &mut events, &mut report)
        .await
        .unwrap();

    assert_eq!(
        events
            .event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        4.0
    );
    assert_eq!(report.game_event_condition_saves.rows, 2);
    assert_eq!(report.game_event_condition_saves.loaded, 1);
    assert_eq!(
        report.game_event_condition_saves.skipped_missing_condition,
        1
    );
}
#[tokio::test]
async fn game_event_condition_save_loader_propagates_typed_failure_without_mutation_like_cpp() {
    let mut events = game_event_store([event_with_condition(
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
        10,
        condition(7.0, 2.0),
    )]);
    let persistence = FakeGameEventConditionSavePersistenceLikeCpp {
        outcome: wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Failed {
            reason: "fixture load failure".to_string(),
        },
    };
    let mut report = CanonicalSpawnStoreLoadReport::default();

    let error = load_game_event_condition_saves_like_cpp(&persistence, &mut events, &mut report)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("fixture load failure"));
    assert_eq!(
        events
            .event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        2.0
    );
    assert_eq!(report.game_event_condition_saves.rows, 0);
}
#[test]
fn game_event_condition_progress_saturates_saves_then_completes_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    let outcome =
        metadata.represented_update_game_event_condition_progress_like_cpp(1, 10, 5.0, 100);

    assert_eq!(
        outcome,
        GameEventConditionProgressOutcomeLikeCpp::Progressed(
            GameEventConditionProgressSummaryLikeCpp {
                event_id: 1,
                condition_id: 10,
                done_before: 1.0,
                done_after: 3.0,
                req_num: 3.0,
                persistence_event_id: 1,
                completed_event: true,
                check_outcome: GameEventConditionCheckOutcomeLikeCpp::Completed(
                    GameEventConditionCheckSummaryLikeCpp {
                        event_id: 1,
                        condition_count: 1,
                        state_before_raw: GameEventStateLikeCpp::WorldConditions as u8,
                        state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
                        next_start_before: 0,
                        next_start_after: 400,
                    }
                ),
                save_world_event_state_requested: true,
                force_game_event_update_requested: true,
            }
        )
    );
}
#[test]
fn game_event_update_world_conditions_true_saves_starts_completed_and_forces_delay_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store_with_max(
            1,
            [event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 7)],
        ));

    let outcome = metadata.update_game_events_like_cpp(500, true, |event_id| event_id == 1);

    assert_eq!(
        outcome.world_conditions_save_requested,
        vec![GameEventWorldStateSaveEvidenceLikeCpp {
            event_id: 1,
            state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            next_start_after: 920,
        }]
    );
    assert_eq!(outcome.queued_activation_event_ids, vec![1]);
    assert!(matches!(
        outcome.start_outcomes.as_slice(),
        [GameEventStartOutcomeLikeCpp::Started(
            GameEventStartSummaryLikeCpp {
                event_id: 1,
                completed: true,
                save_world_event_state_requested: true,
                ..
            }
        )]
    ));
    assert_eq!(outcome.next_event_delay_secs_before_padding, 0);
    assert_eq!(outcome.next_update_delay_millis, 1_000);
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldNextPhase as u8);
    assert_eq!(event.next_start, 920);
    assert!(
        metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}
#[test]
fn game_event_prerequisite_loader_accepts_world_events_dedupes_and_sorts_like_cpp() {
    let mut store = game_event_store([
        event(1, GameEventStateLikeCpp::WorldInactive, 0, 0, 0, 0),
        event(2, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
        event(3, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 0),
        event(4, GameEventStateLikeCpp::Normal, 0, 0, 0, 0),
        event(5, GameEventStateLikeCpp::Internal, 0, 0, 0, 0),
    ]);
    let mut report = GameEventPrerequisiteLoadReportLikeCpp::default();

    for row in [
        GameEventPrerequisiteRowLikeCpp {
            event_id: 1,
            prerequisite_event: 3,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 1,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 1,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 4,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 5,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 99,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 1,
            prerequisite_event: 99,
        },
    ] {
        apply_game_event_prerequisite_row_like_cpp(row, &mut store, &mut report);
    }

    assert_eq!(report.rows, 7);
    assert_eq!(report.loaded, 2);
    assert_eq!(report.duplicate_ignored, 1);
    assert_eq!(report.skipped_non_world_event, 2);
    assert_eq!(report.skipped_out_of_range_event, 1);
    assert_eq!(report.skipped_out_of_range_prerequisite, 1);
    assert_eq!(
        store
            .prerequisite_events_like_cpp(1)
            .expect("test event exists")
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
}
#[test]
fn pool_mgr_loader_skip_order_missing_spawn_before_template_and_chance_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut spawn_report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let spawn = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut spawn_report,
    )
    .unwrap();
    store.add_object_spawn(&spawn, is_personal_phase_like_cpp_represented);
    let mut mgr = PoolMgrLikeCpp::new();
    let mut report = PoolMgrLoadReportLikeCpp::default();

    apply_pool_spawn_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 999,
            pool_spawn_id: 88,
            chance: 200.0,
        },
        &store,
        PoolMemberKindLikeCpp::Creature,
        &mut mgr,
        &mut report,
    );
    apply_pool_spawn_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 100,
            pool_spawn_id: 88,
            chance: 200.0,
        },
        &store,
        PoolMemberKindLikeCpp::Creature,
        &mut mgr,
        &mut report,
    );
    mgr.insert_template_like_cpp(88, PoolTemplateDataLikeCpp::new(1, -1));
    apply_pool_spawn_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 100,
            pool_spawn_id: 88,
            chance: 200.0,
        },
        &store,
        PoolMemberKindLikeCpp::Creature,
        &mut mgr,
        &mut report,
    );

    assert_eq!(report.creature_members.rows, 3);
    assert_eq!(report.creature_members.skipped_missing_spawn, 1);
    assert_eq!(report.creature_members.skipped_missing_template, 1);
    assert_eq!(report.creature_members.skipped_invalid_chance, 1);
    assert_eq!(report.creature_members.loaded, 0);
}
#[test]
fn pool_mgr_loader_map_propagation_mismatch_and_cycle_removal_like_cpp() {
    let mut propagated = PoolMgrLikeCpp::new();
    let mut report = PoolMgrLoadReportLikeCpp::default();
    propagated.insert_template_like_cpp(1, PoolTemplateDataLikeCpp::new(1, 571));
    propagated.insert_template_like_cpp(2, PoolTemplateDataLikeCpp::new(1, -1));
    apply_pool_pool_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 1,
            pool_spawn_id: 2,
            chance: 0.0,
        },
        &mut propagated,
        &mut report,
    );
    apply_pool_map_propagation_like_cpp(&mut propagated, &mut report);
    assert_eq!(propagated.templates.get(&2).unwrap().map_id, 571);
    assert_eq!(report.relation_removals, 0);

    let mut mismatch = PoolMgrLikeCpp::new();
    let mut mismatch_report = PoolMgrLoadReportLikeCpp::default();
    mismatch.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 1));
    mismatch.insert_template_like_cpp(20, PoolTemplateDataLikeCpp::new(1, 2));
    apply_pool_pool_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 10,
            pool_spawn_id: 20,
            chance: 0.0,
        },
        &mut mismatch,
        &mut mismatch_report,
    );
    apply_pool_map_propagation_like_cpp(&mut mismatch, &mut mismatch_report);
    assert!(!mismatch.child_pool_to_parent.contains_key(&10));
    assert_eq!(mismatch_report.map_mismatches, 1);
    assert_eq!(mismatch_report.relation_removals, 1);

    let mut cyclic = PoolMgrLikeCpp::new();
    let mut cycle_report = PoolMgrLoadReportLikeCpp::default();
    cyclic.insert_template_like_cpp(30, PoolTemplateDataLikeCpp::new(1, -1));
    cyclic.insert_template_like_cpp(31, PoolTemplateDataLikeCpp::new(1, -1));
    apply_pool_pool_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 31,
            pool_spawn_id: 30,
            chance: 0.0,
        },
        &mut cyclic,
        &mut cycle_report,
    );
    apply_pool_pool_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 30,
            pool_spawn_id: 31,
            chance: 0.0,
        },
        &mut cyclic,
        &mut cycle_report,
    );
    apply_pool_map_propagation_like_cpp(&mut cyclic, &mut cycle_report);
    assert_eq!(cycle_report.circular_relations, 1);
    assert_eq!(cycle_report.relation_removals, 1);
    assert_eq!(cyclic.child_pool_to_parent.len(), 1);
}
#[test]
fn pool_mgr_loader_autospawn_skips_empty_broken_and_child_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    let mut report = PoolMgrLoadReportLikeCpp::default();
    mgr.insert_template_like_cpp(1, PoolTemplateDataLikeCpp::new(1, 0));
    mgr.insert_template_like_cpp(2, PoolTemplateDataLikeCpp::new(1, 0));
    mgr.insert_template_like_cpp(3, PoolTemplateDataLikeCpp::new(1, 0));
    mgr.insert_template_like_cpp(4, PoolTemplateDataLikeCpp::new(1, 0));
    let mut valid = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 1);
    valid.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 0.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 1, valid)
        .unwrap();
    let mut broken = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 3);
    broken.add_entry_like_cpp(PoolObjectLikeCpp::new(301, 50.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 3, broken)
        .unwrap();
    let mut child = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 4);
    child.add_entry_like_cpp(PoolObjectLikeCpp::new(401, 0.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 4, child)
        .unwrap();

    apply_pool_autospawn_candidate_row_like_cpp(
        PoolAutospawnCandidateRowLikeCpp {
            pool_entry: 1,
            child_pool_id: 0,
            mother_pool_id: 0,
        },
        &mut mgr,
        &mut report,
    );
    apply_pool_autospawn_candidate_row_like_cpp(
        PoolAutospawnCandidateRowLikeCpp {
            pool_entry: 2,
            child_pool_id: 0,
            mother_pool_id: 0,
        },
        &mut mgr,
        &mut report,
    );
    apply_pool_autospawn_candidate_row_like_cpp(
        PoolAutospawnCandidateRowLikeCpp {
            pool_entry: 3,
            child_pool_id: 0,
            mother_pool_id: 0,
        },
        &mut mgr,
        &mut report,
    );
    apply_pool_autospawn_candidate_row_like_cpp(
        PoolAutospawnCandidateRowLikeCpp {
            pool_entry: 4,
            child_pool_id: 4,
            mother_pool_id: 99,
        },
        &mut mgr,
        &mut report,
    );

    assert_eq!(report.autospawn_rows, 4);
    assert_eq!(report.autospawn_loaded, 1);
    assert_eq!(report.autospawn_skipped_empty, 1);
    assert_eq!(report.autospawn_skipped_broken, 1);
    assert_eq!(report.autospawn_skipped_child, 1);
    assert_eq!(mgr.auto_spawn_pools_for_map_like_cpp(0), &[1]);
}
#[test]
fn game_event_data_reserved_zero_is_reported_and_not_loaded() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(0, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );

    let slot_zero = events.event_like_cpp(0).unwrap();
    assert_eq!(slot_zero.start, 1);
    assert_eq!(slot_zero.description, "");
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_reserved_zero, 1);
}
#[test]
fn game_event_spawn_guids_count_pooled_but_still_load_like_cpp() {
    let store = game_event_guid_test_store();
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut creature_report = GameEventObjectGuidLoadReportLikeCpp::default();
    let mut gameobject_report = GameEventObjectGuidLoadReportLikeCpp::default();

    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 101,
            event_id: 1,
        },
        SpawnObjectType::Creature,
        &store,
        &mut guids,
        &mut creature_report,
    );
    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 201,
            event_id: -1,
        },
        SpawnObjectType::GameObject,
        &store,
        &mut guids,
        &mut gameobject_report,
    );

    assert_eq!(guids.creature_guids_like_cpp(1), Some([101].as_slice()));
    assert_eq!(guids.gameobject_guids_like_cpp(-1), Some([201].as_slice()));
    assert_eq!(creature_report.loaded, 1);
    assert_eq!(creature_report.pooled_still_loaded, 1);
    assert_eq!(gameobject_report.loaded, 1);
    assert_eq!(gameobject_report.pooled_still_loaded, 1);
}
#[test]
fn game_event_npc_flag_loader_preserves_order_skips_range_and_u64_like_cpp() {
    let mut npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventNpcFlagLoadReportLikeCpp::default();

    for row in [
        GameEventNpcFlagRowLikeCpp {
            spawn_id: 100,
            event_id: 1,
            npcflag: 0x1_0000_0002,
        },
        GameEventNpcFlagRowLikeCpp {
            spawn_id: 101,
            event_id: 1,
            npcflag: 0x4,
        },
        GameEventNpcFlagRowLikeCpp {
            spawn_id: 200,
            event_id: 3,
            npcflag: 0x8,
        },
        GameEventNpcFlagRowLikeCpp {
            spawn_id: 102,
            event_id: 2,
            npcflag: 0x10,
        },
    ] {
        apply_game_event_npc_flag_row_like_cpp(row, &mut npc_flags, &mut report);
    }
    report.events_touched = npc_flags
        .records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();

    let event_one = npc_flags
        .records_like_cpp(1)
        .expect("event 1 bucket exists");
    assert_eq!(event_one.len(), 2);
    assert_eq!(event_one[0].spawn_id, 100);
    assert_eq!(event_one[0].npcflag, 0x1_0000_0002);
    assert_eq!(event_one[1].spawn_id, 101);
    assert_eq!(event_one[1].npcflag, 0x4);
    assert_eq!(npc_flags.records_like_cpp(2).unwrap()[0].spawn_id, 102);
    assert_eq!(npc_flags.records_like_cpp(3), None);
    assert_eq!(report.rows, 4);
    assert_eq!(report.loaded, 3);
    assert_eq!(report.skipped_out_of_range, 1);
    assert_eq!(report.events_touched, 2);
}
#[test]
fn linked_respawn_loader_validation_invalid_type_and_missing_master_like_cpp() {
    let maps = instanceable_map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut kind_report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let creature = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    store.add_object_spawn(&creature, is_personal_phase_like_cpp_represented);
    let mut linked_store = LinkedRespawnStoreLikeCpp::new();
    let mut report = LinkedRespawnLoadReportLikeCpp::default();

    apply_linked_respawn_row_like_cpp(
        LinkedRespawnRowLikeCpp {
            guid: 100,
            linked_guid: 200,
            link_type: 99,
        },
        &store,
        &maps,
        &mut linked_store,
        &mut report,
    );
    apply_linked_respawn_row_like_cpp(
        LinkedRespawnRowLikeCpp {
            guid: 100,
            linked_guid: 200,
            link_type: LinkedRespawnTypeLikeCpp::CreatureToCreature as u8,
        },
        &store,
        &maps,
        &mut linked_store,
        &mut report,
    );

    assert_eq!(report.rows, 2);
    assert_eq!(report.invalid_type, 1);
    assert_eq!(report.missing_master, 1);
    assert!(linked_store.is_empty());
}
#[test]
fn linked_respawn_loader_validation_difficulty_mismatch_like_cpp() {
    let maps = instanceable_map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0), (1, 1)]);
    let mut kind_report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let slave = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    let master = creature_row_to_spawn_data_like_cpp(
        &creature_row(200, 0, "1"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    store.add_object_spawn(&slave, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&master, is_personal_phase_like_cpp_represented);
    let mut linked_store = LinkedRespawnStoreLikeCpp::new();
    let mut report = LinkedRespawnLoadReportLikeCpp::default();

    apply_linked_respawn_row_like_cpp(
        LinkedRespawnRowLikeCpp {
            guid: 100,
            linked_guid: 200,
            link_type: LinkedRespawnTypeLikeCpp::CreatureToCreature as u8,
        },
        &store,
        &maps,
        &mut linked_store,
        &mut report,
    );

    assert_eq!(report.difficulty_mismatch, 1);
    assert!(linked_store.is_empty());
}
#[test]
fn area_trigger_spawn_loads_cpp_validated_metadata_and_script_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let area_trigger_templates = valid_area_trigger_template_store();
    let mut row = area_trigger_row(302, "0");
    row.script_name = "at_spawn_script".to_string();
    row.spell_for_visuals = Some(1234);
    let mut report = SpawnKindLoadReport::default();
    let mut runtime_rows = BTreeMap::new();

    let spawn = area_trigger_row_to_spawn_data_like_cpp(
        &row,
        &maps,
        &difficulties,
        &area_trigger_templates,
        &mut |spell_id| spell_id == 1234,
        &mut |name| {
            assert_eq!(name, "at_spawn_script");
            wow_data::ScriptIdLikeCpp(77)
        },
        &mut runtime_rows,
        &mut report,
    )
    .expect("valid static area trigger spawn should load");

    assert_eq!(spawn.object_type, SpawnObjectType::AreaTrigger);
    assert_eq!(spawn.id, 789);
    assert_eq!(spawn.script_id, 77);
    assert_eq!(report.validation_skipped, 0);
    assert_eq!(report.script_id_unresolved, 0);
    assert!(report.corrected_invalid_spell_for_visuals.is_empty());
    assert_eq!(
        runtime_rows.get(&302),
        Some(&AreaTriggerSpawnRuntimeRowLikeCpp {
            spawn_id: 302,
            create_properties_id: wow_data::AreaTriggerIdLikeCpp {
                id: 789,
                is_custom: false,
            },
            spell_for_visuals: Some(1234),
        })
    );
}
