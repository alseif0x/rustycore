//! Map selection is not Player attachment. The compatibility entrypoint still
//! composes both synchronously; a decision is not a durable/asynchronous permit.
use super::*;

#[test]
fn map_entry_preparation_preserves_active_and_detached_player_residence() {
    for detached in [false, true] {
        let (mut session, _, output) = make_session();
        install_canonical_player_owner_for_test(&mut session, 0, 0);
        session.set_map_store(crate::teleport_test_fixtures::world_maps([0, 1]));
        let position = Position::new(7.0, 8.0, 9.0, 0.5);
        session.set_player_position_like_cpp(position);
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let handle = session.player_handle_like_cpp.unwrap();
        let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
        let residence = manager.lock().unwrap().player_residence_like_cpp(handle);
        for created in [true, false] {
            let decision = session.prepare_canonical_map_entry_like_cpp(1).unwrap();
            let key = match decision {
                wow_map::CreateMapDecision::Create { key, .. } => {
                    assert!(created);
                    key
                }
                wow_map::CreateMapDecision::Existing { key, .. } => {
                    assert!(!created);
                    key
                }
                other => panic!("unexpected admission {other:?}"),
            };
            assert_eq!(key, wow_map::MapKey::new(1, 0));
            assert_eq!(session.player_handle_like_cpp, Some(handle));
            let manager = manager
                .try_lock()
                .expect("preparation releases its map guard");
            assert_eq!(manager.player_residence_like_cpp(handle), residence);
            assert_eq!(
                manager.with_player_like_cpp(handle, |p| p.unit().world().position()),
                Some(position)
            );
            assert_eq!(manager.find_map(1, 0).unwrap().player_count(), 0);
        }
        assert!(output.is_empty());
    }
}

#[test]
fn map_entry_missing_catalog_does_not_change_the_current_player() {
    let (mut session, _, output) = make_session();
    install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_map_store(crate::teleport_test_fixtures::world_maps([0]));
    let handle = session.player_handle_like_cpp.unwrap();
    assert_eq!(session.prepare_canonical_map_entry_like_cpp(1), None);
    let manager = session
        .canonical_map_manager
        .as_ref()
        .unwrap()
        .try_lock()
        .unwrap();
    assert_eq!(
        manager.player_residence_like_cpp(handle),
        Some(wow_map::PlayerResidenceLikeCpp::Active(
            wow_map::MapKey::new(0, 0)
        ))
    );
    assert!(manager.find_map(1, 0).is_none());
    assert!(output.is_empty());
}
