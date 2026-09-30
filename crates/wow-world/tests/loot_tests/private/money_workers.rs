//! Money application scenarios backed by the original persistence operations.

use super::money_support::*;

use wow_persistence::{
    GroupLootMoneyPersistenceAttemptLikeCpp, GroupLootMoneyPersistenceOutcomeLikeCpp,
    GroupLootMoneyPersistencePortLikeCpp, GroupLootMoneyPersistenceRequestLikeCpp,
    GroupLootMoneyReconciliationLikeCpp, PersistenceFutureLikeCpp,
};

struct GroupLootMoneyPortFixtureLikeCpp {
    calls: Arc<AtomicUsize>,
}

impl GroupLootMoneyPersistencePortLikeCpp for GroupLootMoneyPortFixtureLikeCpp {
    fn attempt_group_loot_money_like_cpp(
        &self,
        request: GroupLootMoneyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyPersistenceAttemptLikeCpp> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        Box::pin(std::future::ready(
            GroupLootMoneyPersistenceAttemptLikeCpp::Applied(
                request
                    .payouts
                    .into_iter()
                    .map(|payout| GroupLootMoneyPersistenceOutcomeLikeCpp {
                        recipient_guid: payout.recipient_guid,
                        before: 0,
                        after: payout.requested_delta,
                        applied_delta: payout.requested_delta,
                    })
                    .collect(),
            ),
        ))
    }

    fn reconcile_group_loot_money_like_cpp(
        &self,
        _outcomes: Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyReconciliationLikeCpp> {
        Box::pin(std::future::ready(
            GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop,
        ))
    }
}

#[tokio::test]
async fn cancelled_money_waiter_cannot_reopen_a_durable_claim_like_cpp() {
    let (mut first, _first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    set_loot_money_persistence_test_result_for_test(&mut first, true);
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();
    let claim = authority.reserve_money_like_cpp(first_guid).await.unwrap();
    let authority_generation = claim.generation_like_cpp();
    let authority_committed = Arc::new(AtomicBool::new(false));
    let application = LootMoneyApplication::new(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        9,
        Arc::new(AtomicU64::new(0)),
        money_tracker_for_test(&second),
        true,
        authority.clone(),
        authority_generation,
        Arc::clone(&authority_committed),
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let delivery = source_money_delivery_for_test(second.session_command_tx(), application);
    let viewer_fanout = LootMoneyFanout::new(
        first_guid,
        first_guid,
        first.session_command_tx(),
        first.player_registry().cloned(),
        money_map_for_test(&first),
        money_instance_for_test(&first).unwrap_or(0),
        owner,
        represented_loot_object_guid_like_cpp(owner),
        authority.clone(),
        authority_generation,
        [second_guid].into_iter().collect(),
    );
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    let persistence = spawn_group_money_worker_for_test(
        &first,
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
    process_pending_for_loot_test(&mut second).await;
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
    assert_eq!(player_gold_for_test(&second), 9);
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
    clear_money_persistence_outcome_for_test(&mut first);
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();

    handle_loot_money_for_test(&mut first, loot_money_packet()).await;
    process_pending_for_loot_test(&mut second).await;
    assert_eq!(
        (player_gold_for_test(&first), player_gold_for_test(&second)),
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
    set_group_money_port_for_test(
        &mut first,
        Arc::new(GroupLootMoneyPortFixtureLikeCpp {
            calls: Arc::clone(&calls),
        }),
    );
    handle_loot_money_for_test(&mut first, loot_money_packet()).await;
    process_pending_for_loot_test(&mut second).await;
    assert_eq!(
        (player_gold_for_test(&first), player_gold_for_test(&second)),
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
async fn money_viewer_opened_during_persistence_receives_coin_removed_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, false,
        ));
    set_loot_money_persistence_test_result_for_test(&mut first, true);
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();
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
    let application = LootMoneyApplication::new(
        first_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        9,
        Arc::new(AtomicU64::new(0)),
        money_tracker_for_test(&first),
        true,
        authority.clone(),
        authority_generation,
        Arc::clone(&authority_committed),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let viewer_fanout = LootMoneyFanout::new(
        first_guid,
        first_guid,
        first.session_command_tx(),
        Some(registry),
        money_map_for_test(&first),
        money_instance_for_test(&first).unwrap_or(0),
        owner,
        represented_loot_object_guid_like_cpp(owner),
        authority.clone(),
        authority_generation,
        [first_guid].into_iter().collect(),
    );
    let persistence = spawn_group_money_worker_for_test(
        &first,
        vec![(first_guid, 9)],
        claim,
        vec![source_money_delivery_for_test(
            first.session_command_tx(),
            application,
        )],
        Arc::clone(&authority_committed),
        viewer_fanout,
    )
    .unwrap();

    authority
        .open_view_with_snapshot_like_cpp(second_guid, |_, _| ())
        .expect("late viewer opens before the detached worker commits");
    persistence.await.unwrap().unwrap();
    process_pending_for_loot_test(&mut second).await;

    assert!(authority_committed.load(Ordering::Acquire));
    assert!(
        drain_server_opcodes_like_cpp(&second_rx)
            .contains(&(wow_constants::ServerOpcodes::CoinRemoved as u16)),
        "a viewer that saw non-zero money during SQL must receive C++ NotifyMoneyRemoved"
    );
}
