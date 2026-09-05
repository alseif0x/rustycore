//! Ordinary spell-book writes operate on native canonical state.

use super::*;

#[test]
fn spellbook_mutation_uses_native_active_and_detached_owner_once() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            session.mutate_player_spell_runtime_like_cpp(
                |runtime: &mut wow_entities::PlayerSpellRuntimeState| {
                    calls.set(calls.get() + 1);
                    assert!(matches!(
                        manager.try_lock(),
                        Err(std::sync::TryLockError::WouldBlock)
                    ));
                    runtime.known_spells = vec![9, 3];
                    runtime.trait_definition_ids.insert(9, 17);
                    runtime.override_spells.entry(3).or_default().insert(9);
                    runtime.rows.insert(
                        9,
                        wow_entities::PlayerKnownSpellRecord {
                            spell_id: 9,
                            state: wow_entities::PlayerSpellLoadState::New,
                            active: true,
                            disabled: false,
                            favorite: true,
                            dependent: false,
                        },
                    );
                    runtime.rows_loaded = true;
                    runtime.rows_complete = true;
                    detached
                }
            ),
            Some(detached)
        );
        assert_eq!(calls.get(), 1);
        assert!(manager.try_lock().is_ok());
        // A normal adapter write preserves unrelated native fields and vector order.
        session.set_represented_favorite_known_spells_like_cpp(HashSet::from([9]));
        let state = session.player_spell_runtime_snapshot_like_cpp().unwrap();
        assert_eq!(state.known_spells, vec![9, 3]);
        assert_eq!(state.trait_definition_ids.get(&9), Some(&17));
        assert_eq!(state.override_spells.get(&3), Some(&BTreeSet::from([9])));
        assert!(state.rows[&9].favorite);
        assert_eq!(
            state.rows[&9].state,
            RepresentedPlayerSpellStateLikeCpp::New
        );
        session.mark_player_spells_saved_like_cpp();
        let saved = session.player_spell_runtime_snapshot_like_cpp().unwrap();
        assert_eq!(
            saved.rows[&9].state,
            RepresentedPlayerSpellStateLikeCpp::Unchanged
        );
        assert_eq!(saved.known_spells, vec![9]);
        assert_eq!(saved.override_spells, state.override_spells);
        assert_eq!(saved.trait_definition_ids, state.trait_definition_ids);
    }
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.gameplay_state_mut().spells.known_spells = vec![42];
    let replacement_handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    assert_eq!(
        session
            .mutate_player_spell_runtime_like_cpp(|_| panic!("stale owner must not run callback")),
        None::<()>
    );
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().spells.known_spells.clone()
            }),
        Some(vec![42])
    );
    session.canonical_map_manager = None;
    assert_eq!(
        session.mutate_player_spell_runtime_like_cpp(|_| panic!(
            "missing owner must not run callback"
        )),
        None::<()>
    );
}
