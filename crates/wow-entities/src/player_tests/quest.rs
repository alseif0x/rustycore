//! Quest scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn quest_completed_bit_zero_does_not_mutate_or_mark_mask_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    assert!(!player.set_quest_completed_bit_like_cpp(0, true));
    assert_eq!(player.quest_completed_block_like_cpp(0), Some(0));
    assert!(!player.active_player_data_changes_mask().is_any_set());
}
#[test]
fn quest_completed_bit_one_sets_block_zero_and_marks_parent_and_child_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    assert!(player.set_quest_completed_bit_like_cpp(1, true));
    assert_eq!(player.quest_completed_block_like_cpp(0), Some(1));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT)
    );
}
#[test]
fn quest_completed_boundary_bits_map_to_cpp_blocks_and_children() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    assert!(player.set_quest_completed_bit_like_cpp(64, true));
    assert_eq!(player.quest_completed_block_like_cpp(0), Some(1u64 << 63));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT)
    );

    player.clear_data_changes();
    assert!(player.set_quest_completed_bit_like_cpp(65, true));
    assert_eq!(player.quest_completed_block_like_cpp(1), Some(1));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT + 1)
    );
}
#[test]
fn quest_completed_clear_removes_only_requested_bit_and_marks_changed_block() {
    let mut player = Player::new(None, false);
    assert!(player.set_quest_completed_bit_like_cpp(1, true));
    assert!(player.set_quest_completed_bit_like_cpp(2, true));
    player.clear_data_changes();

    assert!(player.set_quest_completed_bit_like_cpp(2, false));
    assert_eq!(player.quest_completed_block_like_cpp(0), Some(1));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT)
    );
}
#[test]
fn quest_completed_out_of_range_does_not_mutate_or_mark_mask_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    let out_of_range = (QUESTS_COMPLETED_BITS_SIZE as u32 * QUESTS_COMPLETED_BITS_PER_BLOCK) + 1;
    assert!(!player.set_quest_completed_bit_like_cpp(out_of_range, true));
    assert!(
        player
            .active_data()
            .quest_completed
            .iter()
            .all(|block| *block == 0)
    );
    assert!(!player.active_player_data_changes_mask().is_any_set());
}
#[test]
fn quest_completed_repeated_set_and_clear_without_change_do_not_mark_mask_like_cpp() {
    let mut player = Player::new(None, false);
    assert!(player.set_quest_completed_bit_like_cpp(65, true));
    player.clear_data_changes();

    assert!(!player.set_quest_completed_bit_like_cpp(65, true));
    assert_eq!(player.quest_completed_block_like_cpp(1), Some(1));
    assert!(!player.active_player_data_changes_mask().is_any_set());

    assert!(player.set_quest_completed_bit_like_cpp(65, false));
    player.clear_data_changes();

    assert!(!player.set_quest_completed_bit_like_cpp(65, false));
    assert_eq!(player.quest_completed_block_like_cpp(1), Some(0));
    assert!(!player.active_player_data_changes_mask().is_any_set());
}
