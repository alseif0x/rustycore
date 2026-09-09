//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn area_trigger_tavern_script_continues_unless_dispatcher_consumes_like_cpp() {
    let (mut session, _, _) = make_session();
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));
    session.set_area_trigger_db2_store(Arc::new(wow_data::AreaTriggerDb2Store::from_entries([
        test_db2_area_trigger_like_cpp(42, 1, Position::new(10.0, 20.0, 30.0, 0.0)),
    ])));
    session.set_player_map_position_like_cpp(1, Position::new(11.0, 20.0, 30.0, 0.0));
    let mut script_names = wow_data::ScriptNameInternerLikeCpp::new();
    let scripts = wow_data::AreaTriggerScriptStoreLikeCpp::from_rows_like_cpp(
        [wow_data::AreaTriggerScriptRowLikeCpp {
            entry: 42,
            script_name: "at_test_tavern".to_string(),
        }],
        |entry| entry == 42,
        &mut script_names,
    );
    session.set_area_trigger_script_store(Arc::new(scripts.store));

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(42);
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();
    session.handle_area_trigger(pkt).await;

    assert!(
        session.represented_is_resting_like_cpp(),
        "a DB binding alone is not the C++ callback return value"
    );

    let mut leave = WorldPacket::new_empty();
    leave.write_uint32(42);
    leave.write_bit(false);
    leave.write_bit(false);
    leave.flush_bits();
    session.handle_area_trigger(leave).await;
    assert!(!session.represented_is_resting_like_cpp());

    session.set_area_trigger_script_dispatcher_like_cpp(Arc::new(
        |_session, _script_id, _trigger_id, _entered| true,
    ));
    let mut consumed = WorldPacket::new_empty();
    consumed.write_uint32(42);
    consumed.write_bit(true);
    consumed.write_bit(false);
    consumed.flush_bits();
    session.handle_area_trigger(consumed).await;

    assert!(!session.represented_is_resting_like_cpp());
}
#[test]
fn tavern_rest_revalidation_clears_stale_trigger_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1B2);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.ensure_login_player_controller_like_cpp(
        guid,
        "RestRecheck".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    session.set_area_trigger_db2_store(Arc::new(wow_data::AreaTriggerDb2Store::from_entries([
        test_db2_area_trigger_like_cpp(42, 1, Position::new(10.0, 20.0, 30.0, 0.0)),
    ])));
    session.set_player_map_position_like_cpp(1, Position::new(11.0, 20.0, 30.0, 0.0));

    assert!(session.set_represented_tavern_resting_like_cpp(42, true));
    assert!(session.represented_is_resting_like_cpp());
    assert_eq!(session.represented_inn_area_trigger_id_like_cpp, 42);
    assert!(
        session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(false)
    );

    session.set_player_map_position_like_cpp(1, Position::new(100.0, 20.0, 30.0, 0.0));
    session.revalidate_represented_tavern_resting_like_cpp();

    assert!(!session.represented_is_resting_like_cpp());
    assert_eq!(session.represented_inn_area_trigger_id_like_cpp, 0);
    assert!(
        !session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(true)
    );
}
#[test]
fn give_xp_runtime_dispatches_mutable_script_before_rested_bonus_like_cpp() {
    let (mut session, _, send_rx) = make_session_with_give_player_xp_hook();
    let player = ObjectGuid::create_player(1, XP_HOOK_DOUBLE_PLAYER_COUNTER);
    let victim = test_creature_guid(0xE1D0);
    session.set_player_guid(Some(player));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 150.0);
    install_tapped_xp_victim_like_cpp(&mut session, victim);
    let calls_before = XP_HOOK_DOUBLE_CALLS.load(AtomicOrdering::SeqCst);

    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));

    assert_eq!(
        XP_HOOK_DOUBLE_CALLS.load(AtomicOrdering::SeqCst),
        calls_before + 1
    );
    assert_eq!(session.player_xp_like_cpp(), 200);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 50.0);
    let packets = drain_server_packet_bytes(&send_rx);
    let mut packet = WorldPacket::from_bytes(
        packets
            .iter()
            .find(|bytes| {
                WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::LogXpGain)
            })
            .expect("script-adjusted GiveXP sends LogXPGain"),
    );
    packet.skip_opcode();
    assert_eq!(packet.read_packed_guid().unwrap(), victim);
    assert_eq!(packet.read_int32().unwrap(), 200);
    assert_eq!(packet.read_uint8().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 100);
    assert!((packet.read_float().unwrap() - 1.0).abs() < f32::EPSILON);
    assert_eq!(packet.remaining(), 0);
}
#[test]
fn give_xp_runtime_does_not_reapply_zero_guard_after_script_like_cpp() {
    let (mut session, _, send_rx) = make_session_with_give_player_xp_hook();
    let player = ObjectGuid::create_player(1, XP_HOOK_ZERO_PLAYER_COUNTER);
    let victim = test_creature_guid(0xE1D1);
    session.set_player_guid(Some(player));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 0.5);
    install_tapped_xp_victim_like_cpp(&mut session, victim);
    let calls_before = XP_HOOK_ZERO_CALLS.load(AtomicOrdering::SeqCst);

    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));

    assert_eq!(
        XP_HOOK_ZERO_CALLS.load(AtomicOrdering::SeqCst),
        calls_before + 1
    );
    assert_eq!(session.player_xp_like_cpp(), 0);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.5);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP,
        "C++ still calls GetRestBonusFor(0) after a script zeroes the amount"
    );
    let packets = drain_server_packet_bytes(&send_rx);
    let mut packet = WorldPacket::from_bytes(
        packets
            .iter()
            .find(|bytes| {
                WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::LogXpGain)
            })
            .expect("C++ continues GiveXP after a script changes amount to zero"),
    );
    packet.skip_opcode();
    assert_eq!(packet.read_packed_guid().unwrap(), victim);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_uint8().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
}
#[test]
fn give_xp_runtime_dispatches_script_before_max_level_return_like_cpp() {
    let (mut session, _, send_rx) = make_session_with_give_player_xp_hook();
    let player = ObjectGuid::create_player(1, XP_HOOK_MAX_PLAYER_COUNTER);
    let victim = test_creature_guid(0xE1D2);
    session.set_player_guid(Some(player));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 80, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    install_tapped_xp_victim_like_cpp(&mut session, victim);
    let calls_before = XP_HOOK_MAX_CALLS.load(AtomicOrdering::SeqCst);

    assert!(!session.give_xp_runtime_like_cpp(50, victim, 1.0));

    assert_eq!(
        XP_HOOK_MAX_CALLS.load(AtomicOrdering::SeqCst),
        calls_before + 1
    );
    assert_eq!(session.player_xp_like_cpp(), 0);
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[test]
fn give_xp_runtime_rejection_guards_run_before_script_like_cpp() {
    let (mut session, _, send_rx) = make_session_with_give_player_xp_hook();
    let player = ObjectGuid::create_player(1, XP_HOOK_GUARD_PLAYER_COUNTER);
    session.set_player_guid(Some(player));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    let calls_before = XP_HOOK_GUARD_CALLS.load(AtomicOrdering::SeqCst);

    assert!(!session.give_xp_runtime_like_cpp(0, ObjectGuid::EMPTY, 1.0));
    let untapped = test_creature_guid(0xE1D3);
    install_xp_victim_like_cpp(&mut session, untapped, false);
    assert!(!session.give_xp_runtime_like_cpp(50, untapped, 1.0));

    assert_eq!(
        XP_HOOK_GUARD_CALLS.load(AtomicOrdering::SeqCst),
        calls_before
    );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[test]
fn give_xp_runtime_spends_rested_bonus_for_victim_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let victim = test_creature_guid(77);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);
    install_tapped_xp_victim_like_cpp(&mut session, victim);

    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));

    assert_eq!(session.player_xp_like_cpp(), 100);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 20.0);
    let packets = drain_server_packet_bytes(&send_rx);
    let mut pkt = WorldPacket::from_bytes(
        packets
            .iter()
            .find(|bytes| {
                WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::LogXpGain)
            })
            .expect("GiveXP sends LogXPGain"),
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_packed_guid().unwrap(), victim);
    assert_eq!(pkt.read_int32().unwrap(), 100);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_int32().unwrap(), 50);
    assert!((pkt.read_float().unwrap() - 1.0).abs() < f32::EPSILON);
    assert_eq!(pkt.remaining(), 0);

    let values_packets = packets
        .iter()
        .filter(|bytes| {
            WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::UpdateObject)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        values_packets.len(),
        1,
        "C++ accumulates RestInfo and XP in one Player values update"
    );
    let mut expected_delta = Player::new(None, false);
    expected_delta.clear_data_changes();
    expected_delta.set_xp(100);
    expected_delta.mark_xp_changed_like_cpp();
    expected_delta.set_scaling_player_level_delta_like_cpp(-1);
    expected_delta.mark_scaling_player_level_delta_changed_like_cpp();
    expected_delta.prepare_rest_info_values_update_like_cpp(
        0,
        20,
        REST_STATE_RESTED_LIKE_CPP,
        0x07,
    );
    let expected_packet = player_values_update_to_update_object(
        session.player_guid().expect("loaded test player"),
        session.player_map_id_like_cpp(),
        &expected_delta.values_update(true),
    )
    .expect("combined XP/rest delta")
    .to_bytes();
    assert_eq!(
        values_packets[0].as_slice(),
        expected_packet.as_slice(),
        "the single instance update must contain exactly XP plus RestInfo"
    );
}
#[test]
fn give_xp_runtime_normalizes_zero_integer_rested_award_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let victim = test_creature_guid(0xE1C3);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    // C++ LoadRestBonus preserves this inconsistent persisted pair until
    // GetRestBonusFor unconditionally calls SetRestBonus on the next kill.
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 0.5);
    install_tapped_xp_victim_like_cpp(&mut session, victim);

    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));

    assert_eq!(session.player_xp_like_cpp(), 50);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.5);
    assert_eq!(session.represented_xp_rest_threshold_like_cpp(), 0);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );
    let packets = drain_server_packet_bytes(&send_rx);
    let values_packets = packets
        .iter()
        .filter(|bytes| {
            WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::UpdateObject)
        })
        .collect::<Vec<_>>();
    assert_eq!(values_packets.len(), 1);

    let mut expected_delta = Player::new(None, false);
    expected_delta.clear_data_changes();
    expected_delta.set_xp(50);
    expected_delta.mark_xp_changed_like_cpp();
    expected_delta.set_scaling_player_level_delta_like_cpp(-1);
    expected_delta.mark_scaling_player_level_delta_changed_like_cpp();
    expected_delta.prepare_rest_info_values_update_like_cpp(0, 0, REST_STATE_NORMAL_LIKE_CPP, 0x07);
    let expected_packet = player_values_update_to_update_object(
        session.player_guid().expect("loaded test player"),
        session.player_map_id_like_cpp(),
        &expected_delta.values_update(true),
    )
    .expect("combined XP/rest normalization delta")
    .to_bytes();
    assert_eq!(values_packets[0].as_slice(), expected_packet.as_slice());
}
#[test]
fn give_xp_runtime_zero_integer_rested_award_keeps_consistent_state_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let victim = test_creature_guid(0xE1C4);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.5);
    install_tapped_xp_victim_like_cpp(&mut session, victim);

    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));

    assert_eq!(session.player_xp_like_cpp(), 50);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.5);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );
    let packets = drain_server_packet_bytes(&send_rx);
    let values_packets = packets
        .iter()
        .filter(|bytes| {
            WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::UpdateObject)
        })
        .collect::<Vec<_>>();
    assert_eq!(values_packets.len(), 1);

    let mut expected_delta = Player::new(None, false);
    expected_delta.clear_data_changes();
    expected_delta.set_xp(50);
    expected_delta.mark_xp_changed_like_cpp();
    expected_delta.set_scaling_player_level_delta_like_cpp(-1);
    expected_delta.mark_scaling_player_level_delta_changed_like_cpp();
    let expected_packet = player_values_update_to_update_object(
        session.player_guid().expect("loaded test player"),
        session.player_map_id_like_cpp(),
        &expected_delta.values_update(true),
    )
    .expect("XP-only delta")
    .to_bytes();
    assert_eq!(values_packets[0].as_slice(), expected_packet.as_slice());
}
#[test]
fn give_xp_runtime_routes_log_xp_gain_on_realm_connection_like_cpp() {
    let (mut session, _, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::unbounded();
    let victim = test_creature_guid(0xE1C0);
    session.install_realm_send_channel_for_test(realm_tx);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    install_tapped_xp_victim_like_cpp(&mut session, victim);

    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));

    assert!(
        drain_server_packet_bytes(&instance_rx).iter().all(|bytes| {
            WorldPacket::from_bytes(bytes).server_opcode() != Some(ServerOpcodes::LogXpGain)
        }),
        "C++ registers SMSG_LOG_XP_GAIN as CONNECTION_TYPE_REALM"
    );
    let realm_packets = drain_server_packet_bytes(&realm_rx);
    assert_eq!(
        realm_packets
            .iter()
            .filter(|bytes| {
                WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::LogXpGain)
            })
            .count(),
        1
    );
}
#[test]
fn give_xp_runtime_routes_level_up_info_on_realm_connection_like_cpp() {
    let (mut session, _, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 1, 0);
    session.set_player_next_level_xp_like_cpp(50);

    assert!(session.give_xp_runtime_like_cpp(50, ObjectGuid::EMPTY, 1.0));
    assert_eq!(session.player_level_like_cpp(), 2);

    assert!(
        drain_server_packet_bytes(&instance_rx).iter().all(|bytes| {
            WorldPacket::from_bytes(bytes).server_opcode() != Some(ServerOpcodes::LevelUpInfo)
        }),
        "C++ registers SMSG_LEVEL_UP_INFO as CONNECTION_TYPE_REALM"
    );
    let realm_packets = drain_server_packet_bytes(&realm_rx);
    assert_eq!(
        realm_packets
            .iter()
            .filter(|bytes| {
                WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::LevelUpInfo)
            })
            .count(),
        1
    );
}
#[test]
fn give_xp_runtime_updates_canonical_progression_and_client_fields_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1C2);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(guid));
    session.ensure_login_player_controller_like_cpp(
        guid,
        "Progression".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    let mut player_xp_table = vec![0; 82];
    player_xp_table[10] = 50;
    player_xp_table[11] = 75;
    session.set_player_xp_table(Arc::new(player_xp_table));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    assert!(session.ensure_canonical_player_owner_for_map_like_cpp(
        wow_map::MapKey::new(1, 0),
        Position::new(1.0, 2.0, 3.0, 0.0),
    ));
    let _ = drain_server_packet_bytes(&send_rx);

    assert!(session.give_xp_runtime_like_cpp(50, ObjectGuid::EMPTY, 1.0));

    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| (
            player.unit().data().level,
            player.active_data().xp,
            player.active_data().next_level_xp,
            player.active_data().scaling_player_level_delta,
            player
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_LEVEL_BIT),
            player
                .active_player_data_changes_mask()
                .is_set(wow_entities::ACTIVE_PLAYER_DATA_XP_BIT),
            player
                .active_player_data_changes_mask()
                .is_set(wow_entities::ACTIVE_PLAYER_DATA_NEXT_LEVEL_XP_BIT),
            player
                .active_player_data_changes_mask()
                .is_set(wow_entities::ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT,),
        )),
        Some((11, 0, 75, -1, true, true, true, true))
    );
    assert!(drain_server_packet_bytes(&send_rx).iter().any(|bytes| {
        WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::UpdateObject)
    }));
}
#[test]
fn give_xp_runtime_rejects_dead_player_outside_battleground_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);
    session.set_player_alive_like_cpp(false);

    assert!(!session.give_xp_runtime_like_cpp(50, ObjectGuid::EMPTY, 1.0));
    assert_eq!(session.player_xp_like_cpp(), 0);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 70.0);
    assert!(drain_server_packet_bytes(&send_rx).is_empty());

    session.set_player_battleground_type_id_like_cpp(BATTLEGROUND_WS_LIKE_CPP);
    assert!(session.give_xp_runtime_like_cpp(50, ObjectGuid::EMPTY, 1.0));
    assert_eq!(session.player_xp_like_cpp(), 50);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 70.0);
}
#[test]
fn give_xp_runtime_rejects_no_xp_gain_player_flag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.set_loaded_player_flags_like_cpp(PLAYER_FLAGS_NO_XP_GAIN_LIKE_CPP);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);

    assert!(!session.give_xp_runtime_like_cpp(50, ObjectGuid::EMPTY, 1.0));
    assert_eq!(session.player_xp_like_cpp(), 0);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 70.0);
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[test]
fn give_xp_runtime_raf_awards_triple_xp_without_spending_rested_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 1);
    let recruit_guid = ObjectGuid::create_player(1, 2);
    let victim = test_creature_guid(78);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.set_player_map_position_like_cpp(1, Position::ZERO);
    session.set_recruiter_id_like_cpp(2);
    session.set_recruit_a_friend_xp_config_like_cpp(85, 4);

    let (recruit_tx, _recruit_rx) = flume::bounded(10);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let mut recruit_info = broadcast_info(recruit_guid, recruit_tx);
    recruit_info.placement.map_id = 1;
    recruit_info.placement.position = Position::new(10.0, 0.0, 0.0, 0.0);
    recruit_info.identity.account_id = 2;
    recruit_info.placement.level = 10;
    player_registry.register_or_replace(recruit_guid, recruit_info, Default::default());

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(recruit_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(
        !session.gets_recruit_a_friend_xp_bonus_like_cpp(),
        "C++ LoadFromDB runs before the player enters the world"
    );
    session.set_state(SessionState::LoggedIn);
    assert!(player_registry.fixture_update(recruit_guid, |p| p.is_in_world = false));
    assert!(
        !session.gets_recruit_a_friend_xp_bonus_like_cpp(),
        "C++ IsInMap rejects a grouped member that is not in world"
    );
    assert!(player_registry.fixture_update(recruit_guid, |p| p.is_in_world = true));
    assert!(session.gets_recruit_a_friend_xp_bonus_like_cpp());
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);
    install_tapped_xp_victim_like_cpp(&mut session, victim);

    assert!(session.give_xp_runtime_like_cpp(50, victim, 1.0));

    assert_eq!(session.player_xp_like_cpp(), 150);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 70.0);
}
#[test]
fn give_xp_runtime_applies_rested_xp_consumption_modifier_like_cpp() {
    let (mut session, _, _) = make_session();
    let victim = test_creature_guid(79);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);
    install_tapped_xp_victim_like_cpp(&mut session, victim);
    let effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION,
        effect_base_points: 50,
        ..Default::default()
    };
    session
        .apply_represented_aura_modifier_like_cpp(
            12_345,
            ObjectGuid::EMPTY,
            &effect,
            RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
            30_000,
        )
        .expect("rested consumption aura should apply");

    assert!(session.give_xp_runtime_like_cpp(40, victim, 1.0));

    assert_eq!(session.player_xp_like_cpp(), 80);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 10.0);
}
#[test]
fn give_xp_runtime_does_not_spend_rested_bonus_without_victim_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 70.0);

    assert!(session.give_xp_runtime_like_cpp(50, ObjectGuid::EMPTY, 1.0));

    assert_eq!(session.player_xp_like_cpp(), 50);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 70.0);
    let packets = drain_server_packet_bytes(&send_rx);
    let mut pkt = WorldPacket::from_bytes(
        packets
            .iter()
            .find(|bytes| {
                WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::LogXpGain)
            })
            .expect("GiveXP sends LogXPGain"),
    );
    pkt.skip_opcode();
    assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(pkt.read_int32().unwrap(), 50);
    assert_eq!(pkt.read_uint8().unwrap(), 1);
    assert_eq!(pkt.read_int32().unwrap(), 50);
    assert!((pkt.read_float().unwrap() - 1.0).abs() < f32::EPSILON);
    assert_eq!(pkt.remaining(), 0);
}
#[test]
fn sync_canonical_player_health_sets_current_and_max_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 0xE112);
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "Healthy".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        1,
        80,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);

    assert_eq!(
        session.sync_canonical_player_health_like_cpp(42, 120),
        Some((42, 120))
    );
    assert_eq!(session.player_health_like_cpp(), 42);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((42, 120))
    );
}
#[test]
fn sync_canonical_player_health_zero_sets_corpse_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 0xE113);
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "Dead".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        1,
        80,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);

    assert_eq!(
        session.sync_canonical_player_health_like_cpp(0, 120),
        Some((0, 120))
    );
    assert_eq!(session.player_health_like_cpp(), 0);
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((0, 120))
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::Corpse)
    );
}
#[test]
fn sync_canonical_player_primary_power_sets_create_mana_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 0xE102);
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "Caster".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        10,
        5,
        80,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);

    assert!(session.sync_canonical_player_primary_power_like_cpp(
        PowerType::Mana,
        500,
        1_000,
        3_863,
    ));

    let (current, max, create_mana) = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.get_power(PowerType::Mana),
                player.get_max_power(PowerType::Mana),
                player.unit().get_create_mana_like_cpp(),
            )
        })
        .expect("canonical player");
    assert_eq!(current, 500);
    assert_eq!(max, 1_000);
    assert_eq!(
        create_mana, 3_863,
        "C++ SpellInfo::CalcPowerCost uses Unit::GetCreateMana for mana percentage costs"
    );
}
#[test]
fn sync_canonical_player_primary_power_clears_stale_mana_index_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 0xE103);
    session.set_canonical_map_manager(Arc::clone(&canonical));

    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "Warrior".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        1,
        80,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);

    assert!(session.sync_canonical_player_primary_power_like_cpp(PowerType::Rage, 500, 1_000, 0,));

    let (mana_index, rage_index, mana, rage, rage_max, create_mana) = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.get_power_index(PowerType::Mana),
                player.get_power_index(PowerType::Rage),
                player.get_power(PowerType::Mana),
                player.get_power(PowerType::Rage),
                player.get_max_power(PowerType::Rage),
                player.unit().get_create_mana_like_cpp(),
            )
        })
        .expect("canonical player");
    assert_eq!(
        mana_index, None,
        "C++ DB2Manager::GetPowerIndexByClass has no Mana power slot for warrior"
    );
    assert_eq!(rage_index, Some(0));
    assert_eq!(mana, 0);
    assert_eq!(rage, 500);
    assert_eq!(rage_max, 1_000);
    assert_eq!(create_mana, 0);
}
#[test]
fn update_area_records_enter_leave_area_criteria_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_zone_area_like_cpp(10, 100);

    assert!(session.update_area_represented_like_cpp(101));
    assert_eq!(session.player_zone_area_like_cpp(), Some((10, 101)));
    assert_eq!(
        session.represented_area_zone_criteria_like_cpp(),
        &[
            RepresentedAreaZoneCriteriaLikeCpp::EnterArea(101),
            RepresentedAreaZoneCriteriaLikeCpp::LeaveArea(100),
        ],
        "C++ Player::UpdateArea records EnterArea then LeaveArea after m_areaUpdateId changes"
    );

    assert!(!session.update_area_represented_like_cpp(101));
    assert_eq!(session.represented_area_zone_criteria_like_cpp().len(), 2);
}
#[test]
fn update_area_sets_faction_area_rest_flag_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE19F);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "AllianceRestArea".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    session.set_player_zone_area_like_cpp(10, 100);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 101,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_ALLIANCE_RESTING_LIKE_CPP,
        },
        wow_data::AreaTableEntry {
            id: 102,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_HORDE_RESTING_LIKE_CPP,
        },
    ])));

    assert!(session.update_area_represented_like_cpp(101));
    assert!(session.represented_is_resting_like_cpp());

    assert!(session.update_area_represented_like_cpp(102));
    assert!(!session.represented_is_resting_like_cpp());
}
#[test]
fn update_zone_records_area_then_top_level_criteria_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE1A0);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "ZoneCriteria".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    session.set_player_zone_area_like_cpp(10, 100);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 20,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));

    assert!(session.update_zone_represented_like_cpp(20, 101));
    assert_eq!(session.player_zone_area_like_cpp(), Some((20, 101)));
    assert_eq!(
        session.represented_area_zone_criteria_like_cpp(),
        &[
            RepresentedAreaZoneCriteriaLikeCpp::EnterArea(101),
            RepresentedAreaZoneCriteriaLikeCpp::LeaveArea(100),
            RepresentedAreaZoneCriteriaLikeCpp::EnterTopLevelArea(20),
            RepresentedAreaZoneCriteriaLikeCpp::LeaveTopLevelArea(10),
        ],
        "C++ Player::UpdateZone calls UpdateArea before EnterTopLevelArea/LeaveTopLevelArea"
    );
}
