//! One canonical mutation/read for the Player difficulty preference family.

use super::*;

#[test]
fn difficulty_mutation_runs_once_on_active_and_detached_owner_before_save_projection() {
    let (mut session, _, send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        assert!(session.replace_player_difficulty_preferences_like_cpp(2, 15, 4));
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            session.mutate_player_difficulty_preferences_like_cpp(|dungeon, raid, legacy| {
                calls.set(calls.get() + 1);
                assert!(matches!(
                    manager.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                let before = (*dungeon, *raid, *legacy);
                *raid = 14;
                before
            }),
            Some((2, 15, 4))
        );
        assert_eq!(calls.get(), 1);
        assert!(manager.try_lock().is_ok());
        assert_eq!(
            session.player_difficulty_preferences_snapshot_like_cpp(),
            Some((2, 14, 4))
        );
        let snapshot = session
            .current_player_save_to_db_snapshot_like_cpp()
            .unwrap();
        let request = session
            .current_player_character_save_request_like_cpp(&snapshot, 123)
            .unwrap();
        assert_eq!(
            (
                request.character.dungeon_difficulty,
                request.character.raid_difficulty,
                request.character.legacy_raid_difficulty
            ),
            (2, 14, 4)
        );
        assert!(
            send_rx.is_empty(),
            "the mutation itself does not publish difficulty packets"
        );
    }
}

#[test]
fn difficulty_mutation_does_not_run_for_stale_or_missing_owner() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.replace_difficulty_preferences_like_cpp(9, 8, 7);
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    assert_eq!(
        session.mutate_player_difficulty_preferences_like_cpp(|_, _, _| panic!("stale owner")),
        None::<()>
    );
    session.canonical_map_manager = None;
    assert_eq!(
        session.mutate_player_difficulty_preferences_like_cpp(|_, _, _| panic!("missing owner")),
        None::<()>
    );
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, Player::difficulty_preferences_like_cpp),
        Some((9, 8, 7))
    );
}
