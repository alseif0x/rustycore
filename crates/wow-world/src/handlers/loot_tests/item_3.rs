//! Item scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn loot_item_random_context_stack_compatibility_uses_cpp_store_metadata() {
    let item_guid = ObjectGuid::create_item(1, 901);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let mut item = Item::new(0);
    item.initialize_created_state(ItemCreateInfo {
        guid: item_guid,
        item_id: 25,
        context: ItemContext::DungeonHeroic,
        owner: Some(owner_guid),
        max_durability: 0,
        expiration: 0,
        spell_charges: [0; MAX_ITEM_SPELLS],
    });
    item.set_random_properties_id(-77);
    item.set_property_seed(456);

    let matching = LootEntry {
        loot_list_id: 0,
        item_id: 25,
        quantity: 1,
        random_properties_id: -77,
        random_properties_seed: 456,
        item_context: 2,
        flags: LootEntryFlags::default(),
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    };
    assert!(loot_store_data_can_stack_with_item(
        &matching,
        LootStoreRandomProperties { id: -77, seed: 456 },
        &item
    ));

    let different_random = LootEntry {
        random_properties_id: -78,
        ..matching.clone()
    };
    assert!(loot_store_data_can_stack_with_item(
        &different_random,
        LootStoreRandomProperties { id: -77, seed: 456 },
        &item
    ));
    assert!(!loot_store_data_can_stack_with_item(
        &matching,
        LootStoreRandomProperties { id: 0, seed: 0 },
        &item
    ));
}
#[test]
fn loot_item_store_random_properties_are_generated_from_cpp_random_select() {
    let entry = LootEntry {
        loot_list_id: 0,
        item_id: 25,
        quantity: 1,
        random_properties_id: -77,
        random_properties_seed: 456,
        item_context: 2,
        flags: LootEntryFlags::default(),
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    };
    let mut session = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([test_item_record(
        entry.item_id,
        77,
        0,
    )])));
    session.set_item_random_enchantment_template_store(Arc::new(
        ItemRandomEnchantmentTemplateStore::from_entries([ItemRandomEnchantmentTemplateEntry {
            group_id: 77,
            enchantment_id: 9001,
            chance: 100.0,
        }]),
    ));
    session.set_item_random_properties_store(Arc::new(ItemRandomPropertiesStore::from_entries([
        ItemRandomPropertiesEntry {
            id: 9001,
            enchantments: [1, 2, 3, 0, 0],
        },
    ])));

    let generated = session.generate_loot_store_random_properties_with_rng_like_cpp(
        entry.item_id,
        &mut StdRng::seed_from_u64(1),
    );
    assert_eq!(generated, LootStoreRandomProperties { id: 9001, seed: 0 });
    assert_ne!(generated.id, entry.random_properties_id);
    assert_ne!(generated.seed, entry.random_properties_seed);
}
#[test]
fn loot_item_store_random_suffix_uses_cpp_property_points_seed() {
    let mut session = make_session();
    session.set_item_store(Arc::new(ItemStore::from_records([test_item_record(
        25, 0, 88,
    )])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_random_property_templates([
        (
            25,
            ItemRandomPropertyTemplateEntry {
                item_level: 11,
                quality: ItemQuality::Uncommon as i8,
                inventory_type: InventoryType::Chest as i8,
            },
        ),
    ])));
    session.set_item_random_enchantment_template_store(Arc::new(
        ItemRandomEnchantmentTemplateStore::from_entries([ItemRandomEnchantmentTemplateEntry {
            group_id: 88,
            enchantment_id: 7001,
            chance: 100.0,
        }]),
    ));
    session.set_item_random_suffix_store(Arc::new(ItemRandomSuffixStore::from_entries([
        ItemRandomSuffixEntry {
            id: 7001,
            enchantments: [10, 0, 0, 0, 0],
            allocation_pct: [10000, 0, 0, 0, 0],
        },
    ])));
    session.set_rand_prop_points_store(Arc::new(RandPropPointsStore::from_entries([
        RandPropPointsEntry {
            id: 11,
            damage_replace_stat: 0,
            epic: [900, 0, 0, 0, 0],
            superior: [500, 0, 0, 0, 0],
            good: [123, 0, 0, 0, 0],
        },
    ])));

    let generated = session
        .generate_loot_store_random_properties_with_rng_like_cpp(25, &mut StdRng::seed_from_u64(1));
    assert_eq!(
        generated,
        LootStoreRandomProperties {
            id: -7001,
            seed: 123
        }
    );
}
#[test]
fn loot_is_looted_requires_no_money_and_no_unlooted_items_like_cpp() {
    let mut loot = CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins: 1,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: vec![],
        looted_by_player: false,
    };
    assert!(!loot_is_looted_like_cpp(&loot));

    loot.coins = 0;
    loot.items.push(LootEntry {
        loot_list_id: 0,
        item_id: 25,
        quantity: 1,
        random_properties_id: 0,
        random_properties_seed: 0,
        item_context: 0,
        flags: LootEntryFlags::default(),
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    });
    loot.unlooted_count = 1;
    assert!(!loot_is_looted_like_cpp(&loot));

    loot.items[0].taken = true;
    assert!(!loot_is_looted_like_cpp(&loot));

    loot.unlooted_count = 0;
    assert!(loot_is_looted_like_cpp(&loot));
}
#[tokio::test]
async fn loot_unit_group_loot_first_open_starts_roll_for_blocked_item_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let disconnected_guid = ObjectGuid::create_player(1, 88);
    let owner_guid = test_creature_guid(19_049);
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
                allowed_looters: vec![player_guid, candidate_guid, disconnected_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, owner_guid, player_guid);
    let item_valuation = session.item_valuation_catalogs_for_test_like_cpp();
    session
        .handle_loot_unit_with_catalogs_like_cpp(&item_valuation, loot_unit_packet(owner_guid))
        .await;

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

    let start_roll = send_rx.try_recv().unwrap();
    let mut start_roll = WorldPacket::from_bytes(&start_roll);
    assert_eq!(
        start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
    assert_eq!(start_roll.read_uint8().unwrap(), 0x07);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint32().unwrap(), 0);
    assert_eq!(start_roll.read_uint8().unwrap(), LOOT_METHOD_GROUP_LIKE_CPP);
    assert_eq!(start_roll.read_int32().unwrap(), 0);
    assert_eq!(start_roll.read_bits(2).unwrap(), 0);
    assert_eq!(start_roll.read_bits(3).unwrap(), 1);
    assert!(send_rx.try_recv().is_err());

    let remote_loot_list = candidate_rx.try_recv().unwrap();
    let mut remote_loot_list = WorldPacket::from_bytes(&remote_loot_list);
    assert_eq!(
        remote_loot_list.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootList as u16
    );
    let remote_start_roll = candidate_rx.try_recv().unwrap();
    let mut remote_start_roll = WorldPacket::from_bytes(&remote_start_roll);
    assert_eq!(
        remote_start_roll.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::StartLootRoll as u16
    );
    assert_eq!(remote_start_roll.read_packed_guid().unwrap(), loot_object);

    let state = session
        .represented_loot_rolls
        .get(&(loot_object, 0))
        .unwrap();
    assert_eq!(
        state.voters.get(&player_guid).unwrap().vote,
        ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
    );
    assert_eq!(
        state.voters.get(&candidate_guid).unwrap().vote,
        ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
    );
    assert_eq!(
        state.voters.get(&disconnected_guid).unwrap().vote,
        ROLL_VOTE_NOT_VALID_LIKE_CPP
    );

    let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
    assert!(entry.flags.blocked);
    assert!(!entry.flags.under_threshold);
    assert!(
        session
            .loot_table
            .get(&owner_guid)
            .unwrap()
            .looted_by_player
    );
}
#[tokio::test]
async fn item_loot_releases_ae_view_and_tracks_multiple_items_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let player_guid = ObjectGuid::create_player(1, 61_718);
    let primary_guid = test_creature_guid(61_719);
    let secondary_guid = test_creature_guid(61_720);
    let first_item = ObjectGuid::create_item(1, 61_721);
    let second_item = ObjectGuid::create_item(1, 61_722);
    let secondary_authority =
        open_test_ae_pair_like_cpp(&mut session, player_guid, primary_guid, secondary_guid).await;

    session
        .handle_loot_release(loot_release_packet(primary_guid))
        .await;
    assert!(session.active_loot_guid.is_empty());
    assert!(session.active_loot_view_owners.contains(&secondary_guid));

    session
        .open_active_item_loot_view_like_cpp(player_guid, first_item)
        .await;
    session
        .open_active_item_loot_view_like_cpp(player_guid, second_item)
        .await;

    assert!(session.is_active_loot_guid(first_item));
    assert_eq!(session.active_loot_view_owners.len(), 2);
    assert!(session.active_loot_view_owners.contains(&first_item));
    assert!(session.active_loot_view_owners.contains(&second_item));
    assert!(!session.active_loot_view_owners.contains(&secondary_guid));
    assert!(
        !secondary_authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid),
        "item loot must release the surviving secondary AE viewer first"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        2
    );
}
#[tokio::test]
async fn loot_item_uses_active_loot_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_guid = test_creature_guid(19_001);
    let inactive_guid = test_creature_guid(19_002);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(active_guid);
    session.loot_table.insert(
        inactive_guid,
        CreatureLoot {
            loot_guid: inactive_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
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
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(inactive_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.loot_table.get(&inactive_guid).unwrap().items[0].taken);
}
#[tokio::test]
async fn loot_item_releases_blocked_item_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_003);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
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

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
}
#[tokio::test]
async fn loot_item_releases_when_player_is_not_allowed_looter_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_004);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![other_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
}
#[tokio::test]
async fn loot_item_releases_when_roll_winner_is_different_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let winner_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_005);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: winner_guid,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
}
#[tokio::test]
async fn master_loot_item_without_group_sends_didnt_kill_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_master_loot_item(MasterLootItem {
            target: ObjectGuid::create_player(1, 77),
            loot: Vec::new(),
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        wow_packet::packets::loot::LOOT_ERROR_DIDNT_KILL_LIKE_CPP
    );
}
#[tokio::test]
async fn master_loot_item_uses_group_master_looter_guid_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader_guid = ObjectGuid::create_player(1, 42);
    let master_guid = ObjectGuid::create_player(1, 43);
    let (leader_tx, _leader_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader_guid);
    group.add_member(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        leader_guid,
        broadcast_info(leader_guid, leader_tx),
        Default::default(),
    );
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(leader_guid));

    session
        .handle_master_loot_item(MasterLootItem {
            target: master_guid,
            loot: Vec::new(),
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        wow_packet::packets::loot::LOOT_ERROR_DIDNT_KILL_LIKE_CPP
    );

    session.set_player_guid(Some(master_guid));
    session
        .handle_master_loot_item(MasterLootItem {
            target: leader_guid,
            loot: Vec::new(),
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn master_loot_item_missing_target_sends_player_not_found_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let missing_target = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));

    session
        .handle_master_loot_item(MasterLootItem {
            target: missing_target,
            loot: Vec::new(),
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP
    );
}
#[tokio::test]
async fn master_loot_item_non_master_loot_view_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let loot_owner = test_creature_guid(19_082);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    session.set_active_loot_guid(loot_owner);
    session.loot_table.insert(
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_master_loot_item(MasterLootItem {
            target: master_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn master_loot_item_ineligible_target_sends_master_other_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let target_guid = ObjectGuid::create_player(1, 77);
    let loot_owner = test_creature_guid(19_080);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let (target_tx, _target_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        target_guid,
        broadcast_info(target_guid, target_tx),
        Default::default(),
    );
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    session.set_active_loot_guid(loot_owner);
    session.loot_table.insert(
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid, target_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_master_loot_item(MasterLootItem {
            target: target_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_MASTER_OTHER_LIKE_CPP);
}
