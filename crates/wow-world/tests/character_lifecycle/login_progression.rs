// Existing Character application scenarios, moved with original assertion operands.

use super::fixtures::*;
use super::fixtures::session::make_session;

#[test]
fn first_login_start_all_explored_sets_all_cpp_blocks_and_sends_update() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE001);
    session.character_ensure_login_player_controller_for_test(
        player_guid,
        "Explorer".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);

    assert_eq!(
        session.character_apply_represented_first_login_explored_zones_for_test(),
        0
    );
    assert!(send_rx.try_recv().is_err());

    session.set_start_all_explored_like_cpp(true);
    let applied = session.character_apply_represented_first_login_explored_zones_for_test();

    assert_eq!(
        applied,
        wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP,
        "C++ loops PLAYER_EXPLORED_ZONES_SIZE and applies UI64_MAX to each block"
    );
    assert!(
        session
            .character_represented_explored_zones_db_string_for_test()
            .expect("test Player explored-zones owner resolves")
            .split_whitespace()
            .take(2)
            .eq(["4294967295", "4294967295"]),
        "first-login AddExploredZones must also update the represented DB snapshot"
    );
    {
        let manager = canonical.lock().unwrap();
        let player = manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap();
        assert!(
            (0..wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP)
                .all(|index| player.explored_zones_block_like_cpp(index) == Some(u64::MAX))
        );
    }

    let packet = drain_server_packet_bytes(&send_rx)
        .into_iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::UpdateObject)
        })
        .expect("explored-zone field update");
    assert!(!packet.is_empty());
}

#[test]
fn offline_rested_xp_zero_logout_time_is_rejected_instead_of_cpp_wrap() {
    let (mut session, _, _) = make_session();
    session.character_set_loaded_player_identity_for_test(1, 1, 8, 10, 0);
    session.character_set_player_next_level_xp_for_test(72_000);
    session.character_load_represented_xp_rest_bonus_for_test(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let extra = session.character_apply_offline_xp_rest_bonus_for_test(0, 4_600, true);

    assert_eq!(extra, 0.0);
    assert_eq!(session.character_represented_xp_rest_bonus_for_test(), 0.0);
    assert_eq!(
        session.character_represented_xp_rest_state_for_test(),
        REST_STATE_NORMAL_LIKE_CPP
    );
}

#[test]
fn offline_rested_xp_future_logout_time_is_rejected_instead_of_cpp_wrap() {
    let (mut session, _, _) = make_session();
    session.character_set_loaded_player_identity_for_test(1, 1, 8, 10, 0);
    session.character_set_player_next_level_xp_for_test(72_000);
    session.character_load_represented_xp_rest_bonus_for_test(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let extra = session.character_apply_offline_xp_rest_bonus_for_test(4_601, 4_600, true);

    assert_eq!(extra, 0.0);
    assert_eq!(session.character_represented_xp_rest_bonus_for_test(), 0.0);
    assert_eq!(
        session.character_represented_xp_rest_state_for_test(),
        REST_STATE_NORMAL_LIKE_CPP
    );
}

#[test]
fn offline_rested_xp_login_does_not_modify_current_health_or_power_like_cpp() {
    let (mut session, _, _) = make_session();
    session.character_set_loaded_player_identity_for_test(1, 1, 8, 10, 0);
    session.character_set_player_next_level_xp_for_test(72_000);
    session.character_set_player_health_for_test(41, 100);
    session.character_set_loaded_player_powers_for_test([17, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    session.character_load_represented_xp_rest_bonus_for_test(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let extra = session.character_apply_offline_xp_rest_bonus_for_test(1_000, 4_600, true);

    assert!(extra > 0.0);
    assert_eq!(session.character_player_health_for_test(), 41);
    assert_eq!(
        session
            .character_represented_player_power_values_for_test()
            .expect("power snapshot should be authoritative")[0],
        17
    );
}

#[test]
fn logout_resting_only_selects_offline_rate_and_does_not_restore_online_rest_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1B0);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 1)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.character_set_loaded_player_identity_for_test(1, 1, 8, 10, 0);
    session.character_ensure_login_player_controller_for_test(
        guid,
        "RestLogin".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    session.character_set_player_next_level_xp_for_test(72_000);
    session.character_set_player_zone_area_for_test(10, 100);
    session.character_load_represented_xp_rest_bonus_for_test(REST_STATE_RESTED_LIKE_CPP, 123.0);

    assert!(!session.character_represented_is_resting_for_test());
    assert!(
        !session
            .character_canonical_player_has_player_flag_for_test(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(false)
    );

    let applied = session.character_apply_offline_xp_rest_bonus_for_test(1_000, 1_100, true);

    assert!(
        applied > 0.0,
        "the persisted bit still selects the tavern/city offline rate"
    );
    assert!(!session.character_represented_is_resting_for_test());
    assert_eq!(
        session.character_inn_trigger_for_test(),
        0
    );
    assert_eq!(
        session.character_rest_time_for_test(),
        0
    );
    assert!(
        !session
            .character_canonical_player_has_player_flag_for_test(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(true)
    );
    assert!(session.character_represented_xp_rest_bonus_for_test() > 123.0);
    assert_eq!(
        session.character_represented_xp_rest_state_for_test(),
        REST_STATE_RESTED_LIKE_CPP
    );

    let bonus_after_offline_accrual = session.character_represented_xp_rest_bonus_for_test();
    assert_eq!(
        session.character_update_represented_online_xp_rest_bonus_for_test(1_200),
        (0.0, 0),
        "C++ LoadRestBonus does not initialize RestMgr::_restTime"
    );
    assert_eq!(
        session.character_represented_xp_rest_bonus_for_test(),
        bonus_after_offline_accrual
    );
    assert!(!session.character_represented_is_resting_for_test());
    assert!(
        !session
            .character_canonical_player_has_player_flag_for_test(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(true)
    );
}

#[test]
fn ffa_realm_login_sets_ffa_only_for_non_resting_non_gm_player_like_cpp() {
    for (case, (resting, game_master, expected_ffa)) in [
        (false, false, true),
        (true, false, false),
        (false, true, false),
    ]
    .into_iter()
    .enumerate()
    {
        let (mut session, _, send_rx) = make_session();
        let guid = ObjectGuid::create_player(1, 0xFFA0 + case as i64);
        let canonical = shared_canonical_map_manager();
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.character_ensure_login_player_controller_for_test(
            guid,
            "FfaLogin".to_string(),
            Position::new(1.0, 2.0, 3.0, 0.0),
            1,
            1,
            8,
            10,
            0,
        );
        insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
        session.set_ffa_pvp_realm_like_cpp(true);
        session.character_set_player_game_master_for_test(game_master);
        if resting {
            assert!(session.character_set_represented_rest_flag_for_test(REST_FLAG_IN_TAVERN_LIKE_CPP, 42,));
        }
        let _ = drain_server_packet_bytes(&send_rx);

        session.set_state(SessionState::LoggedIn);

        assert_eq!(
            session
                .character_canonical_player_pvp_flags_for_test(guid)
                .is_some_and(|flags| flags.contains(UnitPvpFlags::FFA_PVP)),
            expected_ffa
        );
        let update_count = drain_server_packet_bytes(&send_rx)
            .iter()
            .filter(|packet| {
                WorldPacket::from_bytes(packet).server_opcode() == Some(ServerOpcodes::UpdateObject)
            })
            .count();
        assert_eq!(update_count, usize::from(expected_ffa));
    }
}

#[test]
fn login_update_zone_rebuilds_city_and_faction_rest_when_ids_are_preseeded_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE1A3);
    session.character_ensure_login_player_controller_for_test(
        player_guid,
        "LoginCityRest".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        10,
        0,
    );
    session.character_set_player_zone_area_for_test(20, 101);
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
            id: 101,
            continent_id: 571,
            parent_area_id: 20,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_ALLIANCE_RESTING_LIKE_CPP,
        },
    ])));

    assert!(!session.character_update_zone_represented_for_test(20, 101));

    assert!(session.character_represented_is_resting_for_test());
    assert_ne!(
        session.character_rest_flags_for_test()
            & REST_FLAG_IN_CITY_LIKE_CPP,
        0
    );
    assert_ne!(
        session.character_rest_flags_for_test()
            & REST_FLAG_IN_FACTION_AREA_LIKE_CPP,
        0
    );
    assert_ne!(
        session.character_rest_time_for_test(),
        0
    );
    assert!(session.character_represented_area_zone_criteria_for_test().is_empty());
}
