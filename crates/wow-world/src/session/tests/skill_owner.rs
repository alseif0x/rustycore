//! Skill authority transitions execute under the generation-checked Player owner.
use super::*;

#[test]
fn skill_replacement_matches_previous_route_with_malformed_keys_and_tombstones() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for loaded in [false, true] {
            for complete in [false, true] {
                for variant in 0..7 {
                    let row = |id, state, value| RepresentedPlayerSkillLikeCpp {
                        skill_id: id,
                        step: 0,
                        value,
                        max: value,
                        profession_slot: -1,
                        state,
                    };
                    use RepresentedPlayerSkillStateLikeCpp as State;
                    let mut records = HashMap::from([
                        (333, row(333, State::Changed, 50)),
                        (755, row(755, State::Unchanged, 0)),
                        (999, row(999, State::Unchanged, 20)),
                        (164, row(164, State::New, 0)),
                    ]);
                    match variant {
                        1 => {
                            records.insert(333, row(333, State::Unchanged, 0));
                        }
                        2 => {
                            records.insert(333, row(333, State::Deleted, 0));
                        }
                        3 => {
                            records.insert(333, row(333, State::Deleted, 1));
                        }
                        4 => {
                            records.remove(&333);
                            records.insert(334, row(333, State::Deleted, 0));
                        }
                        5 => {
                            records.insert(334, row(333, State::Deleted, 0));
                        }
                        6 => {
                            records.insert(0, row(0, State::Unchanged, 0));
                        }
                        _ => {}
                    }
                    let prepare = |player: &mut Player| {
                        player.replace_skill_records_like_cpp(
                            vec![],
                            true,
                            true,
                            Some(7),
                            BTreeSet::from([333, 755, 999, 164, 1000]),
                        );
                        player.clear_data_changes();
                    };
                    let projection = |player: &Player| {
                        (
                            player.skill_records_like_cpp().to_vec(),
                            player.skill_records_loaded_like_cpp(),
                            player.skill_records_complete_like_cpp(),
                            player.occupied_skill_slots_like_cpp(),
                            player.non_durable_skill_tombstones_like_cpp().clone(),
                            player.active_player_data_changes_mask().blocks().to_vec(),
                        )
                    };
                    session.with_owned_player_mut_like_cpp(prepare).unwrap();
                    let expected_return = session.fixture_replace_player_skill_records_like_cpp(
                        records.clone(),
                        loaded,
                        complete,
                    );
                    let expected = session.with_owned_player_like_cpp(projection);
                    session.with_owned_player_mut_like_cpp(prepare).unwrap();
                    assert_eq!(
                        session.replace_player_skill_records_like_cpp(records, loaded, complete),
                        expected_return
                    );
                    assert_eq!(session.with_owned_player_like_cpp(projection), expected);
                }
            }
        }
    }
}

#[test]
fn skill_replacement_rejects_stale_and_missing_owner_without_replacing_tombstones() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.replace_skill_records_like_cpp(
        vec![],
        false,
        false,
        Some(7),
        BTreeSet::from([333]),
    );
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    for missing in [false, true] {
        if missing {
            session.canonical_map_manager = None;
        }
        assert!(!session.replace_player_skill_records_like_cpp(HashMap::new(), true, true));
        assert_eq!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |player| (
                    player.skill_records_loaded_like_cpp(),
                    player.occupied_skill_slots_like_cpp(),
                    player.non_durable_skill_tombstones_like_cpp().clone(),
                )),
            Some((false, Some(7), BTreeSet::from([333])))
        );
    }
}

#[test]
fn skill_lifecycle_finalization_matches_previous_route_on_active_and_detached_player() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for loaded in [false, true] {
            for complete in [false, true] {
                for occupied in [None, Some(0), Some(4)] {
                    for saved in [false, true] {
                        let prepare = |player: &mut Player| {
                            use wow_entities::{PlayerSkillLoadState as State, PlayerSkillRecord};
                            player.replace_skill_records_like_cpp(
                                [
                                    (333, State::New, 1),
                                    (755, State::Changed, 2),
                                    (333, State::Deleted, 3),
                                    (164, State::Unchanged, 4),
                                    (70000, State::Deleted, 5),
                                    (333, State::Changed, 6),
                                    (999, State::Deleted, 0),
                                ]
                                .into_iter()
                                .map(|(id, state, value)| PlayerSkillRecord {
                                    skill_line_id: id,
                                    current_value: value,
                                    max_value: value,
                                    step: 0,
                                    profession_slot: -1,
                                    state,
                                })
                                .collect(),
                                loaded,
                                complete,
                                occupied,
                                BTreeSet::from([333, 1000]),
                            );
                            player.clear_data_changes();
                        };
                        let projection = |player: &Player| {
                            (
                                player.skill_records_like_cpp().to_vec(),
                                player.skill_records_loaded_like_cpp(),
                                player.skill_records_complete_like_cpp(),
                                player.occupied_skill_slots_like_cpp(),
                                player.non_durable_skill_tombstones_like_cpp().clone(),
                                player.active_player_data_changes_mask().blocks().to_vec(),
                            )
                        };
                        session.with_owned_player_mut_like_cpp(prepare).unwrap();
                        if saved {
                            session.fixture_mark_player_skills_saved_like_cpp();
                        } else {
                            session.fixture_clear_player_skill_tombstones_like_cpp();
                        }
                        let expected = session.with_owned_player_like_cpp(projection);
                        session.with_owned_player_mut_like_cpp(prepare).unwrap();
                        if saved {
                            // An unrelated/absent committed group must not consume skill state.
                            let before = session.with_owned_player_like_cpp(projection);
                            session.mark_current_player_save_to_db_committed_like_cpp(
                                &Default::default(),
                            );
                            assert_eq!(session.with_owned_player_like_cpp(projection), before);
                            session.mark_current_player_save_to_db_committed_like_cpp(
                                &wow_persistence::PlayerCharacterCommittedGroupsLikeCpp {
                                    player_skills: true,
                                    ..Default::default()
                                },
                            );
                        } else {
                            session.clear_player_skill_tombstones_like_cpp();
                        }
                        assert_eq!(session.with_owned_player_like_cpp(projection), expected);
                    }
                }
            }
        }
    }
}

#[test]
fn skill_lifecycle_finalization_rejects_stale_and_missing_owner() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.replace_skill_records_like_cpp(
        vec![],
        false,
        false,
        Some(7),
        BTreeSet::from([333]),
    );
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    for missing in [false, true] {
        if missing {
            session.canonical_map_manager = None;
        }
        session.clear_player_skill_tombstones_like_cpp();
        session.mark_current_player_save_to_db_committed_like_cpp(
            &wow_persistence::PlayerCharacterCommittedGroupsLikeCpp {
                player_skills: true,
                ..Default::default()
            },
        );
        assert_eq!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |player| (
                    player.skill_records_loaded_like_cpp(),
                    player.occupied_skill_slots_like_cpp(),
                    player.non_durable_skill_tombstones_like_cpp().clone(),
                )),
            Some((false, Some(7), BTreeSet::from([333])))
        );
    }
}

#[test]
fn occupied_skill_slot_authority_matches_previous_route_for_active_and_detached_player() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for loaded in [false, true] {
            for complete in [false, true] {
                for ids in [
                    vec![],
                    vec![333],
                    vec![333, 333, 70000],
                    (1..=256).collect(),
                    (1..=257).collect(),
                ] {
                    for slots in [0, 1, 2, 256, 257] {
                        let prepare = |player: &mut Player| {
                            player.replace_skill_records_like_cpp(
                                ids.iter()
                                    .map(|id| wow_entities::PlayerSkillRecord {
                                        skill_line_id: *id,
                                        current_value: 0,
                                        max_value: 0,
                                        step: 0,
                                        profession_slot: -1,
                                        state: wow_entities::PlayerSkillLoadState::Deleted,
                                    })
                                    .collect(),
                                loaded,
                                complete,
                                Some(7),
                                BTreeSet::from([333]),
                            );
                        };
                        let projection = |player: &Player| {
                            (
                                player.skill_records_like_cpp().to_vec(),
                                player.skill_records_loaded_like_cpp(),
                                player.skill_records_complete_like_cpp(),
                                player.occupied_skill_slots_like_cpp(),
                                player.non_durable_skill_tombstones_like_cpp().clone(),
                            )
                        };
                        session.with_owned_player_mut_like_cpp(prepare).unwrap();
                        let expected_return =
                            session.fixture_set_player_skill_occupied_slots_like_cpp(slots);
                        let expected = session.with_owned_player_like_cpp(projection);
                        session.with_owned_player_mut_like_cpp(prepare).unwrap();
                        assert_eq!(
                            session.set_player_skill_occupied_slots_like_cpp(slots),
                            expected_return
                        );
                        assert_eq!(session.with_owned_player_like_cpp(projection), expected);
                    }
                }
            }
        }
    }
}

#[test]
fn occupied_skill_slot_authority_rejects_stale_and_missing_owner() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.replace_skill_records_like_cpp(vec![], true, true, Some(7), BTreeSet::from([333]));
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    for missing in [false, true] {
        if missing {
            session.canonical_map_manager = None;
        }
        assert!(!session.set_player_skill_occupied_slots_like_cpp(0));
        assert_eq!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |player| (
                    player.occupied_skill_slots_like_cpp(),
                    player.non_durable_skill_tombstones_like_cpp().clone()
                )),
            Some((Some(7), BTreeSet::from([333])))
        );
    }
}
