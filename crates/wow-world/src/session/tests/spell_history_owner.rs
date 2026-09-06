//! SpellHistory mutations execute on the canonical Unit, not on a write-back copy.

use super::*;

#[test]
fn spell_history_mutation_runs_once_under_active_and_detached_owner() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    let handle = session.player_handle_like_cpp.unwrap();
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
            assert_eq!(
                manager.lock().unwrap().player_residence_like_cpp(handle),
                Some(wow_map::PlayerResidenceLikeCpp::Detached)
            );
        }
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            session.mutate_player_spell_history_like_cpp(|history| {
                calls.set(calls.get() + 1);
                assert!(
                    matches!(manager.try_lock(), Err(std::sync::TryLockError::WouldBlock)),
                    "the callback must execute inside the canonical owner, not on a snapshot"
                );
                history.global_cooldowns.insert(17, 99);
                history.charges_loaded = true;
                detached
            }),
            Some(detached)
        );
        assert_eq!(calls.get(), 1);
        // Owner guard is gone on return, before any persistence/delivery work.
        let owner = manager.try_lock().expect("mutation must release the owner");
        assert_eq!(
            owner.with_player_like_cpp(handle, |player| {
                let history = &player.unit().subsystems().spells.history;
                (
                    history.global_cooldowns.get(&17).copied(),
                    history.charges_loaded,
                )
            }),
            Some((Some(99), true))
        );
    }
}

#[test]
fn spell_history_mutation_rejects_stale_or_missing_owner_without_invoking_callback() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement
        .unit_mut()
        .subsystems_mut()
        .spells
        .history
        .global_cooldowns
        .insert(17, 123);
    let replacement_handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    let calls = std::cell::Cell::new(0);
    assert_eq!(
        session.mutate_player_spell_history_like_cpp(|history| {
            calls.set(calls.get() + 1);
            history.global_cooldowns.clear();
        }),
        None
    );
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player
                    .unit()
                    .subsystems()
                    .spells
                    .history
                    .global_cooldowns
                    .get(&17)
                    .copied()
            }),
        Some(Some(123))
    );
    session.canonical_map_manager = None;
    assert_eq!(
        session.mutate_player_spell_history_like_cpp(|_| calls.set(calls.get() + 1)),
        None
    );
    assert_eq!(calls.get(), 0);
}
