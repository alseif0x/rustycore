//! Rest visibility/save queries borrow one generation-checked Player.

use super::*;

#[test]
fn rest_consumption_matches_previous_route_and_empty_victim_does_not_normalize() {
    let (mut session, _, send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let victim = test_creature_guid(79);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for bonus in [0.5, 70.0] {
            let prepare = |player: &mut Player| {
                player.set_next_level_xp(1000);
                player.load_xp_rest_bonus_like_cpp(REST_STATE_RAF_LINKED_LIKE_CPP, bonus);
                player.clear_data_changes();
            };
            let projection = |player: &Player| {
                (
                    player.rest_state_like_cpp().clone(),
                    player.active_player_data_changes_mask().blocks().to_vec(),
                )
            };
            session.with_owned_player_mut_like_cpp(prepare).unwrap();
            let before = session.with_owned_player_like_cpp(projection);
            assert_eq!(
                session.take_represented_xp_rest_bonus_for_gain_like_cpp(40, ObjectGuid::EMPTY),
                (0, 0)
            );
            assert_eq!(session.with_owned_player_like_cpp(projection), before);
            let expected_award = session.fixture_take_xp_rest_bonus_like_cpp(40, victim);
            let expected = session.with_owned_player_like_cpp(projection);
            session.with_owned_player_mut_like_cpp(prepare).unwrap();
            assert_eq!(
                session.take_represented_xp_rest_bonus_for_gain_like_cpp(40, victim),
                expected_award
            );
            assert_eq!(session.with_owned_player_like_cpp(projection), expected);
            assert!(send_rx.is_empty());
        }
    }
}

#[test]
fn native_rest_bonus_set_and_add_match_previous_projection_for_active_and_detached_player() {
    let (mut session, _, send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for next_xp in [0, 100] {
            for bonus in [-1.0, 0.5, 42.5, 1e9, f32::INFINITY, f32::NAN] {
                for add in [false, true] {
                    let prepare = |player: &mut Player| {
                        player.set_next_level_xp(next_xp);
                        player.load_xp_rest_bonus_like_cpp(REST_STATE_RAF_LINKED_LIKE_CPP, 0.5);
                        player.clear_data_changes();
                    };
                    let projection = |player: &Player| {
                        (
                            player.rest_state_like_cpp().clone(),
                            player.active_data().rest_info[0].threshold,
                            player.active_data().rest_info[0].state_id,
                            player.active_player_data_changes_mask().blocks().to_vec(),
                        )
                    };
                    session.with_owned_player_mut_like_cpp(prepare).unwrap();
                    let expected_mask = session.fixture_set_xp_rest_bonus_like_cpp(if add {
                        0.5 + bonus
                    } else {
                        bonus
                    });
                    let expected = session.with_owned_player_like_cpp(projection).unwrap();
                    session.with_owned_player_mut_like_cpp(prepare).unwrap();
                    let mask = if add {
                        session.add_represented_xp_rest_bonus_like_cpp(bonus)
                    } else {
                        session.set_represented_xp_rest_bonus_like_cpp(bonus)
                    };
                    assert_eq!(mask, expected_mask);
                    assert_eq!(
                        session.with_owned_player_like_cpp(projection),
                        Some(expected)
                    );
                    assert!(send_rx.is_empty());
                }
            }
        }
    }
}

#[test]
fn rest_load_resets_transient_location_but_preserves_loaded_flags_and_unrelated_state() {
    let (mut session, _, send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for loaded_resting in [false, true] {
            for state_id in [
                0,
                REST_STATE_RESTED_LIKE_CPP,
                REST_STATE_NORMAL_LIKE_CPP,
                REST_STATE_RAF_LINKED_LIKE_CPP,
                255,
            ] {
                let flags = 0x8000_0000
                    | if loaded_resting {
                        PLAYER_FLAGS_RESTING_LIKE_CPP
                    } else {
                        0
                    };
                let original = wow_entities::PlayerRestState {
                    rest_xp: 9,
                    rest_bonus: 12.0,
                    rest_honor_bonus: 15.0,
                    rest_state: REST_STATE_RESTED_LIKE_CPP,
                    rest_flag_mask: REST_FLAG_IN_TAVERN_LIKE_CPP,
                    location_initialized: true,
                    defer_flag_sync: true,
                    deferred_flag_update_dirty: true,
                    inn_area_trigger_id: 123,
                    rest_time_secs: 456,
                    logout_time: Some(789),
                    logout_was_resting: true,
                    is_resting_now: true,
                };
                session
                    .with_owned_player_mut_like_cpp(|player| {
                        player.replace_rest_state_like_cpp(original.clone());
                        player.replace_all_player_flags(flags);
                        player.clear_data_changes();
                    })
                    .unwrap();
                session.load_represented_xp_rest_bonus_like_cpp(state_id, 42.5);
                let normalized = if WorldSession::valid_player_rest_state_like_cpp(state_id) {
                    state_id
                } else {
                    REST_STATE_NORMAL_LIKE_CPP
                };
                let expected = wow_entities::PlayerRestState {
                    rest_bonus: 42.5,
                    rest_state: normalized,
                    rest_flag_mask: 0,
                    location_initialized: false,
                    defer_flag_sync: false,
                    deferred_flag_update_dirty: false,
                    inn_area_trigger_id: 0,
                    rest_time_secs: 0,
                    ..original
                };
                session
                    .with_owned_player_like_cpp(|player| {
                        assert_eq!(player.rest_state_like_cpp(), &expected);
                        assert_eq!(player.data().player_flags, flags);
                        assert_eq!(player.active_data().rest_info[0].threshold, 42);
                        assert_eq!(player.active_data().rest_info[0].state_id, normalized);
                    })
                    .unwrap();
                assert!(send_rx.is_empty());
            }
        }
    }
}

#[test]
fn rest_mutation_runs_once_under_active_and_detached_owner_and_matches_old_projection() {
    let (mut session, _, send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for initialized in [false, true] {
            for mask in [0, REST_FLAG_IN_CITY_LIKE_CPP] {
                let state = wow_entities::PlayerRestState {
                    location_initialized: initialized,
                    rest_flag_mask: mask,
                    rest_bonus: 42.5,
                    rest_state: REST_STATE_RESTED_LIKE_CPP,
                    ..Default::default()
                };
                let prepare = |player: &mut Player| {
                    player.replace_rest_state_like_cpp(Default::default());
                    player.set_xp_rest_info_like_cpp(0, REST_STATE_NORMAL_LIKE_CPP);
                    player.set_player_flag(0x8000_0000 | PLAYER_FLAGS_RESTING_LIKE_CPP);
                    player.clear_data_changes();
                };
                let projection = |player: &Player| {
                    (
                        player.rest_state_like_cpp().clone(),
                        player.data().player_flags,
                        player.active_data().rest_info[0].threshold,
                        player.active_data().rest_info[0].state_id,
                        player.active_player_data_changes_mask().blocks().to_vec(),
                    )
                };
                session.with_owned_player_mut_like_cpp(prepare).unwrap();
                assert!(session.replace_player_rest_state_like_cpp(state.clone()));
                let expected = session.with_owned_player_like_cpp(projection).unwrap();
                session.with_owned_player_mut_like_cpp(prepare).unwrap();
                let calls = std::cell::Cell::new(0);
                assert_eq!(
                    session.mutate_player_rest_state_like_cpp(|rest| {
                        assert!(matches!(
                            manager.try_lock(),
                            Err(std::sync::TryLockError::WouldBlock)
                        ));
                        calls.set(calls.get() + 1);
                        *rest = state.clone();
                        123
                    }),
                    Some(123)
                );
                assert_eq!(calls.get(), 1);
                assert_eq!(
                    session.with_owned_player_like_cpp(projection),
                    Some(expected)
                );
                assert!(manager.try_lock().is_ok());
                assert!(send_rx.is_empty());
            }
        }
    }
}

#[test]
fn rest_queries_preserve_loaded_flags_and_initialized_masks_on_active_and_detached_owner() {
    let (mut session, _, send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for initialized in [false, true] {
            for mask in [0, REST_FLAG_IN_CITY_LIKE_CPP, REST_FLAG_IN_TAVERN_LIKE_CPP] {
                for loaded in [false, true] {
                    let flags = 0x8000_0000
                        | if loaded {
                            PLAYER_FLAGS_RESTING_LIKE_CPP
                        } else {
                            0
                        };
                    let state = wow_entities::PlayerRestState {
                        location_initialized: initialized,
                        rest_flag_mask: mask,
                        rest_bonus: 42.5,
                        ..Default::default()
                    };
                    session
                        .with_owned_player_mut_like_cpp(|player| {
                            player.set_player_flag(flags);
                            if !loaded {
                                player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
                            }
                            player.replace_rest_state_like_cpp(state.clone());
                            player.clear_data_changes();
                        })
                        .unwrap();
                    let resting = if initialized { mask != 0 } else { loaded };
                    assert_eq!(session.resolved_visible_resting_like_cpp(), Some(resting));
                    assert_eq!(
                        session.resolved_player_flags_for_rest_state_save_like_cpp(),
                        Some(
                            0x8000_0000
                                | if resting {
                                    PLAYER_FLAGS_RESTING_LIKE_CPP
                                } else {
                                    0
                                }
                        )
                    );
                    session
                        .with_owned_player_like_cpp(|player| {
                            assert_eq!(player.rest_state_like_cpp(), &state);
                            assert_eq!(player.data().player_flags, flags);
                            assert!(!player.active_player_data_changes_mask().is_any_set());
                        })
                        .unwrap();
                    assert!(send_rx.is_empty());
                }
            }
        }
    }
}

#[test]
fn rest_queries_reject_stale_and_missing_owner_even_with_populated_fixtures() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    session.represented_loaded_player_flags_like_cpp = Some(PLAYER_FLAGS_RESTING_LIKE_CPP);
    session.represented_rest_location_initialized_like_cpp = true;
    session.represented_rest_flag_mask_like_cpp = REST_FLAG_IN_CITY_LIKE_CPP;
    assert_eq!(session.resolved_visible_resting_like_cpp(), None);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 99.0);
    assert_eq!(session.player_rest_state_snapshot_like_cpp(), None);
    assert_eq!(
        session.mutate_player_rest_state_like_cpp(|_| panic!("stale owner")),
        None::<()>
    );
    assert_eq!(
        session.resolved_player_flags_for_rest_state_save_like_cpp(),
        None
    );
    session.canonical_map_manager = None;
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 99.0);
    assert_eq!(
        session.mutate_player_rest_state_like_cpp(|_| panic!("missing owner")),
        None::<()>
    );
    assert_eq!(session.resolved_visible_resting_like_cpp(), None);
    assert_eq!(
        session.resolved_player_flags_for_rest_state_save_like_cpp(),
        None
    );
}
