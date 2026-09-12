//! #765 — the canonical Player owns its taxi state and its invariants.
//!
//! C++ keeps the route and the node mask in `PlayerTaxi`
//! (`Entities/Player/PlayerTaxi.h`): `IsTaximaskNodeKnown` (`:44`),
//! `SetTaximaskNode` (`:50`), `ClearTaxiDestinations` (`:67`),
//! `AddTaxiDestination` (`:68`), `GetTaxiSource` (`:69`),
//! `GetTaxiDestination` (`:70`), `NextTaxiDestination` (`:74`) and `empty`
//! (`:80`), and clears them together in `Player::CleanupAfterTaxiFlight`
//! (`Player.cpp:22019`).

use crate::{PlayerTaxiFlightNodeLikeCpp, PlayerTaxiState};

const TAXI_UNIT_FLAGS_LIKE_CPP: u32 = 0b0000_0110;

fn node(map_id: u16, teleport_flag: bool) -> PlayerTaxiFlightNodeLikeCpp {
    PlayerTaxiFlightNodeLikeCpp {
        map_id,
        position: wow_core::Position::new(1.0, 2.0, 3.0, 0.0),
        teleport_flag,
    }
}

#[test]
fn a_fresh_taxi_state_has_no_route_flight_or_known_node_like_cpp() {
    let state = PlayerTaxiState::default();

    assert!(state.destinations_like_cpp().is_empty());
    assert_eq!(state.taxi_destination_like_cpp(), None);
    assert!(!state.is_in_flight_like_cpp());
    assert!(!state.is_taximask_node_known_like_cpp(1));
    assert!(state.known_node_mask_like_cpp().is_empty());
    assert_eq!(state.known_node_mask_text_like_cpp(), None);
}

#[test]
fn the_destination_is_the_node_after_the_source_like_cpp() {
    let one_node = PlayerTaxiState::from_represented_parts_like_cpp(vec![10], None, 0, false);
    assert_eq!(one_node.taxi_destination_like_cpp(), None);

    let route = PlayerTaxiState::from_represented_parts_like_cpp(vec![10, 20, 30], None, 0, false);
    assert_eq!(route.taxi_destination_like_cpp(), Some(20));
    assert_eq!(route.destinations_like_cpp(), [10, 20, 30]);
}

#[test]
fn installing_a_route_replaces_the_previous_one_and_leaves_the_mask_alone_like_cpp() {
    let mut state = PlayerTaxiState::from_represented_parts_like_cpp(vec![10, 20], None, 0, false);
    state.set_taximask_node_like_cpp(9);

    state.replace_destinations_like_cpp(vec![30]);

    assert_eq!(state.destinations_like_cpp(), [30]);
    assert!(state.is_taximask_node_known_like_cpp(9));
}

#[test]
fn taximask_bits_follow_the_cpp_field_and_submask_arithmetic() {
    let mut state = PlayerTaxiState::default();

    assert!(state.set_taximask_node_like_cpp(1));
    assert!(state.set_taximask_node_like_cpp(8));
    assert!(state.set_taximask_node_like_cpp(9));

    assert_eq!(state.known_node_mask_like_cpp(), [0b1000_0001, 0b0000_0001]);
    assert!(state.is_taximask_node_known_like_cpp(1));
    assert!(state.is_taximask_node_known_like_cpp(8));
    assert!(state.is_taximask_node_known_like_cpp(9));
    assert!(!state.is_taximask_node_known_like_cpp(2));
    assert!(!state.is_taximask_node_known_like_cpp(10));
}

#[test]
fn an_already_known_node_is_not_learned_again_like_cpp() {
    let mut state = PlayerTaxiState::default();

    assert!(state.set_taximask_node_like_cpp(5));
    assert!(!state.set_taximask_node_like_cpp(5));
    assert_eq!(state.known_node_mask_like_cpp(), [0b0001_0000]);
}

#[test]
fn node_zero_is_outside_the_one_based_cpp_mask_and_is_refused() {
    let mut state = PlayerTaxiState::default();

    assert!(!state.is_taximask_node_known_like_cpp(0));
    assert!(!state.set_taximask_node_like_cpp(0));
    assert!(state.known_node_mask_like_cpp().is_empty());
}

#[test]
fn a_loaded_mask_replaces_the_previous_one_with_its_text_like_cpp() {
    let mut state = PlayerTaxiState::default();
    state.set_taximask_node_like_cpp(1);

    state.load_taxi_mask_like_cpp(vec![0b0000_0010], Some("2".to_string()));

    assert!(!state.is_taximask_node_known_like_cpp(1));
    assert!(state.is_taximask_node_known_like_cpp(2));
    assert_eq!(state.known_node_mask_text_like_cpp(), Some("2"));
}

#[test]
fn beginning_a_flight_records_the_current_node_and_the_node_after_a_teleport() {
    let mut state = PlayerTaxiState::default();

    state.begin_taxi_flight_like_cpp(node(571, true), Some(node(0, false)));

    let flight = state.flight_like_cpp().expect("flight");
    assert!(state.is_in_flight_like_cpp());
    assert!(flight.current_node.teleport_flag);
    assert_eq!(flight.node_after_teleport.map(|n| n.map_id), Some(0));
}

#[test]
fn advancing_past_a_teleport_promotes_the_following_node_exactly_once() {
    let mut state = PlayerTaxiState::default();
    state.begin_taxi_flight_like_cpp(node(571, true), Some(node(0, false)));

    let taken = state
        .advance_taxi_flight_after_teleport_like_cpp()
        .expect("node after teleport");

    assert_eq!(taken.map_id, 0);
    let flight = state.flight_like_cpp().expect("flight");
    assert_eq!(flight.current_node.map_id, 0);
    assert_eq!(flight.node_after_teleport, None);
    assert_eq!(state.advance_taxi_flight_after_teleport_like_cpp(), None);
}

#[test]
fn advancing_without_a_flight_changes_nothing() {
    let mut state = PlayerTaxiState::default();

    assert_eq!(state.advance_taxi_flight_after_teleport_like_cpp(), None);
    assert!(!state.is_in_flight_like_cpp());
}

#[test]
fn the_landing_cleanup_clears_route_flight_and_mount_like_cpp() {
    let mut state = PlayerTaxiState::from_represented_parts_like_cpp(
        vec![10, 20],
        None,
        TAXI_UNIT_FLAGS_LIKE_CPP | 0b0001_0000,
        true,
    );
    state.begin_taxi_flight_like_cpp(node(571, false), None);
    state.set_taximask_node_like_cpp(3);

    state.cleanup_after_taxi_flight_like_cpp(TAXI_UNIT_FLAGS_LIKE_CPP);

    assert!(state.destinations_like_cpp().is_empty());
    assert!(!state.is_in_flight_like_cpp());
    assert!(!state.mounted_like_cpp());
    assert_eq!(state.unit_flags_like_cpp(), 0b0001_0000);
    assert!(state.is_taximask_node_known_like_cpp(3));
}

#[test]
fn the_cleanup_state_mirror_moves_both_values_together_like_cpp() {
    let mut state = PlayerTaxiState::default();

    state.set_taxi_cleanup_state_like_cpp(TAXI_UNIT_FLAGS_LIKE_CPP, true);

    assert_eq!(state.unit_flags_like_cpp(), TAXI_UNIT_FLAGS_LIKE_CPP);
    assert!(state.mounted_like_cpp());
}

#[test]
fn the_represented_parts_build_the_state_the_owner_receives() {
    let state = PlayerTaxiState::from_represented_parts_like_cpp(
        vec![10, 20],
        Some(crate::PlayerTaxiFlightStateLikeCpp {
            current_node: node(571, false),
            node_after_teleport: None,
        }),
        TAXI_UNIT_FLAGS_LIKE_CPP,
        true,
    );

    assert_eq!(state.destinations_like_cpp(), [10, 20]);
    assert_eq!(state.taxi_destination_like_cpp(), Some(20));
    assert!(state.is_in_flight_like_cpp());
    assert_eq!(state.unit_flags_like_cpp(), TAXI_UNIT_FLAGS_LIKE_CPP);
    assert!(state.mounted_like_cpp());
    assert!(state.known_node_mask_like_cpp().is_empty());
}
