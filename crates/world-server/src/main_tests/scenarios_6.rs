//! Scenarios for [`super`], part 6.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn game_event_db_bridge_classifies_typed_port_failure_and_success_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            state_raw: 2,
            next_start: 0,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.world_conditions_save_requested =
        vec![spawn_store_loader::GameEventWorldStateSaveEvidenceLikeCpp {
            event_id: 1,
            state_after_raw: 2,
            next_start_after: 0,
        }];
    let port = FakeGameEventPersistencePortLikeCpp::default();
    port.fail_mutations
        .store(true, std::sync::atomic::Ordering::Release);
    let mut failed =
        materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);
    execute_game_event_world_event_state_db_bridge_like_cpp(&port, &mut failed).await;
    assert_eq!(failed.saves_executed, 0);
    assert_eq!(failed.saves_failed, 1);
    assert!(failed.operations.is_empty());

    port.fail_mutations
        .store(false, std::sync::atomic::Ordering::Release);
    let mut applied =
        materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);
    execute_game_event_world_event_state_db_bridge_like_cpp(&port, &mut applied).await;
    assert_eq!(applied.saves_executed, 1);
    assert_eq!(applied.saves_failed, 0);
    assert_eq!(port.mutations.lock().unwrap().len(), 2);
}
#[test]
fn game_event_db_bridge_materializes_world_nextphase_and_conditions_in_cpp_order() {
    let metadata = game_event_world_state_metadata_like_cpp(
        3,
        &[
            spawn_store_loader::GameEventDataLikeCpp {
                event_id: 1,
                state_raw: 3,
                next_start: 10,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            },
            spawn_store_loader::GameEventDataLikeCpp {
                event_id: 2,
                state_raw: 4,
                next_start: 20,
                ..spawn_store_loader::GameEventDataLikeCpp::default()
            },
        ],
    );
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.world_nextphase_finished =
        vec![spawn_store_loader::GameEventWorldNextPhaseFinishedLikeCpp {
            event_id: 2,
            was_active_before_queue: true,
            state_before_raw: 1,
            state_after_raw: 4,
            next_start_before: 0,
            next_start_after: 20,
            save_state_requested: true,
        }];
    outcome.world_conditions_save_requested =
        vec![spawn_store_loader::GameEventWorldStateSaveEvidenceLikeCpp {
            event_id: 1,
            state_after_raw: 3,
            next_start_after: 10,
        }];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.saves_queued, 2);
    assert_eq!(summary.operations.len(), 2);
    assert_game_event_save_operation_like_cpp(&summary.operations[0], 2, 4, 20);
    assert_game_event_save_operation_like_cpp(&summary.operations[1], 1, 3, 10);
}
#[test]
fn game_event_db_bridge_materializes_stop_delete_condition_saves_before_event_save() {
    let metadata = game_event_world_state_metadata_like_cpp(1, &[]);
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
        spawn_store_loader::GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: 1,
            state_after_raw: 0,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: true,
            condition_reset_requested: true,
            delete_world_event_state_requested: true,
            delete_condition_saves_requested: true,
        },
    )];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.deletes_queued, 1);
    assert_eq!(summary.condition_delete_rows_queued, 1);
    assert_eq!(summary.operations.len(), 1);
    let operation = &summary.operations[0];
    assert_eq!(
        operation.kind,
        GameEventWorldEventStateDbOperationKindLikeCpp::Delete
    );
    assert!(operation.delete_condition_saves);
    assert!(operation.delete_world_event_state);
    assert_eq!(
        operation.mutation,
        wow_persistence::GameEventPersistenceMutationLikeCpp::DeleteWorldEventState {
            event_id: 1,
            delete_condition_saves: true,
            delete_world_event_state: true,
        }
    );
}
#[test]
fn game_event_db_bridge_finished_no_overwrite_stop_without_delete_flags_is_noop() {
    let metadata = game_event_world_state_metadata_like_cpp(1, &[]);
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
        spawn_store_loader::GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: 2,
            state_after_raw: 2,
            active_removed: false,
            active_was_present: true,
            unapply_event_requested: false,
            serverwide: true,
            condition_reset_requested: false,
            delete_world_event_state_requested: false,
            delete_condition_saves_requested: false,
        },
    )];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.deletes_queued, 0);
    assert_eq!(summary.condition_delete_rows_queued, 0);
    assert!(summary.operations.is_empty());
}
#[test]
fn game_event_db_bridge_out_of_range_event_id_skips_without_panic() {
    let metadata = game_event_world_state_metadata_like_cpp(
        300,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 300,
            state_raw: 1,
            next_start: 0,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut outcome = empty_game_event_update_outcome_for_db_bridge_like_cpp();
    outcome.world_conditions_save_requested =
        vec![spawn_store_loader::GameEventWorldStateSaveEvidenceLikeCpp {
            event_id: 300,
            state_after_raw: 1,
            next_start_after: 0,
        }];
    outcome.stop_outcomes = vec![spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(
        spawn_store_loader::GameEventStopSummaryLikeCpp {
            event_id: 300,
            state_before_raw: 1,
            state_after_raw: 0,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: true,
            condition_reset_requested: true,
            delete_world_event_state_requested: true,
            delete_condition_saves_requested: true,
        },
    )];

    let summary = materialize_game_event_world_event_state_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.saves_skipped_event_id_out_of_range, 1);
    assert_eq!(summary.deletes_skipped_event_id_out_of_range, 1);
    assert_eq!(summary.saves_queued, 0);
    assert_eq!(summary.deletes_queued, 0);
    assert!(summary.operations.is_empty());
}
#[test]
fn game_event_quest_complete_db_bridge_materializes_condition_save_then_world_event_save() {
    let metadata = game_event_world_state_metadata_like_cpp(
        7,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 7,
            state_raw: 3,
            next_start: 1_234,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = game_event_quest_complete_progressed_outcome_like_cpp(true, true);

    let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.condition_save_updates_queued, 1);
    assert_eq!(summary.condition_save_updates_skipped_non_progress, 0);
    assert_eq!(summary.world_event_state_save_requested, 1);
    assert_eq!(summary.force_game_event_update_requested, 1);
    assert!(summary.save_world_event_state_requested);
    assert!(summary.force_game_event_update_requested_flag);
    assert_eq!(summary.operations.len(), 1);

    let operation = &summary.operations[0];
    assert_eq!(operation.event_id, 7);
    assert_eq!(operation.condition_id, 44);
    assert_eq!(
        operation.mutation,
        wow_persistence::GameEventPersistenceMutationLikeCpp::ReplaceConditionSave {
            event_id: 7,
            condition_id: 44,
            done: 5.25,
        }
    );

    assert_eq!(summary.world_event_state_summary.saves_queued, 1);
    assert_eq!(
        summary
            .world_event_state_summary
            .saves_skipped_missing_event,
        0
    );
    assert_eq!(
        summary
            .world_event_state_summary
            .saves_skipped_event_id_out_of_range,
        0
    );
    assert_eq!(summary.world_event_state_summary.operations.len(), 1);
    assert_game_event_save_operation_like_cpp(
        &summary.world_event_state_summary.operations[0],
        7,
        3,
        1_234,
    );
}
#[test]
fn game_event_quest_complete_response_dto_includes_condition_and_world_event_flags_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        7,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 7,
            state_raw: 3,
            next_start: 5_000,
            ..Default::default()
        }],
    );
    let outcome = game_event_quest_complete_progressed_outcome_like_cpp(true, true);

    let mut summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);
    summary.condition_save_updates_executed = 1;
    summary.world_event_state_summary.saves_executed = 1;
    let response = game_event_quest_complete_response_from_summary_like_cpp(1234, &summary);

    assert_eq!(response.quest_id, 1234);
    assert_eq!(response.condition_save_updates_queued, 1);
    assert_eq!(response.condition_save_updates_executed, 1);
    assert_eq!(response.condition_save_updates_failed, 0);
    assert_eq!(response.condition_save_updates_skipped_non_progress, 0);
    assert!(response.save_world_event_state_requested);
    assert_eq!(response.world_event_state_save_requested, 1);
    assert_eq!(response.world_event_state_saves_queued, 1);
    assert_eq!(response.world_event_state_saves_executed, 1);
    assert_eq!(response.world_event_state_saves_failed, 0);
    assert!(response.force_game_event_update_requested);
    assert_eq!(response.force_game_event_update_requests, 1);
    assert!(!response.processor_failed);
}
#[test]
fn game_event_quest_complete_response_dto_reports_non_progress_noop_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(7, &[]);
    let outcome = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping {
        quest_id: 9999,
    };

    let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);
    let response = game_event_quest_complete_response_from_summary_like_cpp(9999, &summary);

    assert_eq!(response.quest_id, 9999);
    assert_eq!(response.condition_save_updates_queued, 0);
    assert_eq!(response.condition_save_updates_skipped_non_progress, 1);
    assert!(!response.save_world_event_state_requested);
    assert_eq!(response.world_event_state_saves_queued, 0);
    assert!(!response.force_game_event_update_requested);
    assert!(!response.processor_failed);
}
#[test]
fn game_event_quest_complete_db_bridge_preserves_condition_save_without_world_event_save() {
    let metadata = game_event_world_state_metadata_like_cpp(
        7,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 7,
            state_raw: 2,
            next_start: 0,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let outcome = game_event_quest_complete_progressed_outcome_like_cpp(false, false);

    let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.condition_save_updates_queued, 1);
    assert_eq!(summary.operations.len(), 1);
    assert_eq!(summary.world_event_state_save_requested, 0);
    assert!(!summary.save_world_event_state_requested);
    assert_eq!(summary.world_event_state_summary.saves_queued, 0);
    assert!(summary.world_event_state_summary.operations.is_empty());
}
#[test]
fn game_event_quest_complete_db_bridge_skips_world_event_save_when_metadata_missing() {
    let metadata = game_event_world_state_metadata_like_cpp(6, &[]);
    let outcome = game_event_quest_complete_progressed_outcome_like_cpp(true, true);

    let summary = materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata);

    assert_eq!(summary.condition_save_updates_queued, 1);
    assert_eq!(summary.operations.len(), 1);
    assert_eq!(summary.world_event_state_save_requested, 1);
    assert!(summary.save_world_event_state_requested);
    assert_eq!(summary.world_event_state_summary.saves_queued, 0);
    assert_eq!(
        summary
            .world_event_state_summary
            .saves_skipped_missing_event,
        1
    );
    assert!(summary.world_event_state_summary.operations.is_empty());
}
#[test]
fn game_event_quest_complete_db_bridge_skips_missing_or_non_progress() {
    let metadata = game_event_world_state_metadata_like_cpp(
        7,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 7,
            state_raw: 3,
            next_start: 1_234,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let missing = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping {
        quest_id: 12_345,
    };
    let missing_summary =
        materialize_game_event_quest_complete_db_bridge_like_cpp(&missing, &metadata);
    assert_eq!(missing_summary.condition_save_updates_queued, 0);
    assert_eq!(
        missing_summary.condition_save_updates_skipped_non_progress,
        1
    );
    assert!(missing_summary.operations.is_empty());
    assert_eq!(missing_summary.world_event_state_summary.saves_queued, 0);
    assert!(
        missing_summary
            .world_event_state_summary
            .operations
            .is_empty()
    );

    let inactive = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
        spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::InactiveEvent { event_id: 7 },
    );
    let inactive_summary =
        materialize_game_event_quest_complete_db_bridge_like_cpp(&inactive, &metadata);
    assert_eq!(inactive_summary.condition_save_updates_queued, 0);
    assert_eq!(
        inactive_summary.condition_save_updates_skipped_non_progress,
        1
    );
    assert!(inactive_summary.operations.is_empty());
    assert_eq!(inactive_summary.world_event_state_summary.saves_queued, 0);
    assert!(
        inactive_summary
            .world_event_state_summary
            .operations
            .is_empty()
    );

    let already_complete = spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
        spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::AlreadyComplete {
            event_id: 7,
            condition_id: 44,
            done: 10.0,
            req_num: 10.0,
        },
    );
    let complete_summary =
        materialize_game_event_quest_complete_db_bridge_like_cpp(&already_complete, &metadata);
    assert_eq!(complete_summary.condition_save_updates_queued, 0);
    assert_eq!(
        complete_summary.condition_save_updates_skipped_non_progress,
        1
    );
    assert!(complete_summary.operations.is_empty());
    assert_eq!(complete_summary.world_event_state_summary.saves_queued, 0);
    assert!(
        complete_summary
            .world_event_state_summary
            .operations
            .is_empty()
    );
}
#[test]
fn game_event_world_state_no_holiday_action_is_represented_noop_like_cpp() {
    let mut metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: 0,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut manager = wow_map::MapManager::default();
    let outcome = game_event_world_state_start_outcome_like_cpp(1);

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[1],
        &outcome,
        false,
    );

    assert!(
        summary
            .actions
            .contains(&GameEventLiveUpdateActionLikeCpp::UpdateWorldStates {
                event_id: 1,
                activate: true,
            })
    );
    assert_eq!(summary.update_world_states_actions, 1);
    assert_eq!(summary.update_world_states_no_holiday, 1);
    assert_eq!(summary.update_world_states_missing_event, 0);
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
}
#[test]
fn game_event_world_state_holiday_lookup_remains_unrepresented_like_cpp() {
    let mut metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: 283,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let mut manager = wow_map::MapManager::default();
    let outcome = game_event_world_state_start_outcome_like_cpp(1);

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[1],
        &outcome,
        false,
    );

    assert_eq!(summary.update_world_states_actions, 1);
    assert_eq!(summary.update_world_states_no_holiday, 0);
    assert_eq!(summary.update_world_states_missing_event, 0);
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 1);
}
#[test]
fn game_event_world_state_missing_event_is_counted_without_panic_like_cpp() {
    let mut metadata = game_event_world_state_metadata_like_cpp(0, &[]);
    let mut manager = wow_map::MapManager::default();
    let outcome = game_event_world_state_start_outcome_like_cpp(1);

    let summary = consume_game_event_live_update_side_effects_like_cpp(
        &mut manager,
        None,
        &mut metadata,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
        None,
        None,
        None,
        &[],
        &outcome,
        false,
    );

    assert_eq!(summary.update_world_states_actions, 1);
    assert_eq!(summary.update_world_states_missing_event, 1);
    assert_eq!(summary.update_world_states_no_holiday, 0);
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
}
#[test]
fn game_event_world_state_holiday_set_value_activate_is_represented_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 777,
        flags: 0,
    }]);

    let summary =
        game_event_update_world_states_like_cpp(&metadata, Some(&store), None, None, 1, true);

    assert_eq!(summary.update_world_states_set_value_represented, 1);
    assert_eq!(summary.update_world_states_last_world_state_id, Some(777));
    assert_eq!(summary.update_world_states_last_world_state_value, Some(1));
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
}
#[test]
fn game_event_world_state_holiday_set_value_deactivate_is_represented_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AB_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AB_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 888,
        flags: 0,
    }]);

    let summary =
        game_event_update_world_states_like_cpp(&metadata, Some(&store), None, None, 1, false);

    assert_eq!(summary.update_world_states_set_value_represented, 1);
    assert_eq!(summary.update_world_states_last_world_state_id, Some(888));
    assert_eq!(summary.update_world_states_last_world_state_value, Some(0));
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 0);
}
#[test]
fn game_event_world_state_live_consumer_propagates_holiday_lookup_counters_like_cpp() {
    fn consume_world_state_summary_like_cpp(
        metadata: &mut spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
        battlemaster_list_store: Option<&wow_data::BattlemasterListStore>,
    ) -> GameEventLiveUpdateSideEffectSummaryLikeCpp {
        let mut manager = wow_map::MapManager::default();
        let outcome = game_event_world_state_start_outcome_like_cpp(1);
        consume_game_event_live_update_side_effects_like_cpp(
            &mut manager,
            None,
            metadata,
            &empty_loaded_grid_creature_respawn_caches_like_cpp(),
            battlemaster_list_store,
            None,
            None,
            &[1],
            &outcome,
            false,
        )
    }

    let mut missing_store_metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let missing_store_summary =
        consume_world_state_summary_like_cpp(&mut missing_store_metadata, None);
    assert_eq!(missing_store_summary.update_world_states_actions, 1);
    assert_eq!(missing_store_summary.update_world_states_store_missing, 1);
    assert_eq!(
        missing_store_summary.update_world_states_holiday_lookup_unrepresented,
        1
    );
    assert_eq!(
        missing_store_summary.update_world_states_battlemaster_list_missing,
        0
    );
    assert_eq!(
        missing_store_summary.update_world_states_holiday_world_state_zero,
        0
    );

    let missing_battlemaster_store = wow_data::BattlemasterListStore::from_entries([]);
    let mut missing_battlemaster_metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let missing_battlemaster_summary = consume_world_state_summary_like_cpp(
        &mut missing_battlemaster_metadata,
        Some(&missing_battlemaster_store),
    );
    assert_eq!(
        missing_battlemaster_summary.update_world_states_store_missing,
        0
    );
    assert_eq!(
        missing_battlemaster_summary.update_world_states_battlemaster_list_missing,
        1
    );
    assert_eq!(
        missing_battlemaster_summary.update_world_states_holiday_lookup_unrepresented,
        1
    );
    assert_eq!(
        missing_battlemaster_summary.update_world_states_holiday_world_state_zero,
        0
    );

    let zero_store =
        wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
            id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
            instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
            holiday_world_state: 0,
            flags: 0,
        }]);
    let mut zero_metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let zero_summary = consume_world_state_summary_like_cpp(&mut zero_metadata, Some(&zero_store));
    assert_eq!(zero_summary.update_world_states_store_missing, 0);
    assert_eq!(
        zero_summary.update_world_states_battlemaster_list_missing,
        0
    );
    assert_eq!(zero_summary.update_world_states_holiday_world_state_zero, 1);
    assert_eq!(
        zero_summary.update_world_states_holiday_lookup_unrepresented,
        0
    );
    assert_eq!(zero_summary.update_world_states_set_value_represented, 0);
}
#[test]
fn game_event_world_state_missing_battlemaster_store_is_explicit_skip_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );

    let summary = game_event_update_world_states_like_cpp(&metadata, None, None, None, 1, true);

    assert_eq!(summary.update_world_states_store_missing, 1);
    assert_eq!(summary.update_world_states_holiday_lookup_unrepresented, 1);
    assert_eq!(summary.update_world_states_set_value_represented, 0);
}
#[test]
fn game_event_world_state_missing_or_zero_battlemaster_row_is_explicit_skip_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let missing_store = wow_data::BattlemasterListStore::from_entries([]);
    let missing_summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&missing_store),
        None,
        None,
        1,
        true,
    );
    assert_eq!(
        missing_summary.update_world_states_battlemaster_list_missing,
        1
    );
    assert_eq!(
        missing_summary.update_world_states_holiday_lookup_unrepresented,
        1
    );
    assert_eq!(missing_summary.update_world_states_set_value_represented, 0);

    let zero_store =
        wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
            id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
            instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
            holiday_world_state: 0,
            flags: 0,
        }]);
    let zero_summary =
        game_event_update_world_states_like_cpp(&metadata, Some(&zero_store), None, None, 1, true);
    assert_eq!(zero_summary.update_world_states_holiday_world_state_zero, 1);
    assert_eq!(
        zero_summary.update_world_states_holiday_lookup_unrepresented,
        0
    );
    assert_eq!(zero_summary.update_world_states_set_value_represented, 0);
}
#[test]
fn game_event_world_state_mgr_realm_default_change_global_message_represented_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 777,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                777, 0,
            )],
            [],
        );

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_set_value_attempts, 1);
    assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 1);
    assert_eq!(summary.update_world_states_realm_unchanged_noop, 0);
    assert_eq!(summary.update_world_states_last_world_state_id, Some(777));
    assert_eq!(summary.update_world_states_last_world_state_value, Some(1));
    assert_eq!(world_state_mgr.realm_value_like_cpp(777), 1);
}
#[test]
fn game_event_world_state_mgr_realm_same_value_is_noop_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 778,
        flags: 0,
    }]);
    let mut world_state_mgr =
        spawn_store_loader::WorldStateMgrLikeCpp::from_templates_and_saved_values(
            [spawn_store_loader::WorldStateTemplateLikeCpp::realm_wide(
                778, 1,
            )],
            [],
        );

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_set_value_attempts, 1);
    assert_eq!(summary.update_world_states_realm_unchanged_noop, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 0);
    assert_eq!(world_state_mgr.realm_value_like_cpp(778), 1);
}
#[test]
fn game_event_world_state_mgr_missing_template_inserts_realm_value_like_cpp() {
    let metadata = game_event_world_state_metadata_like_cpp(
        1,
        &[spawn_store_loader::GameEventDataLikeCpp {
            event_id: 1,
            holiday_id: wow_data::HOLIDAY_CALL_TO_ARMS_AV_LIKE_CPP,
            length: 1,
            ..spawn_store_loader::GameEventDataLikeCpp::default()
        }],
    );
    let store = wow_data::BattlemasterListStore::from_entries([wow_data::BattlemasterListEntry {
        id: wow_data::BATTLEGROUND_AV_LIKE_CPP,
        instance_type: wow_data::MAP_BATTLEGROUND_LIKE_CPP,
        holiday_world_state: 779,
        flags: 0,
    }]);
    let mut world_state_mgr = spawn_store_loader::WorldStateMgrLikeCpp::default();

    let summary = game_event_update_world_states_like_cpp(
        &metadata,
        Some(&store),
        Some(&mut world_state_mgr),
        None,
        1,
        true,
    );

    assert_eq!(summary.update_world_states_set_value_attempts, 1);
    assert_eq!(summary.update_world_states_realm_changed_or_inserted, 1);
    assert_eq!(summary.update_world_states_global_message_represented, 1);
    assert_eq!(world_state_mgr.realm_value_like_cpp(779), 1);
}
