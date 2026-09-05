//! Ordinary spell-book writes operate on native canonical state.

use super::*;

#[test]
fn spell_metadata_transitions_preserve_active_and_detached_owner_contracts() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for traits in [
            vec![],
            vec![(10, 20)],
            vec![(10, 20), (30, 40)],
            vec![(10, 20), (10, 20)],
            vec![(0, 20)],
            vec![(10, 0)],
            vec![(-1, 20)],
            vec![(10, -1)],
        ] {
            let prepare = |player: &mut Player| {
                let mut runtime = wow_entities::PlayerSpellRuntimeState::default();
                runtime.known_spells = vec![99];
                runtime.trait_definition_ids.insert(99, 100);
                runtime.trait_definition_ids_complete = true;
                runtime.override_spells.insert(50, BTreeSet::from([60]));
                runtime.override_spells_complete = true;
                runtime.trait_config_rows.insert(777, (1, 62, 4));
                player.replace_spell_runtime_like_cpp(runtime);
            };
            session.with_owned_player_mut_like_cpp(prepare).unwrap();
            let expected_return =
                session.fixture_set_complete_spell_trait_definition_ids_like_cpp(traits.clone());
            let expected =
                session.with_owned_player_like_cpp(|p| p.spell_runtime_like_cpp().clone());
            session.with_owned_player_mut_like_cpp(prepare).unwrap();
            // Iterator code must run before acquiring the Player owner.
            let observed = std::cell::Cell::new(0);
            let len = traits.len();
            assert_eq!(
                session.set_complete_represented_spell_trait_definition_ids_like_cpp(
                    traits.into_iter().inspect(|_| {
                        assert!(manager.try_lock().is_ok());
                        observed.set(observed.get() + 1);
                    })
                ),
                expected_return
            );
            assert_eq!(observed.get(), len);
            assert_eq!(
                session.with_owned_player_like_cpp(|p| p.spell_runtime_like_cpp().clone()),
                expected
            );
            session.add_represented_override_spell_like_cpp(50, 70);
            session.add_represented_override_spell_like_cpp(50, 70);
            session.add_represented_override_spell_like_cpp(0, 70);
            session.remove_represented_override_spell_like_cpp(50, 60);
            assert_eq!(
                session.represented_override_spells_like_cpp(),
                HashMap::from([(50, BTreeSet::from([70]))])
            );
            session.remove_represented_override_spell_like_cpp(50, 70);
            session.remove_represented_override_spell_like_cpp(50, 70);
            assert!(session.represented_override_spells_like_cpp().is_empty());
            assert!(
                session
                    .with_owned_player_like_cpp(|p| p
                        .spell_runtime_like_cpp()
                        .override_spells_complete)
                    .unwrap()
            );
        }
    }
}

#[test]
fn spell_metadata_rejects_stale_and_missing_owner_without_mutating_replacement() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    let mut expected = wow_entities::PlayerSpellRuntimeState::default();
    expected.trait_definition_ids.insert(99, 100);
    expected.override_spells.insert(50, BTreeSet::from([60]));
    replacement.replace_spell_runtime_like_cpp(expected.clone());
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    for missing in [false, true] {
        if missing {
            session.canonical_map_manager = None;
        }
        assert!(!session.set_complete_represented_spell_trait_definition_ids_like_cpp([(10, 20)]));
        session.add_represented_override_spell_like_cpp(50, 70);
        session.remove_represented_override_spell_like_cpp(50, 60);
        assert_eq!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |p| p.spell_runtime_like_cpp().clone()),
            Some(expected.clone())
        );
    }
}

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
