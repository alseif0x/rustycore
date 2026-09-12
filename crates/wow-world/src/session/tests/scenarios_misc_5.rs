//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn canonical_player_resurrection_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_577);
    let resurrecter = ObjectGuid::create_player(1, 5_578);
    let healer_guid = test_creature_guid(578);
    let request = wow_entities::PlayerResurrectionRequestLikeCpp {
        resurrecter,
        map_id: 571,
        position: Position::new(3701.0, 1501.0, 121.0, 0.5),
        health: 450,
        mana: 120,
        aura: 0,
    };

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ResurrectionOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(session.set_represented_resurrection_request_like_cpp(request));
    assert!(session.schedule_represented_resurrection_after_teleport_like_cpp(request));
    session.add_represented_self_res_spell_like_cpp(21169);
    assert!(session.set_area_spirit_healer_guid_like_cpp(healer_guid));
    assert!(
        session
            .with_owned_player_mut_like_cpp(|player| {
                player.resurrection_state_mut_like_cpp().death_timer_active = true;
            })
            .is_some()
    );
    assert_eq!(
        session.player_resurrection_state_snapshot_like_cpp(),
        Some(wow_entities::PlayerResurrectionStateLikeCpp {
            request: Some(request),
            delayed_after_teleport: Some(request),
            self_res_spells: BTreeSet::from([21169]),
            death_timer_active: true,
            area_spirit_healer_guid: healer_guid,
        })
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert!(session.represented_resurrection_requested_by_like_cpp(resurrecter));
    assert!(session.has_represented_self_res_spell_like_cpp(21169));
    assert_eq!(
        session.area_spirit_healer_guid_like_cpp(),
        Some(healer_guid)
    );

    let replacement_resurrecter = ObjectGuid::create_player(1, 5_579);
    let replacement_healer = test_creature_guid(579);
    let replacement_request = wow_entities::PlayerResurrectionRequestLikeCpp {
        resurrecter: replacement_resurrecter,
        map_id: 1,
        position: Position::new(1.0, 2.0, 3.0, 0.0),
        health: 99,
        mana: 33,
        aura: 7,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_resurrection_request_like_cpp(replacement_request);
    replacement
        .resurrection_state_mut_like_cpp()
        .self_res_spells
        .insert(20608);
    replacement
        .resurrection_state_mut_like_cpp()
        .area_spirit_healer_guid = replacement_healer;
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.player_resurrection_state_snapshot_like_cpp(), None);
    assert!(!session.set_represented_resurrection_request_like_cpp(request));
    assert!(!session.schedule_represented_resurrection_after_teleport_like_cpp(request));
    session.add_represented_self_res_spell_like_cpp(21169);
    assert!(!session.set_area_spirit_healer_guid_like_cpp(healer_guid));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .resurrection_state_like_cpp()
                .clone()),
        Some(wow_entities::PlayerResurrectionStateLikeCpp {
            request: Some(replacement_request),
            delayed_after_teleport: None,
            self_res_spells: BTreeSet::from([20608]),
            death_timer_active: false,
            area_spirit_healer_guid: replacement_healer,
        })
    );
}
#[test]
fn canonical_player_taxi_and_titles_follow_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_564);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TravelOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    let owned_taxi = wow_entities::PlayerTaxiState::from_represented_parts_like_cpp(
        vec![10, 20],
        Some(wow_entities::PlayerTaxiFlightStateLikeCpp {
            current_node: wow_entities::PlayerTaxiFlightNodeLikeCpp {
                map_id: 571,
                position: Position::new(1.0, 2.0, 3.0, 0.0),
                teleport_flag: false,
            },
            node_after_teleport: None,
        }),
        UnitFlags::ON_TAXI.bits(),
        true,
    );

    assert!(session.replace_player_taxi_state_like_cpp(owned_taxi.clone()));
    session.represented_learn_title_like_cpp(42);
    session.represented_set_chosen_title_like_cpp(42);
    assert_eq!(
        session.player_taxi_state_snapshot_like_cpp(),
        Some(owned_taxi.clone())
    );
    assert!(session.represented_has_title_like_cpp(42));
    assert_eq!(session.represented_chosen_title_like_cpp(), 42);
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.player_taxi_state_snapshot_like_cpp(),
        Some(owned_taxi.clone())
    );
    assert!(session.represented_has_title_like_cpp(42));

    let replacement_taxi =
        wow_entities::PlayerTaxiState::from_represented_parts_like_cpp(vec![90], None, 0, false);
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.replace_taxi_state_like_cpp(replacement_taxi.clone());
    replacement.learn_title_like_cpp(99);
    replacement.set_chosen_title_like_cpp(99);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.player_taxi_state_snapshot_like_cpp(), None);
    assert!(!session.represented_has_title_like_cpp(42));
    assert!(!session.replace_player_taxi_state_like_cpp(owned_taxi));
    session.represented_learn_title_like_cpp(42);
    session.represented_set_chosen_title_like_cpp(42);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.taxi_state_like_cpp().clone(),
                player.has_title_like_cpp(99),
                player.has_title_like_cpp(42),
                player.data().player_title,
            )),
        Some((replacement_taxi, true, false, 99))
    );
}
#[test]
fn canonical_player_rest_manager_follows_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_565);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RestOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);
    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0));
    let active = session
        .player_rest_state_snapshot_like_cpp()
        .expect("active rest owner");
    assert_eq!(active.rest_bonus_like_cpp(), 70.0);
    assert_eq!(active.rest_state_like_cpp(), REST_STATE_RESTED_LIKE_CPP);
    assert!(active.has_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP));
    assert!(active.is_location_initialized_like_cpp());

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert!(session.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP));
    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP, 77));
    let detached = session
        .player_rest_state_snapshot_like_cpp()
        .expect("detached rest owner");
    assert_eq!(detached.rest_bonus_like_cpp(), 70.0);
    assert!(detached.has_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP));
    assert_eq!(detached.inn_trigger_id_like_cpp(), 77);

    let replacement_state = wow_entities::PlayerRestState::from_represented_parts_like_cpp(
        REST_STATE_NORMAL_LIKE_CPP,
        500.0,
        REST_FLAG_IN_TAVERN_LIKE_CPP,
        true,
        false,
        false,
        77,
        1234,
    );
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.replace_rest_state_like_cpp(replacement_state.clone());
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.player_rest_state_snapshot_like_cpp(), None);
    assert_eq!(session.add_represented_xp_rest_bonus_like_cpp(25.0), 0);
    assert!(!session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0));
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 1.0);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.rest_state_like_cpp().clone()
            }),
        Some(replacement_state)
    );
}
#[test]
fn canonical_player_homebind_follows_detached_and_stale_handle_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_565);
    let original = RepresentedHomebindLikeCpp {
        map_id: 571,
        area_id: 67,
        position: Position::new(3700.0, 1500.0, 120.0, 0.5),
    };
    let replacement_homebind = RepresentedHomebindLikeCpp {
        map_id: 0,
        area_id: 12,
        position: Position::new(-8949.0, -132.0, 84.0, 1.0),
    };

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "HomebindOwner".to_string(),
        original.position,
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    assert!(session.set_represented_homebind_like_cpp(original));
    assert_eq!(session.represented_homebind_like_cpp(), Some(original));

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.represented_homebind_like_cpp(), Some(original));

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().homebind = Some(replacement_homebind);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.represented_homebind_like_cpp(), None);
    assert!(!session.set_represented_homebind_like_cpp(original));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().homebind
            }),
        Some(Some(replacement_homebind))
    );
}
#[test]
fn canonical_player_cinematic_state_follows_detached_and_stale_handle_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_566);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "CinematicOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    session.set_represented_cinematic_like_cpp_for_test(Some(444));
    session.set_represented_movie_like_cpp_for_test(Some(177));
    assert_eq!(session.represented_cinematic_like_cpp(), Some(444));
    assert_eq!(session.represented_movie_like_cpp(), Some(177));

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(session.represented_cinematic_like_cpp(), Some(444));
    assert_eq!(session.represented_movie_like_cpp(), Some(177));

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    let replacement_cinematic = {
        let cinematic = &mut replacement.gameplay_state_mut().cinematic;
        cinematic.begin_cinematic_like_cpp(900, [1, 2, 3, 0, 0, 0, 0, 0]);
        assert_eq!(cinematic.next_cinematic_camera_like_cpp(), Some(1));
        assert_eq!(cinematic.next_cinematic_camera_like_cpp(), Some(2));
        cinematic.set_movie_like_cpp(Some(901));
        *cinematic
    };
    assert_eq!(replacement_cinematic.camera_index_like_cpp(), 1);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.represented_cinematic_like_cpp(), None);
    assert_eq!(session.represented_movie_like_cpp(), None);
    session.set_represented_cinematic_like_cpp_for_test(Some(1));
    session.set_represented_movie_like_cpp_for_test(Some(2));
    session.complete_represented_cinematic_like_cpp();
    session.complete_represented_movie_like_cpp();
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().cinematic
            }),
        Some(replacement_cinematic)
    );
}
#[test]
fn canonical_player_trait_config_authority_follows_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_571);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TraitConfigOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(
        session.complete_represented_trait_config_authority_load_like_cpp(
            [(1, 1, 71, 1), (2, 1, 72, 1), (3, 1, 73, 1)],
            true,
        )
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    let detached = session
        .player_spell_runtime_snapshot_like_cpp()
        .expect("detached canonical trait-config owner");
    assert!(detached.trait_config_rows_complete);
    assert!(detached.trait_entry_rows_complete);
    assert!(detached.trait_entry_rows_empty);
    assert_eq!(detached.trait_config_rows.len(), 3);

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert!(session.player_spell_runtime_snapshot_like_cpp().is_none());
    assert!(
        !session.complete_represented_trait_config_authority_load_like_cpp(
            [(9, 1, 71, 1), (10, 1, 72, 1), (11, 1, 73, 1)],
            true,
        ),
        "a stale Session generation must not publish trait authority"
    );
    let replacement_runtime = canonical
        .lock()
        .unwrap()
        .with_player_like_cpp(replacement_handle, |player| {
            player.spell_runtime_like_cpp().clone()
        })
        .expect("replacement spell owner");
    assert!(replacement_runtime.trait_config_rows_like_cpp().is_empty());
    assert!(!replacement_runtime.trait_config_rows_complete_like_cpp());
    assert!(!replacement_runtime.trait_entry_rows_complete_like_cpp());
    assert!(!replacement_runtime.trait_entry_rows_empty_like_cpp());
}
#[test]
fn canonical_player_collection_authority_follows_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_572);
    let temporary_item_guid = ObjectGuid::create_item(1, 9_001);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "CollectionOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");

    assert!(
        session
            .mutate_player_collection_state_like_cpp(|collections| {
                collections.add_mount_like_cpp(100, 1);
                collections.add_heirloom_like_cpp(
                    200,
                    wow_entities::PlayerAccountHeirloomDataLikeCpp {
                        flags: 2,
                        bonus_id: 3,
                    },
                );
                collections.add_toy_like_cpp(300, 4);
                collections.install_appearance_collection_like_cpp(
                    std::collections::HashSet::from([400]),
                    vec![5],
                    std::collections::HashMap::from([(
                        400,
                        wow_entities::PlayerFavoriteAppearanceStateLikeCpp::New,
                    )]),
                );
                collections.add_temporary_item_appearance_like_cpp(401, temporary_item_guid);
                collections
                    .replace_transmog_illusions_like_cpp(std::collections::HashSet::from([500]));
            })
            .is_some()
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    let detached = session
        .player_collection_state_snapshot_like_cpp()
        .expect("detached canonical collection owner");
    assert_eq!(detached.mounts_like_cpp().get(&100), Some(&1));
    assert_eq!(
        detached
            .heirlooms_like_cpp()
            .get(&200)
            .map(|data| data.bonus_id),
        Some(3)
    );
    assert_eq!(detached.toys_like_cpp().get(&300), Some(&4));
    assert!(detached.item_appearances_like_cpp().contains(&400));
    assert_eq!(detached.item_appearance_blocks_like_cpp(), vec![5]);
    assert_eq!(
        detached.temporary_item_appearances_like_cpp().get(&401),
        Some(&HashSet::from([temporary_item_guid]))
    );
    assert!(
        detached
            .favorite_item_appearances_like_cpp()
            .contains_key(&400)
    );
    assert!(detached.transmog_illusions_like_cpp().contains(&500));

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert!(
        session
            .player_collection_state_snapshot_like_cpp()
            .is_none()
    );
    assert!(
        session
            .mutate_player_collection_state_like_cpp(|collections| {
                collections.add_mount_like_cpp(999, 0);
            })
            .is_none(),
        "a stale Session generation must not mutate collection authority"
    );
    let replacement_collections = canonical
        .lock()
        .unwrap()
        .with_player_like_cpp(replacement_handle, |player| {
            player.gameplay_state().collections.clone()
        })
        .expect("replacement collection owner");
    assert_eq!(
        replacement_collections,
        wow_entities::PlayerCollectionStateLikeCpp::default()
    );
}
#[test]
fn canonical_access_requirement_min_level_sends_notification_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 91);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessLevelNotify".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        79,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    install_access_notification_stores_like_cpp(&mut session);
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.level_min = 80;
    install_access_requirement_store_like_cpp(&mut session, requirement);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("missing level notification"),
        PrintNotification {
            notify_text: "You must be at least level 80 to enter.".to_string(),
        }
        .to_bytes()
    );
    assert_eq!(
        send_rx.try_recv().expect("missing level abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ERROR_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_access_requirement_ignore_level_config_bypasses_level_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 84);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessIgnoreLevel".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        1,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    session.set_instance_ignore_level_like_cpp(true);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.level_min = 80;
    install_access_requirement_store_like_cpp(&mut session, requirement);

    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_access_requirement_current_player_achievement_matches_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 88);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AccessAchievement".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    let mut requirement = access_requirement_like_cpp(631, 3);
    requirement.completed_achievement = 9001;
    install_access_requirement_store_like_cpp(&mut session, requirement);

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("missing achievement abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_ERROR_LIKE_CPP,
        }
        .to_bytes()
    );

    session.load_completed_achievement_rows_like_cpp([9001, 9001, 0]);
    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn initial_canonical_player_sets_display_mount_collision_shape_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 42);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "Tester".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        10,
        0,
    ));
    configure_player_shape_mount_collision_stores_like_cpp(&mut session);
    session.update_player_collision_height_like_cpp();
    session.player_mount_display_id_like_cpp = 4321;
    session.update_player_collision_height_like_cpp();
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical map");

    let native_display_id = crate::handlers::character::default_display_id(
        session.player_race_like_cpp(),
        session.player_gender_like_cpp(),
    );
    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert_eq!(player.unit().data().display_id, native_display_id as i32);
    assert_eq!(
        player.unit().data().native_display_id,
        native_display_id as i32
    );
    assert_eq!(player.unit().data().mount_display_id, 4321);
    assert!(
        (player.unit().collision_height_like_cpp() - session.player_collision_height_like_cpp)
            .abs()
            < 0.0001
    );
    assert!(
        (player.unit().world().collision_height_like_cpp()
            - session.player_collision_height_like_cpp)
            .abs()
            < 0.0001
    );
    assert!(
        (player.unit().world().object().scale() - session.player_object_scale_like_cpp).abs()
            < 0.0001
    );
}
