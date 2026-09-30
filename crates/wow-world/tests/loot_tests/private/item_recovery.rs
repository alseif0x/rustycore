//! Durable claim recovery using the real worker and completion tracker.

use super::recovery_support::*;

#[tokio::test]
async fn durable_item_completion_never_auto_releases_creature_or_gameobject_owner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_820);
    session.set_player_guid(Some(player_guid));
    for owner_guid in [
        test_creature_guid(61_821),
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 1, 61_822),
    ] {
        set_active_loot_guid_for_test(&mut session, owner_guid);
        set_loot_for_test(
            &mut session,
            owner_guid,
            CreatureLoot {
                loot_guid: owner_guid,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: vec![player_guid],
                items: vec![represented_loot_entry(0, 25, player_guid)],
                looted_by_player: false,
            },
        );
        let guard = begin_loot_persistence_for_test(&session);
        spawn_loot_claim_worker_for_test(
            async { Ok::<(), ()>(()) },
            None,
            Some((
                guard,
                LootCompletion::new(
                    owner_guid,
                    0,
                    player_guid,
                    false,
                    None,
                    None,
                    None,
                    None,
                    Arc::new(AtomicBool::new(true)),
                ),
            )),
        )
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    }

    wait_for_loot_persistence_for_test(&mut session).await;

    assert!(!session.is_disconnecting());
    assert!(loot_recovery_cache_values_for_test(&session).all(|loot| !loot.items[0].taken));
    assert!(
        !drain_server_opcodes_like_cpp(&send_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRelease as u16))
    );
}

#[tokio::test]
async fn item_grant_commit_unknown_quarantines_claim_and_kicks_even_when_queue_full() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let (command_tx, command_rx) = flume::bounded(1);
    command_tx
        .send(SessionCommand::KickLikeCpp(KickLikeCppCommand {
            reason: "preexisting".to_string(),
        }))
        .unwrap();

    let worker = spawn_loot_item_worker_for_test(
        async {
            PersistenceOutcomeLikeCpp::Unknown {
                reason: "lost COMMIT reply".to_string(),
            }
        },
        Some(claim),
        None,
        command_tx,
    )
    .unwrap();
    let result = worker.await.unwrap();

    assert!(matches!(
        result,
        Err(LootPersistenceError::Persistence(reason))
            if reason == "lost COMMIT reply"
    ));
    assert_eq!(
        authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Quarantined
    );
    assert!(
        authority
            .reserve_item_for_award_like_cpp(first_guid, 0)
            .await
            .is_err(),
        "unknown COMMIT must never reopen the object-owned item claim"
    );

    let _preexisting = command_rx.recv().unwrap();
    let queued = tokio::time::timeout(Duration::from_secs(1), command_rx.recv_async())
        .await
        .expect("full command queue fallback must eventually enqueue the kick")
        .unwrap();
    assert!(matches!(queued, SessionCommand::KickLikeCpp(_)));
}

#[tokio::test]
async fn cancelled_item_waiter_cannot_reopen_a_durable_claim_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = loot_recovery_authority_for_test(&mut first, owner).unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let worker = spawn_loot_claim_worker_for_test(
        async move {
            let _ = started_tx.send(());
            let _ = release_rx.await;
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

    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 0);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(first_guid, 0)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn durable_item_completion_auto_releases_only_after_items_and_coins_are_empty_like_cpp() {
    for (coins, should_release) in [(0, true), (7, false)] {
        let (mut session, send_rx) = make_session_with_send_capacity(16);
        let player_guid = ObjectGuid::create_player(1, 61_801 + i64::from(coins));
        let owner_guid = ObjectGuid::create_item(1, 61_811 + i64::from(coins));
        install_active_item_loot_completion_fixture_like_cpp(
            &mut session,
            player_guid,
            owner_guid,
            coins,
        );
        let runtime_inventory_applied = Arc::new(AtomicBool::new(true));
        let guard = begin_loot_persistence_for_test(&session);
        spawn_loot_claim_worker_for_test(
            async { Ok::<(), ()>(()) },
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
                    runtime_inventory_applied,
                ),
            )),
        )
        .unwrap()
        .await
        .unwrap()
        .unwrap();

        wait_for_loot_persistence_for_test(&mut session).await;

        assert!(!session.is_disconnecting());
        assert_eq!(has_loot_for_test(&session, owner_guid), !should_release);
        assert_eq!(
            is_active_loot_guid_for_test(&session, owner_guid),
            !should_release
        );
        if !should_release {
            let loot = loot_recovery_cache_for_test(&session, owner_guid).unwrap();
            assert!(loot.items[0].taken);
            assert_eq!(loot.coins, coins);
        }
        assert_eq!(
            drain_server_opcodes_like_cpp(&send_rx)
                .into_iter()
                .filter(|opcode| { *opcode == wow_constants::ServerOpcodes::LootRelease as u16 })
                .count(),
            usize::from(should_release),
            "C++ StoreLootItem checks Loot::isLooted(), including money"
        );
    }
}

#[tokio::test]
async fn cancelled_item_handler_after_commit_releases_and_forces_inventory_reload_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_830);
    let owner_guid = ObjectGuid::create_item(1, 61_831);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 0);
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
                true,
                None,
                None,
                None,
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

    assert!(session.is_disconnecting());
    assert!(!has_loot_for_test(&session, owner_guid));
    assert!(!is_active_loot_guid_for_test(&session, owner_guid));
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        1
    );
}

#[tokio::test]
async fn failed_item_persistence_publishes_no_removal_or_release_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_840);
    let owner_guid = ObjectGuid::create_item(1, 61_841);
    install_active_item_loot_completion_fixture_like_cpp(&mut session, player_guid, owner_guid, 0);
    let guard = begin_loot_persistence_for_test(&session);
    let result = spawn_loot_claim_worker_for_test(
        async { Err::<(), ()>(()) },
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
                Arc::new(AtomicBool::new(false)),
            ),
        )),
    )
    .unwrap()
    .await
    .unwrap();
    assert!(result.is_err());

    wait_for_loot_persistence_for_test(&mut session).await;

    assert!(!session.is_disconnecting());
    assert!(is_active_loot_guid_for_test(&session, owner_guid));
    assert!(
        !loot_recovery_cache_for_test(&session, owner_guid)
            .unwrap()
            .items[0]
            .taken
    );
    assert!(
        !drain_server_opcodes_like_cpp(&send_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRelease as u16))
    );
}
