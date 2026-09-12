//! #779 — the canonical Player owns its rest state and its invariants.
//!
//! C++ keeps this on the `RestMgr` the Player owns
//! (`Entities/Player/RestMgr.h`): `SetRestBonus` (`:67`), `HasRestFlag`
//! (`:70`), `SetRestFlag` (`:71`), `RemoveRestFlag` (`:72`) and
//! `GetInnTriggerID` (`:75`) over `_restTime` (`:86`) and `_restFlagMask`
//! (`:89`), with `Player::SetRestState` (`Player.h:2652`) writing the update
//! field.

use crate::PlayerRestState;

const IN_TAVERN: u32 = 0x1;
const IN_CITY: u32 = 0x2;

fn now_1000() -> u64 {
    1000
}

#[test]
fn a_fresh_rest_state_has_no_flags_and_no_location_like_cpp() {
    let state = PlayerRestState::default();

    assert!(!state.is_resting_by_flag_like_cpp());
    assert!(!state.has_rest_flag_like_cpp(IN_TAVERN));
    assert!(!state.is_location_initialized_like_cpp());
    assert_eq!(state.inn_trigger_id_like_cpp(), 0);
    assert_eq!(state.rest_time_secs_like_cpp(), 0);
}

#[test]
fn the_rest_clock_starts_only_when_the_first_flag_arrives_like_cpp_set_rest_flag() {
    let mut state = PlayerRestState::default();

    assert!(state.set_flag_like_cpp(IN_CITY, 0, now_1000));
    assert_eq!(state.rest_time_secs_like_cpp(), 1000);
    assert!(state.is_location_initialized_like_cpp());

    assert!(!state.set_flag_like_cpp(IN_TAVERN, 0, || 2000));
    assert_eq!(state.rest_time_secs_like_cpp(), 1000);
}

#[test]
fn the_inn_trigger_is_recorded_only_when_one_is_given_like_cpp() {
    let mut state = PlayerRestState::default();
    state.set_flag_like_cpp(IN_TAVERN, 77, now_1000);
    assert_eq!(state.inn_trigger_id_like_cpp(), 77);

    state.set_flag_like_cpp(IN_CITY, 0, now_1000);
    assert_eq!(state.inn_trigger_id_like_cpp(), 77);
}

#[test]
fn leaving_the_tavern_drops_its_trigger_like_cpp_remove_rest_flag() {
    let mut state = PlayerRestState::default();
    state.set_flag_like_cpp(IN_TAVERN, 77, now_1000);
    state.set_flag_like_cpp(IN_CITY, 0, now_1000);

    assert!(!state.remove_flag_like_cpp(IN_TAVERN));

    assert_eq!(state.inn_trigger_id_like_cpp(), 0);
    assert!(state.has_rest_flag_like_cpp(IN_CITY));
    assert_eq!(state.rest_time_secs_like_cpp(), 1000);
}

#[test]
fn the_rest_clock_stops_with_the_last_flag_like_cpp() {
    let mut state = PlayerRestState::default();
    state.set_flag_like_cpp(IN_CITY, 0, now_1000);

    assert!(state.remove_flag_like_cpp(IN_CITY));

    assert!(!state.is_resting_by_flag_like_cpp());
    assert_eq!(state.rest_time_secs_like_cpp(), 0);
}

#[test]
fn a_deferred_publication_is_marked_only_while_the_session_holds_it_back() {
    let mut held = PlayerRestState::default();
    held.defer_flag_sync_like_cpp();
    held.set_flag_like_cpp(IN_CITY, 0, now_1000);
    assert!(held.deferred_flag_update_dirty_like_cpp());

    let mut live = PlayerRestState::default();
    live.set_flag_like_cpp(IN_CITY, 0, now_1000);
    assert!(!live.deferred_flag_update_dirty_like_cpp());
}

#[test]
fn ending_the_deferred_sync_answers_the_owed_update_once() {
    let mut state = PlayerRestState::default();
    state.defer_flag_sync_like_cpp();
    state.set_flag_like_cpp(IN_CITY, 0, now_1000);

    assert!(state.end_deferred_flag_sync_like_cpp());
    assert!(!state.defers_flag_sync_like_cpp());
    assert!(state.deferred_flag_update_dirty_like_cpp());

    assert!(state.take_deferred_flag_update_like_cpp());
    assert!(!state.take_deferred_flag_update_like_cpp());
}

#[test]
fn clearing_the_owed_update_leaves_nothing_to_take() {
    let mut state = PlayerRestState::default();
    state.defer_flag_sync_like_cpp();
    state.set_flag_like_cpp(IN_CITY, 0, now_1000);

    state.clear_deferred_flag_update_like_cpp();

    assert!(!state.deferred_flag_update_dirty_like_cpp());
    assert!(!state.take_deferred_flag_update_like_cpp());
}

#[test]
fn the_loaded_rest_values_travel_together_like_cpp() {
    let mut state = PlayerRestState::default();

    state.install_loaded_rest_like_cpp(2, 70.0);

    assert_eq!(state.rest_state_like_cpp(), 2);
    assert_eq!(state.rest_bonus_like_cpp(), 70.0);
}

#[test]
fn resetting_the_location_tracking_keeps_the_loaded_rest_values() {
    let mut state = PlayerRestState::default();
    state.install_loaded_rest_like_cpp(2, 70.0);
    state.set_rest_xp_like_cpp(9);
    state.defer_flag_sync_like_cpp();
    state.set_flag_like_cpp(IN_TAVERN, 77, now_1000);

    state.reset_location_tracking_like_cpp();

    assert!(!state.is_resting_by_flag_like_cpp());
    assert!(!state.is_location_initialized_like_cpp());
    assert!(!state.defers_flag_sync_like_cpp());
    assert!(!state.deferred_flag_update_dirty_like_cpp());
    assert_eq!(state.inn_trigger_id_like_cpp(), 0);
    assert_eq!(state.rest_time_secs_like_cpp(), 0);
    assert_eq!(state.rest_state_like_cpp(), 2);
    assert_eq!(state.rest_bonus_like_cpp(), 70.0);
    assert_eq!(state.rest_xp_like_cpp(), 9);
}

#[test]
fn removing_a_flag_that_was_not_held_changes_nothing_like_cpp() {
    let mut state = PlayerRestState::default();
    state.set_flag_like_cpp(IN_CITY, 0, now_1000);

    assert!(!state.remove_flag_like_cpp(IN_TAVERN));

    assert!(state.has_rest_flag_like_cpp(IN_CITY));
    assert_eq!(state.rest_time_secs_like_cpp(), 1000);
}

#[test]
fn the_logout_record_moves_both_values_together() {
    let mut state = PlayerRestState::default();

    state.set_logout_like_cpp(Some(789), true);

    assert_eq!(state.logout_time_like_cpp(), Some(789));
    assert!(state.logout_was_resting_like_cpp());

    state.set_logout_like_cpp(None, false);
    assert_eq!(state.logout_time_like_cpp(), None);
    assert!(!state.logout_was_resting_like_cpp());
}

#[test]
fn the_represented_parts_rebuild_the_state_the_mirror_holds() {
    let state = PlayerRestState::from_represented_parts_like_cpp(
        2, 70.0, IN_TAVERN, true, true, true, 77, 456,
    );

    assert_eq!(state.rest_state_like_cpp(), 2);
    assert_eq!(state.rest_bonus_like_cpp(), 70.0);
    assert!(state.has_rest_flag_like_cpp(IN_TAVERN));
    assert_eq!(state.rest_flag_mask_like_cpp(), IN_TAVERN);
    assert!(state.is_location_initialized_like_cpp());
    assert!(state.defers_flag_sync_like_cpp());
    assert!(state.deferred_flag_update_dirty_like_cpp());
    assert_eq!(state.inn_trigger_id_like_cpp(), 77);
    assert_eq!(state.rest_time_secs_like_cpp(), 456);
    assert_eq!(state.logout_time_like_cpp(), None);
}
