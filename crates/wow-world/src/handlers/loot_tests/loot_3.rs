//! Loot scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn represented_fishing_node_loot_walks_parent_area_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_051);
    let item_id = 80_001;
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        AreaTableEntry {
            id: 77,
            continent_id: 0,
            parent_area_id: 10,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        AreaTableEntry {
            id: 10,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    install_limited_test_item_template(&mut session, item_id, 0);
    let mut fishing_store = LootStore::for_kind_like_cpp(LootStoreKind::Fishing);
    fishing_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 10,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Fishing, fishing_store);
    session.set_loot_stores(Arc::new(stores));

    session
        .open_represented_fishing_node_loot_like_cpp(gameobject_guid, 77, false)
        .await;

    let loot = session.loot_table.get(&gameobject_guid).unwrap();
    assert_eq!(loot.loot_type, LOOT_TYPE_FISHING_LIKE_CPP);
    assert_eq!(loot.items.len(), 1);
    assert_eq!(loot.items[0].item_id, item_id);
    assert!(session.is_active_loot_guid(gameobject_guid));
}
#[tokio::test]
async fn represented_fishing_node_junk_loot_uses_default_zone_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_052);
    let item_id = 80_002;
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    install_limited_test_item_template(&mut session, item_id, 0);
    let mut fishing_store = LootStore::for_kind_like_cpp(LootStoreKind::Fishing);
    fishing_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 1,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_JUNK_FISH_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Fishing, fishing_store);
    session.set_loot_stores(Arc::new(stores));

    session
        .open_represented_fishing_node_loot_like_cpp(gameobject_guid, 77, true)
        .await;

    let loot = session.loot_table.get(&gameobject_guid).unwrap();
    assert_eq!(loot.loot_type, LOOT_TYPE_FISHING_JUNK_LIKE_CPP);
    assert_eq!(loot.items.len(), 1);
    assert_eq!(loot.items[0].item_id, item_id);
    assert!(session.is_active_loot_guid(gameobject_guid));
}
#[tokio::test]
async fn loot_unit_dead_player_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_033);
    session.set_player_guid(Some(player_guid));
    session.set_player_alive_like_cpp(false);
    install_active_spell_cast(&mut session, player_guid);
    install_visible_aura_with_interrupt_flags(
        &mut session,
        3,
        777,
        player_guid,
        SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP,
    );
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
    assert!(session.active_spell_cast_snapshot_like_cpp().is_some());
    assert!(session.visible_auras.contains_key(&3));
}
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
    session.loot_table.insert(
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

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
    assert!(
        session
            .loot_table
            .get(&owner_guid)
            .is_some_and(|loot| loot.looted_by_player)
    );
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
    session.loot_table.insert(
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

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
    session.loot_table.insert(
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

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
    session.loot_table.insert(
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

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
            skill_line_id: u32::from(crate::session::SKILL_ENCHANTING_LIKE_CPP),
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
    session.loot_table.insert(
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

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
#[test]
fn represented_disenchant_loot_template_row_guards_match_cpp_shape() {
    let valid = LootStoreItem {
        item_id: 10940,
        reference: 0,
        chance: 100.0,
        needs_quest: false,
        loot_mode: super::super::LOOT_MODE_DEFAULT_LIKE_CPP,
        group_id: 0,
        min_count: 1,
        max_count: 2,
    };
    assert!(super::super::represented_disenchant_loot_plain_row_can_roll_like_cpp(&valid, true));

    let mut missing_item = valid;
    missing_item.item_id = 0;
    assert!(
        !super::super::represented_disenchant_loot_plain_row_can_roll_like_cpp(&missing_item, true)
    );

    let mut bad_count = valid;
    bad_count.max_count = 0;
    assert!(
        !super::super::represented_disenchant_loot_plain_row_can_roll_like_cpp(&bad_count, true)
    );

    let reference = LootStoreItem {
        item_id: 0,
        reference: 700,
        chance: 100.0,
        needs_quest: false,
        loot_mode: super::super::LOOT_MODE_DEFAULT_LIKE_CPP,
        group_id: 0,
        min_count: 1,
        max_count: 1,
    };
    assert!(super::super::represented_disenchant_loot_reference_row_can_roll_like_cpp(&reference));
}
#[test]
fn represented_disenchant_loot_template_frame_splits_group_rows_like_cpp() {
    let rows = vec![
        LootStoreItem {
            item_id: 10940,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: super::super::LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 0,
            min_count: 1,
            max_count: 1,
        },
        LootStoreItem {
            item_id: 10978,
            reference: 0,
            chance: 0.0,
            needs_quest: false,
            loot_mode: super::super::LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 2,
            min_count: 1,
            max_count: 1,
        },
        LootStoreItem {
            item_id: 0,
            reference: 700,
            chance: 100.0,
            needs_quest: false,
            loot_mode: super::super::LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 2,
            min_count: 1,
            max_count: 1,
        },
    ];

    let frame = super::super::disenchant_loot_template_frame_like_cpp(rows, 0);

    assert_eq!(frame.template.entries().len(), 2);
    assert_eq!(frame.template.groups().len(), 2);
    assert_eq!(frame.template.groups()[1].equal_chanced().len(), 1);
    assert_eq!(frame.template.entries()[1].reference, 700);
    assert_eq!(frame.template.entries()[1].group_id, 2);
}
#[test]
fn represented_disenchant_group_roll_uses_caller_rng_like_cpp_count() {
    let rows = vec![LootStoreItem {
        item_id: 10940,
        reference: 0,
        chance: 100.0,
        needs_quest: false,
        loot_mode: super::super::LOOT_MODE_DEFAULT_LIKE_CPP,
        group_id: 1,
        min_count: 2,
        max_count: 7,
    }];
    let frame = super::super::disenchant_loot_template_frame_like_cpp(rows, 1);
    let group = &frame.template.groups()[0];

    let mut expected_rng = StdRng::seed_from_u64(0xD15E);
    let _expected_group = group.roll_like_cpp(
        super::super::LOOT_MODE_DEFAULT_LIKE_CPP,
        &mut expected_rng,
        |_| true,
    );
    let expected_count = expected_rng.gen_range(2..=7);

    let mut rng = StdRng::seed_from_u64(0xD15E);
    let row = group
        .roll_like_cpp(super::super::LOOT_MODE_DEFAULT_LIKE_CPP, &mut rng, |_| true)
        .expect("group should roll the guaranteed disenchant row");
    let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));

    assert_eq!(row.item_id, 10940);
    assert_eq!(count, expected_count);
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
    session.loot_table.insert(
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let _response = send_rx.try_recv().unwrap();
    let _loot_list = send_rx.try_recv().unwrap();
    assert!(send_rx.try_recv().is_err());

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
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
    session.pass_on_group_loot = true;
    install_group_loot_group(&mut session, player_guid, candidate_guid);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.loot_table.insert(
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
    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

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

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(entry.flags.blocked);
    assert!(!entry.flags.under_threshold);
}
