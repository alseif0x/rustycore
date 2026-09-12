//! #781 — the canonical Player owns its world-local state.
//!
//! C++ writes these on the Player as it moves: `Player::UpdateZone` and
//! `Player::UpdateArea` (`Entities/Player/Player.cpp`) store the zone and
//! area, `Player::UpdatePvPState` maintains `pvpInfo.IsHostile` and
//! `pvpInfo.EndTimer`, `Player::UpdateContestedPvP` maintains
//! `m_contestedPvPTimer`, and `WorldObject::IsOutdoors()` answers from the
//! terrain the position refresh established.

use crate::PlayerWorldLocalState;

#[test]
fn a_fresh_world_local_state_has_no_established_location() {
    let state = PlayerWorldLocalState::default();

    assert_eq!(state.zone_area_like_cpp(), (0, 0));
    assert!(!state.has_zone_area_authority_like_cpp());
    assert_eq!(state.is_outdoors_like_cpp(), None);
    assert!(!state.is_pvp_hostile_like_cpp());
    assert_eq!(state.pvp_end_timer_like_cpp(), None);
    assert_eq!(state.contested_pvp_timer_like_cpp(), 0);
}

#[test]
fn a_new_zone_drops_the_authority_that_vouched_for_the_old_pair() {
    let mut state = PlayerWorldLocalState::default();
    state.set_zone_area_like_cpp(1, 12);
    state.set_zone_area_authority_like_cpp(true);

    state.set_zone_id_like_cpp(2);

    assert_eq!(state.zone_id_like_cpp(), 2);
    assert!(!state.has_zone_area_authority_like_cpp());
}

#[test]
fn a_new_area_drops_the_authority_too() {
    let mut state = PlayerWorldLocalState::default();
    state.set_zone_area_like_cpp(1, 12);
    state.set_zone_area_authority_like_cpp(true);

    state.set_area_id_like_cpp(13);

    assert_eq!(state.area_id_like_cpp(), 13);
    assert!(!state.has_zone_area_authority_like_cpp());
}

#[test]
fn rewriting_the_same_zone_or_area_keeps_the_authority() {
    let mut state = PlayerWorldLocalState::default();
    state.set_zone_area_like_cpp(1, 12);
    state.set_zone_area_authority_like_cpp(true);

    state.set_zone_id_like_cpp(1);
    state.set_area_id_like_cpp(12);
    state.set_zone_area_like_cpp(1, 12);

    assert!(state.has_zone_area_authority_like_cpp());
}

#[test]
fn installing_a_resolved_pair_drops_the_authority_when_either_half_changed() {
    let mut state = PlayerWorldLocalState::default();
    state.set_zone_area_like_cpp(1, 12);
    state.set_zone_area_authority_like_cpp(true);

    state.set_zone_area_like_cpp(1, 13);

    assert_eq!(state.zone_area_like_cpp(), (1, 13));
    assert!(!state.has_zone_area_authority_like_cpp());
}

#[test]
fn terrain_authority_is_recorded_on_its_own() {
    let mut state = PlayerWorldLocalState::default();
    state.set_zone_area_like_cpp(1, 12);

    state.set_zone_area_authority_like_cpp(true);
    assert!(state.has_zone_area_authority_like_cpp());

    state.set_zone_area_authority_like_cpp(false);
    assert!(!state.has_zone_area_authority_like_cpp());
    assert_eq!(state.zone_area_like_cpp(), (1, 12));
}

#[test]
fn the_pvp_facts_are_independent_of_the_location_like_cpp() {
    let mut state = PlayerWorldLocalState::default();
    state.set_pvp_hostile_like_cpp(true);
    state.set_pvp_end_timer_like_cpp(Some(123));
    state.set_contested_pvp_timer_like_cpp(456);

    state.set_zone_id_like_cpp(2);

    assert!(state.is_pvp_hostile_like_cpp());
    assert_eq!(state.pvp_end_timer_like_cpp(), Some(123));
    assert_eq!(state.contested_pvp_timer_like_cpp(), 456);
}

#[test]
fn the_pvp_timer_clears_to_the_cpp_zero() {
    let mut state = PlayerWorldLocalState::default();
    state.set_pvp_end_timer_like_cpp(Some(123));

    state.set_pvp_end_timer_like_cpp(None);

    assert_eq!(state.pvp_end_timer_like_cpp(), None);
}

#[test]
fn the_outdoors_value_keeps_its_three_states() {
    let mut state = PlayerWorldLocalState::default();
    assert_eq!(state.is_outdoors_like_cpp(), None);

    state.set_is_outdoors_like_cpp(Some(true));
    assert_eq!(state.is_outdoors_like_cpp(), Some(true));

    state.set_is_outdoors_like_cpp(Some(false));
    assert_eq!(state.is_outdoors_like_cpp(), Some(false));

    state.set_is_outdoors_like_cpp(None);
    assert_eq!(state.is_outdoors_like_cpp(), None);
}

#[test]
fn the_represented_parts_rebuild_the_state_the_mirror_holds() {
    let state = PlayerWorldLocalState::from_represented_parts_like_cpp(
        900,
        901,
        true,
        true,
        Some(123),
        456,
        Some(true),
    );

    assert_eq!(state.zone_area_like_cpp(), (900, 901));
    assert!(state.has_zone_area_authority_like_cpp());
    assert!(state.is_pvp_hostile_like_cpp());
    assert_eq!(state.pvp_end_timer_like_cpp(), Some(123));
    assert_eq!(state.contested_pvp_timer_like_cpp(), 456);
    assert_eq!(state.is_outdoors_like_cpp(), Some(true));
}
