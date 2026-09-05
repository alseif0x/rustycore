//! Talent/glyph mutations stay inside one canonical Player access.

use super::*;

#[test]
fn talent_mutation_runs_on_active_and_detached_player_without_writeback() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            session.mutate_player_talent_runtime_like_cpp(|runtime| {
                calls.set(calls.get() + 1);
                assert!(matches!(
                    manager.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                runtime.talent_groups[0].insert(42, 1);
                runtime.glyph_groups[0][2] = 700;
                runtime.reset_talents_cost = 100_000;
                runtime.talents_loaded = true;
                detached
            }),
            Some(detached)
        );
        assert_eq!(calls.get(), 1);
        assert!(
            manager.try_lock().is_ok(),
            "no guard survives the operation"
        );
        // The normal loaded-state setter must preserve every unrelated value.
        session.mark_represented_glyphs_loaded_like_cpp();
        let state = session.player_talent_runtime_snapshot_like_cpp().unwrap();
        assert_eq!(state.talent_groups[0].get(&42), Some(&1));
        assert_eq!(state.glyph_groups[0][2], 700);
        assert_eq!(state.reset_talents_cost, 100_000);
        assert!(state.talents_loaded && state.glyphs_loaded);
    }
    session.canonical_map_manager = None;
    assert_eq!(
        session.mutate_player_talent_runtime_like_cpp(|_| panic!("missing owner")),
        None::<()>
    );
}
