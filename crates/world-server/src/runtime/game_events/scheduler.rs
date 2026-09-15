use super::super::*;
use wow_persistence::GameEventPersistencePortLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CanonicalGameEventSchedulerLikeCpp {
    pub(crate) timer_ms: u32,
    pub(crate) interval_ms: u32,
}

impl CanonicalGameEventSchedulerLikeCpp {
    pub(crate) fn start_system(next_delay_ms: u64) -> Self {
        let interval_ms = clamp_game_event_delay_ms_like_cpp(next_delay_ms).max(1);
        Self {
            timer_ms: interval_ms,
            interval_ms,
        }
    }

    pub(crate) fn update(&mut self, diff_ms: u32) -> bool {
        if self.timer_ms <= diff_ms {
            self.timer_ms = self.interval_ms;
            true
        } else {
            self.timer_ms -= diff_ms;
            false
        }
    }

    pub(crate) fn set_interval_and_reset(&mut self, next_delay_ms: u64) {
        self.interval_ms = clamp_game_event_delay_ms_like_cpp(next_delay_ms).max(1);
        self.timer_ms = self.interval_ms;
    }

    #[cfg(test)]
    pub(crate) const fn timer_ms(&self) -> u32 {
        self.timer_ms
    }

    #[cfg(test)]
    pub(crate) const fn interval_ms(&self) -> u32 {
        self.interval_ms
    }
}

pub(crate) fn clamp_game_event_delay_ms_like_cpp(delay_ms: u64) -> u32 {
    u32::try_from(delay_ms).unwrap_or(u32::MAX)
}

pub(crate) fn current_unix_time_secs_like_cpp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub(crate) fn game_event_quest_complete_response_from_summary_like_cpp(
    quest_id: u32,
    summary: &GameEventQuestCompleteDbBridgeSummaryLikeCpp,
) -> GameEventQuestCompleteResponseLikeCpp {
    GameEventQuestCompleteResponseLikeCpp {
        quest_id,
        condition_save_updates_queued: summary.condition_save_updates_queued,
        condition_save_updates_executed: summary.condition_save_updates_executed,
        condition_save_updates_failed: summary.condition_save_updates_failed,
        condition_save_updates_skipped_non_progress: summary
            .condition_save_updates_skipped_non_progress,
        save_world_event_state_requested: summary.save_world_event_state_requested,
        world_event_state_save_requested: summary.world_event_state_save_requested,
        world_event_state_saves_queued: summary.world_event_state_summary.saves_queued,
        world_event_state_saves_executed: summary.world_event_state_summary.saves_executed,
        world_event_state_saves_failed: summary.world_event_state_summary.saves_failed,
        world_event_state_saves_skipped_event_id_out_of_range: summary
            .world_event_state_summary
            .saves_skipped_event_id_out_of_range,
        world_event_state_saves_skipped_missing_event: summary
            .world_event_state_summary
            .saves_skipped_missing_event,
        force_game_event_update_requested: summary.force_game_event_update_requested_flag,
        force_game_event_update_requests: summary.force_game_event_update_requested,
        processor_failed: false,
    }
}

pub(crate) fn game_event_quest_complete_processor_failed_response_like_cpp(
    quest_id: u32,
) -> GameEventQuestCompleteResponseLikeCpp {
    GameEventQuestCompleteResponseLikeCpp {
        quest_id,
        processor_failed: true,
        ..GameEventQuestCompleteResponseLikeCpp::default()
    }
}

pub(crate) async fn run_game_event_quest_complete_processor_like_cpp(
    command_rx: flume::Receiver<GameEventQuestCompleteCommandLikeCpp>,
    canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    game_event_persistence: Arc<dyn GameEventPersistencePortLikeCpp>,
) {
    while let Ok(command) = command_rx.recv_async().await {
        let quest_id = command.quest_id;
        let maybe_summary = {
            let Ok(mut metadata) = canonical_spawn_metadata.lock() else {
                tracing::error!(
                    quest_id,
                    "CanonicalSpawnMetadataLikeCpp mutex poisoned during C++ GameEventMgr::HandleQuestComplete bridge"
                );
                let _ = command.response_tx.try_send(
                    game_event_quest_complete_processor_failed_response_like_cpp(quest_id),
                );
                continue;
            };
            let outcome = metadata.represented_handle_game_event_quest_complete_like_cpp(
                quest_id,
                current_unix_time_secs_like_cpp(),
            );
            materialize_game_event_quest_complete_db_bridge_like_cpp(&outcome, &metadata)
        };

        let mut summary = maybe_summary;
        execute_game_event_quest_complete_condition_save_db_bridge_like_cpp(
            game_event_persistence.as_ref(),
            &mut summary,
        )
        .await;
        execute_game_event_world_event_state_db_bridge_like_cpp(
            game_event_persistence.as_ref(),
            &mut summary.world_event_state_summary,
        )
        .await;

        let response = game_event_quest_complete_response_from_summary_like_cpp(quest_id, &summary);
        let _ = command.response_tx.try_send(response);
    }
}

pub(crate) fn represented_game_event_world_conditions_met_like_cpp(_event_id: u16) -> bool {
    false
}
