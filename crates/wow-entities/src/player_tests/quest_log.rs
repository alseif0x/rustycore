// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Native Player QuestLog update-field ownership and nested-mask tests.

use super::*;
use crate::PlayerQuestStatusRecord;

const QUEST_STATE_COMPLETE: u32 = 1;
const QUEST_STATE_FAIL: u32 = 2;

fn quest_status(quest_id: u32, slot: u8) -> PlayerQuestStatusRecord {
    PlayerQuestStatusRecord {
        quest_id,
        status: 1,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: vec![],
        slot,
    }
}

#[test]
fn removing_status_keeps_native_quest_log_until_slot_is_cleared() {
    let mut player = Player::new(None, false);
    player
        .gameplay_state_mut()
        .quests
        .insert_status_like_cpp(700, quest_status(700, 3));
    assert!(player.set_quest_slot_like_cpp(3, 700));
    assert!(player.set_quest_slot_end_time_like_cpp(3, 123));
    assert!(player.set_quest_slot_state_like_cpp(3, QUEST_STATE_COMPLETE));
    assert!(player.set_quest_slot_counter_like_cpp(3, 23, 44));

    assert!(
        player
            .gameplay_state_mut()
            .quests
            .remove_status_like_cpp(700)
            .is_some()
    );
    assert!(
        player
            .gameplay_state()
            .quests
            .statuses_like_cpp()
            .is_empty()
    );
    assert_eq!(player.quest_slot_quest_id_like_cpp(3), Some(700));
    assert_eq!(player.find_quest_slot_like_cpp(700), Some(3));
    let retained = player.quest_log_values_like_cpp()[3];
    assert_eq!(retained.end_time(), 123);
    assert_eq!(retained.state_flags(), QUEST_STATE_COMPLETE);
    assert_eq!(retained.objective_progress()[23], 44);

    assert!(player.set_quest_slot_like_cpp(3, 0));
    assert_eq!(player.quest_slot_quest_id_like_cpp(3), None);
    assert_eq!(player.find_quest_slot_like_cpp(700), None);
    assert_eq!(player.quest_log_values_like_cpp()[3], Default::default());
}

#[test]
fn complete_and_fail_bits_accumulate_and_remove_independently() {
    let mut player = Player::new(None, false);
    assert!(player.set_quest_slot_like_cpp(0, 701));
    assert!(player.set_quest_slot_state_like_cpp(0, QUEST_STATE_COMPLETE));
    assert!(player.set_quest_slot_state_like_cpp(0, QUEST_STATE_FAIL));
    assert_eq!(
        player.quest_log_values_like_cpp()[0].state_flags(),
        QUEST_STATE_COMPLETE | QUEST_STATE_FAIL
    );

    assert!(player.remove_quest_slot_state_like_cpp(0, QUEST_STATE_COMPLETE));
    assert_eq!(
        player.quest_log_values_like_cpp()[0].state_flags(),
        QUEST_STATE_FAIL
    );
}

#[test]
fn native_slot_preserves_signed_and_wrapped_cpp_field_bits() {
    let mut player = Player::new(None, false);
    let quest_id = 0xf123_4567;
    assert!(player.set_quest_slot_like_cpp(2, quest_id));
    assert_eq!(
        player.quest_log_values_like_cpp()[2].quest_id(),
        quest_id as i32
    );
    assert_eq!(player.quest_slot_quest_id_like_cpp(2), Some(quest_id));
    assert_eq!(player.find_quest_slot_like_cpp(quest_id), Some(2));

    assert!(player.set_quest_slot_end_time_like_cpp(2, -1));
    assert!(player.set_quest_slot_counter_like_cpp(2, 0, (-1_i32) as u16));
    assert!(player.set_quest_slot_counter_like_cpp(2, 23, 0x1_0001_u32 as u16));
    let row = player.quest_log_values_like_cpp()[2];
    assert_eq!(row.end_time(), 4_294_967_295);
    assert_eq!(row.objective_progress()[0], u16::MAX);
    assert_eq!(row.objective_progress()[23], 1);
}

#[test]
fn persisted_flag_objective_getter_keeps_the_pinned_raw_index_mask_quirk() {
    let mut player = Player::new(None, false);
    assert!(player.set_quest_slot_like_cpp(0, 703));
    assert!(player.set_quest_slot_state_like_cpp(0, QUEST_STATE_COMPLETE | QUEST_STATE_FAIL));

    // Player.cpp:15929-15948 returns uint8(state & StorageIndex), not the
    // gameplay writer's `256 << StorageIndex` objective bit.
    assert_eq!(
        player.quest_slot_objective_data_for_save_like_cpp(0, 0, true),
        Some(0)
    );
    assert_eq!(
        player.quest_slot_objective_data_for_save_like_cpp(0, 1, true),
        Some(1)
    );
    assert_eq!(
        player.quest_slot_objective_data_for_save_like_cpp(0, 2, true),
        Some(1)
    );
    assert_eq!(
        player.quest_slot_objective_data_for_save_like_cpp(0, 3, true),
        Some(1)
    );
    assert_eq!(
        player.quest_slot_objective_data_for_save_like_cpp(0, -1, true),
        Some(0)
    );
    assert!(player.remove_quest_slot_state_like_cpp(0, QUEST_STATE_COMPLETE | QUEST_STATE_FAIL));
    assert!(player.set_quest_slot_state_like_cpp(0, 1 << 8));
    assert_eq!(
        player.quest_slot_objective_data_for_save_like_cpp(0, 1, true),
        Some(0),
        "the persisted raw-index test ignores the gameplay objective bit at 256 << index"
    );
}

#[test]
fn quest_log_rows_queue_object_updates_and_preserve_exact_nested_masks() {
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().add_to_world();
    assert!(!player.unit().world().object().is_object_updated());

    assert!(player.set_quest_slot_like_cpp(4, 702));
    assert!(player.unit().world().object().is_object_updated());
    let update = player.values_update(false).player_data.unwrap();
    assert!(update.mask.is_set(35));
    assert!(update.mask.is_set(36 + 4));
    assert_eq!(update.quest_log[4].change_mask(), (1 << 29) - 1);

    player.clear_data_changes();
    assert!(!player.unit().world().object().is_object_updated());
    assert!(player.set_quest_slot_counter_like_cpp(4, 7, 0));
    assert!(!player.unit().world().object().is_object_updated());
    assert_eq!(
        player.values_update(false).player_data.unwrap().quest_log[4].change_mask(),
        (1 << 0) | (1 << 4) | (1 << (5 + 7))
    );

    player.clear_data_changes();
    player.set_inebriation_like_cpp(17);
    assert!(player.unit().world().object().is_object_updated());
    assert!(player.set_quest_slot_counter_like_cpp(4, 7, 0));
    let update = player.values_update(false).player_data.unwrap();
    assert_eq!(
        update.quest_log[4].change_mask(),
        (1 << 0) | (1 << 4) | (1 << (5 + 7))
    );
    assert!(player.unit().world().object().is_object_updated());

    player.clear_player_data_changes();
    player.set_inebriation_like_cpp(18);
    let cleared = player.values_update(false).player_data.unwrap();
    assert!(cleared.quest_log.iter().all(|row| row.change_mask() == 0));
}
