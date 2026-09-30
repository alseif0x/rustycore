//! Staged reset contracts: optional writeback, lazy reads and ordered accounting.

use super::*;

#[test]
fn absent_and_empty_events_do_not_request_writeback_or_read_bits() {
    for empty_bucket in [false, true] {
        let mut state = PlayerQuestGameplayState::default();
        if empty_bucket {
            state.ensure_seasonal_event_like_cpp(7);
        }
        let mut plan = state.plan_seasonal_reset(7, 100);
        assert!(plan.take_updated_seasonal_quests().is_none());
        let outcome = plan.finish(|_| panic!("no removed quest may read a bit"));
        assert_eq!(
            outcome.reason,
            if empty_bucket {
                SeasonalQuestResetReason::EmptyEvent
            } else {
                SeasonalQuestResetReason::MissingEvent
            }
        );
        assert!(outcome.removed_quest_ids.is_empty());
        assert!(!outcome.seasonal_quest_changed);
    }
}

#[test]
fn nonempty_event_with_no_older_rows_still_requests_one_writeback() {
    let mut state = PlayerQuestGameplayState::default();
    state.seed_seasonal_quest_like_cpp(7, 1001, 100);
    let mut plan = state.plan_seasonal_reset(7, 100);
    let updated = plan.take_updated_seasonal_quests().expect("nonempty writeback");
    assert_eq!(updated.get(&7), Some(&BTreeMap::from([(1001, 100)])));
    assert!(plan.take_updated_seasonal_quests().is_none());
    let outcome = plan.finish(|_| panic!("equal timestamps do not read bits"));
    assert_eq!(outcome.reason, SeasonalQuestResetReason::NoOlderCompletions);
    assert!(!outcome.event_bucket_erased);
}

#[test]
fn removed_quest_callbacks_follow_btree_order_and_accumulate_all_four_observations() {
    let mut state = PlayerQuestGameplayState::default();
    for quest_id in [1004, 1002, 1003, 1001] {
        state.seed_seasonal_quest_like_cpp(7, quest_id, 99);
    }
    let mut plan = state.plan_seasonal_reset(7, 100);
    assert!(plan.take_updated_seasonal_quests().unwrap().is_empty());
    let mut calls = Vec::new();
    let outcome = plan.finish(|quest_id| {
        calls.push(quest_id);
        match quest_id {
            1001 => SeasonalQuestBitReset::MissingCatalog,
            1002 => SeasonalQuestBitReset::ZeroUniqueBit,
            1003 => SeasonalQuestBitReset::Cleared,
            1004 => SeasonalQuestBitReset::NoChangeOrNoop,
            _ => panic!("unexpected removed quest"),
        }
    });
    assert_eq!(calls, vec![1001, 1002, 1003, 1004]);
    assert_eq!(outcome.removed_quest_ids, calls);
    assert_eq!(outcome.completed_bit_skipped_no_quest_v2_store, 1);
    assert_eq!(outcome.completed_bit_skipped_zero_unique_bit, 1);
    assert_eq!(outcome.completed_bit_cleared, 1);
    assert_eq!(outcome.completed_bit_no_change_or_noop, 1);
    assert_eq!(outcome.completed_bit_clear_unrepresented, 0);
    assert!(!outcome.seasonal_quest_changed);
}

#[test]
fn dropping_a_plan_neither_applies_its_snapshot_nor_clears_player_bits() {
    let mut player = Player::new(Some(1), false);
    player
        .gameplay_state_mut()
        .quests
        .seed_seasonal_quest_like_cpp(7, 1001, 99);
    assert!(player.set_quest_completed_bit_like_cpp(65, true));
    let plan = player.gameplay_state().quests.clone().plan_seasonal_reset(7, 100);
    drop(plan);
    assert_eq!(
        player.gameplay_state().quests.seasonal_event_quests_like_cpp(7),
        Some(&BTreeMap::from([(1001, 99)]))
    );
    assert_eq!(player.quest_completed_block_like_cpp(1), Some(1));
}
