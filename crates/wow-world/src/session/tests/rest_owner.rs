//! Rest visibility/save queries borrow one generation-checked Player.

use super::*;

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
    assert_eq!(
        session.resolved_player_flags_for_rest_state_save_like_cpp(),
        None
    );
    session.canonical_map_manager = None;
    assert_eq!(session.resolved_visible_resting_like_cpp(), None);
    assert_eq!(
        session.resolved_player_flags_for_rest_state_save_like_cpp(),
        None
    );
}
