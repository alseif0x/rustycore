//! Public creature-loot packet scenarios.

use super::support::*;

#[tokio::test]
async fn loot_unit_live_creature_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_006);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, true));

    handle_loot_unit_for_test(&mut session, loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!is_active_loot_guid_for_test(&session, loot_guid));
}

#[tokio::test]
async fn loot_unit_non_creature_guid_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_019);
    session.set_player_guid(Some(player_guid));

    handle_loot_unit_for_test(&mut session, loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!is_active_loot_guid_for_test(&session, loot_guid));
    assert!(!has_loot_for_test(&session, loot_guid));
}

#[tokio::test]
async fn loot_unit_creature_too_far_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_016);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    let mut creature = test_creature(loot_guid, false);
    creature.current_pos = Position::new(31.0, 0.0, 0.0, 0.0);
    register_test_creature_like_cpp(&mut session, creature);

    handle_loot_unit_for_test(&mut session, loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!is_active_loot_guid_for_test(&session, loot_guid));
    assert!(!has_loot_for_test(&session, loot_guid));
}

#[tokio::test]
async fn loot_money_non_allowed_active_creature_does_not_take_coins_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_100);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, other_guid, 7);

    handle_loot_money_for_test(&mut session, loot_money_packet()).await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(player_gold_for_test(&session), 0);
    assert_eq!(loot_for_test(&session, loot_guid).unwrap().coins, 7);
}

#[tokio::test]
async fn loot_item_creature_too_far_uses_cpp_error() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_008);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_for_test(&mut session, loot_guid);

    let mut creature = test_creature(loot_guid, false);
    creature.current_pos = Position::new(31.0, 0.0, 0.0, 0.0);
    register_test_creature_like_cpp(&mut session, creature);
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
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0)).await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_TOO_FAR_LIKE_CPP
    );
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
    assert!(is_active_loot_guid_for_test(&session, loot_guid));
}

#[tokio::test]
async fn loot_item_creature_distance_can_use_canonical_map_object_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_018);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    set_active_loot_guid_for_test(&mut session, loot_guid);
    attach_canonical_map_object(
        &mut session,
        AccessorObjectKind::Creature,
        canonical_world_object(loot_guid, 0, Position::new(31.0, 0.0, 0.0, 0.0)),
    );
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
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0)).await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_TOO_FAR_LIKE_CPP
    );
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
    assert!(is_active_loot_guid_for_test(&session, loot_guid));
}

#[tokio::test]
async fn loot_item_missing_creature_uses_cpp_no_loot_error() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_009);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
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
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_loot_item_for_test(&mut session, loot_item_packet(loot_guid, 0)).await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_NO_LOOT_LIKE_CPP
    );
    assert!(!loot_for_test(&session, loot_guid).unwrap().items[0].taken);
    assert!(is_active_loot_guid_for_test(&session, loot_guid));
}
