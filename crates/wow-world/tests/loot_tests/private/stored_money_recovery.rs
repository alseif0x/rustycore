//! Durable claim recovery using the real worker and completion tracker.

use super::recovery_support::*;

#[tokio::test]
async fn cancelled_stored_item_money_before_commit_retries_without_local_consumption_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_845);
    let owner_guid = ObjectGuid::create_item(1, 61_846);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    set_player_gold_for_test(&mut session, 100);

    let durable_source_row = Arc::new(AtomicBool::new(true));
    let first_source = Arc::clone(&durable_source_row);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (_commit_tx, commit_rx) = tokio::sync::oneshot::channel::<()>();
    let first_balance_applied = Arc::new(AtomicBool::new(false));
    let first_runtime_applied = Arc::new(AtomicBool::new(false));
    let first_guard = begin_loot_persistence_for_test(&session);
    let first_worker = spawn_loot_claim_worker_for_test(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            first_source
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| ())
                .map_err(|_| ())
        },
        None,
        Some((
            first_guard,
            LootCompletion::new(
                owner_guid,
                0,
                player_guid,
                false,
                Some(7),
                Some(7),
                Some(Arc::clone(&first_balance_applied)),
                None,
                Arc::clone(&first_runtime_applied),
            ),
        )),
    )
    .unwrap();
    started_rx.await.unwrap();
    first_worker.abort();
    assert!(first_worker.await.unwrap_err().is_cancelled());

    wait_for_loot_persistence_for_test(&mut session).await;
    assert!(durable_source_row.load(Ordering::Acquire));
    assert_eq!(player_gold_for_test(&session), 100);
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner_guid)
            .unwrap()
            .coins,
        7
    );
    assert!(!first_runtime_applied.load(Ordering::Acquire));

    let retry_source = Arc::clone(&durable_source_row);
    let retry_balance_applied = Arc::new(AtomicBool::new(false));
    let retry_runtime_applied = Arc::new(AtomicBool::new(false));
    let retry_guard = begin_loot_persistence_for_test(&session);
    spawn_loot_claim_worker_for_test(
        async move {
            retry_source
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| ())
                .map_err(|_| ())
        },
        None,
        Some((
            retry_guard,
            LootCompletion::new(
                owner_guid,
                0,
                player_guid,
                false,
                Some(7),
                Some(7),
                Some(Arc::clone(&retry_balance_applied)),
                None,
                Arc::clone(&retry_runtime_applied),
            ),
        )),
    )
    .unwrap()
    .await
    .unwrap()
    .unwrap();

    wait_for_loot_persistence_for_test(&mut session).await;
    assert!(!durable_source_row.load(Ordering::Acquire));
    assert_eq!(player_gold_for_test(&session), 107);
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner_guid)
            .unwrap()
            .coins,
        0
    );
    assert!(retry_runtime_applied.load(Ordering::Acquire));
    assert!(is_active_loot_guid_for_test(&session, owner_guid));
}

#[tokio::test]
async fn cancelled_stored_item_money_after_commit_is_replayed_before_disconnect_save_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_847);
    let owner_guid = ObjectGuid::create_item(1, 61_848);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    set_player_gold_for_test(&mut session, 100);

    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let balance_applied = Arc::new(AtomicBool::new(false));
    let runtime_money_applied = Arc::new(AtomicBool::new(false));
    let guard = begin_loot_persistence_for_test(&session);
    let worker = spawn_loot_claim_worker_for_test(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        None,
        Some((
            guard,
            LootCompletion::new(
                owner_guid,
                0,
                player_guid,
                false,
                Some(7),
                Some(7),
                Some(Arc::clone(&balance_applied)),
                None,
                Arc::clone(&runtime_money_applied),
            ),
        )),
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });
    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;

    let mut save = Box::pin(disconnect_loot_save_for_test(&mut session));
    assert!(
        tokio::time::timeout(Duration::from_millis(5), &mut save)
            .await
            .is_err(),
        "disconnect save must remain pending until durable loot publication is idle"
    );
    commit_tx.send(()).unwrap();
    save.await;

    assert_eq!(player_gold_for_test(&session), 107);
    assert!(runtime_money_applied.load(Ordering::Acquire));
}

#[tokio::test]
async fn stored_item_money_save_reconciled_balance_still_publishes_source_once_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_848);
    let owner_guid = ObjectGuid::create_item(1, 61_849);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    set_player_gold_for_test(&mut session, 107);
    let balance_applied = Arc::new(AtomicBool::new(true));
    let publication_applied = Arc::new(AtomicBool::new(false));
    let mut guard = begin_loot_persistence_for_test(&session);
    guard.mark_committed(LootCompletion::new(
        owner_guid,
        0,
        player_guid,
        false,
        Some(7),
        Some(7),
        Some(Arc::clone(&balance_applied)),
        None,
        Arc::clone(&publication_applied),
    ));
    drop(guard);

    apply_loot_completions_for_test(&mut session).await;

    assert_eq!(player_gold_for_test(&session), 107);
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner_guid)
            .unwrap()
            .coins,
        0
    );
    assert!(balance_applied.load(Ordering::Acquire));
    assert!(publication_applied.load(Ordering::Acquire));
    assert!(
        drain_server_opcodes_like_cpp(&send_rx)
            .contains(&(wow_constants::ServerOpcodes::LootMoneyNotify as u16))
    );
}

#[tokio::test]
async fn stored_item_money_delete_cas_allows_exactly_one_durable_grant_like_cpp() {
    assert_eq!(
        wow_persistence::STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP,
        1,
        "the source-row delete must fail the whole transaction after another winner"
    );
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_849);
    let owner_guid = ObjectGuid::create_item(1, 61_850);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 7);
    set_player_gold_for_test(&mut session, 100);

    let source_row = Arc::new(AtomicBool::new(true));
    let durable_grants = Arc::new(AtomicUsize::new(0));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let source_row = Arc::clone(&source_row);
        let durable_grants = Arc::clone(&durable_grants);
        let guard = begin_loot_persistence_for_test(&session);
        workers.push(
            spawn_loot_claim_worker_for_test(
                async move {
                    tokio::task::yield_now().await;
                    source_row
                        .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                        .map_err(|_| ())?;
                    durable_grants.fetch_add(1, Ordering::SeqCst);
                    Ok::<(), ()>(())
                },
                None,
                Some((
                    guard,
                    LootCompletion::new(
                        owner_guid,
                        0,
                        player_guid,
                        false,
                        Some(7),
                        Some(7),
                        Some(Arc::new(AtomicBool::new(false))),
                        None,
                        Arc::new(AtomicBool::new(false)),
                    ),
                )),
            )
            .unwrap(),
        );
    }

    let mut successes = 0;
    for worker in workers {
        if matches!(worker.await, Ok(Ok(()))) {
            successes += 1;
        }
    }
    wait_for_loot_persistence_for_test(&mut session).await;

    assert_eq!(successes, 1);
    assert_eq!(durable_grants.load(Ordering::SeqCst), 1);
    assert_eq!(player_gold_for_test(&session), 107);
    assert_eq!(
        loot_recovery_cache_for_test(&session, owner_guid)
            .unwrap()
            .coins,
        0
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| { *opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16 })
            .count(),
        1
    );
}

#[tokio::test]
async fn cancelled_zero_stored_item_money_notifies_once_and_normal_completion_does_not_replay_like_cpp()
 {
    for cancelled_waiter in [true, false] {
        let (mut session, send_rx) = make_session_with_send_capacity(16);
        let player_guid =
            ObjectGuid::create_player(1, if cancelled_waiter { 61_851 } else { 61_852 });
        let owner_guid = ObjectGuid::create_item(1, if cancelled_waiter { 61_853 } else { 61_854 });
        install_active_item_loot_completion_fixture_like_cpp(
            &mut session,
            player_guid,
            owner_guid,
            0,
        );

        if cancelled_waiter {
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
            let guard = begin_loot_persistence_for_test(&session);
            let worker = spawn_loot_claim_worker_for_test(
                async move {
                    let _ = started_tx.send(());
                    let _ = commit_rx.await;
                    Ok::<(), ()>(())
                },
                None,
                Some((
                    guard,
                    LootCompletion::new(
                        owner_guid,
                        0,
                        player_guid,
                        false,
                        Some(0),
                        Some(0),
                        Some(Arc::new(AtomicBool::new(false))),
                        None,
                        Arc::new(AtomicBool::new(false)),
                    ),
                )),
            )
            .unwrap();
            let waiter = tokio::spawn(async move { worker.await });
            started_rx.await.unwrap();
            waiter.abort();
            let _ = waiter.await;
            commit_tx.send(()).unwrap();
            wait_for_loot_persistence_for_test(&mut session).await;
        } else {
            set_loot_money_persistence_test_result_for_test(&mut session, true);
            handle_loot_money_for_test(&mut session, loot_money_packet()).await;
            wait_for_loot_persistence_for_test(&mut session).await;
        }

        let opcodes = drain_server_opcodes_like_cpp(&send_rx);
        assert_eq!(
            opcodes
                .iter()
                .filter(|opcode| {
                    **opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16
                })
                .count(),
            1,
            "zero money still notifies, while a normally published completion is not replayed"
        );
        wait_for_loot_persistence_for_test(&mut session).await;
        assert!(drain_server_opcodes_like_cpp(&send_rx).is_empty());
    }
}

#[tokio::test]
async fn disconnect_waits_for_item_publication_and_releases_once_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_850);
    let owner_guid = ObjectGuid::create_item(1, 61_851);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 0);
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let guard = begin_loot_persistence_for_test(&session);
    let worker = spawn_loot_claim_worker_for_test(
        async move {
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        None,
        Some((
            guard,
            LootCompletion::new(
                owner_guid,
                0,
                player_guid,
                true,
                None,
                None,
                None,
                None,
                Arc::new(AtomicBool::new(true)),
            ),
        )),
    )
    .unwrap();
    let release_commit = tokio::spawn(async move {
        tokio::task::yield_now().await;
        commit_tx.send(()).unwrap();
        worker.await.unwrap().unwrap();
    });

    disconnect_loot_cleanup_for_test(&mut session).await;
    release_commit.await.unwrap();

    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        1
    );
}
