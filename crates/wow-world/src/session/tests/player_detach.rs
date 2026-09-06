//! C++ removes a Player with delete=false for transfer; the same object survives.
use super::*;

fn legacy_record() -> (WorldSession, SharedCanonicalMapManager, ObjectGuid) {
    let (mut session, _, _) = make_session();
    let manager = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 0x5781);
    session.set_player_guid(Some(guid));
    session.set_canonical_map_manager(Arc::clone(&manager));
    add_canonical_test_player_on_map(&manager, guid, Position::new(1.0, 2.0, 3.0, 0.5), 571, 7);
    assert!(session.player_handle_like_cpp.is_none());
    (session, manager, guid)
}

#[test]
fn detach_adopts_the_unique_legacy_record_without_discarding_its_player() {
    let (mut session, manager, guid) = legacy_record();
    let address = {
        let mut manager = manager.lock().unwrap();
        let player = manager
            .find_map_mut(571, 7)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(guid)
            .unwrap();
        player.set_money(12345);
        player as *const Player as usize
    };
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let handle = session.player_handle_like_cpp.unwrap();
    for _ in 0..2 {
        assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        assert_eq!(session.player_handle_like_cpp, Some(handle));
        let manager = manager.lock().unwrap();
        assert_eq!(
            manager.player_residence_like_cpp(handle),
            Some(wow_map::PlayerResidenceLikeCpp::Detached)
        );
        manager
            .with_player_like_cpp(handle, |player| {
                assert_eq!(player as *const Player as usize, address);
                assert_eq!(player.money(), 12345);
                assert_eq!(player.unit().world().map_id(), 571);
                assert_eq!(player.unit().world().instance_id(), 7);
            })
            .unwrap();
        assert!(
            manager
                .find_map(571, 7)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .is_none()
        );
    }
}

#[test]
fn detach_rejects_ambiguous_legacy_records_without_removing_either() {
    let (mut session, manager, guid) = legacy_record();
    add_canonical_test_player_on_map(&manager, guid, Position::default(), 0, 0);
    assert!(!session.remove_current_player_from_canonical_current_map_like_cpp());
    assert!(session.player_handle_like_cpp.is_none());
    let manager = manager.lock().unwrap();
    for (map, instance) in [(571, 7), (0, 0)] {
        assert!(
            manager
                .find_map(map, instance)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .is_some()
        );
    }
}

#[test]
fn detach_with_a_stale_handle_cannot_adopt_or_remove_its_replacement() {
    let (mut session, manager, guid) = legacy_record();
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    let original = session.player_handle_like_cpp;
    let replacement = {
        let mut manager = manager.lock().unwrap();
        let mut player = Box::new(Player::new(Some(1), false));
        player.unit_mut().world_mut().object_mut().create(guid);
        player.set_money(9876);
        let handle = manager.install_detached_player_like_cpp(player).unwrap();
        manager
            .attach_player_like_cpp(handle, wow_map::MapKey::new(571, 7), Position::default())
            .unwrap();
        handle
    };
    assert!(!session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(session.player_handle_like_cpp, original);
    let manager = manager.lock().unwrap();
    assert_eq!(
        manager.player_residence_like_cpp(replacement),
        Some(wow_map::PlayerResidenceLikeCpp::Active(
            wow_map::MapKey::new(571, 7)
        ))
    );
    assert_eq!(
        manager.with_player_like_cpp(replacement, Player::money),
        Some(9876)
    );
}
