use crate::spawn_store_loader;
use wow_persistence::{
    GameEventPersistenceMutationLikeCpp, GameEventPersistenceMutationOutcomeLikeCpp,
    GameEventPersistencePortLikeCpp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameEventWorldEventStateDbOperationKindLikeCpp {
    Save,
    Delete,
}

#[derive(Debug, Clone)]
pub(crate) struct GameEventWorldEventStateDbOperationLikeCpp {
    pub(crate) event_id: u8,
    pub(crate) kind: GameEventWorldEventStateDbOperationKindLikeCpp,
    pub(crate) delete_condition_saves: bool,
    pub(crate) delete_world_event_state: bool,
    pub(crate) mutation: GameEventPersistenceMutationLikeCpp,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct GameEventWorldEventStateDbBridgeSummaryLikeCpp {
    pub(crate) saves_queued: usize,
    pub(crate) saves_executed: usize,
    pub(crate) saves_failed: usize,
    pub(crate) saves_skipped_event_id_out_of_range: usize,
    pub(crate) saves_skipped_missing_event: usize,
    pub(crate) deletes_queued: usize,
    pub(crate) deletes_executed: usize,
    pub(crate) deletes_failed: usize,
    pub(crate) deletes_skipped_event_id_out_of_range: usize,
    pub(crate) condition_delete_rows_queued: usize,
    pub(crate) condition_delete_rows_executed: usize,
    pub(crate) condition_delete_rows_failed: usize,
    pub(crate) operations: Vec<GameEventWorldEventStateDbOperationLikeCpp>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct GameEventQuestCompleteConditionSaveDbOperationLikeCpp {
    pub(crate) event_id: u8,
    pub(crate) condition_id: u32,
    pub(crate) mutation: GameEventPersistenceMutationLikeCpp,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub(crate) struct GameEventQuestCompleteDbBridgeSummaryLikeCpp {
    pub(crate) condition_save_updates_queued: usize,
    pub(crate) condition_save_updates_executed: usize,
    pub(crate) condition_save_updates_failed: usize,
    pub(crate) condition_save_updates_skipped_non_progress: usize,
    pub(crate) world_event_state_save_requested: usize,
    pub(crate) force_game_event_update_requested: usize,
    pub(crate) save_world_event_state_requested: bool,
    pub(crate) force_game_event_update_requested_flag: bool,
    pub(crate) world_event_state_summary: GameEventWorldEventStateDbBridgeSummaryLikeCpp,
    pub(crate) operations: Vec<GameEventQuestCompleteConditionSaveDbOperationLikeCpp>,
}

#[allow(dead_code)]
pub(crate) fn materialize_game_event_quest_complete_db_bridge_like_cpp(
    outcome: &spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp,
    metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
) -> GameEventQuestCompleteDbBridgeSummaryLikeCpp {
    let mut summary = GameEventQuestCompleteDbBridgeSummaryLikeCpp::default();
    let spawn_store_loader::GameEventQuestCompleteOutcomeLikeCpp::Progress(
        spawn_store_loader::GameEventConditionProgressOutcomeLikeCpp::Progressed(progress),
    ) = outcome
    else {
        summary.condition_save_updates_skipped_non_progress += 1;
        return summary;
    };

    if progress.save_world_event_state_requested {
        summary.world_event_state_save_requested += 1;
        summary.save_world_event_state_requested = true;
    }
    if progress.force_game_event_update_requested {
        summary.force_game_event_update_requested += 1;
        summary.force_game_event_update_requested_flag = true;
    }

    summary.condition_save_updates_queued += 1;
    summary
        .operations
        .push(GameEventQuestCompleteConditionSaveDbOperationLikeCpp {
            event_id: progress.persistence_event_id,
            condition_id: progress.condition_id,
            mutation: GameEventPersistenceMutationLikeCpp::ReplaceConditionSave {
                event_id: progress.persistence_event_id,
                condition_id: progress.condition_id,
                done: progress.done_after,
            },
        });

    if progress.save_world_event_state_requested {
        game_event_world_event_state_db_save_operation_like_cpp(
            progress.event_id,
            metadata,
            &mut summary.world_event_state_summary,
        );
    }

    summary
}

#[allow(dead_code)]
pub(crate) async fn execute_game_event_quest_complete_condition_save_db_bridge_like_cpp(
    persistence: &dyn GameEventPersistencePortLikeCpp,
    summary: &mut GameEventQuestCompleteDbBridgeSummaryLikeCpp,
) {
    let operation_total = summary.operations.len();
    for (operation_index, operation) in summary.operations.drain(..).enumerate() {
        match persistence
            .execute_mutation_like_cpp(operation.mutation)
            .await
        {
            GameEventPersistenceMutationOutcomeLikeCpp::Applied => {
                summary.condition_save_updates_executed += 1
            }
            GameEventPersistenceMutationOutcomeLikeCpp::Failed { reason } => {
                summary.condition_save_updates_failed += 1;
                tracing::error!(
                    error = %reason,
                    operation_index = operation_index + 1,
                    operation_total,
                    event_id = operation.event_id,
                    condition_id = operation.condition_id,
                    "Failed to execute C++ GameEventMgr quest-complete condition-save DB transaction; continuing live update loop"
                );
            }
        }
    }
}

pub(crate) fn game_event_world_event_state_db_save_operation_like_cpp(
    event_id: u16,
    metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    summary: &mut GameEventWorldEventStateDbBridgeSummaryLikeCpp,
) {
    let Ok(event_id_u8) = u8::try_from(event_id) else {
        summary.saves_skipped_event_id_out_of_range += 1;
        return;
    };
    let Some(event) = metadata.game_event_like_cpp(event_id) else {
        summary.saves_skipped_missing_event += 1;
        return;
    };
    let Ok(next_start) = i64::try_from(event.next_start) else {
        summary.saves_skipped_missing_event += 1;
        return;
    };

    summary.saves_queued += 1;
    summary
        .operations
        .push(GameEventWorldEventStateDbOperationLikeCpp {
            event_id: event_id_u8,
            kind: GameEventWorldEventStateDbOperationKindLikeCpp::Save,
            delete_condition_saves: false,
            delete_world_event_state: false,
            mutation: GameEventPersistenceMutationLikeCpp::SaveWorldEventState {
                event_id: event_id_u8,
                state: event.state_raw,
                next_start,
            },
        });
}

pub(crate) fn game_event_world_event_state_db_delete_operation_like_cpp(
    event_id: u16,
    delete_condition_saves_requested: bool,
    delete_world_event_state_requested: bool,
    summary: &mut GameEventWorldEventStateDbBridgeSummaryLikeCpp,
) {
    if !delete_condition_saves_requested && !delete_world_event_state_requested {
        return;
    }
    let Ok(event_id_u8) = u8::try_from(event_id) else {
        summary.deletes_skipped_event_id_out_of_range += 1;
        return;
    };

    if delete_condition_saves_requested {
        summary.condition_delete_rows_queued += 1;
    }
    if delete_world_event_state_requested {
        summary.deletes_queued += 1;
    }

    summary
        .operations
        .push(GameEventWorldEventStateDbOperationLikeCpp {
            event_id: event_id_u8,
            kind: GameEventWorldEventStateDbOperationKindLikeCpp::Delete,
            delete_condition_saves: delete_condition_saves_requested,
            delete_world_event_state: delete_world_event_state_requested,
            mutation: GameEventPersistenceMutationLikeCpp::DeleteWorldEventState {
                event_id: event_id_u8,
                delete_condition_saves: delete_condition_saves_requested,
                delete_world_event_state: delete_world_event_state_requested,
            },
        });
}

pub(crate) fn materialize_game_event_world_event_state_db_bridge_like_cpp(
    outcome: &spawn_store_loader::GameEventUpdateOutcomeLikeCpp,
    metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
) -> GameEventWorldEventStateDbBridgeSummaryLikeCpp {
    let mut summary = GameEventWorldEventStateDbBridgeSummaryLikeCpp::default();

    for save in &outcome.world_nextphase_finished {
        if save.save_state_requested {
            game_event_world_event_state_db_save_operation_like_cpp(
                save.event_id,
                metadata,
                &mut summary,
            );
        }
    }
    for save in &outcome.world_conditions_save_requested {
        game_event_world_event_state_db_save_operation_like_cpp(
            save.event_id,
            metadata,
            &mut summary,
        );
    }
    for start_outcome in &outcome.start_outcomes {
        if let spawn_store_loader::GameEventStartOutcomeLikeCpp::Started(start) = start_outcome {
            if start.save_world_event_state_requested {
                game_event_world_event_state_db_save_operation_like_cpp(
                    start.event_id,
                    metadata,
                    &mut summary,
                );
            }
        }
    }
    for stop_outcome in &outcome.stop_outcomes {
        if let spawn_store_loader::GameEventStopOutcomeLikeCpp::Stopped(stop) = stop_outcome {
            game_event_world_event_state_db_delete_operation_like_cpp(
                stop.event_id,
                stop.delete_condition_saves_requested,
                stop.delete_world_event_state_requested,
                &mut summary,
            );
        }
    }

    summary
}

pub(crate) async fn execute_game_event_world_event_state_db_bridge_like_cpp(
    persistence: &dyn GameEventPersistencePortLikeCpp,
    summary: &mut GameEventWorldEventStateDbBridgeSummaryLikeCpp,
) {
    let operation_total = summary.operations.len();
    for (operation_index, operation) in summary.operations.drain(..).enumerate() {
        match persistence
            .execute_mutation_like_cpp(operation.mutation)
            .await
        {
            GameEventPersistenceMutationOutcomeLikeCpp::Applied => match operation.kind {
                GameEventWorldEventStateDbOperationKindLikeCpp::Save => summary.saves_executed += 1,
                GameEventWorldEventStateDbOperationKindLikeCpp::Delete => {
                    if operation.delete_world_event_state {
                        summary.deletes_executed += 1;
                    }
                    if operation.delete_condition_saves {
                        summary.condition_delete_rows_executed += 1;
                    }
                }
            },
            GameEventPersistenceMutationOutcomeLikeCpp::Failed { reason } => {
                match operation.kind {
                    GameEventWorldEventStateDbOperationKindLikeCpp::Save => {
                        summary.saves_failed += 1;
                    }
                    GameEventWorldEventStateDbOperationKindLikeCpp::Delete => {
                        if operation.delete_world_event_state {
                            summary.deletes_failed += 1;
                        }
                        if operation.delete_condition_saves {
                            summary.condition_delete_rows_failed += 1;
                        }
                    }
                }
                tracing::error!(
                    error = %reason,
                    operation_index = operation_index + 1,
                    operation_total,
                    event_id = operation.event_id,
                    operation_kind = ?operation.kind,
                    "Failed to execute C++ GameEventMgr world-event state DB transaction; continuing live update loop"
                );
            }
        }
    }
}
