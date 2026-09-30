//! Public gameobject-loot item rejection scenarios.

use super::support::*;

#[tokio::test]
async fn loot_item_missing_gameobject_uses_cpp_release() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_010);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    set_loot_for_test(&mut session,
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
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
    assert!(is_active_loot_guid_for_test(&session, loot_guid));
}

#[tokio::test]
async fn loot_item_gameobject_too_far_uses_cpp_release() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_029);
    let go_position = Position::new(6.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_for_test(&mut session, loot_guid);
    attach_canonical_map_object(
        &mut session,
        AccessorObjectKind::GameObject,
        canonical_world_object(loot_guid, 0, go_position),
    );
    record_represented_gameobject_runtime_state_for_test(&mut session, 
        0,
        loot_guid,
        loot_guid.entry(),
        go_position,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
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

    handle_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
    assert!(is_active_loot_guid_for_test(&session, loot_guid));
}
