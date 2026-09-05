//! Ordinary spell-book writes operate on native canonical state.

use super::*;

#[test]
fn narrow_spell_queries_match_full_snapshot_for_active_and_detached_owner() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for flags in 0..16 {
            session
                .with_owned_player_mut_like_cpp(|player| {
                    let mut runtime = wow_entities::PlayerSpellRuntimeState::default();
                    runtime.known_spells = vec![20, -1, 10, 20];
                    runtime.dependent_known_spells = BTreeSet::from([-1, 20]);
                    runtime.favorite_known_spells = BTreeSet::from([10, 30]);
                    runtime.override_spells.insert(10, BTreeSet::from([-1, 20]));
                    runtime.trait_definition_ids.insert(20, 200);
                    runtime.rows_loaded = flags & 1 != 0;
                    runtime.rows_complete = flags & 2 != 0;
                    runtime.trait_definition_ids_complete = flags & 4 != 0;
                    runtime.override_spells_complete = flags & 8 != 0;
                    runtime.rows.insert(
                        10,
                        wow_entities::PlayerKnownSpellRecord {
                            spell_id: 10,
                            state: wow_entities::PlayerSpellLoadState::Removed,
                            active: true,
                            disabled: true,
                            favorite: true,
                            dependent: true,
                        },
                    );
                    player.replace_spell_runtime_like_cpp(runtime);
                })
                .unwrap();
            let expected = session.player_spell_runtime_snapshot_like_cpp().unwrap();
            assert_eq!(
                session.resolved_known_spells_like_cpp(),
                Some(expected.known_spells.clone())
            );
            assert_eq!(session.known_spells_like_cpp(), expected.known_spells);
            assert_eq!(
                session.represented_dependent_known_spells_like_cpp(),
                expected.dependent_known_spells
            );
            assert_eq!(
                session.represented_favorite_known_spells_like_cpp(),
                expected.favorite_known_spells
            );
            assert_eq!(
                session.represented_override_spells_like_cpp(),
                expected.override_spells
            );
            assert_eq!(
                session.represented_spell_trait_definition_ids_like_cpp(),
                expected.trait_definition_ids
            );
            assert_eq!(
                session.complete_represented_override_spells_like_cpp(),
                expected
                    .override_spells_complete
                    .then_some(expected.override_spells)
            );
            assert_eq!(
                session.complete_represented_spell_trait_definition_ids_like_cpp(),
                expected
                    .trait_definition_ids_complete
                    .then_some(expected.trait_definition_ids)
            );
            assert_eq!(
                session.represented_player_spell_rows_loaded_like_cpp(),
                expected.rows_loaded
            );
            assert_eq!(
                session.complete_represented_player_spell_rows_like_cpp(),
                (expected.rows_loaded && expected.rows_complete).then_some(expected.rows)
            );
            let calls = std::cell::Cell::new(0);
            assert_eq!(
                session.with_player_spell_runtime_like_cpp(|runtime| {
                    calls.set(calls.get() + 1);
                    assert!(manager.try_lock().is_err());
                    runtime.known_spells.len()
                }),
                Some(4)
            );
            assert_eq!(calls.get(), 1);
            assert!(manager.try_lock().is_ok());
        }
    }
}

#[test]
fn narrow_spell_queries_reject_stale_and_missing_owner_without_fabricated_authority() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.gameplay_state_mut().spells.known_spells = vec![42];
    manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    for missing in [false, true] {
        if missing {
            session.canonical_map_manager = None;
        }
        assert_eq!(
            session.with_player_spell_runtime_like_cpp(|_| panic!(
                "unresolved owner must not run query"
            )),
            None::<()>
        );
        assert_eq!(session.resolved_known_spells_like_cpp(), None);
        assert_eq!(
            session.complete_represented_player_spell_rows_like_cpp(),
            None
        );
        assert_eq!(
            session.complete_represented_override_spells_like_cpp(),
            None
        );
        assert_eq!(
            session.complete_represented_spell_trait_definition_ids_like_cpp(),
            None
        );
        assert!(session.known_spells_like_cpp().is_empty());
        assert!(
            session
                .represented_dependent_known_spells_like_cpp()
                .is_empty()
        );
        assert!(
            session
                .represented_favorite_known_spells_like_cpp()
                .is_empty()
        );
        assert!(session.represented_override_spells_like_cpp().is_empty());
        assert!(
            session
                .represented_spell_trait_definition_ids_like_cpp()
                .is_empty()
        );
        assert!(!session.represented_player_spell_rows_loaded_like_cpp());
    }
}

#[test]
fn spell_save_finalization_matches_previous_active_and_detached_owner() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for flags in 0..8 {
            for complete in [false, true] {
                for committed in [false, true] {
                    let mut initial = wow_entities::PlayerSpellRuntimeState::default();
                    for (index, state) in [
                        wow_entities::PlayerSpellLoadState::Unchanged,
                        wow_entities::PlayerSpellLoadState::New,
                        wow_entities::PlayerSpellLoadState::Changed,
                        wow_entities::PlayerSpellLoadState::Removed,
                        wow_entities::PlayerSpellLoadState::Temporary,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let id = index as i32 + 10;
                        initial.rows.insert(
                            id,
                            wow_entities::PlayerKnownSpellRecord {
                                spell_id: id,
                                state,
                                active: false,
                                disabled: flags & 1 != 0,
                                dependent: flags & 2 != 0,
                                favorite: flags & 4 != 0,
                            },
                        );
                        initial.trait_definition_ids.insert(id, 100 + id);
                    }
                    initial.rows_loaded = true;
                    initial.rows_complete = complete;
                    initial.trait_definition_ids_complete = complete;
                    initial.override_spells_complete = complete;
                    initial.known_spells = vec![99];
                    initial.removed_known_spells.insert(99);
                    initial.fallback_rows = initial.rows.clone();
                    initial.override_spells.insert(10, BTreeSet::from([20]));
                    session
                        .with_owned_player_mut_like_cpp(|p| {
                            p.replace_spell_runtime_like_cpp(initial.clone())
                        })
                        .unwrap();
                    if committed {
                        session.fixture_mark_player_spells_saved_like_cpp();
                    }
                    let expected =
                        session.with_owned_player_like_cpp(|p| p.spell_runtime_like_cpp().clone());
                    session
                        .with_owned_player_mut_like_cpp(|p| {
                            p.replace_spell_runtime_like_cpp(initial)
                        })
                        .unwrap();
                    session.mark_current_player_save_to_db_committed_like_cpp(
                        &wow_persistence::PlayerCharacterCommittedGroupsLikeCpp {
                            player_spells: committed,
                            ..Default::default()
                        },
                    );
                    assert_eq!(
                        session.with_owned_player_like_cpp(|p| p.spell_runtime_like_cpp().clone()),
                        expected
                    );
                }
            }
        }
    }
}

#[test]
fn spell_save_finalization_cannot_touch_a_replacement_or_missing_owner() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    let mut initial = wow_entities::PlayerSpellRuntimeState::default();
    initial.known_spells = vec![99];
    initial.removed_known_spells.insert(10);
    replacement.replace_spell_runtime_like_cpp(initial.clone());
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    for missing in [false, true] {
        if missing {
            session.canonical_map_manager = None;
        }
        session.mark_current_player_save_to_db_committed_like_cpp(
            &wow_persistence::PlayerCharacterCommittedGroupsLikeCpp {
                player_spells: true,
                ..Default::default()
            },
        );
        assert_eq!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |p| p.spell_runtime_like_cpp().clone()),
            Some(initial.clone())
        );
    }
}

#[test]
fn loaded_spell_reconciliation_matches_previous_active_and_detached_owner() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for complete in [false, true] {
            for state in [
                RepresentedPlayerSpellStateLikeCpp::Unchanged,
                RepresentedPlayerSpellStateLikeCpp::New,
                RepresentedPlayerSpellStateLikeCpp::Changed,
                RepresentedPlayerSpellStateLikeCpp::Removed,
                RepresentedPlayerSpellStateLikeCpp::Temporary,
            ] {
                for active in [false, true] {
                    for disabled in [false, true] {
                        for dependent in [false, true] {
                            let row = RepresentedPlayerSpellLikeCpp {
                                spell_id: 10,
                                active,
                                disabled,
                                dependent,
                                favorite: true,
                                state,
                            };
                            let mut initial = wow_entities::PlayerSpellRuntimeState::default();
                            initial.known_spells = vec![10, 30];
                            initial.rows_complete = true;
                            initial.trait_definition_ids_complete = true;
                            initial.override_spells_complete = true;
                            for id in [10, 20] {
                                initial.fallback_rows.insert(
                                    id,
                                    canonical_player_spell_record_like_cpp(
                                        RepresentedPlayerSpellLikeCpp {
                                            spell_id: id,
                                            dependent: !dependent,
                                            favorite: false,
                                            ..row
                                        },
                                    ),
                                );
                            }
                            let rows = vec![
                                row,
                                RepresentedPlayerSpellLikeCpp {
                                    spell_id: 30,
                                    ..row
                                },
                            ];
                            session
                                .with_owned_player_mut_like_cpp(|p| {
                                    p.replace_spell_runtime_like_cpp(initial.clone())
                                })
                                .unwrap();
                            let expected_result = session
                                .fixture_replace_loaded_spell_rows_like_cpp(rows.clone(), complete);
                            let expected = session
                                .with_owned_player_like_cpp(|p| p.spell_runtime_like_cpp().clone());
                            session
                                .with_owned_player_mut_like_cpp(|p| {
                                    p.replace_spell_runtime_like_cpp(initial)
                                })
                                .unwrap();
                            assert_eq!(
                                session.replace_loaded_represented_player_spell_rows_like_cpp(
                                    rows.into_iter()
                                        .inspect(|_| assert!(manager.try_lock().is_ok())),
                                    complete
                                ),
                                expected_result
                            );
                            assert_eq!(
                                session.with_owned_player_like_cpp(|p| p
                                    .spell_runtime_like_cpp()
                                    .clone()),
                                expected
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn loaded_spell_reconciliation_rejects_invalid_rows_and_stale_owner() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let row = RepresentedPlayerSpellLikeCpp {
        spell_id: 10,
        active: true,
        disabled: false,
        dependent: false,
        favorite: true,
        state: RepresentedPlayerSpellStateLikeCpp::New,
    };
    let mut initial = wow_entities::PlayerSpellRuntimeState::default();
    initial
        .rows
        .insert(10, canonical_player_spell_record_like_cpp(row));
    initial.fallback_rows = initial.rows.clone();
    initial.rows_complete = true;
    initial.rows_loaded = true;
    initial.override_spells_complete = true;
    for rows in [
        vec![],
        vec![row, row],
        vec![RepresentedPlayerSpellLikeCpp { spell_id: 0, ..row }],
        vec![RepresentedPlayerSpellLikeCpp {
            spell_id: -1,
            ..row
        }],
    ] {
        session
            .with_owned_player_mut_like_cpp(|p| p.replace_spell_runtime_like_cpp(initial.clone()))
            .unwrap();
        let result = session.fixture_replace_loaded_spell_rows_like_cpp(rows.clone(), true);
        let expected = session.with_owned_player_like_cpp(|p| p.spell_runtime_like_cpp().clone());
        session
            .with_owned_player_mut_like_cpp(|p| p.replace_spell_runtime_like_cpp(initial.clone()))
            .unwrap();
        assert_eq!(
            session.replace_loaded_represented_player_spell_rows_like_cpp(rows, true),
            result
        );
        assert_eq!(
            session.with_owned_player_like_cpp(|p| p.spell_runtime_like_cpp().clone()),
            expected
        );
    }
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.replace_spell_runtime_like_cpp(initial.clone());
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    for missing in [false, true] {
        if missing {
            session.canonical_map_manager = None;
        }
        assert!(!session.replace_loaded_represented_player_spell_rows_like_cpp([row], true));
        assert_eq!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |p| p.spell_runtime_like_cpp().clone()),
            Some(initial.clone())
        );
    }
}

#[test]
fn trait_config_lifecycle_matches_previous_route_for_active_and_detached_owner() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for begin in [false, true] {
            for empty in [false, true] {
                for configs in [
                    vec![],
                    vec![(1, 1, 62, 4)],
                    vec![(1, 0, -1, -2), (2, 3, 0, 0)],
                    vec![(1, 1, 62, 4), (1, 2, 0, 0)],
                    vec![(0, 1, 62, 4)],
                    vec![(-1, 1, 62, 4)],
                ] {
                    let prepare = |player: &mut Player| {
                        let mut runtime = wow_entities::PlayerSpellRuntimeState::default();
                        runtime.known_spells = vec![99];
                        runtime.trait_definition_ids.insert(99, 100);
                        runtime.trait_definition_ids_complete = true;
                        runtime.trait_config_rows.insert(777, (1, 62, 4));
                        runtime.trait_config_rows_complete = true;
                        runtime.trait_entry_rows_complete = true;
                        runtime.trait_entry_rows_empty = true;
                        runtime.override_spells.insert(50, BTreeSet::from([60]));
                        player.replace_spell_runtime_like_cpp(runtime);
                    };
                    let projection = |player: &Player| player.spell_runtime_like_cpp().clone();
                    session.with_owned_player_mut_like_cpp(prepare).unwrap();
                    if begin {
                        session.fixture_begin_trait_config_authority_load_like_cpp();
                    }
                    let expected_begin = session.with_owned_player_like_cpp(projection);
                    let expected_return = session
                        .fixture_complete_trait_config_authority_load_like_cpp(
                            configs.clone(),
                            empty,
                        );
                    let expected = session.with_owned_player_like_cpp(projection);
                    session.with_owned_player_mut_like_cpp(prepare).unwrap();
                    if begin {
                        session.begin_represented_trait_config_authority_load_like_cpp();
                    }
                    assert_eq!(
                        session.with_owned_player_like_cpp(projection),
                        expected_begin
                    );
                    assert_eq!(
                        session.complete_represented_trait_config_authority_load_like_cpp(
                            configs.into_iter().inspect(|_| {
                                assert!(manager.try_lock().is_ok());
                            }),
                            empty
                        ),
                        expected_return
                    );
                    assert_eq!(session.with_owned_player_like_cpp(projection), expected);
                }
            }
        }
    }
}

#[test]
fn trait_config_lifecycle_does_not_reset_replacement_through_stale_or_missing_owner() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    let mut expected = wow_entities::PlayerSpellRuntimeState::default();
    expected.trait_definition_ids.insert(99, 100);
    expected.trait_config_rows.insert(777, (1, 62, 4));
    expected.trait_entry_rows_complete = true;
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
        session.begin_represented_trait_config_authority_load_like_cpp();
        for id in [0, 1] {
            assert!(
                !session.complete_represented_trait_config_authority_load_like_cpp(
                    [(id, 1, 62, 4)],
                    true
                )
            );
        }
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
