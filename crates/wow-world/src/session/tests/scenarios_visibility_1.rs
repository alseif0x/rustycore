//! Session scenarios exercising the represented visibility responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn visible_object_values_update_command_sends_only_when_visible_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let object_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 9901);
    let packet_bytes = vec![0x34, 0x12, 0xAA, 0xBB];
    session.state = SessionState::LoggedIn;
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendVisibleObjectValuesUpdate(
            SendVisibleObjectValuesUpdateCommand {
                object_guid,
                map_id: 571,
                packet_bytes: packet_bytes.clone(),
                unit_values_update: None,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(send_rx.try_recv().is_err());

    session.client_visible_guids_like_cpp.insert(object_guid);
    session
        .session_command_tx()
        .try_send(SessionCommand::SendVisibleObjectValuesUpdate(
            SendVisibleObjectValuesUpdateCommand {
                object_guid,
                map_id: 571,
                packet_bytes: packet_bytes.clone(),
                unit_values_update: None,
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        send_rx.try_recv().expect("visible object update"),
        packet_bytes
    );
    assert!(send_rx.try_recv().is_err());
}
/// (1) Packet is NOT sent when source GUID is not in `client_visible_guids_like_cpp`.
/// Mirrors C++ `HaveAtClient` gate in `MessageDistDeliverer::SendPacket`.
#[tokio::test]
async fn send_if_visible_command_not_sent_when_source_not_visible_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1001);
    let packet_bytes = vec![0x11, 0x22, 0x33];
    session.state = SessionState::LoggedIn;
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(send_rx.try_recv().is_err(), "must not send if not visible");
}
/// (2) Packet IS sent when source GUID IS in `client_visible_guids_like_cpp`.
/// Mirrors C++ `HaveAtClient` positive path.
#[tokio::test]
async fn send_if_visible_command_sent_when_source_visible_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1002);
    let packet_bytes = vec![0x44, 0x55, 0x66];
    let manager = shared_map_manager();
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            source_guid,
            777,
            Position::ZERO,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );
    session.state = SessionState::LoggedIn;
    session.set_map_manager(manager);
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert_eq!(
        send_rx.try_recv().expect("must send when visible"),
        packet_bytes
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn send_realm_if_visible_uses_legacy_source_when_canonical_mirror_is_missing_like_cpp() {
    let (mut session, _, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(8);
    session.install_realm_send_channel_for_test(realm_tx);
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1013);
    let packet_bytes = vec![0x44, 0x55, 0x67];
    let manager = shared_map_manager();
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            source_guid,
            777,
            Position::ZERO,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(571, 0);
    session.state = SessionState::LoggedIn;
    session.set_map_manager(manager);
    session.set_canonical_map_manager(canonical);
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendRealmIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("ordinary realm-visible command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(
        realm_rx.try_recv().is_err(),
        "only provenance-marked legacy-source commands may cross the map-manager boundary"
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::SendRealmIfVisibleFromLegacySourceLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("realm-visible command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(realm_rx.try_recv().expect("realm packet"), packet_bytes);
    assert!(instance_rx.try_recv().is_err());
}
/// (5) Packet is NOT sent when session is not LoggedIn.
/// Mirrors C++ player-object-required guard at the top of dispatch.
#[tokio::test]
async fn send_if_visible_command_rejected_when_not_logged_in_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1005);
    // state is NOT LoggedIn (default after make_session is Authed)
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 0,
                packet_bytes: vec![0x99],
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "must not send when session is not LoggedIn"
    );
}
#[test]
fn visibility_gated_durable_packets_follow_older_general_refresh_like_cpp() {
    let (session, _, _) = make_session();
    let creature_guid = test_creature_guid(1012);
    session
        .durable_creature_runtime_commands_like_cpp
        .lock()
        .unwrap()
        .publish_send_if_visible_like_cpp(SendIfVisibleLikeCppCommand {
            queued_at: Instant::now(),
            source_guid: creature_guid,
            map_id: 571,
            instance_id: 0,
            packet_bytes: vec![1, 2, 3],
        });
    session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id: 571,
                instance_id: 0,
            },
        ))
        .expect("refresh queued");

    let commands = session.drain_session_commands();
    assert!(matches!(
        commands.first(),
        Some(SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(_))
    ));
    assert!(matches!(
        commands.get(1),
        Some(SessionCommand::SendIfVisibleLikeCpp(_))
    ));
}
#[test]
fn visibility_barrier_preserves_entire_durable_suffix_fifo_like_cpp() {
    let (session, _, _) = make_session();
    let creature_guid = test_creature_guid(1013);
    let victim_guid = ObjectGuid::create_player(1, 7007);
    {
        let mut durable = session
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .unwrap();
        assert!(
            durable.publish_attack_start_like_cpp(CreatureAttackStartLikeCppCommand {
                attacker_guid: creature_guid,
                victim_guid,
                previous_victim_guid: None,
                map_id: 571,
                instance_id: 0,
                packet_already_broadcast: false,
            },)
        );
        for packet_bytes in [vec![0x37, 0x2C], vec![0x36, 0x2C]] {
            assert!(
                durable.publish_send_if_visible_like_cpp(SendIfVisibleLikeCppCommand {
                    queued_at: Instant::now(),
                    source_guid: creature_guid,
                    map_id: 571,
                    instance_id: 0,
                    packet_bytes,
                },)
            );
        }
        assert!(
            durable.publish_melee_damage_like_cpp(ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid: creature_guid,
                victim_guid,
                map_id: 571,
                instance_id: 0,
                damage: 3,
                over_damage: -1,
                target_level: 80,
                victim_health_after: 97,
                victim_health_state_revision_after: 1,
            },)
        );
    }
    session
        .session_command_tx()
        .try_send(SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id: 571,
                instance_id: 0,
            },
        ))
        .expect("refresh queued");

    let commands = session.drain_session_commands();
    assert_eq!(commands.len(), 5);
    assert!(matches!(
        &commands[0],
        SessionCommand::CreatureAttackStartLikeCpp(_)
    ));
    assert!(matches!(
        &commands[1],
        SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(_)
    ));
    let SessionCommand::SendIfVisibleLikeCpp(start) = &commands[2] else {
        panic!("START must remain first in the deferred durable suffix");
    };
    let SessionCommand::SendIfVisibleLikeCpp(go) = &commands[3] else {
        panic!("GO must remain immediately behind START");
    };
    assert_eq!(start.packet_bytes, vec![0x37, 0x2C]);
    assert_eq!(go.packet_bytes, vec![0x36, 0x2C]);
    assert!(matches!(
        &commands[4],
        SessionCommand::ApplyCreatureMeleeDamageLikeCpp(_)
    ));
}
#[test]
fn visible_dynamic_objects_skip_not_in_world_canonical_objects_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 49_612);
    let position = Position::new(100.0, 200.0, 30.0, 0.0);
    let in_world_guid = test_dynamic_object_guid(49_612, 49_612);
    let removed_guid = test_dynamic_object_guid(49_613, 49_613);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        in_world_guid,
        player_guid,
        49_612,
        position,
        571,
        0,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        removed_guid,
        player_guid,
        49_613,
        Position::new(101.0, 201.0, 30.0, 0.0),
        571,
        0,
    );
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_dynamic_object_mut(removed_guid)
        .unwrap()
        .world_mut()
        .object_mut()
        .remove_from_world();

    let visible = session
        .visible_dynamic_objects_from_canonical_map_like_cpp(571, &position, 100.0)
        .expect("canonical map");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid, in_world_guid);
}
#[test]
fn add_farsight_session_caller_preserves_destination_radius_duration_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90);
    let spell_id = 12_345;
    let destination = Position::new(100.0, 200.0, 30.0, 1.5);

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
        "Farseer".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: 1,
            attributes: [0; 15],
            difficulty_id: 0,
            casting_time_index: 0,
            duration_index: 7,
            range_index: 0,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_id: spell_id as u32,
        },
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 7,
            duration: -5000,
            duration_per_level: 0,
            max_duration: 0,
        },
    ])));
    session.set_spell_radius_store(Arc::new(wow_data::SpellRadiusStore::from_entries([
        wow_data::SpellRadiusEntry {
            id: 11,
            radius: 0.0,
            radius_per_level: 0.0,
            radius_min: 0.0,
            radius_max: 25.0,
        },
    ])));

    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_FARSIGHT,
        effect_radius_index_1: 11,
        ..Default::default()
    };
    let target_data = SpellTargetData {
        flags: 0x40,
        dst_location: Some(wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::EMPTY,
            position: destination,
        }),
        ..Default::default()
    };

    let outcome = session
        .apply_effect_add_farsight_like_cpp(spell_id, &effect, &target_data, 678, 1500)
        .expect("canonical map/player/destination should create farsight object");

    assert_eq!(
        outcome.status,
        wow_map::map::FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_guid = outcome
        .dynamic_object_guid
        .expect("created farsight outcome should carry dynamic object guid");
    let guard = canonical.lock().unwrap();
    let dynamic_object = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_dynamic_object(dynamic_guid)
        .expect("created farsight dynamic object should be canonical map-owned");
    assert_eq!(dynamic_object.world().position(), destination);
    assert_eq!(dynamic_object.radius(), 25.0);
    assert_eq!(dynamic_object.duration_ms(), 5000);
    assert_eq!(dynamic_object.caster_guid(), player_guid);
    assert_eq!(dynamic_object.spell_id(), spell_id);
    assert_eq!(dynamic_object.data().spell_visual_id, 678);
    assert_eq!(dynamic_object.data().cast_time_ms, 1500);
}
#[test]
fn add_farsight_session_caller_requires_destination_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_FARSIGHT,
        effect_radius_index_1: 11,
        ..Default::default()
    };

    assert!(
        session
            .apply_effect_add_farsight_like_cpp(
                12_345,
                &effect,
                &SpellTargetData::default(),
                678,
                1500,
            )
            .is_none()
    );
}
#[tokio::test]
async fn dynamic_object_values_snapshot_direct_player_out_of_range_keeps_visible_no_send_like_cpp()
{
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_517);
    let dynamic_guid = test_dynamic_object_guid(601_517, 50_518);

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
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601_517,
        Position::new(10.0 + visibility_range + 25.0, 20.0, 30.0, 0.0),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 38.5);
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
async fn dynamic_object_values_snapshot_dynamic_object_seer_near_same_phase_sends_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_519);
    let dynamic_guid = test_dynamic_object_guid(601_519, 50_520);
    let seer_guid = test_dynamic_object_guid(601_520, 50_521);

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
    let updated_position = Position::new(10.0 + visibility_range + 25.0, 20.0, 30.0, 0.0);
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_guid,
        player_guid,
        601_519,
        updated_position,
        571,
        7,
    );
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        seer_guid,
        player_guid,
        601_520,
        Position::new(
            updated_position.x + 1.0,
            updated_position.y,
            updated_position.z,
            0.0,
        ),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 39.5);
    session.represented_seer_guid_like_cpp = Some(seer_guid);
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
}
#[tokio::test]
async fn dynamic_object_values_snapshot_shared_vision_phase_mismatch_no_send_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let viewer_guid = ObjectGuid::create_player(1, 50_535);
    let source_guid = test_creature_guid(50_536);
    let dynamic_guid = test_dynamic_object_guid(601_535, 50_537);

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
    add_canonical_test_creature_on_map(
        &canonical,
        source_guid,
        601_535,
        Position::new(
            updated_position.x + 1.0,
            updated_position.y,
            updated_position.z,
            0.0,
        ),
        0,
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
        viewer_guid,
        601_535,
        updated_position,
        571,
        7,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(571, 7).unwrap().map_mut();
        *map.get_typed_creature_mut(source_guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([10]);
        *map.get_typed_dynamic_object_mut(dynamic_guid)
            .unwrap()
            .world_mut()
            .phase_shift_mut() = PhaseShift::from_phases([20]);
    }
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 44.5);
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
async fn dynamic_object_values_snapshot_non_visible_emits_nothing_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 50_411);
    let dynamic_guid = test_dynamic_object_guid(601011, 50_412);

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
        601011,
        Position::new(11.0, 21.0, 31.0, 0.0),
        571,
        7,
    );
    prepare_dynamic_object_values_snapshot_like_cpp(&canonical, 571, 7, dynamic_guid, 37.5);

    session.process_pending().await;

    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[test]
fn add_farsight_set_viewpoint_target_visibility_sends_far_dynamic_object_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 50_000);
    let dynamic_object_guid = test_dynamic_object_guid(50_000, 50_000);
    let player_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(10_000.0, 20_000.0, 30.0, 1.5);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Farseer".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_object_guid,
        player_guid,
        50_000,
        destination,
        571,
        0,
    );
    assert!(
        !destination.is_within_dist(&player_position, 200.0),
        "test target must be outside the normal represented visibility radius"
    );

    let create_data = {
        let guard = canonical.lock().unwrap();
        let dynamic_object = guard
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_dynamic_object(dynamic_object_guid)
            .expect("canonical typed DynamicObject should exist");
        crate::session_rules::dynamic_object_create_data_from_canonical_like_cpp(
            dynamic_object_guid,
            dynamic_object,
        )
    };
    let outcome = wow_map::map::FarsightDynamicObjectCreateOutcomeLikeCpp {
        status: wow_map::map::FarsightDynamicObjectCreateStatusLikeCpp::Created,
        caster_player_guid: player_guid,
        dynamic_object_guid: Some(dynamic_object_guid),
        low_guid: Some(50_000),
        add_to_map: None,
        caster_viewpoint: Some(wow_map::map::DynamicObjectCasterViewpointOutcomeLikeCpp {
            player_guid,
            dynamic_object_guid,
            apply: true,
            status: wow_map::map::DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved,
            dynamic_object_viewpoint_toggled: true,
            player_set_viewpoint: wow_map::map::PlayerSetViewpointOutcomeLikeCpp {
                player_guid,
                target_guid: dynamic_object_guid,
                apply: true,
                status: wow_map::map::PlayerSetViewpointStatusLikeCpp::Applied,
                set_world_object: None,
                update_visibility_requested: true,
                set_seer_requested: true,
            },
        }),
    };

    assert!(session.consume_add_farsight_set_seer_outcome_like_cpp(&outcome));
    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(dynamic_object_guid)
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&dynamic_object_guid),
        "direct C++ UpdateVisibilityOf(target) consumption should mark far DynamicObject visible"
    );
    let expected_create = expected_dynamic_object_create_packet_like_cpp(
        session.player_map_id_like_cpp(),
        create_data,
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        packets.iter().any(|bytes| bytes == &expected_create),
        "SetViewpoint(apply=true) should send a target-only DynamicObject create packet"
    );
}
#[test]
fn set_viewpoint_target_visibility_already_visible_sends_no_duplicate_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 50_010);
    let dynamic_object_guid = test_dynamic_object_guid(50_010, 50_010);
    let player_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let dynamic_object_position = Position::new(10_000.0, 20_000.0, 30.0, 1.5);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Farseer".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    add_canonical_test_dynamic_object_on_map(
        &canonical,
        dynamic_object_guid,
        player_guid,
        50_010,
        dynamic_object_position,
        571,
        0,
    );
    session
        .client_visible_guids_like_cpp
        .insert(dynamic_object_guid);

    assert!(!session.send_set_viewpoint_target_visibility_like_cpp(dynamic_object_guid));
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&dynamic_object_guid)
    );
}
