//! Money application scenarios backed by the original persistence operations.

use super::money_support::*;

#[tokio::test]
async fn failed_authoritative_money_persistence_rolls_back_for_retry_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    set_loot_money_persistence_test_result_for_test(&mut first, false);

    handle_loot_money_for_test(&mut first, loot_money_packet()).await;
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();
    assert_eq!(player_gold_for_test(&first), 0);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        7
    );

    set_loot_money_persistence_test_result_for_test(&mut first, true);
    handle_loot_money_for_test(&mut first, loot_money_packet()).await;
    assert_eq!(player_gold_for_test(&first), 7);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}

#[test]
fn pickpocket_money_is_not_shared_with_the_group_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    let owner = test_creature_guid(19_501);
    session.set_player_guid(Some(player_guid));
    prepare_money_player_residence_for_test(&mut session);
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    install_group_loot_group(&mut session, player_guid, member_guid);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    registry.register_or_replace(
        member_guid,
        broadcast_info(member_guid, member_tx),
        Default::default(),
    );
    session.set_player_registry(registry);
    let mut loot = authoritative_test_loot_like_cpp(8, false);
    loot.loot_type = LOOT_TYPE_PICKPOCKETING_LIKE_CPP;
    loot.allowed_looters = vec![player_guid, member_guid];
    set_loot_for_test(&mut session, owner, loot);

    assert_eq!(
        money_recipients_for_test(&session, owner),
        vec![player_guid]
    );
}

#[tokio::test]
async fn remote_group_money_is_one_atomic_durable_fanout_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    install_group_loot_group(&mut first, first_guid, second_guid);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (first_presence_tx, _first_presence_rx) = flume::bounded(1);
    player_registry.register_or_replace(
        first_guid,
        broadcast_info(first_guid, first_presence_tx),
        Default::default(),
    );
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    handle_loot_money_for_test(&mut first, loot_money_packet()).await;
    process_pending_for_loot_test(&mut second).await;

    // C++ divides integral copper and discards the remainder.
    assert_eq!(player_gold_for_test(&first), 4);
    assert_eq!(player_gold_for_test(&second), 4);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
    for opcodes in [
        drain_server_opcodes_like_cpp(&first_rx),
        drain_server_opcodes_like_cpp(&second_rx),
    ] {
        let coin = opcodes
            .iter()
            .position(|opcode| *opcode == wow_constants::ServerOpcodes::CoinRemoved as u16)
            .expect("active viewer must receive CoinRemoved");
        let money = opcodes
            .iter()
            .position(|opcode| *opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16)
            .expect("payout recipient must receive LootMoneyNotify");
        assert!(coin < money, "C++ removes coins before notifying payout");
    }
}

#[tokio::test]
async fn stale_active_money_view_cannot_claim_replacement_generation_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();
    let opened_generation =
        active_money_generation_for_test(&first, owner).expect("opened loot generation");
    let mut replacement = authoritative_test_loot_like_cpp(11, false);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    assert_ne!(opened_generation, replacement_generation);

    handle_loot_money_for_test(&mut first, loot_money_packet()).await;

    assert_eq!(player_gold_for_test(&first), 0);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert_eq!(snapshot.loot.coins, 11);
}

#[tokio::test]
async fn two_sessions_claim_one_authoritative_money_pool_exactly_once_like_cpp() {
    let (first, _first_rx, second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let first_barrier = Arc::clone(&barrier);
    let first_task = tokio::spawn(async move {
        let mut first = first;
        first_barrier.wait().await;
        handle_loot_money_for_test(&mut first, loot_money_packet()).await;
        first
    });
    let second_barrier = Arc::clone(&barrier);
    let second_task = tokio::spawn(async move {
        let mut second = second;
        second_barrier.wait().await;
        handle_loot_money_for_test(&mut second, loot_money_packet()).await;
        second
    });
    barrier.wait().await;

    let mut first = first_task.await.unwrap();
    let second = second_task.await.unwrap();
    assert_eq!(
        player_gold_for_test(&first) + player_gold_for_test(&second),
        9
    );
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}

#[test]
fn vehicle_corpse_money_shares_and_pool_allowed_looters_control_membership_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    let owner = ObjectGuid::create_vehicle_like_cpp(1, 0, 1, 19_502);
    session.set_player_guid(Some(player_guid));
    prepare_money_player_residence_for_test(&mut session);
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    install_group_loot_group(&mut session, player_guid, member_guid);
    let registry = Arc::new(PlayerRegistry::default());
    let (player_tx, _player_rx) = flume::bounded(1);
    registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, player_tx),
        Default::default(),
    );
    let (member_tx, _member_rx) = flume::bounded(1);
    registry.register_or_replace(
        member_guid,
        broadcast_info(member_guid, member_tx),
        Default::default(),
    );
    session.set_player_registry(registry);

    let mut loot = authoritative_test_loot_like_cpp(8, false);
    loot.loot_type = LOOT_TYPE_CORPSE_LIKE_CPP;
    loot.allowed_looters = vec![player_guid];
    set_loot_for_test(&mut session, owner, loot);
    assert_eq!(
        money_recipients_for_test(&session, owner),
        vec![player_guid]
    );

    allow_money_looter_for_test(&mut session, owner, member_guid);
    assert_eq!(
        money_recipients_for_test(&session, owner),
        vec![player_guid, member_guid]
    );
}

#[test]
fn corpse_money_reward_distance_ignores_range_only_in_same_dungeon_instance_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    let owner = test_creature_guid(19_503);
    session.set_player_guid(Some(player_guid));
    set_player_position_for_loot_test(&mut session, Position::ZERO);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    let mut member = broadcast_info(member_guid, member_tx.clone());
    member.placement.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    registry.register_or_replace(member_guid, member, Default::default());
    session.set_player_registry(Arc::clone(&registry));
    let mut loot = authoritative_test_loot_like_cpp(8, false);
    loot.allowed_looters = vec![player_guid, member_guid];
    set_loot_for_test(&mut session, owner, loot);

    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(canonical);
    attach_money_player_controller_for_test(
        &mut session,
        player_guid,
        "LootOwner".to_string(),
        Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    );
    ensure_money_player_map_for_test(&mut session).expect("canonical loot owner map");
    install_group_loot_group(&mut session, player_guid, member_guid);
    assert_eq!(
        money_recipients_for_test(&session, owner),
        vec![player_guid]
    );

    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    assert_eq!(
        money_recipients_for_test(&session, owner),
        vec![player_guid, member_guid]
    );

    let mut wrong_instance = broadcast_info(member_guid, member_tx);
    wrong_instance.placement.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    wrong_instance.placement.instance_id = 1;
    registry.register_or_replace(member_guid, wrong_instance, Default::default());
    assert_eq!(
        money_recipients_for_test(&session, owner),
        vec![player_guid]
    );
}

#[tokio::test]
async fn failed_remote_group_money_transaction_credits_nobody_and_retries_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    install_group_loot_group(&mut first, first_guid, second_guid);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (first_presence_tx, _first_presence_rx) = flume::bounded(1);
    player_registry.register_or_replace(
        first_guid,
        broadcast_info(first_guid, first_presence_tx),
        Default::default(),
    );
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);
    set_loot_money_persistence_test_result_for_test(&mut first, false);
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();

    handle_loot_money_for_test(&mut first, loot_money_packet()).await;
    process_pending_for_loot_test(&mut second).await;
    assert_eq!(player_gold_for_test(&first), 0);
    assert_eq!(player_gold_for_test(&second), 0);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        9
    );

    set_loot_money_persistence_test_result_for_test(&mut first, true);
    handle_loot_money_for_test(&mut first, loot_money_packet()).await;
    process_pending_for_loot_test(&mut second).await;
    assert_eq!(player_gold_for_test(&first), 4);
    assert_eq!(player_gold_for_test(&second), 4);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}
