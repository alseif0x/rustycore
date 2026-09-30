//! Public loot-handler packet and roll scenarios.

use super::support::*;
use wow_loot::{LOOT_METHOD_GROUP_LIKE_CPP, LOOT_METHOD_MASTER_LIKE_CPP};

#[tokio::test]
async fn loot_unit_master_looter_first_open_sends_candidate_list_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(3);
    let master_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_046);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    session.set_player_guid(Some(master_guid));
    install_master_loot_group(&mut session, master_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    set_loot_for_test(
        &mut session,
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 1,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![master_guid, candidate_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, master_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;

    let response = send_rx.try_recv().unwrap();
    let mut response = WorldPacket::from_bytes(&response);
    assert_eq!(
        response.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );

    let loot_list = send_rx.try_recv().unwrap();
    let mut loot_list = WorldPacket::from_bytes(&loot_list);
    assert_eq!(
        loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );
    assert_eq!(loot_list.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(loot_list.read_packed_guid().unwrap(), loot_object);
    assert!(!loot_list.read_bit().unwrap());
    assert!(!loot_list.read_bit().unwrap());

    let candidate_list = send_rx.try_recv().unwrap();
    let mut candidate_list = WorldPacket::from_bytes(&candidate_list);
    assert_eq!(
        candidate_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::MasterLootCandidateList as u16
    );
    assert_eq!(candidate_list.read_packed_guid().unwrap(), loot_object);
    assert_eq!(candidate_list.read_uint32().unwrap(), 2);
    assert_eq!(candidate_list.read_packed_guid().unwrap(), master_guid);
    assert_eq!(candidate_list.read_packed_guid().unwrap(), candidate_guid);
    assert!(send_rx.try_recv().is_err());
    assert!(loot_for_test(&session, owner_guid).is_some_and(|loot| loot.looted_by_player));
}

#[tokio::test]
async fn loot_unit_master_looter_candidate_list_is_first_open_only_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let master_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_047);
    session.set_player_guid(Some(master_guid));
    install_master_loot_group(&mut session, master_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    set_loot_for_test(
        &mut session,
        owner_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
            coins: 1,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![master_guid, candidate_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, master_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;

    let mut master_candidate_lists = 0;
    while let Ok(sent) = send_rx.try_recv() {
        let mut sent = WorldPacket::from_bytes(&sent);
        if sent.read_uint16().unwrap()
            == wow_constants::ServerOpcodes::MasterLootCandidateList as u16
        {
            master_candidate_lists += 1;
        }
    }

    assert_eq!(master_candidate_lists, 1);
}

#[tokio::test]
async fn loot_unit_master_loot_notify_list_fans_out_to_allowed_looters_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let master_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_048);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(master_guid));
    install_master_loot_group(&mut session, master_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    set_loot_for_test(
        &mut session,
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![master_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    ..Default::default()
                },
                allowed_looters: vec![master_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, master_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let local_loot_list = send_rx.try_recv().unwrap();
    let mut local_loot_list = WorldPacket::from_bytes(&local_loot_list);
    assert_eq!(
        local_loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );
    assert_eq!(local_loot_list.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(local_loot_list.read_packed_guid().unwrap(), loot_object);
    assert!(local_loot_list.read_bit().unwrap());
    assert!(!local_loot_list.read_bit().unwrap());
    local_loot_list.reset_bits();
    assert_eq!(local_loot_list.read_packed_guid().unwrap(), master_guid);

    let remote_loot_list = candidate_rx.try_recv().unwrap();
    let mut remote_loot_list = WorldPacket::from_bytes(&remote_loot_list);
    assert_eq!(
        remote_loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );
    assert_eq!(remote_loot_list.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(remote_loot_list.read_packed_guid().unwrap(), loot_object);
    assert!(remote_loot_list.read_bit().unwrap());
    assert!(!remote_loot_list.read_bit().unwrap());
    remote_loot_list.reset_bits();
    assert_eq!(remote_loot_list.read_packed_guid().unwrap(), master_guid);
}

#[tokio::test]
async fn loot_unit_group_loot_can_only_roll_greed_removes_need_from_start_mask_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_058);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, _candidate_rx) = flume::bounded::<Vec<u8>>(4);
    let player_registry = Arc::new(PlayerRegistry::default());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    install_limited_test_item_template_with_flags2(
        &mut session,
        25,
        0,
        ItemFlags2::CanOnlyRollGreed as u32,
    );
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    set_loot_for_test(
        &mut session,
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let start_roll = send_rx.try_recv().unwrap();
    let mut start_roll = WorldPacket::from_bytes(&start_roll);
    assert_eq!(
        start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
    assert_eq!(
        start_roll.read_uint8().unwrap(),
        ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP & !ROLL_FLAG_TYPE_NEED_LIKE_CPP
    );
}

#[tokio::test]
async fn loot_unit_group_loot_disenchant_mask_uses_cpp_skill_required_gate() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_059);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, _candidate_rx) = flume::bounded::<Vec<u8>>(4);
    let player_registry = Arc::new(PlayerRegistry::default());
    let candidate_info = broadcast_info(candidate_guid, candidate_tx);
    player_registry.register_or_replace(candidate_guid, candidate_info, Default::default());
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let mut candidate = wow_entities::Player::new(Some(1), false);
    candidate
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(candidate_guid);
    candidate.unit_mut().world_mut().set_map(0, 0).unwrap();
    candidate.unit_mut().world_mut().object_mut().add_to_world();
    candidate
        .gameplay_state_mut()
        .skills
        .push(wow_entities::PlayerSkillRecord {
            skill_line_id: u32::from(ENCHANTING_SKILL_FOR_TEST),
            current_value: 175,
            max_value: 225,
            step: 0,
            profession_slot: -1,
            state: wow_entities::PlayerSkillLoadState::Unchanged,
        });
    canonical
        .lock()
        .unwrap()
        .create_world_map(0, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(candidate).unwrap())
        .unwrap();
    assert!(player_registry.bind_canonical_map_manager(canonical));
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    install_disenchantable_test_item_template(&mut session, 25);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    set_loot_for_test(
        &mut session,
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let start_roll = send_rx.try_recv().unwrap();
    let mut start_roll = WorldPacket::from_bytes(&start_roll);
    assert_eq!(
        start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
    assert_eq!(start_roll.read_uint8().unwrap(), 0x0F);
}

#[tokio::test]
async fn loot_unit_group_loot_single_candidate_unblocks_under_threshold_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_050);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    session.set_player_guid(Some(player_guid));
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    set_loot_for_test(
        &mut session,
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    assert!(send_rx.try_recv().is_err());

    let entry = &loot_for_test(&session, owner_guid).unwrap().items[0];
    assert!(!entry.flags.blocked);
    assert!(entry.flags.under_threshold);
}

#[tokio::test]
async fn loot_unit_group_loot_pass_on_loot_suppresses_current_prompt_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_051);
    let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
    let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(4);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        candidate_guid,
        broadcast_info(candidate_guid, candidate_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);
    session.set_player_guid(Some(player_guid));
    set_pass_on_group_loot_for_test_like_cpp(&mut session, true);
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    set_loot_for_test(
        &mut session,
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, candidate_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    follow_loot_rules: true,
                    blocked: true,
                    ..Default::default()
                },
                allowed_looters: vec![player_guid, candidate_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    handle_loot_unit_for_test(&mut session, loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    let local_auto_pass = send_rx.try_recv().unwrap();
    let mut local_auto_pass = WorldPacket::from_bytes(&local_auto_pass);
    assert_eq!(
        local_auto_pass.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(local_auto_pass.read_packed_guid().unwrap(), loot_object);
    assert_eq!(local_auto_pass.read_packed_guid().unwrap(), player_guid);
    assert_eq!(local_auto_pass.read_int32().unwrap(), -1);
    assert_eq!(
        local_auto_pass.read_uint8().unwrap(),
        ROLL_VOTE_PASS_LIKE_CPP
    );
    assert!(send_rx.try_recv().is_err());

    let _remote_loot_list = candidate_rx.try_recv().unwrap();
    let remote_start_roll = candidate_rx.try_recv().unwrap();
    let mut remote_start_roll = WorldPacket::from_bytes(&remote_start_roll);
    assert_eq!(
        remote_start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    let remote_auto_pass = candidate_rx.try_recv().unwrap();
    let mut remote_auto_pass = WorldPacket::from_bytes(&remote_auto_pass);
    assert_eq!(
        remote_auto_pass.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRoll as u16
    );
    assert_eq!(remote_auto_pass.read_packed_guid().unwrap(), loot_object);
    assert_eq!(remote_auto_pass.read_packed_guid().unwrap(), player_guid);
    assert_eq!(remote_auto_pass.read_int32().unwrap(), -1);
    assert_eq!(
        remote_auto_pass.read_uint8().unwrap(),
        ROLL_VOTE_PASS_LIKE_CPP
    );

    let entry = &loot_for_test(&session, owner_guid).unwrap().items[0];
    assert!(entry.flags.blocked);
    assert!(!entry.flags.under_threshold);
}
