// Existing Character application scenarios, moved with original assertion operands.

use super::fixtures::session::make_session;
use super::fixtures::*;

#[test]
fn canonical_player_logout_retires_detached_handle_like_cpp() {
    run_canonical_player_owner_test(|| {
        let (mut session, _, _) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 56);

        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
        session.character_attach_player_controller_for_test(
            player_guid,
            "LogoutDetached".to_string(),
            Position::new(3700.0, 1500.0, 120.0, 0.0),
            571,
            1,
            1,
            80,
            0,
        );
        session
            .character_ensure_canonical_world_map_for_current_player_for_test()
            .expect("initial world map");
        let handle = session
            .character_player_handle_for_test()
            .expect("canonical handle");
        assert!(session.character_remove_current_player_from_canonical_current_map_for_test());

        session.cleanup_shared_runtime_state();

        assert_eq!(session.character_player_handle_for_test(), None);
        assert_eq!(
            canonical.lock().unwrap().player_residence_like_cpp(handle),
            None
        );
    });
}
