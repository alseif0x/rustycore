//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn update_zone_coalesces_faction_to_city_zero_crossings_into_one_final_update_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE1A5);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "RestTransition".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    session.set_player_zone_area_like_cpp(10, 101);
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

    assert!(session.update_zone_represented_like_cpp(20, 102));

    let update_count = drain_server_packet_bytes(&send_rx)
        .iter()
        .filter(|packet| {
            packet.len() >= 2
                && u16::from_le_bytes([packet[0], packet[1]]) == ServerOpcodes::UpdateObject as u16
        })
        .count();
    assert_eq!(
        update_count, 1,
        "C++ leaves the update-field bit dirty across the faction-off/city-on sequence and flushes one final value"
    );
    assert_eq!(
        session.represented_rest_flag_mask_like_cpp,
        REST_FLAG_IN_CITY_LIKE_CPP
    );
    assert!(
        session
            .canonical_player_has_player_flag_like_cpp(player_guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(false)
    );
}
#[test]
fn update_zone_with_overlapping_tavern_flag_does_not_dirty_player_flags_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE1A6);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "RestOverlap".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        10,
        0,
    );
    session.set_player_zone_area_like_cpp(10, 101);
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
    assert!(!session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP, 42));
    let _ = drain_server_packet_bytes(&send_rx);

    assert!(session.update_zone_represented_like_cpp(20, 102));

    let update_count = drain_server_packet_bytes(&send_rx)
        .iter()
        .filter(|packet| {
            packet.len() >= 2
                && u16::from_le_bytes([packet[0], packet[1]]) == ServerOpcodes::UpdateObject as u16
        })
        .count();
    assert_eq!(update_count, 0, "the RestMgr mask never crossed zero");
    assert_eq!(
        session.represented_rest_flag_mask_like_cpp,
        REST_FLAG_IN_TAVERN_LIKE_CPP | REST_FLAG_IN_CITY_LIKE_CPP
    );
}
#[test]
fn update_zone_missing_zone_keeps_area_criteria_but_skips_top_level_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE1A1);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "MissingZone".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    session.set_player_zone_area_like_cpp(10, 100);
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([])));

    assert!(session.update_zone_represented_like_cpp(20, 101));
    assert_eq!(session.player_zone_area_like_cpp(), Some((20, 101)));
    assert_eq!(
        session.represented_area_zone_criteria_like_cpp(),
        &[
            RepresentedAreaZoneCriteriaLikeCpp::EnterArea(101),
            RepresentedAreaZoneCriteriaLikeCpp::LeaveArea(100),
        ],
        "C++ Player::UpdateZone returns after UpdateArea when AreaTable lacks the new zone"
    );
}
#[tokio::test]
async fn check_area_explore_marks_block_sends_update_and_records_criteria_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE202);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "Explorer".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 9_001,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: 65,
            exploration_level: 12,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());

    assert!(
        session
            .check_area_explore_and_outdoor_represented_like_cpp(9_001)
            .await
    );
    assert_eq!(
        session
            .represented_explored_zones_db_string_like_cpp()
            .expect("test Player explored-zones owner resolves")
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>(),
        vec!["0", "0", "2", "0"]
    );
    assert_eq!(
        session.represented_reveal_world_map_overlay_criteria_like_cpp(),
        &[9_001]
    );
    {
        let manager = canonical.lock().unwrap();
        let player = manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap();
        assert_eq!(player.explored_zones_block_like_cpp(1), Some(2));
    }
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::UpdateObject),
        "C++ SetUpdateFieldFlagValue must be visible to the player through UpdateObject"
    );

    assert!(
        !session
            .check_area_explore_and_outdoor_represented_like_cpp(9_001)
            .await
    );
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_reveal_world_map_overlay_criteria_like_cpp(),
        &[9_001],
        "C++ only updates criteria when the area bit was newly discovered"
    );
}
#[tokio::test]
async fn check_area_explore_awards_exploration_xp_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE204);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "ExplorerXp".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        10,
        0,
    );
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 9_004,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: 66,
            exploration_level: 12,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.set_exploration_base_xp_store_like_cpp(Arc::new(
        ExplorationBaseXpStoreLikeCpp::from_rows_like_cpp([
            wow_data::ExplorationBaseXpRowLikeCpp {
                level: 12,
                base_xp: 120,
            },
        ]),
    ));
    session.set_exploration_xp_rate_like_cpp(1.5);
    session.set_min_discovered_scaled_xp_ratio_like_cpp(0);

    assert!(
        session
            .check_area_explore_and_outdoor_represented_like_cpp(9_004)
            .await
    );
    assert_eq!(session.player_xp_like_cpp(), 180);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::LogXpGain)
    }));
    let exploration = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::ExplorationExperience)
        })
        .expect("C++ sends SMSG_EXPLORATION_EXPERIENCE after GiveXP");
    let mut pkt = wow_packet::WorldPacket::from_bytes(exploration);
    pkt.skip_opcode();
    assert_eq!(pkt.read_int32().unwrap(), 9_004);
    assert_eq!(pkt.read_int32().unwrap(), 180);
    assert_eq!(pkt.remaining(), 0);
}
#[tokio::test]
async fn check_area_explore_max_level_sends_zero_exploration_xp_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE205);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "ExplorerMax".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 9_005,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: 67,
            exploration_level: 12,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.set_exploration_base_xp_store_like_cpp(Arc::new(
        ExplorationBaseXpStoreLikeCpp::from_rows_like_cpp([
            wow_data::ExplorationBaseXpRowLikeCpp {
                level: 12,
                base_xp: 120,
            },
        ]),
    ));

    assert!(
        session
            .check_area_explore_and_outdoor_represented_like_cpp(9_005)
            .await
    );
    assert_eq!(session.player_xp_like_cpp(), 0);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(!packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::LogXpGain)
    }));
    let exploration = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::ExplorationExperience)
        })
        .expect("C++ sends SMSG_EXPLORATION_EXPERIENCE with zero at max level");
    let mut pkt = wow_packet::WorldPacket::from_bytes(exploration);
    pkt.skip_opcode();
    assert_eq!(pkt.read_int32().unwrap(), 9_005);
    assert_eq!(pkt.read_int32().unwrap(), 0);
    assert_eq!(pkt.remaining(), 0);
}
#[tokio::test]
async fn check_area_explore_rejects_missing_and_invalid_area_bits_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 9_002,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 9_003,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: (wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP * 64) as i16,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));

    assert!(
        !session
            .check_area_explore_and_outdoor_represented_like_cpp(0)
            .await
    );
    assert!(
        !session
            .check_area_explore_and_outdoor_represented_like_cpp(123_456)
            .await
    );
    assert!(
        !session
            .check_area_explore_and_outdoor_represented_like_cpp(9_002)
            .await
    );
    assert!(
        !session
            .check_area_explore_and_outdoor_represented_like_cpp(9_003)
            .await
    );
    assert!(
        session
            .represented_explored_zones_db_string_like_cpp()
            .expect("test Player explored-zones owner resolves")
            .split_whitespace()
            .all(|token| token == "0")
    );
    assert!(
        session
            .represented_reveal_world_map_overlay_criteria_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn check_area_explore_indoor_outdoor_removal_is_config_gated_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(9_201, test_spell_info_like_cpp(9_201));
    let mut indoor_attributes = [0; 15];
    indoor_attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_ONLY_INDOORS;
    spell_store.insert_spell_misc_attributes_like_cpp(9_201, indoor_attributes);
    session.set_spell_store(Arc::new(spell_store));
    session.visible_auras.insert(1, test_visible_aura(1, 9_201));
    session.set_represented_is_outdoors_like_cpp(true);

    assert!(
        !session
            .check_area_explore_and_outdoor_represented_like_cpp(0)
            .await
    );
    assert!(
        session.visible_auras.contains_key(&1),
        "C++ only calls RemoveAurasWithAttribute when CONFIG_VMAP_INDOOR_CHECK is enabled"
    );
}
#[test]
fn canonical_player_cuf_profiles_follow_active_detached_and_stale_handle_ownership_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 59_198);
    let position = Position::new(3700.0, 1500.0, 120.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "CufOwner".to_string(),
        position,
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

    assert!(
        session.represented_save_cuf_profiles_like_cpp(vec![cuf_profile_for_save_test(
            "Canonical",
            72
        ),])
    );
    session.mark_represented_cuf_profiles_loaded_like_cpp();
    assert_eq!(
        session
            .represented_load_cuf_profiles_packet_like_cpp()
            .expect("active canonical Player")
            .profiles[0]
            .profile_name,
        "Canonical"
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session
            .owned_player_cuf_profiles_like_cpp()
            .expect("detached canonical Player")
            .0[0]
            .as_ref()
            .unwrap()
            .profile_name,
        "Canonical"
    );

    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().cuf_profiles = vec![
        Some(player_cuf_profile_from_packet_like_cpp(
            cuf_profile_for_save_test("Replacement", 64),
        )),
        None,
        None,
        None,
        None,
    ];
    replacement.gameplay_state_mut().cuf_profiles_loaded = true;
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert!(
        session
            .represented_load_cuf_profiles_packet_like_cpp()
            .is_none()
    );
    assert!(
        !session.represented_save_cuf_profiles_like_cpp(vec![cuf_profile_for_save_test(
            "StaleWrite",
            80
        ),])
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .gameplay_state()
                .cuf_profiles[0]
                .as_ref()
                .unwrap()
                .profile_name
                .clone(),),
        Some("Replacement".to_string())
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        None
    );
}
#[test]
fn tutorial_flags_default_to_zeroes_like_cpp() {
    let (mut session, _, _) = make_session();
    session.load_tutorials_data_values_like_cpp(None);

    assert_eq!(
        session.tutorial_flags_packet_like_cpp().tutorial_data,
        [0; 8],
        "C++ LoadTutorialsData zeroes _tutorials when account_tutorial has no row"
    );
}
#[test]
fn tutorial_clear_and_reset_match_cpp_actions() {
    let (mut session, _, _) = make_session();
    session.load_tutorials_data_values_like_cpp(Some([1, 2, 3, 4, 5, 6, 7, 8]));

    assert!(session.apply_tutorial_action_like_cpp(
        wow_packet::packets::misc::TUTORIAL_ACTION_CLEAR_LIKE_CPP,
        None
    ));
    assert_eq!(
        session.tutorial_flags_packet_like_cpp().tutorial_data,
        [u32::MAX; 8]
    );

    assert!(session.apply_tutorial_action_like_cpp(
        wow_packet::packets::misc::TUTORIAL_ACTION_RESET_LIKE_CPP,
        None
    ));
    assert_eq!(
        session.tutorial_flags_packet_like_cpp().tutorial_data,
        [0; 8]
    );
}
#[test]
fn player_attack_game_master_typed_player_is_rejected_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 52);
    let victim = ObjectGuid::create_player(1, 53);

    canonical.lock().unwrap().create_world_map(571, 0);
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
        attacker,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(9), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player.set_game_master_like_cpp(true);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(attacker).unwrap().unit().attacking(),
        None
    );
    assert!(
        !map.get_typed_player(victim)
            .unwrap()
            .unit()
            .has_attacker_like_cpp(attacker)
    );
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
}
#[test]
fn mounted_player_attack_is_rejected_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 49);
    let victim = test_creature_guid(18_007);

    canonical.lock().unwrap().create_world_map(571, 0);
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
        player,
        "Mounted".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_mounted_like_cpp(true);

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(player_entity.unit().attacking(), None);
    assert_eq!(player_entity.unit().data().target, ObjectGuid::EMPTY);
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
}
#[test]
fn player_attack_rejects_canonical_uber_player_flag_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 62);
    let victim = test_creature_guid(18_013);

    canonical.lock().unwrap().create_world_map(571, 0);
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
        player,
        "Uber".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_UBER_LIKE_CPP);
        })
        .unwrap();

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(player_entity.unit().attacking(), None);
    assert_eq!(player_entity.unit().data().target, ObjectGuid::EMPTY);
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
}
#[tokio::test]
async fn handle_attack_swing_invalid_vehicle_seat_sends_stop_without_start_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 79);
    let victim = test_creature_guid(18_030);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Passenger".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session.player_vehicle_seat_flags_like_cpp = Some(0);
    register_test_creature(&mut session, manager.clone(), victim, 40);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&victim);
    session.handle_attack_swing(pkt).await;

    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackStop as u16);
    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
    let guard = manager.read().unwrap();
    assert_ne!(
        guard
            .find_creature(0, 0, victim)
            .unwrap()
            .creature
            .ai_state(),
        wow_entities::CreatureAiState::InCombat
    );
}
#[tokio::test]
async fn handle_attack_swing_same_target_no_change_sends_no_stop_or_duplicate_start_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 81);
    let victim = test_creature_guid(18_032);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Attacker".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical attacking Player map");
    register_test_creature(&mut session, manager.clone(), victim, 40);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&victim);
    session.handle_attack_swing(pkt).await;

    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::AttackStart as u16);
    let _ = drain_server_opcodes(&send_rx);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&victim);
    session.handle_attack_swing(pkt).await;

    assert_eq!(drain_server_opcodes(&send_rx), Vec::<ServerOpcodes>::new());
    assert_eq!(
        session.resolved_combat_target_like_cpp(),
        Some(Some(victim))
    );
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(true));
}
