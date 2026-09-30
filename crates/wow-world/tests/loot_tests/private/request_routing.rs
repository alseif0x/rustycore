//! Original application cases using the feature fixtures and production operations.
use super::support::*;

#[tokio::test]
async fn loot_item_uses_active_loot_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_guid = test_creature_guid(19_001);
    let inactive_guid = test_creature_guid(19_002);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, active_guid);
    set_loot_for_test(
        &mut session,
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

    handle_loot_item_for_test(&mut session, loot_item_packet(inactive_guid, 0)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!loot_for_test(&session, inactive_guid).unwrap().items[0].taken);
}

#[tokio::test]
async fn loot_item_request_uses_loot_object_to_find_active_owner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_023);
    let loot_object_guid = represented_loot_object_guid_like_cpp(owner_guid);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, owner_guid);
    set_loot_for_test(
        &mut session,
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object_guid,
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
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_loot_item_for_test(&mut session, loot_item_packet(loot_object_guid, 0)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object_guid);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_NO_LOOT_LIKE_CPP);
    assert!(!loot_for_test(&session, owner_guid).unwrap().items[0].taken);
    assert!(is_active_loot_guid_for_test(&session, owner_guid));
}

#[tokio::test]
async fn loot_item_request_can_use_secondary_active_loot_object_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let primary_owner = test_creature_guid(19_027);
    let secondary_owner = test_creature_guid(19_028);
    let secondary_loot_object = represented_loot_object_guid_like_cpp(secondary_owner);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, primary_owner);
    add_active_loot_view_owner_for_test(&mut session, secondary_owner);
    set_loot_for_test(
        &mut session,
        secondary_owner,
        CreatureLoot {
            loot_guid: secondary_loot_object,
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
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_loot_item_for_test(&mut session, loot_item_packet(secondary_loot_object, 0)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_loot_object);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_NO_LOOT_LIKE_CPP);
    assert!(!loot_for_test(&session, secondary_owner).unwrap().items[0].taken);
    assert!(active_loot_view_owners_for_test(&session).contains(&primary_owner));
    assert!(active_loot_view_owners_for_test(&session).contains(&secondary_owner));
}
