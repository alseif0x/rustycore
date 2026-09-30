//! Original loot application cases with explicit operation-local fixture permission.
use super::interaction_support::*;

#[tokio::test]
async fn loot_item_releases_blocked_item_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_003);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    set_loot_for_test(
        &mut session,
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

    // Explicit setup replaces only the historical cfg(test) first-request binding.
    install_cached_test_creature_loot_authority_for_test(&mut session, loot_guid, player_guid);
    let authority = loot_recovery_authority_for_test(&mut session, loot_guid).unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    bind_loot_view_for_test(&mut session, loot_guid, generation, &authority);

    take_local_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(is_active_loot_guid_for_test(&session, loot_guid));
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
}

#[tokio::test]
async fn loot_item_releases_when_player_is_not_allowed_looter_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_004);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    set_loot_for_test(
        &mut session,
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

    // Explicit setup replaces only the historical cfg(test) first-request binding.
    install_cached_test_creature_loot_authority_for_test(&mut session, loot_guid, player_guid);
    let authority = loot_recovery_authority_for_test(&mut session, loot_guid).unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    bind_loot_view_for_test(&mut session, loot_guid, generation, &authority);

    take_local_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(is_active_loot_guid_for_test(&session, loot_guid));
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
}

#[tokio::test]
async fn loot_item_releases_when_roll_winner_is_different_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let winner_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_005);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    set_loot_for_test(
        &mut session,
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

    // Explicit setup replaces only the historical cfg(test) first-request binding.
    install_cached_test_creature_loot_authority_for_test(&mut session, loot_guid, player_guid);
    let authority = loot_recovery_authority_for_test(&mut session, loot_guid).unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    bind_loot_view_for_test(&mut session, loot_guid, generation, &authority);

    take_local_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootReleaseAll as u16
    );
    assert_eq!(sent.remaining(), 0);
    assert!(is_active_loot_guid_for_test(&session, loot_guid));
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
}
