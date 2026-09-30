// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[test]
fn rest_accrual_preserves_guards_timer_boundary_and_computed_extra_return() {
    let mut player = Player::new(None, false);
    player.set_next_level_xp(72_000);
    player.load_xp_rest_bonus_like_cpp(6, 0.5);
    let before = player.rest_state_like_cpp().clone();
    for (logout, now) in [(0, 100), (100, 99), (100, 100)] {
        assert_eq!(
            player.apply_offline_xp_rest_bonus_like_cpp(logout, now, 0.125, false, false),
            0.0
        );
        assert_eq!(player.rest_state_like_cpp(), &before);
    }
    player.gameplay_state_mut().rest.rest_time_secs = 100;
    for now in [99, 100, 109] {
        assert_eq!(
            player.update_online_xp_rest_bonus_like_cpp(now, 0.125, false, false),
            (0.0, 0)
        );
        assert_eq!(player.rest_state_like_cpp().rest_time_secs, 100);
    }
    assert_eq!(
        player.update_online_xp_rest_bonus_like_cpp(110, 0.125, false, false),
        (1.25, 7)
    );
    assert_eq!(player.rest_state_like_cpp().rest_time_secs, 110);
    assert_eq!(
        player.update_online_xp_rest_bonus_like_cpp(110, 0.125, false, false),
        (0.0, 0)
    );
    assert_eq!(
        player.apply_offline_xp_rest_bonus_like_cpp(1, 1_000_001, 0.125, false, false),
        125_000.0
    );
    assert_eq!(player.rest_state_like_cpp().rest_bonus, 54_000.0);
    assert_eq!(
        player.update_online_xp_rest_bonus_like_cpp(120, 0.125, true, false),
        (0.0, 7)
    );
    assert_eq!(player.rest_state_like_cpp().rest_time_secs, 120);
    assert_eq!(player.rest_state_like_cpp().rest_bonus, 0.0);
}

#[test]
fn rest_consumption_preserves_integer_percentage_bounds_and_zero_award_normalization() {
    for (xp, pct, remaining) in [
        (40, 50, 10.0),
        (40, 0, 30.0),
        (40, -50, 50.0),
        (40, -200, 70.0),
        (3, -50, 68.0),
        (40, i32::MAX, 0.0),
        (40, i32::MIN, 70.0),
        (100, 0, 0.0),
    ] {
        let mut player = Player::new(None, false);
        player.set_next_level_xp(1000);
        player.load_xp_rest_bonus_like_cpp(1, 70.0);
        let (award, mask) = player.take_xp_rest_bonus_like_cpp(xp, pct, false, false);
        assert_eq!(award, xp.min(70));
        assert_eq!(player.rest_state_like_cpp().rest_bonus, remaining);
        assert_eq!(mask, if remaining == 70.0 { 0 } else { 7 });
    }
    let mut player = Player::new(None, false);
    player.set_next_level_xp(1000);
    player.load_xp_rest_bonus_like_cpp(6, 0.5);
    assert_eq!(
        player.take_xp_rest_bonus_like_cpp(10, 50, false, false),
        (0, 7)
    );
    assert_eq!(player.rest_state_like_cpp().rest_bonus, 0.5);
    assert_eq!(player.rest_state_like_cpp().rest_state, 2);
}

#[test]
fn rest_bonus_preserves_max_level_raf_priority_and_fractional_no_change_mask() {
    let mut player = Player::new(None, false);
    player.set_next_level_xp(100);
    player.load_xp_rest_bonus_like_cpp(2, 0.5);
    assert_eq!(player.set_xp_rest_bonus_like_cpp(0.9, false, false), 0);
    assert_eq!(player.rest_state_like_cpp().rest_bonus, 0.9);
    assert_eq!(player.set_xp_rest_bonus_like_cpp(200.0, false, false), 7);
    assert_eq!(player.rest_state_like_cpp().rest_bonus, 75.0);
    assert_eq!(player.rest_state_like_cpp().rest_state, 1);
    assert_eq!(player.set_xp_rest_bonus_like_cpp(200.0, true, true), 7);
    assert_eq!(player.rest_state_like_cpp().rest_bonus, 0.0);
    assert_eq!(player.rest_state_like_cpp().rest_state, 6);
    assert_eq!(player.set_xp_rest_bonus_like_cpp(200.0, true, false), 7);
    assert_eq!(player.rest_state_like_cpp().rest_state, 2);
}

#[test]
fn rest_flags_only_start_and_stop_time_at_zero_crossings() {
    for deferred in [false, true] {
        let mut state = PlayerRestState {
            defer_flag_sync: deferred,
            ..Default::default()
        };
        assert!(!state.set_flag_like_cpp(0, 0, || panic!("empty mask reads no clock")));
        assert!(state.location_initialized);
        let calls = std::cell::Cell::new(0);
        assert!(state.set_flag_like_cpp(1, 77, || {
            calls.set(calls.get() + 1);
            100
        }));
        assert_eq!(calls.get(), 1);
        assert_eq!(state.rest_time_secs, 100);
        assert_eq!(state.inn_area_trigger_id, 77);
        assert_eq!(state.deferred_flag_update_dirty, deferred);
        state.deferred_flag_update_dirty = false;
        assert!(!state.set_flag_like_cpp(1, 88, || panic!("repeat reads no clock")));
        assert!(!state.set_flag_like_cpp(2, 0, || panic!("second flag reads no clock")));
        assert_eq!(state.inn_area_trigger_id, 88);
        assert!(!state.deferred_flag_update_dirty);
        assert!(!state.remove_flag_like_cpp(1));
        assert_eq!(state.inn_area_trigger_id, 0);
        assert_eq!(state.rest_time_secs, 100);
        assert_eq!(state.rest_flag_mask, 2);
        assert!(!state.deferred_flag_update_dirty);
        assert!(state.remove_flag_like_cpp(2));
        assert_eq!(state.rest_time_secs, 0);
        assert_eq!(state.rest_flag_mask, 0);
        assert_eq!(state.deferred_flag_update_dirty, deferred);
        state.deferred_flag_update_dirty = false;
        assert!(!state.remove_flag_like_cpp(2));
        assert!(!state.deferred_flag_update_dirty);
    }
}

#[test]
fn absent_tavern_removal_preserves_uninitialized_location_and_other_rest_fields() {
    let mut state = PlayerRestState {
        inn_area_trigger_id: 77,
        rest_bonus: 123.5,
        rest_honor_bonus: 55.0,
        rest_time_secs: 100,
        deferred_flag_update_dirty: true,
        ..Default::default()
    };
    let mut expected = state.clone();
    expected.inn_area_trigger_id = 0;
    assert!(!state.remove_flag_like_cpp(1));
    assert_eq!(state, expected);
}
