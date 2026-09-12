//! #756 — the canonical Player owns its quest state and its invariants.
//!
//! C++ `Player` holds `QuestStatusMap m_QuestStatus` (`Player.h:2947`),
//! `RewardedQuestSet m_RewardedQuests` (`:2951`), `m_DFQuests` (`:2481`) and
//! `m_seasonalquests` (`:2838`), and performs the transitions itself:
//! `SetQuestStatus` (`Player.cpp:15557`), `RemoveActiveQuest` (`:15575`),
//! `RemoveRewardedQuest` (`:15595`), `SetQuestObjectiveData` (`:16426`) and the
//! daily/weekly/seasonal/monthly setters (`:24037`, `:24061`, `:24067`,
//! `:24077`).

use super::*;

use std::collections::{BTreeMap, BTreeSet};

use crate::{PlayerQuestGameplayState, PlayerQuestStatusRecord};

fn status(quest_id: u32, slot: u8) -> PlayerQuestStatusRecord {
    PlayerQuestStatusRecord {
        quest_id,
        status: 1,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: vec![0],
        slot,
    }
}

#[test]
fn a_fresh_quest_state_is_unhydrated_and_empty_like_cpp() {
    let state = PlayerQuestGameplayState::default();

    assert!(state.statuses_like_cpp().is_empty());
    assert!(state.rewarded_quest_ids_like_cpp().is_empty());
    assert!(!state.status_authority_complete_like_cpp());
    assert!(!state.seasonal_quest_changed_like_cpp());
    assert_eq!(state.last_daily_quest_time_secs_like_cpp(), 0);
    assert!(state.pending_share_like_cpp().is_none());
}

#[test]
fn an_authoritative_empty_status_load_is_not_an_unhydrated_owner_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();

    state.replace_statuses_like_cpp(BTreeMap::new(), true);

    assert!(state.status_authority_complete_like_cpp());
    assert!(state.statuses_like_cpp().is_empty());

    state.set_status_authority_complete_like_cpp(false);
    assert!(!state.status_authority_complete_like_cpp());
}

#[test]
fn quest_statuses_insert_and_remove_by_id_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();

    state.insert_status_like_cpp(10, status(10, 0));
    assert_eq!(state.status_like_cpp(10).map(|s| s.slot), Some(0));

    // C++ `SetQuestStatus` overwrites the record for a quest already tracked.
    state.insert_status_like_cpp(10, status(10, 3));
    assert_eq!(state.statuses_like_cpp().len(), 1);
    assert_eq!(state.status_like_cpp(10).map(|s| s.slot), Some(3));

    assert!(state.remove_status_like_cpp(10).is_some());
    assert!(state.remove_status_like_cpp(10).is_none());
    assert!(state.status_like_cpp(10).is_none());
}

#[test]
fn the_periodic_buckets_track_membership_independently_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();

    state.set_daily_like_cpp(10, true);
    state.set_weekly_like_cpp(20, true);
    state.set_monthly_like_cpp(30, true);
    state.set_df_quest_like_cpp(40, true);
    state.set_rewarded_like_cpp(50, true);

    assert!(state.daily_quest_ids_like_cpp().contains(&10));
    assert!(state.weekly_quest_ids_like_cpp().contains(&20));
    assert!(state.monthly_quest_ids_like_cpp().contains(&30));
    assert!(state.df_quest_ids_like_cpp().contains(&40));
    assert!(state.rewarded_quest_ids_like_cpp().contains(&50));

    state.set_daily_like_cpp(10, false);
    state.set_rewarded_like_cpp(50, false);
    assert!(state.daily_quest_ids_like_cpp().is_empty());
    assert!(state.rewarded_quest_ids_like_cpp().is_empty());
    // The other buckets are untouched by a daily or rewarded change.
    assert!(state.weekly_quest_ids_like_cpp().contains(&20));
}

#[test]
fn a_seasonal_quest_is_keyed_under_its_event_and_marks_the_change_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();

    state.set_seasonal_like_cpp(7, 10, 1_700);
    state.set_seasonal_like_cpp(7, 20, 1_800);
    state.set_seasonal_like_cpp(9, 30, 1_900);

    assert_eq!(
        state.seasonal_event_quests_like_cpp(7).map(|b| b.len()),
        Some(2)
    );
    assert_eq!(
        state
            .seasonal_event_quests_like_cpp(7)
            .and_then(|b| b.get(&10)),
        Some(&1_700)
    );
    assert!(state.seasonal_quest_changed_like_cpp());
}

#[test]
fn a_seasonal_reset_removes_only_quests_completed_before_the_new_start_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();
    state.set_seasonal_like_cpp(7, 10, 1_000);
    state.set_seasonal_like_cpp(7, 20, 3_000);
    state.set_seasonal_quest_changed_like_cpp(false);

    let reset = state.reset_seasonal_event_like_cpp(7, 2_000);

    assert_eq!(reset.removed_quest_ids, vec![10]);
    assert!(!reset.event_bucket_erased);
    assert!(state.seasonal_quest_changed_like_cpp());
    assert_eq!(
        state.seasonal_event_quests_like_cpp(7).map(|b| b.len()),
        Some(1)
    );
}

#[test]
fn a_seasonal_reset_erases_the_event_once_its_last_quest_is_gone_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();
    state.set_seasonal_like_cpp(7, 10, 1_000);

    let reset = state.reset_seasonal_event_like_cpp(7, 2_000);

    assert_eq!(reset.removed_quest_ids, vec![10]);
    assert!(reset.event_bucket_erased);
    assert!(state.seasonal_event_quests_like_cpp(7).is_none());
}

#[test]
fn a_seasonal_reset_of_an_unknown_or_untouched_event_changes_nothing_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();
    state.set_seasonal_like_cpp(7, 10, 3_000);
    state.set_seasonal_quest_changed_like_cpp(false);

    let missing = state.reset_seasonal_event_like_cpp(9, 2_000);
    assert!(missing.removed_quest_ids.is_empty());
    assert!(!missing.event_bucket_erased);

    let untouched = state.reset_seasonal_event_like_cpp(7, 2_000);
    assert!(untouched.removed_quest_ids.is_empty());
    assert!(!untouched.event_bucket_erased);
    assert!(!state.seasonal_quest_changed_like_cpp());
    assert_eq!(
        state.seasonal_event_quests_like_cpp(7).map(|b| b.len()),
        Some(1)
    );
}

#[test]
fn seeding_a_stored_seasonal_completion_does_not_mark_the_state_changed_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();

    // C++ `SetSeasonalQuestStatus` is a live completion and sets the dirty
    // flag; restoring a stored row must not, or the next save would treat
    // untouched state as dirty.
    state.seed_seasonal_quest_like_cpp(7, 10, 1_000);
    assert!(!state.seasonal_quest_changed_like_cpp());
    assert_eq!(
        state
            .seasonal_event_quests_like_cpp(7)
            .and_then(|b| b.get(&10)),
        Some(&1_000)
    );

    state.set_seasonal_like_cpp(7, 20, 2_000);
    assert!(state.seasonal_quest_changed_like_cpp());
}

#[test]
fn ensuring_a_seasonal_event_does_not_touch_its_quests_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();
    state.set_seasonal_like_cpp(7, 10, 1_000);
    state.set_seasonal_quest_changed_like_cpp(false);

    state.ensure_seasonal_event_like_cpp(7);
    state.ensure_seasonal_event_like_cpp(9);

    assert_eq!(
        state.seasonal_event_quests_like_cpp(7).map(|b| b.len()),
        Some(1)
    );
    assert_eq!(
        state.seasonal_event_quests_like_cpp(9).map(|b| b.len()),
        Some(0)
    );
    assert!(!state.seasonal_quest_changed_like_cpp());
}

#[test]
fn rewarded_rows_are_tracked_apart_from_rewarded_quests_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();

    state.set_rewarded_like_cpp(10, true);
    state.set_rewarded_row_like_cpp(20, true);

    // The persisted rows and the in-memory rewarded set are distinct facts.
    assert!(state.rewarded_quest_ids_like_cpp().contains(&10));
    assert!(!state.rewarded_quest_ids_like_cpp().contains(&20));
    assert!(state.rewarded_quest_rows_like_cpp().contains(&20));
    assert!(!state.rewarded_quest_rows_like_cpp().contains(&10));

    state.clear_rewarded_quest_rows_like_cpp();
    assert!(state.rewarded_quest_rows_like_cpp().is_empty());
    assert!(state.rewarded_quest_ids_like_cpp().contains(&10));
}

#[test]
fn the_player_owns_the_quest_state_for_its_whole_lifetime_like_cpp() {
    let mut player = Player::new(Some(1), false);

    player
        .gameplay_state_mut()
        .quests
        .insert_status_like_cpp(10, status(10, 2));
    player
        .gameplay_state_mut()
        .quests
        .replace_rewarded_quest_ids_like_cpp(BTreeSet::from([20]));
    player
        .gameplay_state_mut()
        .quests
        .set_pending_share_like_cpp(Some((wow_core::ObjectGuid::create_player(1, 7), 30)));

    let quests = &player.gameplay_state().quests;
    assert_eq!(quests.status_like_cpp(10).map(|s| s.slot), Some(2));
    assert!(quests.rewarded_quest_ids_like_cpp().contains(&20));
    assert_eq!(quests.pending_share_like_cpp().map(|(_, id)| id), Some(30));
}
