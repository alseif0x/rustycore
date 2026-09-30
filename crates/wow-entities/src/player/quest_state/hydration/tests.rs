use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use wow_constants::quest::{QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_NONE_LIKE_CPP};

fn objective(storage_index: i8, obj_type: u8) -> QuestObjective {
    QuestObjective {
        id: 11,
        quest_id: 1,
        obj_type,
        order: 0,
        storage_index,
        object_id: 42,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    }
}

fn active(state: &mut PlayerQuestGameplayState, id: u32, counts: Vec<i32>) {
    state.insert_status_like_cpp(
        id,
        PlayerQuestStatusRecord {
            quest_id: id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 12,
            end_time_secs: 34,
            objective_counts: counts,
            slot: 0,
        },
    );
}

#[test]
fn active_rows_preserve_status_range_normalization_raw_times_and_explored() {
    let mut state = PlayerQuestGameplayState::default();
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    let mut queries = Vec::new();
    for raw in [0, 1, 2, 3, 4, 5, 7, 255] {
        hydration.hydrate_active_status(&mut state, u32::from(raw), raw, raw, -12, -34, |id| {
            queries.push(id);
            2
        });
    }
    for (slot, raw) in [0, 1, 2, 3, 4, 5, 7, 255].into_iter().enumerate() {
        let record = &state.statuses_like_cpp()[&u32::from(raw)];
        assert_eq!(
            record.status,
            if raw < 7 {
                raw
            } else {
                QUEST_STATUS_INCOMPLETE_LIKE_CPP
            }
        );
        assert_eq!(record.slot, slot as u8);
        assert_eq!(record.explored, raw != 0);
        assert_eq!((record.accept_time_secs, record.end_time_secs), (-12, -34));
        assert_eq!(record.objective_counts, vec![0, 0]);
    }
    assert_eq!(queries, vec![0, 1, 2, 3, 4, 5, 7, 255]);
    hydration.rewarded_rows_loaded();
    assert!(hydration.finish(&mut state).is_empty());
    assert!(state.status_authority_complete_like_cpp());
}

#[test]
fn active_rewarded_rows_skip_metadata_and_preserve_stale_order_and_duplicates() {
    let mut state = PlayerQuestGameplayState::default();
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    for id in [9, 2, 9] {
        hydration.hydrate_active_status(
            &mut state,
            id,
            QUEST_STATUS_REWARDED_LIKE_CPP,
            1,
            1,
            2,
            |_| panic!("rewarded active rows must not resolve metadata"),
        );
    }
    hydration.hydrate_active_status(&mut state, 3, 3, 0, 1, 2, |_| 0);
    assert_eq!(state.statuses_like_cpp()[&3].slot, 0);
    assert_eq!(
        state
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![2, 9]
    );
    assert!(state.rewarded_quest_rows_like_cpp().is_empty());
    hydration.rewarded_rows_loaded();
    assert_eq!(hydration.finish(&mut state), vec![9, 2, 9]);
    assert!(state.status_authority_complete_like_cpp());
}

#[test]
fn duplicate_active_rows_consume_slots_overwrite_and_invalidate_authority() {
    let mut state = PlayerQuestGameplayState::default();
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    let mut queries = Vec::new();
    hydration.hydrate_active_status(&mut state, 1, 3, 0, 10, 20, |id| {
        queries.push(id);
        4
    });
    hydration.hydrate_active_status(&mut state, 1, 1, 2, 30, 40, |id| {
        queries.push(id);
        1
    });
    hydration.hydrate_active_status(&mut state, 2, 0, 0, 50, 60, |id| {
        queries.push(id);
        0
    });
    assert_eq!(queries, vec![1, 1, 2]);
    assert_eq!(
        state.statuses_like_cpp()[&1],
        PlayerQuestStatusRecord {
            quest_id: 1,
            status: 1,
            explored: true,
            accept_time_secs: 30,
            end_time_secs: 40,
            objective_counts: vec![0],
            slot: 1,
        }
    );
    assert_eq!(state.statuses_like_cpp()[&2].slot, 2);
    hydration.rewarded_rows_loaded();
    hydration.finish(&mut state);
    assert!(!state.status_authority_complete_like_cpp());
}

#[test]
fn full_capacity_skips_metadata_and_duplicate_detection_but_accepts_rewarded() {
    let mut state = PlayerQuestGameplayState::default();
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    for id in 0..u32::from(MAX_QUEST_LOG_SIZE) {
        hydration.hydrate_active_status(&mut state, id, QUEST_STATUS_NONE_LIKE_CPP, 0, 0, 0, |_| 0);
    }
    for id in [0, 100] {
        hydration.hydrate_active_status(&mut state, id, 3, 1, 10, 20, |_| {
            panic!("full capacity must not resolve metadata")
        });
    }
    hydration.hydrate_active_status(&mut state, 101, 6, 0, 0, 0, |_| {
        panic!("rewarded still bypasses metadata after capacity")
    });
    hydration.rewarded_rows_loaded();
    assert_eq!(hydration.finish(&mut state), vec![101]);
    assert_eq!(
        state.statuses_like_cpp().len(),
        usize::from(MAX_QUEST_LOG_SIZE)
    );
    assert_eq!(
        state.statuses_like_cpp()[&0].status,
        QUEST_STATUS_NONE_LIKE_CPP
    );
    assert!(!state.statuses_like_cpp().contains_key(&100));
    assert!(state.status_authority_complete_like_cpp());
}

#[test]
fn rejected_active_rows_invalidate_without_consuming_slots() {
    let mut state = PlayerQuestGameplayState::default();
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    for _ in 0..5 {
        hydration.reject_active_row();
    }
    hydration.hydrate_active_status(&mut state, 1, 3, 0, 0, 0, |_| 0);
    hydration.rewarded_rows_loaded();
    hydration.finish(&mut state);
    assert_eq!(state.statuses_like_cpp()[&1].slot, 0);
    assert!(!state.status_authority_complete_like_cpp());
}

#[test]
fn active_slot_advance_precedes_callback_and_insert_follows_callback() {
    let mut state = PlayerQuestGameplayState::default();
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            hydration.hydrate_active_status(&mut state, 1, 3, 0, 0, 0, |_| {
                panic!("observe the callback boundary")
            });
        }))
        .is_err()
    );
    assert!(state.statuses_like_cpp().is_empty());
    hydration.hydrate_active_status(&mut state, 2, 3, 0, 0, 0, |_| 0);
    assert_eq!(state.statuses_like_cpp()[&2].slot, 1);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            hydration.hydrate_active_status(&mut state, 2, 1, 1, 10, 20, |_| {
                panic!("duplicate coherence is evaluated after the callback")
            });
        }))
        .is_err()
    );
    assert_eq!(state.statuses_like_cpp()[&2].slot, 1);
    hydration.rewarded_rows_loaded();
    hydration.finish(&mut state);
    assert!(state.status_authority_complete_like_cpp());
}

#[test]
fn objectives_resolve_once_even_without_status_and_do_not_create_records() {
    let mut state = PlayerQuestGameplayState::default();
    let rows = [objective(0, 0)];
    let mut queries = Vec::new();
    state.hydrate_objective_count(9, 0, 5, |id| {
        queries.push(id);
        Some(&rows)
    });
    state.hydrate_objective_count(10, 0, 5, |id| {
        queries.push(id);
        None
    });
    assert_eq!(queries, vec![9, 10]);
    assert!(state.statuses_like_cpp().is_empty());
}

#[test]
fn absent_objective_metadata_or_match_keeps_counts_and_record_fields() {
    let mut state = PlayerQuestGameplayState::default();
    active(&mut state, 1, vec![8]);
    let before = state.clone();
    state.hydrate_objective_count(1, 3, 99, |_| None);
    state.hydrate_objective_count(1, 3, 99, |_| Some(&[]));
    let rows = [objective(0, 0)];
    state.hydrate_objective_count(1, 3, 99, |_| Some(&rows));
    assert_eq!(state, before);
}

#[test]
fn objectives_use_first_matching_checked_index_and_preserve_raw_values() {
    let mut state = PlayerQuestGameplayState::default();
    active(&mut state, 1, vec![]);
    let rows = [objective(2, 0), objective(2, 14)];
    state.hydrate_objective_count(1, 2, -42, |_| Some(&rows));
    assert_eq!(
        state.statuses_like_cpp()[&1].objective_counts,
        vec![0, 0, -42]
    );
    state.hydrate_objective_count(1, 2, i32::MAX, |_| Some(&rows));
    assert_eq!(
        state.statuses_like_cpp()[&1].objective_counts,
        vec![0, 0, i32::MAX]
    );
    let negative = [objective(-1, 0)];
    state.hydrate_objective_count(1, 255, 9, |_| Some(&negative));
    assert_eq!(state.statuses_like_cpp()[&1].objective_counts.len(), 3);
}

#[test]
fn flag_counts_normalize_nonzero_and_duplicate_rows_overwrite_without_clamping() {
    let mut state = PlayerQuestGameplayState::default();
    active(&mut state, 1, vec![]);
    let rows = [objective(1, 14), objective(1, 0)];
    for (data, expected) in [(i32::MIN, 1), (0, 0), (i32::MAX, 1), (0, 0)] {
        state.hydrate_objective_count(1, 1, data, |_| Some(&rows));
        assert_eq!(
            state.statuses_like_cpp()[&1].objective_counts,
            vec![0, expected]
        );
    }
}

#[test]
fn objective_hydration_retains_absent_slot_status_gates_and_large_resize() {
    let mut state = PlayerQuestGameplayState::default();
    active(&mut state, 1, vec![4]);
    let record = state.status_mut_like_cpp(1).unwrap();
    record.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
    record.slot = u8::MAX;
    state.set_status_authority_complete_like_cpp(true);
    let rows = [objective(i8::MAX, 0)];
    state.hydrate_objective_count(1, i8::MAX as u8, i32::MIN, |_| Some(&rows));
    let record = &state.statuses_like_cpp()[&1];
    assert_eq!(record.objective_counts.len(), 128);
    assert_eq!(record.objective_counts[0], 4);
    assert!(
        record.objective_counts[1..127]
            .iter()
            .all(|count| *count == 0)
    );
    assert_eq!(record.objective_counts[127], i32::MIN);
    assert_eq!(
        (
            record.status,
            record.slot,
            record.accept_time_secs,
            record.end_time_secs
        ),
        (1, 255, 12, 34)
    );
    assert!(state.status_authority_complete_like_cpp());
}

#[test]
fn authority_distinguishes_successful_empty_rewarded_query_from_failure() {
    for loaded in [false, true] {
        let mut state = PlayerQuestGameplayState::default();
        state.set_status_authority_complete_like_cpp(true);
        let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
        if loaded {
            hydration.rewarded_rows_loaded();
        }
        assert!(hydration.finish(&mut state).is_empty());
        assert_eq!(state.status_authority_complete_like_cpp(), loaded);
    }
}

#[test]
fn rewarded_rows_separate_persisted_membership_from_logical_membership() {
    let mut state = PlayerQuestGameplayState::default();
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    hydration.hydrate_active_status(&mut state, 4, 6, 0, 0, 0, |_| panic!("no lookup"));
    hydration.rewarded_rows_loaded();
    let mut queries = Vec::new();
    for (id, can_increase) in [(1, Some(true)), (2, Some(false)), (3, None), (4, None)] {
        hydration.hydrate_rewarded_row(&mut state, Some(id), |id| {
            queries.push(id);
            can_increase
        });
    }
    assert_eq!(queries, vec![1, 2, 3, 4]);
    assert_eq!(
        state
            .rewarded_quest_rows_like_cpp()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(
        state
            .rewarded_quest_ids_like_cpp()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![1, 4]
    );
    assert_eq!(hydration.finish(&mut state), vec![4]);
    assert!(state.status_authority_complete_like_cpp());
}

#[test]
fn null_rewarded_id_skips_callback_and_keeps_incoherence_after_valid_rows() {
    let mut state = PlayerQuestGameplayState::default();
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    hydration.rewarded_rows_loaded();
    hydration.hydrate_rewarded_row(&mut state, None, |_| panic!("null must not resolve"));
    hydration.hydrate_rewarded_row(&mut state, Some(0), |_| Some(true));
    hydration.finish(&mut state);
    assert_eq!(
        state
            .rewarded_quest_rows_like_cpp()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![0]
    );
    assert!(state.rewarded_quest_ids_like_cpp().contains(&0));
    assert!(!state.status_authority_complete_like_cpp());
}

#[test]
fn rewarded_row_insert_precedes_callback_and_logical_insert_follows_it() {
    let mut state = PlayerQuestGameplayState::default();
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    hydration.rewarded_rows_loaded();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            hydration.hydrate_rewarded_row(&mut state, Some(9), |_| {
                panic!("observe the callback boundary")
            });
        }))
        .is_err()
    );
    assert!(state.rewarded_quest_rows_like_cpp().contains(&9));
    assert!(!state.rewarded_quest_ids_like_cpp().contains(&9));
}

#[test]
fn hydration_closes_authority_without_touching_recurrence_or_other_state() {
    let mut state = PlayerQuestGameplayState::default();
    state.set_daily_like_cpp(10, true);
    state.set_df_quest_like_cpp(11, true);
    state.set_weekly_like_cpp(12, true);
    state.set_monthly_like_cpp(13, true);
    state.set_seasonal_like_cpp(3, 14, 15);
    state.set_last_daily_quest_time_secs_like_cpp(-16);
    let mut expected = state.clone();
    expected.set_status_authority_complete_like_cpp(true);
    let mut hydration = PlayerQuestGameplayState::begin_status_hydration();
    hydration.rewarded_rows_loaded();
    hydration.finish(&mut state);
    assert_eq!(state, expected);
}
