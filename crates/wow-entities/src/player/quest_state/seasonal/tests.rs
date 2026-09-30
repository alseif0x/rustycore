//! Original seasonal recurrence cases on the Domain owner and real bit storage.

use super::*;
use crate::Player;
use std::collections::BTreeSet;
use super::SeasonalQuestResetReason as ResetSeasonalQuestStatusReasonLikeCpp;

mod stages;

fn reset(
    player: &mut Player,
    unique_bits: Option<&BTreeMap<u32, u16>>,
    event_id: u16,
    event_start_time: u64,
) -> SeasonalQuestResetOutcome {
    player
        .gameplay_state_mut()
        .quests
        .set_seasonal_quest_changed_like_cpp(false);
    let recurrence = player.gameplay_state().quests.clone();
    let mut plan = recurrence.plan_seasonal_reset(event_id, event_start_time);
    if let Some(seasonal_quests) = plan.take_updated_seasonal_quests() {
        player
            .gameplay_state_mut()
            .quests
            .replace_seasonal_quests_like_cpp(seasonal_quests, false);
    }
    plan.finish(|quest_id| {
        let Some(unique_bits) = unique_bits else {
            return SeasonalQuestBitReset::MissingCatalog;
        };
        let quest_bit = unique_bits.get(&quest_id).copied().unwrap_or(0);
        if quest_bit == 0 {
            return SeasonalQuestBitReset::ZeroUniqueBit;
        }
        if player.set_quest_completed_bit_like_cpp(u32::from(quest_bit), false) {
            SeasonalQuestBitReset::Cleared
        } else {
            SeasonalQuestBitReset::NoChangeOrNoop
        }
    })
}

/// Same set witness as the original fixture, projected from actual Player bits.
fn completed_bits(player: &Player) -> BTreeSet<u16> {
    let mut bits = BTreeSet::new();
    let mut block_index = 0;
    while let Some(block) = player.quest_completed_block_like_cpp(block_index) {
        for bit in 0..u64::BITS {
            if block & (1u64 << bit) != 0 {
                bits.insert((block_index * u64::BITS as usize + bit as usize + 1) as u16);
            }
        }
        block_index += 1;
    }
    bits
}

#[test]
fn reset_seasonal_missing_quest_v2_store_removes_without_inventing_bit_like_cpp() {
    let mut player = Player::new(Some(1), false);
    player
        .gameplay_state_mut()
        .quests
        .seed_seasonal_quest_like_cpp(7, 1001, 99);

    let outcome = reset(&mut player, None, 7, 100);

    assert_eq!(outcome.removed_quest_ids, vec![1001]);
    assert_eq!(outcome.completed_bit_cleared, 0);
    assert_eq!(outcome.completed_bit_skipped_no_quest_v2_store, 1);
    assert_eq!(outcome.completed_bit_clear_unrepresented, 0);
    assert_eq!(
        player.gameplay_state().quests.seasonal_event_quests_like_cpp(7),
        None
    );
    assert!(completed_bits(&player).is_empty());
}


#[test]
fn reset_seasonal_removes_older_than_start_and_erases_emptied_bucket_like_cpp() {
    let mut player = Player::new(Some(1), false);
    player
        .gameplay_state_mut()
        .quests
        .seed_seasonal_quest_like_cpp(7, 1001, 99);
    player
        .gameplay_state_mut()
        .quests
        .set_seasonal_quest_changed_like_cpp(true);

    let outcome = reset(&mut player, None, 7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::RemovedOlderCompletions
    );
    assert_eq!(outcome.removed_quest_ids, vec![1001]);
    assert_eq!(outcome.completed_bit_cleared, 0);
    assert_eq!(outcome.completed_bit_skipped_no_quest_v2_store, 1);
    assert_eq!(outcome.completed_bit_clear_unrepresented, 0);
    assert!(outcome.event_bucket_erased);
    assert_eq!(
        player.gameplay_state().quests.seasonal_event_quests_like_cpp(7),
        None
    );
    assert!(!player.gameplay_state().quests.seasonal_quest_changed_like_cpp());
    assert!(!outcome.seasonal_quest_changed);
}


#[test]
fn reset_seasonal_keeps_equal_and_newer_completions_like_cpp() {
    let mut player = Player::new(Some(1), false);
    player
        .gameplay_state_mut()
        .quests
        .seed_seasonal_quest_like_cpp(7, 1001, 100);
    player
        .gameplay_state_mut()
        .quests
        .seed_seasonal_quest_like_cpp(7, 1002, 101);
    let unique_bits = BTreeMap::from([(1001, 65), (1002, 66)]);
    assert!(player.set_quest_completed_bit_like_cpp(65, true));
    assert!(player.set_quest_completed_bit_like_cpp(66, true));
    player
        .gameplay_state_mut()
        .quests
        .set_seasonal_quest_changed_like_cpp(true);

    let outcome = reset(&mut player, Some(&unique_bits), 7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::NoOlderCompletions
    );
    assert!(outcome.removed_quest_ids.is_empty());
    assert_eq!(outcome.completed_bit_cleared, 0);
    let bucket = player.gameplay_state().quests.seasonal_event_quests_like_cpp(7)
        .expect("bucket kept");
    assert_eq!(bucket.get(&1001), Some(&100));
    assert_eq!(bucket.get(&1002), Some(&101));
    assert_eq!(completed_bits(&player), BTreeSet::from([65, 66]));
    assert!(!player.gameplay_state().quests.seasonal_quest_changed_like_cpp());
}


#[test]
fn reset_seasonal_zero_or_missing_unique_bit_removes_without_inventing_bit_like_cpp() {
    let mut player = Player::new(Some(1), false);
    player
        .gameplay_state_mut()
        .quests
        .seed_seasonal_quest_like_cpp(7, 1001, 99);
    player
        .gameplay_state_mut()
        .quests
        .seed_seasonal_quest_like_cpp(7, 1002, 98);
    let unique_bits = BTreeMap::from([(1001, 0)]);

    let outcome = reset(&mut player, Some(&unique_bits), 7, 100);

    assert_eq!(outcome.removed_quest_ids, vec![1001, 1002]);
    assert_eq!(outcome.completed_bit_cleared, 0);
    assert_eq!(outcome.completed_bit_skipped_zero_unique_bit, 2);
    assert_eq!(outcome.completed_bit_skipped_no_quest_v2_store, 0);
    assert_eq!(outcome.completed_bit_clear_unrepresented, 0);
    assert_eq!(
        player.gameplay_state().quests.seasonal_event_quests_like_cpp(7),
        None
    );
    assert!(completed_bits(&player).is_empty());
}


#[test]
fn reset_seasonal_missing_event_leaves_changed_false_like_cpp() {
    let mut player = Player::new(Some(1), false);
    player
        .gameplay_state_mut()
        .quests
        .set_seasonal_quest_changed_like_cpp(true);

    let outcome = reset(&mut player, None, 7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::MissingEvent
    );
    assert!(outcome.removed_quest_ids.is_empty());
    assert!(!player.gameplay_state().quests.seasonal_quest_changed_like_cpp());
    assert!(!outcome.seasonal_quest_changed);
}


#[test]
fn reset_seasonal_preexisting_empty_bucket_is_preserved_like_cpp() {
    let mut player = Player::new(Some(1), false);
    player
        .gameplay_state_mut()
        .quests
        .ensure_seasonal_event_like_cpp(7);
    player
        .gameplay_state_mut()
        .quests
        .set_seasonal_quest_changed_like_cpp(true);

    let outcome = reset(&mut player, None, 7, 100);

    assert_eq!(
        outcome.reason,
        ResetSeasonalQuestStatusReasonLikeCpp::EmptyEvent
    );
    assert!(outcome.removed_quest_ids.is_empty());
    assert!(player.gameplay_state().quests.seasonal_event_quests_like_cpp(7).is_some());
    assert!(!player.gameplay_state().quests.seasonal_quest_changed_like_cpp());
}
