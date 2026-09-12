//! Taxi mutation must not overwrite a read snapshot after releasing the owner.

use super::*;

#[test]
fn taxi_mutation_runs_once_on_active_and_detached_owner_and_rejects_stale_handle() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            session.mutate_player_taxi_state_like_cpp(|taxi| {
                calls.set(calls.get() + 1);
                assert!(matches!(
                    manager.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                taxi.replace_destinations_like_cpp(vec![20, 10]);
                taxi.set_taxi_cleanup_state_like_cpp(UnitFlags::ON_TAXI.bits(), true);
                detached
            }),
            Some(detached)
        );
        assert_eq!(calls.get(), 1);
        assert!(manager.try_lock().is_ok());
        session.set_taxi_destinations_like_cpp(vec![30, 20]);
        let taxi = session.player_taxi_state_snapshot_like_cpp().unwrap();
        assert_eq!(taxi.destinations_like_cpp(), [30, 20]);
        assert!(taxi.mounted_like_cpp());
        assert_eq!(taxi.unit_flags_like_cpp(), UnitFlags::ON_TAXI.bits());
    }
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement
        .gameplay_state_mut()
        .taxi
        .replace_destinations_like_cpp(vec![42]);
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    assert_eq!(
        session.mutate_player_taxi_state_like_cpp(|_| panic!("stale owner")),
        None::<()>
    );
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| {
                player
                    .taxi_state_like_cpp()
                    .destinations_like_cpp()
                    .to_vec()
            }),
        Some(vec![42])
    );
    session.canonical_map_manager = None;
    assert_eq!(
        session.mutate_player_taxi_state_like_cpp(|_| panic!("missing owner")),
        None::<()>
    );
}
