//! Route boundaries added for the per-call legacy fixture policy.
//! These cases supplement the nineteen preserved original money cases.

use super::money_support::*;

fn install_personal_money_cache(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid) {
    insert_client_visible_guid_for_test(session, owner);
    set_loot_for_test(
        session,
        owner,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(owner),
            coins: 999,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 733,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player, ObjectGuid::create_player(1, 77)],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    set_personal_loot_for_loot_test(session, owner, player, 123);
    set_personal_loot_for_loot_test(session, owner, ObjectGuid::create_player(1, 77), 456);
}

#[tokio::test]
async fn normal_money_without_authority_stays_closed() {
    let (mut session, rx) = make_session_with_send_capacity(8);
    let player = ObjectGuid::create_player(1, 42);
    let owner = test_gameobject_guid(91_014);
    session.set_player_guid(Some(player));
    prepare_personal_money_fixture(&mut session, true);
    install_personal_money_cache(&mut session, owner, player);
    set_active_loot_guid_for_test(&mut session, owner);

    handle_loot_money_for_test(&mut session, loot_money_packet()).await;

    assert!(rx.try_recv().is_err());
    assert_eq!(
        personal_loot_marker_for_test(&session, owner, player).1,
        Some(123)
    );
    assert_eq!(
        personal_loot_marker_for_test(&session, owner, ObjectGuid::create_player(1, 77)).1,
        Some(456)
    );
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner).unwrap().coins,
        999
    );
    assert_eq!(player_gold_for_test(&session), 0);
}

#[tokio::test]
async fn fixture_money_cannot_consume_retired_owner() {
    let (mut session, rx) = make_session_with_send_capacity(8);
    let player = ObjectGuid::create_player(1, 42);
    let owner = test_gameobject_guid(91_015);
    session.set_player_guid(Some(player));
    let mut gameobject =
        make_canonical_gameobject_for_loot_test(&session, owner, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.retire_loot_authority_like_cpp();
    let authority = gameobject.loot_authority_like_cpp().clone();
    let generation = authority.generation_like_cpp();
    attach_canonical_gameobject_for_loot_test(&mut session, gameobject);
    prepare_personal_money_fixture(&mut session, true);
    install_personal_money_cache(&mut session, owner, player);
    set_active_loot_guid_for_test(&mut session, owner);

    consume_personal_money_loot_for_test(&mut session, loot_money_packet()).await;

    assert!(rx.try_recv().is_err());
    assert_eq!(
        personal_loot_marker_for_test(&session, owner, player).1,
        Some(123)
    );
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner).unwrap().coins,
        999
    );
    assert_eq!(player_gold_for_test(&session), 0);
    assert!(authority.is_retired_like_cpp());
    assert_eq!(authority.generation_like_cpp(), generation);
    assert!(authority.snapshot_for_player_like_cpp(player).is_none());
}

#[tokio::test]
async fn fixture_open_rejects_deactivated_owner() {
    let (mut session, rx) = make_session_with_send_capacity(8);
    let player = ObjectGuid::create_player(1, 42);
    let owner = test_gameobject_guid(91_016);
    session.set_player_guid(Some(player));
    let mut gameobject =
        make_canonical_gameobject_for_loot_test(&session, owner, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.retire_loot_authority_like_cpp();
    let authority = gameobject.loot_authority_like_cpp().clone();
    let generation = authority.generation_like_cpp();
    attach_canonical_gameobject_for_loot_test(&mut session, gameobject);
    prepare_personal_money_fixture(&mut session, true);
    install_personal_money_cache(&mut session, owner, player);
    let source = GameObjectLootSource {
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        ..Default::default()
    };

    open_personal_money_loot_for_test(&mut session, owner, source).await;

    assert!(
        !drain_server_opcodes_like_cpp(&rx)
            .contains(&(wow_constants::ServerOpcodes::LootResponse as u16))
    );
    assert!(loot_recovery_cache_for_test(&session, owner).is_none());
    assert_eq!(active_loot_guid_for_test(&session), ObjectGuid::EMPTY);
    assert!(authority.is_retired_like_cpp());
    assert_eq!(authority.generation_like_cpp(), generation);
    assert!(authority.snapshot_for_player_like_cpp(player).is_none());
}

#[tokio::test]
async fn fixture_personal_consumption_updates_the_resident_player() {
    let (mut session, rx) = make_session_with_send_capacity(8);
    let player = ObjectGuid::create_player(1, 42);
    let owner = test_gameobject_guid(91_017);
    session.set_player_guid(Some(player));
    prepare_personal_money_fixture(&mut session, true);
    install_personal_money_cache(&mut session, owner, player);
    set_active_loot_guid_for_test(&mut session, owner);

    consume_personal_money_loot_for_test(&mut session, loot_money_packet()).await;

    assert_eq!(player_gold_for_test(&session), 123);
    assert_eq!(
        personal_loot_marker_for_test(&session, owner, player).1,
        Some(0)
    );
    assert_eq!(
        personal_loot_marker_for_test(&session, owner, ObjectGuid::create_player(1, 77)).1,
        Some(456)
    );
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner).unwrap().coins,
        999
    );
    let mut notify = recv_packet_with_opcode(&rx, wow_constants::ServerOpcodes::LootMoneyNotify);
    assert_eq!(notify.read_uint64().unwrap(), 123);
}

#[tokio::test]
async fn normal_open_without_authority_stays_closed() {
    let (mut session, rx) = make_session_with_send_capacity(8);
    let player = ObjectGuid::create_player(1, 42);
    let owner = test_gameobject_guid(91_018);
    session.set_player_guid(Some(player));
    prepare_personal_money_fixture(&mut session, true);
    install_personal_money_cache(&mut session, owner, player);
    let source = GameObjectLootSource {
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        ..Default::default()
    };

    open_money_loot_normally_for_test(&mut session, owner, source).await;

    assert!(
        !drain_server_opcodes_like_cpp(&rx)
            .contains(&(wow_constants::ServerOpcodes::LootResponse as u16))
    );
    assert!(loot_recovery_cache_for_test(&session, owner).is_none());
    assert_eq!(active_loot_guid_for_test(&session), ObjectGuid::EMPTY);
    assert_eq!(
        personal_loot_marker_for_test(&session, owner, player).1,
        Some(123)
    );
    assert_eq!(
        personal_loot_marker_for_test(&session, owner, ObjectGuid::create_player(1, 77)).1,
        Some(456)
    );
    assert_eq!(player_gold_for_test(&session), 0);
}

#[tokio::test]
async fn fixture_money_rejects_observed_owner_without_player_snapshot() {
    let (mut session, rx) = make_session_with_send_capacity(8);
    let player = ObjectGuid::create_player(1, 42);
    let other = ObjectGuid::create_player(1, 77);
    let owner = test_gameobject_guid(91_019);
    session.set_player_guid(Some(player));
    let gameobject =
        make_canonical_gameobject_for_loot_test(&session, owner, GAMEOBJECT_TYPE_CHEST as u8);
    let authority = gameobject.loot_authority_like_cpp().clone();
    let mut other_pool = authoritative_test_loot_like_cpp(456, false);
    other_pool.loot_guid = represented_loot_object_guid_like_cpp(owner);
    other_pool.allowed_looters = vec![other];
    let generation = authority.replace_like_cpp(None, HashMap::from([(other, other_pool)]));
    attach_canonical_gameobject_for_loot_test(&mut session, gameobject);
    prepare_personal_money_fixture(&mut session, true);
    install_personal_money_cache(&mut session, owner, player);
    set_active_loot_guid_for_test(&mut session, owner);

    consume_personal_money_loot_for_test(&mut session, loot_money_packet()).await;

    assert!(rx.try_recv().is_err());
    assert!(authority.snapshot_for_player_like_cpp(player).is_none());
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(other)
            .unwrap()
            .loot
            .coins,
        456
    );
    assert_eq!(authority.generation_like_cpp(), generation);
    assert_eq!(
        personal_loot_marker_for_test(&session, owner, player).1,
        Some(123)
    );
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner).unwrap().coins,
        999
    );
    assert_eq!(player_gold_for_test(&session), 0);
}

#[tokio::test]
async fn fixture_money_rejects_stale_open_generation() {
    let (mut session, rx) = make_session_with_send_capacity(8);
    let player = ObjectGuid::create_player(1, 42);
    let owner = test_gameobject_guid(91_020);
    session.set_player_guid(Some(player));
    let gameobject =
        make_canonical_gameobject_for_loot_test(&session, owner, GAMEOBJECT_TYPE_CHEST as u8);
    let authority = gameobject.loot_authority_like_cpp().clone();
    let mut pool = authoritative_test_loot_like_cpp(123, false);
    pool.loot_guid = represented_loot_object_guid_like_cpp(owner);
    pool.allowed_looters = vec![player];
    let opened_generation = authority.replace_like_cpp(Some(pool), HashMap::new());
    attach_canonical_gameobject_for_loot_test(&mut session, gameobject);
    prepare_personal_money_fixture(&mut session, true);
    insert_client_visible_guid_for_test(&mut session, owner);
    let source = GameObjectLootSource {
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        ..Default::default()
    };
    open_personal_money_loot_for_test(&mut session, owner, source).await;
    assert_eq!(
        active_money_generation_for_test(&session, owner),
        Some(opened_generation)
    );
    let _ = drain_server_opcodes_like_cpp(&rx);
    let mut replacement = authoritative_test_loot_like_cpp(456, false);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![player];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    assert_ne!(replacement_generation, opened_generation);

    consume_personal_money_loot_for_test(&mut session, loot_money_packet()).await;

    assert!(rx.try_recv().is_err());
    assert_eq!(player_gold_for_test(&session), 0);
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner).unwrap().coins,
        123
    );
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(player)
            .unwrap()
            .loot
            .coins,
        456
    );
    assert_eq!(authority.generation_like_cpp(), replacement_generation);
    assert_eq!(
        active_money_generation_for_test(&session, owner),
        Some(opened_generation)
    );
}
