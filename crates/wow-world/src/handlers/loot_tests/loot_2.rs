//! Loot scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn corpse_money_reward_distance_ignores_range_only_in_same_dungeon_instance_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    let owner = test_creature_guid(19_503);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    let mut member = broadcast_info(member_guid, member_tx.clone());
    member.placement.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    registry.register_or_replace(member_guid, member, Default::default());
    session.set_player_registry(Arc::clone(&registry));
    let mut loot = authoritative_test_loot_like_cpp(8, false);
    loot.allowed_looters = vec![player_guid, member_guid];
    session.loot_table.insert(owner, loot);

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
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "LootOwner".to_string(),
        Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical loot owner map");
    install_group_loot_group(&mut session, player_guid, member_guid);
    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
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
        session.represented_loot_money_recipients_like_cpp(owner),
        vec![player_guid, member_guid]
    );

    let mut wrong_instance = broadcast_info(member_guid, member_tx);
    wrong_instance.placement.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    wrong_instance.placement.instance_id = 1;
    registry.register_or_replace(member_guid, wrong_instance, Default::default());
    assert_eq!(
        session.represented_loot_money_recipients_like_cpp(owner),
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
    first.set_loot_money_persistence_test_result_like_cpp(false);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();

    first.handle_loot_money(loot_money_packet()).await;
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(first.player_gold_like_cpp(), 0);
    assert_eq!(second.player_gold_like_cpp(), 0);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        9
    );

    first.set_loot_money_persistence_test_result_like_cpp(true);
    first.handle_loot_money(loot_money_packet()).await;
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(first.player_gold_like_cpp(), 4);
    assert_eq!(second.player_gold_like_cpp(), 4);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}
#[tokio::test]
async fn group_loot_money_worker_requires_and_uses_the_typed_persistence_port_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    install_group_loot_group(&mut first, first_guid, second_guid);
    let registry = Arc::new(PlayerRegistry::default());
    let (first_presence_tx, _first_presence_rx) = flume::bounded(1);
    registry.register_or_replace(
        first_guid,
        broadcast_info(first_guid, first_presence_tx),
        Default::default(),
    );
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(registry);
    first.clear_loot_money_persistence_test_result_like_cpp();
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();

    first.handle_loot_money(loot_money_packet()).await;
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(
        (first.player_gold_like_cpp(), second.player_gold_like_cpp()),
        (0, 0)
    );
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        9,
        "production-shaped group payout fails closed without its typed port"
    );

    let calls = Arc::new(AtomicUsize::new(0));
    first.set_group_loot_money_persistence_port_like_cpp(Arc::new(
        GroupLootMoneyPortFixtureLikeCpp {
            calls: Arc::clone(&calls),
        },
    ));
    first.handle_loot_money(loot_money_packet()).await;
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(
        (first.player_gold_like_cpp(), second.player_gold_like_cpp()),
        (4, 4)
    );
    assert_eq!(calls.load(Ordering::Acquire), 1);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
}
#[tokio::test]
async fn cancelled_money_waiter_cannot_reopen_a_durable_claim_like_cpp() {
    let (mut first, _first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    first.set_loot_money_persistence_test_result_like_cpp(true);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority.reserve_money_like_cpp(first_guid).await.unwrap();
    let authority_generation = claim.generation_like_cpp();
    let authority_committed = Arc::new(AtomicBool::new(false));
    let application = ApplyLootMoneyLikeCppCommand {
        recipient: second_guid,
        loot_owner: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        amount: 9,
        durable_applied_amount: Arc::new(AtomicU64::new(0)),
        durable_persistence_tracker: second.durable_loot_money_persistence_tracker_like_cpp(),
        sole_looter: true,
        authority: authority.clone(),
        authority_generation,
        authority_committed: Arc::clone(&authority_committed),
        send_coin_removed: Arc::new(AtomicBool::new(true)),
        applied: Arc::new(AtomicBool::new(false)),
        published: Arc::new(AtomicBool::new(false)),
    };
    let delivery = (
        LootMoneyDeliveryAddressLikeCpp::Source(second.session_command_tx()),
        SessionCommand::ApplyLootMoneyLikeCpp(application),
    );
    let viewer_fanout = LootMoneyViewerFanoutLikeCpp {
        scope_player: first_guid,
        source_player: first_guid,
        source_command_tx: first.session_command_tx(),
        player_registry: first.player_registry().cloned(),
        map_id: first.player_map_id_like_cpp(),
        instance_id: first
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0),
        loot_owner: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        authority: authority.clone(),
        authority_generation,
        payout_recipients: [second_guid].into_iter().collect(),
    };
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    let persistence = first
        .spawn_group_loot_money_persistence_like_cpp(
            vec![(second_guid, 9)],
            claim,
            vec![delivery],
            Arc::clone(&authority_committed),
            viewer_fanout,
        )
        .unwrap();

    // The outer packet task owns only the JoinHandle. Aborting it must not
    // cancel the detached SQL+authority worker that owns the lease.
    let waiter = tokio::spawn(async move { persistence.await });
    waiter.abort();
    let _ = waiter.await;
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .coins,
        0
    );
    let zero = authority
        .reserve_money_like_cpp(first_guid)
        .await
        .expect("C++ keeps the view and serializes a later zero-money observation");
    assert_eq!(zero.payload_like_cpp(), &LootClaimPayload::Money(0));
    assert!(zero.commit_like_cpp().unwrap());
    assert!(authority_committed.load(Ordering::Acquire));
    assert_eq!(second.player_gold_like_cpp(), 9);
    let opcodes = drain_server_opcodes_like_cpp(&second_rx);
    let coin = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::CoinRemoved as u16)
        .unwrap();
    let money = opcodes
        .iter()
        .position(|opcode| *opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16)
        .unwrap();
    assert!(coin < money);
}
#[tokio::test]
async fn money_viewer_opened_during_persistence_receives_coin_removed_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    first.set_loot_money_persistence_test_result_like_cpp(true);
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    assert!(authority.remove_viewer_like_cpp(second_guid));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));

    let claim = authority.reserve_money_like_cpp(first_guid).await.unwrap();
    let authority_generation = claim.generation_like_cpp();
    let authority_committed = Arc::new(AtomicBool::new(false));
    let application = ApplyLootMoneyLikeCppCommand {
        recipient: first_guid,
        loot_owner: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        amount: 9,
        durable_applied_amount: Arc::new(AtomicU64::new(0)),
        durable_persistence_tracker: first.durable_loot_money_persistence_tracker_like_cpp(),
        sole_looter: true,
        authority: authority.clone(),
        authority_generation,
        authority_committed: Arc::clone(&authority_committed),
        send_coin_removed: Arc::new(AtomicBool::new(false)),
        applied: Arc::new(AtomicBool::new(false)),
        published: Arc::new(AtomicBool::new(false)),
    };
    let viewer_fanout = LootMoneyViewerFanoutLikeCpp {
        scope_player: first_guid,
        source_player: first_guid,
        source_command_tx: first.session_command_tx(),
        player_registry: Some(registry),
        map_id: first.player_map_id_like_cpp(),
        instance_id: first
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0),
        loot_owner: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        authority: authority.clone(),
        authority_generation,
        payout_recipients: [first_guid].into_iter().collect(),
    };
    let persistence = first
        .spawn_group_loot_money_persistence_like_cpp(
            vec![(first_guid, 9)],
            claim,
            vec![(
                LootMoneyDeliveryAddressLikeCpp::Source(first.session_command_tx()),
                SessionCommand::ApplyLootMoneyLikeCpp(application),
            )],
            Arc::clone(&authority_committed),
            viewer_fanout,
        )
        .unwrap();

    authority
        .open_view_with_snapshot_like_cpp(second_guid, |_, _| ())
        .expect("late viewer opens before the detached worker commits");
    persistence.await.unwrap().unwrap();
    second.process_represented_session_commands_like_cpp().await;

    assert!(authority_committed.load(Ordering::Acquire));
    assert!(
        drain_server_opcodes_like_cpp(&second_rx)
            .contains(&(wow_constants::ServerOpcodes::CoinRemoved as u16)),
        "a viewer that saw non-zero money during SQL must receive C++ NotifyMoneyRemoved"
    );
}
#[tokio::test]
async fn remote_master_loot_command_transports_and_commits_claim_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };
    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);

    let request = first.request_represented_remote_master_loot_give_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        entry,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}
#[tokio::test]
async fn remote_roll_winner_command_transports_and_commits_claim_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };
    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);

    let request = first.request_represented_remote_loot_roll_winner_store_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        vec![entry],
        false,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}
#[tokio::test]
async fn remote_roll_timeout_then_release_fans_out_once_and_finalizes_corpse_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    first
        .mutate_world_creature(owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };

    // The target no longer has a live loot window. The source closes its
    // window only after the remote request times out with the target's
    // detached persistence worker still owning the claim.
    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let commit_gate = Arc::new(tokio::sync::Notify::new());
    second.set_loot_item_store_test_commit_gate_like_cpp(Arc::clone(&commit_gate));

    let mut request = Box::pin(
        first.request_represented_remote_loot_roll_winner_store_like_cpp(
            second_guid,
            owner,
            represented_loot_object_guid_like_cpp(owner),
            0,
            0,
            vec![entry],
            false,
            Some(claim),
        ),
    );
    let mut target = Box::pin(async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    });
    let result = tokio::select! {
        result = &mut request => result,
        _ = &mut target => panic!("target must remain behind the COMMIT gate"),
    };
    drop(request);
    assert_eq!(result, MasterLootGiveResult::TargetMismatch);

    first.handle_loot_release(loot_release_packet(owner)).await;
    commit_gate.notify_one();
    target.await;

    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken,
        "the roll claim is terminal after the durable commit"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&first_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRemoved as u16)
            .count(),
        1,
        "the pre-COMMIT route publishes the roll removal exactly once"
    );
    assert!(
        !first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "post-COMMIT completion must finish the corpse lifecycle without an active view"
    );
}
#[test]
fn represented_personal_loot_remote_context_uses_registry_fields_like_cpp() {
    let (session, _) = make_session_with_send_capacity(1);
    let remote_context = RepresentedLootPlayerContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 80,
        known_spells: Vec::new(),
        active_quest_statuses: HashMap::new(),
        active_quest_objective_counts: HashMap::new(),
        rewarded_quests: HashSet::new(),
        inventory_item_counts: HashMap::new(),
        is_current: false,
    };

    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(6, 469, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(15, 1, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(16, 1, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(20, 0, 0, 0),
            &remote_context,
        ),
        Some(true)
    );
    assert_eq!(
        session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
            &loot_condition(27, 70, 3, 0),
            &remote_context,
        ),
        Some(true)
    );
}
#[tokio::test]
async fn loot_response_success_keeps_cpp_failure_and_threshold_defaults() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(19_118);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(creature_guid, false));

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.loot_method = LOOT_METHOD_GROUP_LIKE_CPP;
    group.loot_threshold = 4;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session.loot_table.insert(
        creature_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(creature_guid),
            coins: 1,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: player_guid,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    install_cached_test_creature_loot_authority_like_cpp(&mut session, creature_guid, player_guid);

    let response = session
        .represented_loot_response_for_owner_like_cpp(creature_guid, player_guid, false)
        .await
        .unwrap();

    assert_eq!(response.loot_method, LOOT_METHOD_GROUP_LIKE_CPP);
    assert_eq!(
        response.failure_reason,
        LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP
    );
    assert_eq!(response.threshold, LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP);
}
#[tokio::test]
async fn loot_error_response_keeps_cpp_threshold_default_like_cpp() {
    let (session, send_rx) = make_session_with_send();
    let owner = test_creature_guid(19_119);
    let loot_obj = represented_loot_object_guid_like_cpp(owner);

    session.send_loot_error_like_cpp(loot_obj, owner, LOOT_ERROR_TOO_FAR_LIKE_CPP);

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_TOO_FAR_LIKE_CPP
    );
    assert_eq!(
        loot_response_threshold(&sent),
        LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP
    );
}
#[tokio::test]
async fn dungeon_trash_builds_one_personal_pool_for_selected_group_looter_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .set_map_store(Arc::new(wow_data::MapStore::from_entries([
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
    let groups = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(fixture.first_tapper);
    group.add_member(fixture.second_tapper);
    group.looter_guid = fixture.second_tapper;
    let group_guid = group.group_guid;
    groups.register_group_like_cpp(group_guid, group);
    fixture.session.group_guid = Some(group_guid);
    fixture
        .session
        .set_group_registry(Arc::clone(&groups), Arc::new(PendingInvites::default()));

    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;

    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 1);
    assert!(!personal.contains_key(&fixture.first_tapper));
    let selected = &personal[&fixture.second_tapper].loot;
    assert_eq!(selected.dungeon_encounter_id, 0);
    assert_eq!(selected.allowed_looters, vec![fixture.second_tapper]);
    assert!(
        selected
            .items
            .iter()
            .all(|entry| { entry.allowed_looters == vec![fixture.second_tapper] })
    );
    assert_eq!(
        groups.get(&group_guid).unwrap().looter_guid_like_cpp(),
        fixture.first_tapper,
        "non-empty dungeon trash advances the round-robin group looter"
    );
}
#[test]
fn personal_encounter_late_upsert_cannot_cross_clear_loot_like_cpp() {
    let mut session = make_session();
    let first_player = ObjectGuid::create_player(1, 342);
    let late_player = ObjectGuid::create_player(1, 377);
    let gameobject_guid = test_gameobject_guid(91_020);
    let mut gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    let mut first_pool = authoritative_test_loot_like_cpp(0, true);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(gameobject_guid);
    first_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    first_pool.allowed_looters = vec![first_player];
    first_pool.items[0].allowed_looters = vec![first_player];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(first_player, first_pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    let observation = session
        .represented_gameobject_loot_install_observation_like_cpp(gameobject_guid)
        .expect("late generation observes the active chest lifetime");

    let mut stale_late_pool = authoritative_test_loot_like_cpp(0, true);
    stale_late_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        gameobject_guid.realm_id(),
        gameobject_guid.map_id(),
        0,
        0,
        gameobject_guid.counter() + 1,
    );
    stale_late_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    stale_late_pool.allowed_looters = vec![late_player];
    stale_late_pool.items[0].allowed_looters = vec![late_player];
    session
        .represented_personal_loot_owners
        .insert(gameobject_guid);
    session
        .represented_personal_loot_money
        .insert((gameobject_guid, late_player), 0);

    session.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
        gameobject.clear_loot_like_cpp();
    });

    assert!(
        session
            .upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
                gameobject_guid,
                late_player,
                stale_late_pool,
                false,
                true,
                &observation,
            )
            .is_none()
    );
    assert!(authority.is_retired_like_cpp());
    assert!(authority.personal_snapshots_like_cpp().is_empty());
    assert!(
        !session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, late_player))
    );
    assert!(!session.loot_table.contains_key(&gameobject_guid));
}
