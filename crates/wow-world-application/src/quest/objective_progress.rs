// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Per-quest status persistence used while objective progress is drained.

use wow_persistence::{
    PersistenceOutcomeLikeCpp, PlayerQuestStatusPersistenceRequestLikeCpp,
    QuestStatusPersistenceLikeCpp,
};
use wow_world_core::session::{QuestObjectiveAccessLikeCpp, SessionCatalogs};
use wow_world_lifecycle::SessionLifecycleState;

use super::SessionQuestState;

pub use wow_world_core::session::MAX_QUEST_LOG_SIZE_LIKE_CPP;

pub fn plan_quest_status_save_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &SessionQuestState,
    catalogs: &SessionCatalogs,
    quest_id: u32,
    status: u8,
    fixture_fallback: bool,
) -> Option<PlayerQuestStatusPersistenceRequestLikeCpp> {
    let owner_guid = owner.player_guid_like_cpp()?.counter() as u64;
    let quest_state =
        current_quest_gameplay_snapshot_like_cpp(owner, quest_state, fixture_fallback);
    let mut projection = match quest_state
        .as_ref()
        .and_then(|state| state.statuses_like_cpp().get(&quest_id))
    {
        Some(saved) => catalogs.represented_quest_status_persistence_like_cpp(saved),
        None if status == wow_conditions::QUEST_STATUS_REWARDED_LIKE_CPP => {
            QuestStatusPersistenceLikeCpp {
                quest_id,
                status,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objectives: Vec::new(),
            }
        }
        None => {
            tracing::warn!(
                account = owner.account_id_like_cpp(),
                quest_id,
                "Quest status save skipped because canonical Player quest state is unavailable"
            );
            return None;
        }
    };
    projection.status = status;
    Some(PlayerQuestStatusPersistenceRequestLikeCpp::Save {
        owner_guid,
        status: projection,
    })
}

pub async fn save_quest_to_db_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &SessionQuestState,
    catalogs: &SessionCatalogs,
    lifecycle: &SessionLifecycleState,
    quest_id: u32,
    status: u8,
    fixture_fallback: bool,
) {
    let port = match lifecycle.player_quest_persistence_port_like_cpp() {
        Some(port) => port,
        None => return,
    };
    let Some(request) = plan_quest_status_save_like_cpp(
        owner,
        quest_state,
        catalogs,
        quest_id,
        status,
        fixture_fallback,
    ) else {
        return;
    };

    match port.persist_status_like_cpp(request).await {
        PersistenceOutcomeLikeCpp::Applied { .. } => {}
        PersistenceOutcomeLikeCpp::Failed { reason } => tracing::warn!(
            account = owner.account_id_like_cpp(),
            quest_id,
            error = %reason,
            "Failed to save quest status"
        ),
        PersistenceOutcomeLikeCpp::Unknown { reason } => tracing::warn!(
            account = owner.account_id_like_cpp(),
            quest_id,
            error = %reason,
            "Quest status save commit outcome is unknown"
        ),
    }
}

pub async fn save_changed_quest_statuses_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &SessionQuestState,
    catalogs: &SessionCatalogs,
    lifecycle: &SessionLifecycleState,
    quest_ids: &mut Vec<u32>,
    fixture_fallback: bool,
) {
    quest_ids.sort_unstable();
    quest_ids.dedup();
    for quest_id in quest_ids.drain(..) {
        if let Some(status) =
            current_quest_gameplay_snapshot_like_cpp(owner, quest_state, fixture_fallback).and_then(
                |state| {
                    state
                        .statuses_like_cpp()
                        .get(&quest_id)
                        .map(|status| status.status)
                },
            )
        {
            save_quest_to_db_like_cpp(
                owner,
                quest_state,
                catalogs,
                lifecycle,
                quest_id,
                status,
                fixture_fallback,
            )
            .await;
        }
    }
}

pub fn invalidate_player_quest_status_authority_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &mut SessionQuestState,
    world_test_consumer: bool,
) {
    #[cfg(any(test, feature = "test-fixtures"))]
    let fixture_owner = world_test_consumer && owner.owner_handle_absent_like_cpp();

    #[cfg(any(test, feature = "test-fixtures"))]
    let canonical = if fixture_owner {
        let mut state = quest_state.player_quest_gameplay_fixture_like_cpp();
        state.set_status_authority_complete_like_cpp(false);
        quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
        Some(())
    } else {
        owner.mark_quest_status_authority_incomplete_like_cpp()
    };

    #[cfg(not(any(test, feature = "test-fixtures")))]
    let canonical = owner.mark_quest_status_authority_incomplete_like_cpp();

    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer
        && !fixture_owner
        && canonical.is_some()
        && let Some(state) = owner.player_quest_gameplay_snapshot_like_cpp()
    {
        quest_state.apply_player_quest_core_compatibility_like_cpp(&state);
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = (canonical, world_test_consumer);

    owner.invalidate_spell_hit_aura_authority_like_cpp();
}

pub fn update_objective_count_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &mut SessionQuestState,
    quest_id: u32,
    objective_index: usize,
    required: i32,
    add_count: i32,
    world_test_consumer: bool,
) -> Option<i32> {
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer && owner.owner_handle_absent_like_cpp() {
        let mut state = quest_state.player_quest_gameplay_fixture_like_cpp();
        let Some(status) = state.status_mut_like_cpp(quest_id) else {
            quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
            return None;
        };
        if status.objective_counts.len() <= objective_index {
            status.objective_counts.resize(objective_index + 1, 0);
        }
        if add_count >= 0 && status.objective_counts[objective_index] >= required {
            quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
            return None;
        }
        status.objective_counts[objective_index] = status.objective_counts[objective_index]
            .saturating_add(add_count)
            .clamp(0, required);
        let current = status.objective_counts[objective_index];
        quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
        return Some(current);
    }

    let canonical =
        owner.update_objective_count_like_cpp(quest_id, objective_index, required, add_count);
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer
        && canonical.is_some()
        && let Some(state) = owner.player_quest_gameplay_snapshot_like_cpp()
    {
        quest_state.apply_player_quest_core_compatibility_like_cpp(&state);
    }
    canonical.flatten()
}

pub fn update_storing_flag_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &mut SessionQuestState,
    quest_id: u32,
    objective_index: usize,
    add_count: i32,
    world_test_consumer: bool,
) -> Option<(bool, bool)> {
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer && owner.owner_handle_absent_like_cpp() {
        let mut state = quest_state.player_quest_gameplay_fixture_like_cpp();
        let Some(status) = state.status_mut_like_cpp(quest_id) else {
            quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
            return None;
        };
        if status.objective_counts.len() <= objective_index {
            status.objective_counts.resize(objective_index + 1, 0);
        }
        let before = status.objective_counts[objective_index] != 0;
        status.objective_counts[objective_index] = i32::from(add_count > 0);
        let after = status.objective_counts[objective_index] != 0;
        quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
        return Some((before, after));
    }

    let canonical = owner.update_storing_flag_like_cpp(quest_id, objective_index, add_count);
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer
        && canonical.is_some()
        && let Some(state) = owner.player_quest_gameplay_snapshot_like_cpp()
    {
        quest_state.apply_player_quest_core_compatibility_like_cpp(&state);
    }
    canonical.flatten()
}

pub fn mark_quest_incomplete_if_complete_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &mut SessionQuestState,
    quest_id: u32,
    complete_status: u8,
    incomplete_status: u8,
    world_test_consumer: bool,
) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer && owner.owner_handle_absent_like_cpp() {
        let mut state = quest_state.player_quest_gameplay_fixture_like_cpp();
        let Some(status) = state.status_mut_like_cpp(quest_id) else {
            quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
            return false;
        };
        if status.status != complete_status {
            quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
            return false;
        }
        status.status = incomplete_status;
        quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
        return true;
    }

    let canonical = owner.mark_quest_incomplete_if_complete_like_cpp(
        quest_id,
        complete_status,
        incomplete_status,
    );
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer
        && canonical.is_some()
        && let Some(state) = owner.player_quest_gameplay_snapshot_like_cpp()
    {
        quest_state.apply_player_quest_core_compatibility_like_cpp(&state);
    }
    canonical.unwrap_or(false)
}

pub(super) fn current_quest_gameplay_snapshot_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &SessionQuestState,
    fixture_fallback: bool,
) -> Option<wow_entities::PlayerQuestGameplayState> {
    #[cfg(any(test, feature = "test-fixtures"))]
    {
        if fixture_fallback && owner.owner_handle_absent_like_cpp() {
            return Some(quest_state.player_quest_gameplay_fixture_like_cpp());
        }
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = fixture_fallback;

    owner.player_quest_gameplay_snapshot_like_cpp()
}

/// Return the represented active quest slot using the same selected snapshot
/// path as objective persistence and mutation. `max_quest_log_size` is supplied
/// by the owning quest protocol module so this application does not duplicate
/// its slot-count constant.
pub fn find_quest_slot_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &SessionQuestState,
    quest_id: u32,
    world_test_consumer: bool,
) -> Option<u8> {
    current_quest_gameplay_snapshot_like_cpp(owner, quest_state, world_test_consumer)?
        .statuses_like_cpp()
        .get(&quest_id)
        .and_then(|status| {
            (status.slot < MAX_QUEST_LOG_SIZE_LIKE_CPP
                && matches!(
                    status.status,
                    wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                        | wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
                        | wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP
                ))
            .then_some(status.slot)
        })
}
