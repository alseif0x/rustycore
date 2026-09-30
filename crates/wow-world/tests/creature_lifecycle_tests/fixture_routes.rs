use super::*;

#[test]
fn normal_feature_tick_keeps_missing_phase_rejection_and_explicit_fixture_runs_old_corpse_path() {
    let (mut session, send_rx, guid) = setup_dead_creature_past_despawn(91_010);
    let output = session.fixture_lifecycle_normal_tick();
    assert!(output.packets.is_empty());
    assert!(send_rx.try_recv().is_err());
    let manager = session.fixture_lifecycle_manager().as_ref().unwrap().clone();
    assert!(manager.read().unwrap().find_creature(0, 0, guid).is_some());
    assert_eq!(manager.read().unwrap().respawn_queue_len(0, 0), 0);
    let output = session.fixture_lifecycle_tick();
    assert!(!output.packets.is_empty());
    assert!(manager.read().unwrap().find_creature(0, 0, guid).is_none());
    assert_eq!(manager.read().unwrap().respawn_queue_len(0, 0), 1);
    session.fixture_lifecycle_flush(output);
    assert!(send_rx.try_recv().is_ok());
}

#[test]
fn explicit_fixture_owner_read_preserves_global_legacy_without_session_tick() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    manager.write().unwrap().set_tick_owner(map_manager::RuntimeTickOwner::GlobalLegacy);
    let guid = test_creature_guid(91_011);
    register_test_creature(&mut session, manager.clone(), guid, 100);
    assert_eq!(session.fixture_lifecycle_tick_owner(), map_manager::RuntimeTickOwner::GlobalLegacy);
    let clock = manager.read().unwrap().find_creature(0, 0, guid).unwrap().runtime_elapsed_ms_like_cpp();
    if session.fixture_lifecycle_tick_owner() == map_manager::RuntimeTickOwner::Session {
        session.fixture_lifecycle_tick_sync();
    }
    assert_eq!(manager.read().unwrap().find_creature(0, 0, guid).unwrap().runtime_elapsed_ms_like_cpp(), clock);
}
