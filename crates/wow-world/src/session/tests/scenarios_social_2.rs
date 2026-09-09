//! Session scenarios exercising the represented social responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn canonical_current_expansion_raid_group_allows_entry_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 79);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RaidGroup".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
        &mut session,
        631,
        3,
        77,
        2,
        2,
        25,
    );
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.convert_to_raid_like_cpp();
    group.raid_difficulty_id = 3;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create {
            key,
            difficulty_id: 3,
            ..
        }) if key == wow_map::MapKey::new(631, 1)
    ));
    assert!(send_rx.try_recv().is_err());
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(631, 1)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .is_some()
    );
}
#[test]
fn canonical_old_expansion_raid_skips_raid_group_requirement_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 80);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "OldRaidNoGroup".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
        &mut session,
        631,
        3,
        77,
        2,
        1,
        25,
    );

    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_game_master_bypasses_raid_group_requirement_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let gm = ObjectGuid::create_player(1, 81);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        gm,
        "RaidGmNoGroup".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.set_player_game_master_like_cpp(true);
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
        &mut session,
        631,
        3,
        77,
        2,
        2,
        25,
    );

    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn party_member_reads_canonical_power_without_registry_republish_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let registry = Arc::new(PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("Tester".to_string());
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, guid, position, 571, 0);
    let vitals = (100, 100, PowerType::Energy, 45, 120, 0);
    assert!(set_vitals(&canonical, guid, vitals));

    session.register_in_player_registry();
    assert!(set_party_flags(&canonical, guid));

    let info = registry.party_member(guid).expect("canonical party member");
    assert_eq!(info.power_type, PowerType::Energy as u8);
    assert_eq!(info.current_power, 45);
    assert_eq!(info.max_power, 120);
    assert!(info.is_pvp);
    assert!(info.is_ffa_pvp);
    assert!(info.is_ghost);
}
#[test]
fn player_registry_publishes_home_group_party_type_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 46);
    let registry = Arc::new(PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, guid, Position::new(1.0, 2.0, 3.0, 0.0), 571, 0);
    let group_registry = Arc::new(GroupRegistry::default());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    let group = GroupInfo::new(guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("PartyTypeTester".to_string());
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&registry));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session.register_in_player_registry();

    let party_type = canonical_party_type_for_test(&canonical, guid);
    assert_eq!(
        party_type,
        [
            wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
            wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
        ]
    );
}
#[test]
fn player_registry_publishes_party_member_vehicle_seat_id_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 47);
    let registry = Arc::new(PlayerRegistry::default());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    bind_canonical_test_player_to_registry_like_cpp(&mut session, &registry, guid, position, 571);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("VehicleSeatTester".to_string());
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK);
    session.player_vehicle_seat_id_like_cpp = Some(1001);
    session.set_player_registry(Arc::clone(&registry));

    session.register_in_player_registry();

    let info = registry.party_member(guid).expect("registered player");
    assert!(info.in_vehicle);
    assert_eq!(info.party_member_vehicle_seat, 1001);
}
#[test]
fn represented_request_vehicle_exit_clears_party_vehicle_state_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 50);
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("VehicleExitTester".to_string());
    session.player_vehicle_seat_flags_like_cpp =
        Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ENTER_OR_EXIT);
    session.player_vehicle_seat_id_like_cpp = Some(1001);
    session.set_player_registry(Arc::clone(&registry));
    session.register_in_player_registry();

    assert!(session.represented_request_vehicle_exit_like_cpp());

    assert!(session.player_vehicle_seat_flags_like_cpp.is_none());
    assert!(session.player_vehicle_seat_id_like_cpp.is_none());
    let info = registry.party_member(guid).expect("registered player");
    assert!(!info.in_vehicle);
    assert_eq!(info.party_member_vehicle_seat, 0);
}
#[test]
fn player_attack_uses_private_summon_group_owner_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 68);
    let summon = test_creature_guid(669);

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
        "Hunter".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        3,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.register_world_creature(
        571,
        Position::new(11.0, 20.0, 30.0, 0.0),
        test_creature_create_data(summon, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );

    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(attacker);
    let group_guid = group.group_guid;
    let properties = SummonPropertiesEntry {
        id: 2,
        control: 0,
        faction: 0,
        title: 0,
        slot: 0,
        flags: [
            SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP as i32,
            0,
        ],
    };
    assert_eq!(
        session.summon_private_object_owner_like_cpp(attacker, ObjectGuid::EMPTY, &properties),
        attacker
    );
    let private_owner = ObjectGuid::create_group(group_guid);
    assert!(session.set_canonical_creature_private_object_owner_like_cpp(summon, private_owner));

    assert_eq!(
        session.start_player_attack_like_cpp(summon),
        PlayerAttackStartLikeCppResult::Rejected
    );

    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(session.set_owned_player_group_like_cpp(Some((group_guid, 0))));
    assert_eq!(
        session.summon_private_object_owner_like_cpp(attacker, ObjectGuid::EMPTY, &properties),
        private_owner
    );
    assert_eq!(
        session.start_player_attack_like_cpp(summon),
        PlayerAttackStartLikeCppResult::Accepted {
            send_attack_start: true
        }
    );
}
#[test]
fn give_xp_runtime_preserves_cpp_group_rate_in_log_packet() {
    let (mut session, _, send_rx) = make_session();
    let victim = test_creature_guid(0xE1BF);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    install_tapped_xp_victim_like_cpp(&mut session, victim);

    let group_rate = 1.166;
    assert!(session.give_xp_runtime_like_cpp(50, victim, group_rate));
    assert_eq!(session.player_xp_like_cpp(), 50);

    let packets = drain_server_packet_bytes(&send_rx);
    let mut packet = WorldPacket::from_bytes(
        packets
            .iter()
            .find(|bytes| {
                WorldPacket::from_bytes(bytes).server_opcode() == Some(ServerOpcodes::LogXpGain)
            })
            .expect("GiveXP sends LogXPGain"),
    );
    packet.skip_opcode();
    assert_eq!(packet.read_packed_guid().unwrap(), victim);
    assert_eq!(packet.read_int32().unwrap(), 50);
    assert_eq!(packet.read_uint8().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 50);
    assert!((packet.read_float().unwrap() - group_rate).abs() < f32::EPSILON);
    assert_eq!(packet.remaining(), 0);
}
#[test]
fn update_zone_enemies_pvp_flagged_uses_faction_group_mask_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xE1A4);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "FactionCityRest".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        10,
        0,
    );
    session.set_player_faction_template_like_cpp(100);
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            wow_data::progression_rewards::FactionTemplateEntry {
                id: 100,
                faction: 1,
                flags: 0,
                faction_group: 1,
                friend_group: 1,
                enemy_group: 2,
                enemies: [0; 8],
                friend: [0; 8],
            },
        ]),
    ));
    let mut areas = wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 60,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_LINKED_CHAT_LIKE_CPP
                | AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP,
        },
        wow_data::AreaTableEntry {
            id: 70,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: wow_data::AREA_FLAG_LINKED_CHAT_LIKE_CPP
                | AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP,
        },
    ]);
    areas.set_faction_group_mask_like_cpp(60, 1);
    areas.set_faction_group_mask_like_cpp(70, 2);
    session.set_area_table_store(Arc::new(areas));

    assert!(session.update_zone_represented_like_cpp(60, 60));
    assert!(session.represented_is_resting_like_cpp());
    assert!(!session.player_pvp_hostile_like_cpp);

    assert!(session.update_zone_represented_like_cpp(70, 70));
    assert!(session.player_pvp_hostile_like_cpp);
    assert!(
        session.represented_is_resting_like_cpp(),
        "C++ hostile LinkedChat branch leaves an existing city-rest flag untouched"
    );
}
#[test]
fn update_talent_data_uses_bonus_talent_group_count_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_represented_active_talent_group_like_cpp(1);
    session.set_represented_bonus_talent_groups_like_cpp(1);
    assert!(session.load_represented_glyph_row_like_cpp(&glyph_catalog::catalog(321), 1, 2, 321));

    let packet = session.represented_update_talent_data_packet_like_cpp();

    assert_eq!(packet.active_group, 1);
    assert_eq!(packet.groups.len(), 2);
    assert_eq!(packet.groups[1].glyph_ids[2], 321);
}
#[test]
fn player_attack_accepts_in_progress_duel_before_sanctuary_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 87);
    let victim = ObjectGuid::create_player(1, 88);

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
    session
        .mutate_canonical_player_by_guid_like_cpp(attacker, |player| {
            player.set_duel_opponent_in_progress_like_cpp(victim);
            player
                .unit_mut()
                .set_pvp_flag_like_cpp(UnitPvpFlags::SANCTUARY);
        })
        .unwrap();

    let mut victim_player = Player::new(Some(8), false);
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
    victim_player
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::SANCTUARY);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    session.start_player_attack_like_cpp(victim);
    {
        let guard = canonical.lock().unwrap();
        let map = guard.find_map(571, 0).unwrap().map();
        let attacker_entity = map.get_typed_player(attacker).unwrap();
        let victim_entity = map.get_typed_player(victim).unwrap();
        assert_eq!(attacker_entity.unit().attacking(), Some(victim));
        assert_eq!(attacker_entity.unit().data().target, victim);
        assert!(victim_entity.unit().has_attacker_like_cpp(attacker));
    }
    assert_eq!(
        session.resolved_combat_target_like_cpp(),
        Some(Some(victim))
    );
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(true));
}
#[test]
fn virtual_realm_address_uses_realmlist_region_and_battlegroup_like_cpp() {
    let (mut session, _, _) = make_session();

    session.set_realm_handle_like_cpp(5, 6, 9);

    assert_eq!(session.realm_id(), 9);
    assert_eq!(session.virtual_realm_address(), 0x0506_0009);
}
