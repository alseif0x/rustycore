//! Instance scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn cancelled_after_runtime_apply_retains_multiviewer_fanout_and_corpse_lifecycle_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
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

    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    let corpse_before = first
        .mutate_world_creature(owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.set_corpse_despawn_at(Some(Instant::now() + Duration::from_secs(120)));
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
            creature.corpse_despawn_at()
        })
        .unwrap();
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let context = LootItemClaimCommitContextLikeCpp {
        owner_guid: owner,
        loot_obj: represented_loot_object_guid_like_cpp(owner),
        loot_list_id: 0,
        player_guid: first_guid,
        free_for_all: false,
    };
    let fanout = first
        .prepare_durable_loot_item_fanout_like_cpp(&claim, context)
        .expect("pre-COMMIT fanout route");
    assert_eq!(fanout.precommit_snapshot.loot.players_looting.len(), 2);

    let runtime_inventory_applied = Arc::new(AtomicBool::new(true));
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let guard = first.begin_durable_item_loot_persistence_like_cpp();
    let worker = super::super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        Some(claim),
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid: owner,
                loot_list_id: 0,
                player_guid: first_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
                item_fanout: Some(fanout),
                runtime_inventory_applied: Arc::clone(&runtime_inventory_applied),
            },
        )),
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });
    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;

    first.handle_loot_release(loot_release_packet(owner)).await;
    second.handle_loot_release(loot_release_packet(owner)).await;
    commit_tx.send(()).unwrap();
    first.wait_for_active_loot_persistence_like_cpp().await;

    assert!(!first.is_disconnecting());
    assert!(runtime_inventory_applied.load(Ordering::Acquire));
    assert!(
        drain_server_opcodes_like_cpp(&first_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRemoved as u16))
    );
    assert!(
        drain_server_opcodes_like_cpp(&second_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRemoved as u16))
    );
    let corpse_after = first
        .mutate_world_creature(owner, |creature| {
            (
                creature.corpse_despawn_at(),
                creature.has_lootable_dynamic_flag_like_cpp(),
            )
        })
        .unwrap();
    assert!(!corpse_after.1);
    assert!(corpse_after.0 <= corpse_before);
}
#[tokio::test]
async fn cancelled_disenchant_waiter_cannot_reopen_durable_batch_like_cpp() {
    let (mut session, _rx, _second, _second_rx, owner, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(player_guid, generation, 0, false, Some(player_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(player_guid, 0)
        .await
        .unwrap();
    let durable_materials = Arc::new(AtomicUsize::new(0));
    let durable_materials_worker = Arc::clone(&durable_materials);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let worker = super::super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = release_rx.await;
            durable_materials_worker.fetch_add(2, Ordering::SeqCst);
            Ok::<(), ()>(())
        },
        Some(claim),
        None,
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });

    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;
    release_tx.send(()).unwrap();
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }

    assert_eq!(durable_materials.load(Ordering::SeqCst), 2);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(player_guid, 0)
            .await
            .is_err()
    );
}
