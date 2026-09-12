//! #777 — the canonical Player owns its cinematic and movie state.
//!
//! C++ splits these between the Player and the `CinematicMgr` it owns:
//! `CinematicMgr::BeginCinematic` (`Entities/Player/CinematicMgr.h:39`),
//! `NextCinematicCamera` (`CinematicMgr.cpp:46`) and `EndCinematic` (`:83`),
//! with `Player::SendCinematicStart` and `Player::SendMovieStart`
//! (`Player.cpp`) sending the packets around them.

use crate::PlayerCinematicStateLikeCpp;

const CAMERAS: [u16; 8] = [11, 12, 13, 0, 0, 0, 0, 0];

#[test]
fn a_fresh_state_plays_nothing_and_sits_before_the_first_camera_like_cpp() {
    let state = PlayerCinematicStateLikeCpp::default();

    assert_eq!(state.cinematic_id_like_cpp(), None);
    assert_eq!(state.camera_ids_like_cpp(), None);
    assert_eq!(state.camera_index_like_cpp(), -1);
    assert_eq!(state.movie_id_like_cpp(), None);
}

#[test]
fn beginning_a_cinematic_stores_it_with_its_cameras_like_cpp() {
    let mut state = PlayerCinematicStateLikeCpp::default();

    state.begin_cinematic_like_cpp(444, CAMERAS);

    assert_eq!(state.cinematic_id_like_cpp(), Some(444));
    assert_eq!(state.camera_ids_like_cpp(), Some(CAMERAS));
    assert_eq!(state.camera_index_like_cpp(), -1);
}

#[test]
fn a_new_cinematic_never_continues_the_previous_camera_walk_like_cpp() {
    let mut state = PlayerCinematicStateLikeCpp::default();
    state.begin_cinematic_like_cpp(444, CAMERAS);
    assert_eq!(state.next_cinematic_camera_like_cpp(), Some(11));

    state.begin_cinematic_like_cpp(555, [21, 22, 0, 0, 0, 0, 0, 0]);

    assert_eq!(state.camera_index_like_cpp(), -1);
    assert_eq!(state.next_cinematic_camera_like_cpp(), Some(21));
}

#[test]
fn the_camera_walk_follows_the_stored_list_in_order_like_cpp() {
    let mut state = PlayerCinematicStateLikeCpp::default();
    state.begin_cinematic_like_cpp(444, CAMERAS);

    assert_eq!(state.next_cinematic_camera_like_cpp(), Some(11));
    assert_eq!(state.next_cinematic_camera_like_cpp(), Some(12));
    assert_eq!(state.next_cinematic_camera_like_cpp(), Some(13));
    assert_eq!(state.camera_index_like_cpp(), 2);
}

#[test]
fn the_walk_refuses_the_out_of_bounds_edge_cpp_does_not_guard() {
    let mut state = PlayerCinematicStateLikeCpp::default();
    state.begin_cinematic_like_cpp(444, CAMERAS);
    for _ in 0..8 {
        state.next_cinematic_camera_like_cpp();
    }

    assert_eq!(state.camera_index_like_cpp(), 7);
    assert_eq!(state.next_cinematic_camera_like_cpp(), None);
    assert_eq!(state.next_cinematic_camera_like_cpp(), None);
}

#[test]
fn no_cinematic_and_no_cameras_both_answer_nothing_like_cpp() {
    let mut without_cinematic = PlayerCinematicStateLikeCpp::default();
    assert_eq!(without_cinematic.next_cinematic_camera_like_cpp(), None);

    let mut ended = PlayerCinematicStateLikeCpp::default();
    ended.begin_cinematic_like_cpp(444, CAMERAS);
    ended.end_cinematic_like_cpp();
    assert_eq!(ended.next_cinematic_camera_like_cpp(), None);
}

#[test]
fn ending_a_cinematic_answers_it_and_drops_its_cameras_like_cpp() {
    let mut state = PlayerCinematicStateLikeCpp::default();
    state.begin_cinematic_like_cpp(444, CAMERAS);
    state.next_cinematic_camera_like_cpp();

    let ended = state.end_cinematic_like_cpp();

    assert_eq!(ended, Some(444));
    assert_eq!(state.cinematic_id_like_cpp(), None);
    assert_eq!(state.camera_ids_like_cpp(), None);
    assert_eq!(state.camera_index_like_cpp(), -1);
}

#[test]
fn ending_twice_reports_the_cinematic_once_like_cpp() {
    let mut state = PlayerCinematicStateLikeCpp::default();
    state.begin_cinematic_like_cpp(444, CAMERAS);

    assert_eq!(state.end_cinematic_like_cpp(), Some(444));
    assert_eq!(state.end_cinematic_like_cpp(), None);
}

#[test]
fn the_movie_is_independent_of_the_cinematic_like_cpp() {
    let mut state = PlayerCinematicStateLikeCpp::default();
    state.begin_cinematic_like_cpp(444, CAMERAS);
    state.set_movie_like_cpp(Some(901));

    assert_eq!(state.end_cinematic_like_cpp(), Some(444));
    assert_eq!(state.movie_id_like_cpp(), Some(901));

    assert_eq!(state.take_movie_like_cpp(), Some(901));
    assert_eq!(state.take_movie_like_cpp(), None);
}

#[test]
fn clearing_the_movie_through_the_setter_leaves_nothing_to_take() {
    let mut state = PlayerCinematicStateLikeCpp::default();
    state.set_movie_like_cpp(Some(901));

    state.set_movie_like_cpp(None);

    assert_eq!(state.movie_id_like_cpp(), None);
    assert_eq!(state.take_movie_like_cpp(), None);
}
