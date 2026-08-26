// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Semantic input contract for the Player character save.
//!
//! The Session snapshots represented Player groups without naming SQL or
//! prepared statements. The MariaDB adapter test owns the frozen exact SQL
//! order in `player-save-plan-order.json`.

use super::*;

fn populate(session: &mut WorldSession, counter: i64) {
    session.set_player_guid(Some(ObjectGuid::create_player(1, counter)));
    session.set_state(SessionState::LoggedIn);
    session.set_player_map_position_like_cpp(
        0,
        Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: 0.5,
        },
    );
    session.mark_represented_glyphs_loaded_like_cpp();
    session.tutorials_changed_like_cpp = true;
    session.tutorials_loaded_coherently_like_cpp = true;
}

fn request(session: &mut WorldSession) -> wow_persistence::PlayerCharacterSaveRequestLikeCpp {
    let snapshot = session
        .sync_session_from_save_to_db_snapshot_like_cpp()
        .expect("a selected character yields a save snapshot");
    session
        .current_player_character_save_request_like_cpp(&snapshot, 1_700_000_000)
        .expect("a determinate session yields a semantic save request")
}

#[test]
fn the_character_save_request_is_semantic_and_deterministic_like_cpp() {
    let (mut session, _, _) = make_session();
    populate(&mut session, 0x8100_0001);
    let first = request(&mut session);

    assert_eq!(first.player_guid, 0x8100_0001);
    assert_eq!(first.character.position.x, 1.0);
    assert_eq!(first.character.position.orientation, 0.5);
    assert_eq!(first.glyphs.as_ref().map(Vec::len), Some(24));
    assert!(first.tutorials.is_some());

    let (mut twin, _, _) = make_session();
    populate(&mut twin, 0x8100_0001);
    assert_eq!(first, request(&mut twin));
    assert_eq!(first, request(&mut session));
}
