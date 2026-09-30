//! Negative integration cases for the explicitly selected fixture rail.

use super::fixtures::*;

fn ordinary_session() -> WorldSession {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, _send_rx) = flume::bounded(16);
    WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "enUS".into(),
        pkt_rx,
        send_tx,
    )
}

#[test]
fn ordinary_feature_session_does_not_enable_handleless_character_fallback() {
    let mut session = ordinary_session();
    let before = session.character_fixture_progression_inputs_for_test();
    assert!(!session.character_lifecycle_fixture_is_enabled_for_test());
    assert!(!session.character_set_player_xp_for_test(91));
    assert!(!session.character_set_player_next_level_xp_for_test(72_000));
    assert!(!session.character_set_fall_information_for_test(500, 80.0));
    assert!(
        session
            .character_player_rest_state_snapshot_for_test()
            .is_none()
    );
    assert_eq!(
        session.character_apply_offline_xp_rest_bonus_for_test(100, 200, true),
        0.0
    );
    assert_eq!(session.character_fixture_progression_inputs_for_test(), before);
    assert_eq!(session.character_player_handle_for_test(), None);
}

#[test]
fn opted_in_character_fixture_without_guid_cannot_create_or_read_a_map_owner() {
    let (mut session, _send_rx) = make_session();
    let before = session.character_fixture_progression_inputs_for_test();
    assert!(session.character_lifecycle_fixture_is_enabled_for_test());
    assert_eq!(session.player_guid(), None);
    assert!(
        session
            .character_ensure_canonical_world_map_for_current_player_for_test()
            .is_none()
    );
    assert_eq!(
        session.character_canonical_player_pvp_flags_for_test(ObjectGuid::EMPTY),
        None
    );
    assert_eq!(session.character_player_handle_for_test(), None);
    assert_eq!(session.character_fixture_progression_inputs_for_test(), before);
}

#[test]
fn canonical_character_values_take_priority_over_handleless_fixture_inputs() {
    run_canonical_player_owner_test(|| {
        let (mut session, _send_rx) = make_session();
        install_canonical_player_owner_for_test(&mut session, 571, 0);
        assert!(session.character_set_player_xp_for_test(10));
        assert!(session.character_set_fall_information_for_test(20, 30.0));
        mutate_canonical_player_for_test(&session, |player| {
            player.set_xp(99);
            player.set_fall_information_like_cpp(80, 90.0);
        })
        .unwrap();
        assert_eq!(session.character_player_xp_for_test(), 99);
        assert_eq!(session.character_fall_information_for_test(), (80, 90.0));
        let inputs = session.character_fixture_progression_inputs_for_test();
        assert_eq!(inputs.0, 10);
        assert_eq!(inputs.2, (20, 30.0));
    });
}

#[test]
fn stale_some_handle_never_reaches_replacement_guid_or_partially_writes_fixture_inputs() {
    run_canonical_player_owner_test(|| {
        let (mut session, _send_rx) = make_session();
        let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
        let old = session.character_player_handle_for_test().unwrap();
        let canonical = canonical_map_manager_for_test(&session).unwrap().clone();
        let replacement = {
            let mut manager = canonical.lock().unwrap();
            let player = manager.retire_player_like_cpp(old).unwrap();
            let replacement = manager.install_detached_player_like_cpp(player).unwrap();
            manager.create_world_map(571, 0);
            manager
                .attach_player_like_cpp(
                    replacement,
                    wow_map::MapKey::new(571, 0),
                    Position::default(),
                )
                .unwrap();
            manager
                .with_player_mut_like_cpp(replacement, |player| {
                    player.set_xp(77);
                    player.set_fall_information_like_cpp(88, 99.0);
                })
                .unwrap();
            replacement
        };
        let before = session.character_fixture_progression_inputs_for_test();
        assert_eq!(session.character_player_handle_for_test(), Some(old));
        assert!(!session.character_set_player_xp_for_test(1));
        assert!(!session.character_set_player_next_level_xp_for_test(2));
        assert!(!session.character_set_fall_information_for_test(3, 4.0));
        session.character_load_represented_xp_rest_bonus_for_test(
            REST_STATE_RESTED_LIKE_CPP,
            5.0,
        );
        assert!(
            session
                .character_player_rest_state_snapshot_for_test()
                .is_none()
        );
        assert_eq!(session.character_canonical_player_pvp_flags_for_test(guid), None);
        assert!(!session.character_ensure_login_player_controller_for_test(
            guid,
            "Rejected".into(),
            Position::default(),
            0,
            1,
            1,
            1,
            0,
        ));
        assert_eq!(session.character_fixture_progression_inputs_for_test(), before);
        let manager = canonical.lock().unwrap();
        assert_eq!(
            manager.with_player_like_cpp(replacement, |player| player.active_data().xp),
            Some(77)
        );
        assert_eq!(
            manager.with_player_like_cpp(replacement, |player| player.fall_information_like_cpp()),
            Some((88, 99.0))
        );
    });
}
