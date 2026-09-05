//! Save projection reads one generation-checked Player without publishing state.

use super::*;

#[test]
fn save_snapshot_reads_active_and_detached_owner_without_changing_state() {
    let (mut session, _, send_rx) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 7);
    session.current_map_id = 571;
    session.player_level = 17;
    session.set_loaded_player_powers_like_cpp([111, 222, 0, 0, 0, 0, 0, 0, 0, 0]);
    let position = Position::new(1.0, 2.0, 3.0, 0.5);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().world_mut().relocate(position);
            player.unit_mut().set_level(60);
            player.unit_mut().set_max_health(900);
            player.unit_mut().set_health(456);
            player.set_xp(1234);
            player.set_money(5678);
            player.clear_data_changes();
        })
        .unwrap();
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let snapshot = session
            .current_player_save_to_db_snapshot_like_cpp()
            .unwrap();
        assert_eq!(
            snapshot,
            PlayerSaveToDbSnapshotLikeCpp {
                guid,
                map_id: 571,
                instance_id: if detached { 0 } else { 7 },
                position,
                level: 17, // Session identity staging is separate remaining ownership debt.
                xp: 1234,
                money: 5678,
                health: 456,
                max_health: 900,
                powers: loaded_character_power_snapshot_like_cpp([
                    111, 222, 0, 0, 0, 0, 0, 0, 0, 0
                ]),
            }
        );
        assert_eq!(
            Some(snapshot),
            session.fixture_player_save_to_db_snapshot_like_cpp()
        );
        assert!(manager.try_lock().is_ok());
        session
            .with_owned_player_like_cpp(|player| {
                assert_eq!(player.unit().world().position(), position);
                assert_eq!(player.unit().data().level, 60);
                assert!(!player.active_player_data_changes_mask().is_any_set());
            })
            .unwrap();
        assert!(send_rx.is_empty());
    }
}

#[test]
fn save_snapshot_keeps_pending_destination_precedence_without_relocating_player() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 7);
    session.current_map_id = 571;
    let original = session.player_position_like_cpp().unwrap();
    let near = Position::new(11.0, 22.0, 33.0, 0.5);
    let far = Position::new(44.0, 55.0, 66.0, 1.5);
    session
        .with_owned_player_mut_like_cpp(|player| {
            let teleport = &mut player.gameplay_state_mut().teleport;
            teleport.near_pending = true;
            teleport.near_destination = Some((1, near));
        })
        .unwrap();
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for pending in [None, Some((u32::MAX, far))] {
            session.pending_teleport = pending;
            let snapshot = session
                .current_player_save_to_db_snapshot_like_cpp()
                .unwrap();
            assert_eq!(
                (snapshot.map_id, snapshot.instance_id, snapshot.position),
                if pending.is_some() {
                    (u16::MAX, 0, far)
                } else {
                    (1, 0, near)
                }
            );
            assert_eq!(
                Some(snapshot),
                session.fixture_player_save_to_db_snapshot_like_cpp()
            );
            assert_eq!(session.player_position_like_cpp(), Some(original));
        }
    }
}

#[test]
fn save_snapshot_preserves_existing_residence_specific_dead_health_projection() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_max_health(900);
            player.unit_mut().set_health(456);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Corpse);
        })
        .unwrap();
    for (detached, expected) in [(false, 0), (true, 456)] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let snapshot = session
            .current_player_save_to_db_snapshot_like_cpp()
            .unwrap();
        assert_eq!(snapshot.health, expected);
        assert_eq!(
            Some(snapshot),
            session.fixture_player_save_to_db_snapshot_like_cpp()
        );
    }
}

#[test]
fn save_snapshot_cannot_read_replacement_or_fallback_after_owner_loss() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.set_money(999);
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    assert_eq!(session.current_player_save_to_db_snapshot_like_cpp(), None);
    session.canonical_map_manager = None;
    assert_eq!(session.current_player_save_to_db_snapshot_like_cpp(), None);
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, Player::money),
        Some(999)
    );
}
