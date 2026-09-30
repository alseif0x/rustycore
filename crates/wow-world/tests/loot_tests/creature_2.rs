//! Public creature-loot release scenarios.

use super::support::*;

#[tokio::test]
async fn loot_release_keeps_unlooted_creature_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = test_creature_guid(19_013);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    set_world_creature_personal_loot_for_test(
        &mut session,
        loot_guid,
        player_guid,
        CreatureOwnedLoot::new(0, 1),
    );
    let corpse_despawn_before = world_creature_corpse_deadline_for_test(&mut session, loot_guid)
        .expect("C++ arms corpse removal when the creature reaches JUST_DIED");
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, other_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!is_active_loot_guid_for_test(&session, loot_guid));
    assert!(
        !has_loot_for_test(&session, loot_guid),
        "the closed session view is a discardable cache; the creature authority keeps loot"
    );
    assert!(reconcile_loot_cache_for_test(&mut session, loot_guid, player_guid));
    assert_eq!(loot_for_test(&session, loot_guid).unwrap().coins, 7);
    assert!(has_loot_for_test(&session, loot_guid));
    assert_eq!(
        loot_for_test(&session, loot_guid).unwrap().players_looting,
        vec![other_guid]
    );
    assert_eq!(
        world_creature_corpse_deadline_for_test(&mut session, loot_guid),
        Some(corpse_despawn_before),
        "releasing partial loot must not change the existing corpse timer"
    );
}

#[tokio::test]
async fn creature_owned_loot_release_partial_uses_canonical_is_fully_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = test_creature_guid(19_113);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let corpse_despawn_before = world_creature_corpse_deadline_for_test(&mut session, loot_guid)
        .expect("C++ arms corpse removal when the creature reaches JUST_DIED");
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, other_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(!is_active_loot_guid_for_test(&session, loot_guid));
    assert!(!has_loot_for_test(&session, loot_guid));
    let canonical = canonical_creature_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(7, 0))
    );
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        canonical.loot_for_player_like_cpp(other_guid),
        Some(&CreatureOwnedLoot::new(7, 0))
    );
    assert!(!canonical.is_fully_looted_like_cpp());
    assert!(reconcile_loot_cache_for_test(&mut session, loot_guid, player_guid));
    assert_eq!(loot_for_test(&session, loot_guid).unwrap().coins, 7);
    assert!(has_loot_for_test(&session, loot_guid));
    assert_eq!(
        world_creature_corpse_deadline_for_test(&mut session, loot_guid),
        Some(corpse_despawn_before),
        "releasing partial canonical loot must not change the existing corpse timer"
    );
}

#[tokio::test]
async fn creature_owned_loot_release_fully_consumed_uses_canonical_is_fully_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_114);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    set_world_creature_corpse_delay_for_test(&mut session, loot_guid, 120, false);
    assert_eq!(
        world_creature_corpse_delay_for_test(&mut session, loot_guid),
        Some(120)
    );
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
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(!is_active_loot_guid_for_test(&session, loot_guid));
    assert!(!has_loot_for_test(&session, loot_guid));
    let canonical = canonical_creature_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::default())
    );
    assert!(canonical.is_fully_looted_like_cpp());
    let corpse_despawn_at = world_creature_corpse_despawn_at_for_test(&mut session, loot_guid)
        .expect("fully looted corpse should start decay timer");
    let remaining = corpse_despawn_at.saturating_duration_since(Instant::now());
    assert!(
        (55..=60).contains(&remaining.as_secs()),
        "C++ uses corpse_delay * Rate.Corpse.Decay.Looted; got {remaining:?}"
    );
}

#[tokio::test]
async fn creature_owned_loot_release_does_not_extend_expired_corpse_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_118);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, true));
    let expired = Instant::now() - Duration::from_secs(1);
    let deadline_before =
        set_world_creature_expired_corpse_for_test(&mut session, loot_guid, expired).unwrap();
    assert!(world_creature_corpse_despawn_due_for_test(
        &mut session,
        loot_guid
    ));
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
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(!has_loot_for_test(&session, loot_guid));
    let deadline_after = world_creature_corpse_deadline_for_test(&mut session, loot_guid)
        .expect("expired corpse must retain its existing lifecycle deadline");
    assert_eq!(deadline_after, deadline_before);
    assert!(world_creature_corpse_despawn_due_for_test(
        &mut session,
        loot_guid
    ));
}

#[tokio::test]
async fn creature_owned_loot_release_fully_consumed_removes_lootable_dynflag_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_116);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    insert_client_visible_guid_for_test(&mut session, loot_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    apply_world_creature_corpse_loot_flags_for_test(&mut session, loot_guid, true, false);
    assert!(world_creature_has_lootable_dynamic_flag_for_test(
        &mut session,
        loot_guid
    ));
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
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx),
        vec![
            wow_constants::ServerOpcodes::LootRelease as u16,
            wow_constants::ServerOpcodes::UpdateObject as u16,
        ],
        "C++ ForceUpdateFieldChange must become a visible VALUES update so the client removes the loot cursor"
    );
    assert!(
        !world_creature_has_lootable_dynamic_flag_for_test(&mut session, loot_guid),
        "C++ LootHandler::DoLootRelease removes UNIT_DYNFLAG_LOOTABLE when the creature is fully looted"
    );
}

#[tokio::test]
async fn player_corpse_loot_release_removes_corpse_lootable_dynflag_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let corpse_guid = test_corpse_guid(19_117);
    let corpse = make_canonical_corpse_for_loot_test(&session, corpse_guid);
    attach_canonical_corpse_for_loot_test(&mut session, corpse);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, corpse_guid);
    set_loot_for_test(
        &mut session,
        corpse_guid,
        CreatureLoot {
            loot_guid: corpse_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_INSIGNIA_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    assert_eq!(
        canonical_corpse_snapshot_for_loot_test(&session, corpse_guid)
            .unwrap()
            .data()
            .dynamic_flags
            & CORPSE_DYNFLAG_LOOTABLE,
        CORPSE_DYNFLAG_LOOTABLE
    );

    session
        .handle_loot_release(loot_release_packet(corpse_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let corpse = canonical_corpse_snapshot_for_loot_test(&session, corpse_guid).unwrap();
    assert_eq!(
        corpse.data().dynamic_flags & CORPSE_DYNFLAG_LOOTABLE,
        0,
        "C++ DoLootRelease removes CORPSE_DYNFLAG_LOOTABLE from fully looted player corpses"
    );
    assert!(
        corpse
            .corpse_data_changes_mask()
            .is_set(wow_entities::CORPSE_DATA_DYNAMIC_FLAGS_BIT)
    );
}

#[tokio::test]
async fn creature_skinning_loot_release_despawns_corpse_immediately_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_115);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    set_active_loot_guid_for_test(&mut session, loot_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    set_world_creature_corpse_delay_for_test(&mut session, loot_guid, 120, false);
    set_loot_for_test(&mut session,
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_SKINNING_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let corpse_despawn_at = world_creature_corpse_despawn_at_for_test(&mut session, loot_guid)
        .expect("skinned corpse should start decay timer");
    let remaining = corpse_despawn_at.saturating_duration_since(Instant::now());
    assert_eq!(
        remaining.as_secs(),
        0,
        "C++ sets m_corpseRemoveTime = now for fully looted LOOT_SKINNING; got {remaining:?}"
    );
}
