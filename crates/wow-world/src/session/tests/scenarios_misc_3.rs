//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn dynamic_object_values_snapshot_direct_player_vertical_only_separation_sends_like_cpp() {
    // C++ visibility distance is 2D (CanSeeOrDetect -> GetSightRange ->
    // IsWithinDist(obj, range, is3D=false); Object.cpp:1587-1609). A dynamic object at
    // the player's exact X/Y but a large Z offset is within the 2D sight range even
    // though its 3D distance exceeds it. A 3D check would wrongly drop it (the bug this
    // fixes); the 2D check sends the values update like C++.
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_530);
    let dynamic_guid = test_dynamic_object_guid(601_530, 50_531);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    let visibility_range = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .map()
        .visibility_range();
    // Same X/Y as the player (10, 20); Z far enough above that the 3D distance exceeds
    // the sight range while the 2D distance stays 0.
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601_530,
        Position::new(10.0, 20.0, 30.0 + visibility_range + 25.0, 0.0),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 38.5);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
}
#[tokio::test]
async fn dynamic_object_values_snapshot_player_shared_vision_no_seer_gate_sends_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_526);
    let source_guid = ObjectGuid::create_player(1, 50_527);
    let dynamic_guid = test_dynamic_object_guid(601_526, 50_528);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    let visibility_range = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .map()
        .visibility_range();
    let updated_position = Position::new(10.0 + visibility_range + 25.0, 20.0, 30.0, 0.0);
    add_canonical_test_player_on_map(
        &canonical,
        source_guid,
        Position::new(
            updated_position.x + 1.0,
            updated_position.y,
            updated_position.z,
            0.0,
        ),
        571,
        7,
    );
    add_shared_vision_viewer_to_canonical_target_like_cpp(
        &canonical,
        571,
        7,
        source_guid,
        viewer_guid,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        source_guid,
        601_526,
        updated_position,
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 41.5);
    session.represented_seer_guid_like_cpp = Some(viewer_guid);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&dynamic_guid)
    );
    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        0
    );
}
#[tokio::test]
async fn dynamic_object_values_snapshot_shared_vision_requires_source_lists_viewer_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_532);
    let source_guid = ObjectGuid::create_player(1, 50_533);
    let dynamic_guid = test_dynamic_object_guid(601_532, 50_534);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        viewer_guid,
        571,
        7,
    );
    let visibility_range = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .map()
        .visibility_range();
    let updated_position = Position::new(10.0 + visibility_range + 25.0, 20.0, 30.0, 0.0);
    add_canonical_test_player_on_map(
        &canonical,
        source_guid,
        Position::new(
            updated_position.x + 1.0,
            updated_position.y,
            updated_position.z,
            0.0,
        ),
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        source_guid,
        601_532,
        updated_position,
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 43.5);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    assert_eq!(
        session.send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp(),
        0
    );
    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&dynamic_guid)
    );
}
#[tokio::test]
async fn dynamic_object_values_snapshot_repeated_process_pending_does_not_resend_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_421);
    let dynamic_guid = test_dynamic_object_guid(601021, 50_422);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601021,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 37.5);
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    session.process_pending().await;
    session.process_pending().await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject]
    );
}
#[tokio::test]
async fn dynamic_object_values_snapshot_later_same_bytes_resends_on_new_generation_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_441);
    let dynamic_guid = test_dynamic_object_guid(601041, 50_442);

    configure_dynamic_object_values_snapshot_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601041,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    session.client_visible_guids_like_cpp.insert(dynamic_guid);

    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 37.5);
    let first_generation = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .update_calls()
        .len();
    session.process_pending().await;
    session.process_pending().await;
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject],
        "same stable snapshot/generation must be consumed only once"
    );

    // Force a real value transition, then transition back to the original
    // radius before session consumption. The latest stable snapshot has the
    // same VALUES bytes as the first send, but a later `Map::Update`
    // generation, matching C++ per-drain delivery semantics.
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 38.5);
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 37.5);
    let second_generation = canonical
        .lock()
        .unwrap()
        .find_map(571, 7)
        .unwrap()
        .update_calls()
        .len();
    assert!(
        second_generation > first_generation,
        "test must exercise a later ManagedMap update generation"
    );

    session.process_pending().await;
    session.process_pending().await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject],
        "identical DynamicObject VALUES bytes from a later map update generation must resend once"
    );
}
#[tokio::test]
async fn post_add_flushes_deferred_rest_flag_update_after_world_states_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 74_334);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PostAddRest".to_string(),
        player_position,
        571,
        1,
        1,
        10,
        0,
    ));
    session.set_player_zone_area_like_cpp(20, 102);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 20,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_LINKED_CHAT_LIKE_CPP,
        },
        wow_data::AreaTableEntry {
            id: 102,
            continent_id: 571,
            parent_area_id: 20,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_FACTION_AREA_LIKE_CPP, 0));
    let _ = drain_server_packet_bytes(&send_rx);

    session
        .send_initial_packets_after_add_to_map(player_guid, &player_position, 571, false)
        .await;

    let opcodes: Vec<_> = drain_server_packet_bytes(&send_rx)
        .iter()
        .filter_map(|packet| {
            (packet.len() >= 2).then(|| u16::from_le_bytes([packet[0], packet[1]]))
        })
        .collect();
    let init_world_states_index = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::InitWorldStates as u16)
        .expect("post-add InitWorldStates");
    let rest_update_indices: Vec<_> = opcodes
        .iter()
        .enumerate()
        .filter_map(|(index, opcode)| {
            (*opcode == ServerOpcodes::UpdateObject as u16).then_some(index)
        })
        .collect();
    assert_eq!(
        rest_update_indices.len(),
        1,
        "the deferred zone rest transition flushes as one final PlayerFlags update"
    );
    assert!(rest_update_indices[0] > init_world_states_index);
    assert_eq!(session.represented_rest_flag_mask_like_cpp, 0);
}
#[tokio::test]
async fn far_sight_process_pending_canonical_clear_resets_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_906);
    let stale_dynamic_object_guid = test_dynamic_object_guid(49_907, 49_907);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_state(SessionState::LoggedIn);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightProcessPendingClear".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.represented_seer_guid_like_cpp = Some(stale_dynamic_object_guid);
    session.last_visibility_pos = Some(player_position);

    session.process_pending().await;

    assert_eq!(session.represented_seer_guid_like_cpp(), Some(player_guid));
    assert_eq!(
        session.last_visibility_pos, None,
        "live tick consumption should invalidate the visibility throttle without requiring update_visibility"
    );
    let expected_farsight_clear = expected_active_player_farsight_object_values_update_like_cpp(
        player_guid,
        session.player_map_id_like_cpp(),
        ObjectGuid::EMPTY,
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets
            .iter()
            .filter(|bytes| *bytes == &expected_farsight_clear)
            .count(),
        1,
        "logged-in process_pending should emit exactly one represented VALUES-empty update"
    );
    assert_eq!(
        update_object_packet_count_like_cpp(&packets),
        1,
        "process_pending should not need update_visibility to consume canonical farsight clear"
    );
}
#[tokio::test]
async fn far_sight_process_pending_non_logged_in_keeps_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_908);
    let stale_dynamic_object_guid = test_dynamic_object_guid(49_909, 49_909);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightProcessPendingAuthed".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.represented_seer_guid_like_cpp = Some(stale_dynamic_object_guid);
    session.last_visibility_pos = Some(player_position);

    session.process_pending().await;

    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(stale_dynamic_object_guid),
        "non-logged-in process_pending must not consume canonical farsight clear"
    );
    assert_eq!(session.last_visibility_pos, Some(player_position));
    let expected_farsight_clear = expected_active_player_farsight_object_values_update_like_cpp(
        player_guid,
        session.player_map_id_like_cpp(),
        ObjectGuid::EMPTY,
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        !packets
            .iter()
            .any(|bytes| bytes == &expected_farsight_clear),
        "non-logged-in process_pending must not emit represented VALUES-empty update"
    );
}
#[tokio::test]
async fn far_sight_process_pending_non_empty_canonical_keeps_session_seer_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_910);
    let dynamic_object_guid = test_dynamic_object_guid(49_911, 49_911);
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);

    session.set_state(SessionState::LoggedIn);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "FarsightProcessPendingKeep".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    set_canonical_player_farsight_object_like_cpp(&canonical, player_guid, dynamic_object_guid);
    session.represented_seer_guid_like_cpp = Some(dynamic_object_guid);
    session.last_visibility_pos = Some(player_position);

    session.process_pending().await;

    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(dynamic_object_guid),
        "non-empty canonical FarsightObject must preserve represented m_seer"
    );
    assert_eq!(session.last_visibility_pos, Some(player_position));
    let expected_farsight_clear = expected_active_player_farsight_object_values_update_like_cpp(
        player_guid,
        session.player_map_id_like_cpp(),
        ObjectGuid::EMPTY,
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        !packets
            .iter()
            .any(|bytes| bytes == &expected_farsight_clear),
        "non-empty canonical FarsightObject must not emit represented VALUES-empty update"
    );
}
#[test]
fn npc_interaction_resolves_canonical_trainer_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let trainer_guid = test_creature_guid(10);
    let vendor_guid = test_creature_guid(11);
    let pet_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 502, 12);

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
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical map");
    session.set_player_faction_template_like_cpp(1);

    add_canonical_test_creature(
        &canonical,
        trainer_guid,
        500,
        Position::new(14.0, 0.0, 0.0, 0.0),
        wow_constants::unit::NPCFlags1::TRAINER.bits(),
    );
    add_canonical_test_creature(
        &canonical,
        vendor_guid,
        501,
        Position::new(14.0, 0.0, 0.0, 0.0),
        wow_constants::unit::NPCFlags1::VENDOR.bits(),
    );
    {
        let mut canonical = canonical.lock().unwrap();
        let managed = canonical.find_map_mut(571, 0).unwrap();
        managed
            .map_mut()
            .get_typed_creature_mut(vendor_guid)
            .unwrap()
            .set_npc_flags2_runtime_like_cpp(wow_constants::unit::NPCFlags2::TRADESKILL_NPC.bits());
    }
    {
        let mut pet = wow_entities::Pet::new(player_guid, wow_entities::PetType::Summon);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(502);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .set_map(571, 0)
            .unwrap();
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .relocate(Position::new(14.0, 1.0, 0.0, 0.0));
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .set_combat_reach(1.0);
        pet.creature_mut().unit_mut().set_level(80);
        pet.creature_mut().unit_mut().set_max_health(100);
        pet.creature_mut().unit_mut().set_health(100);
        pet.creature_mut().set_ai_identity_runtime(
            1,
            35,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        );
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        canonical
            .lock()
            .unwrap()
            .create_world_map(571, 0)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
            .unwrap();
    }

    assert_eq!(
        session.canonical_creature_access_like_cpp(trainer_guid),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 500,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::TRAINER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 500,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::TRAINER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            pet_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 502,
            position: Position::new(14.0, 1.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::TRAINER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
        })
        .unwrap();
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().world_mut().object_mut().add_to_world();
        })
        .unwrap();
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            vendor_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            vendor_guid,
            0,
            wow_constants::unit::NPCFlags2::TRADESKILL_NPC.bits(),
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 501,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::VENDOR.bits(),
            npc_flags2: wow_constants::unit::NPCFlags2::TRADESKILL_NPC.bits(),
            faction_template_id: 35,
            trainer_class: 0,
        })
    );

    session.set_taxi_flight_state_like_cpp(
        RepresentedTaxiFlightNodeLikeCpp {
            map_id: 571,
            position: Position::new(10.0, 0.0, 0.0, 0.0),
            teleport_flag: false,
        },
        None,
    );
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    assert!(session.replace_player_taxi_state_like_cpp(Default::default()));

    session.set_player_alive_like_cpp(false);
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    session.set_player_alive_like_cpp(true);

    {
        let mut canonical = canonical.lock().unwrap();
        let managed = canonical.find_map_mut(571, 0).unwrap();
        managed
            .map_mut()
            .get_typed_creature_mut(trainer_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .set_charmer(player_guid, true);
    }
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    {
        let mut canonical = canonical.lock().unwrap();
        let managed = canonical.find_map_mut(571, 0).unwrap();
        managed
            .map_mut()
            .get_typed_creature_mut(trainer_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .remove_charmer();
    }

    session.set_player_faction_template_like_cpp(1);
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(35, 35, 0, 0, 1),
            faction_template_entry(1, 1, 0, 0, 0),
        ]),
    ));
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
    let legacy_manager = shared_map_manager();
    legacy_manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            trainer_guid,
            500,
            Position::new(14.0, 0.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
    );
    session.set_map_manager(legacy_manager);
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None,
        "C++ GetNPCIfCanInteractWith rejects the canonical hostile NPC; legacy mirrors must not bypass that rejection"
    );

    {
        let mut canonical = canonical.lock().unwrap();
        let managed = canonical.find_map_mut(571, 0).unwrap();
        managed
            .map_mut()
            .get_typed_creature_mut(trainer_guid)
            .unwrap()
            .unit_mut()
            .set_unit_flags2_like_cpp(wow_constants::unit::UnitFlags2::INTERACT_WHILE_HOSTILE);
        let player = managed
            .map()
            .get_typed_player(player_guid)
            .expect("canonical Player owner remains active");
        assert!(player.unit().world().object().is_in_world());
        assert!(player.unit().is_alive());
        assert_eq!(player.unit().data().health, 100);
    }
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 500,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::TRAINER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );

    session.set_player_position_like_cpp(Position::new(40.0, 0.0, 0.0, 0.0));
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .relocate(Position::new(40.0, 0.0, 0.0, 0.0));
        })
        .unwrap();
    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            wow_constants::unit::NPCFlags1::TRAINER.bits(),
            0,
        ),
        None
    );
}
