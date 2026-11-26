// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Production-linked receipt recovery for an indeterminate group-loot payout.

use std::{
    collections::{HashMap, HashSet},
    future::{Future, poll_fn},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::Poll,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use tokio::{sync::oneshot, time::timeout};
use wow_core::ObjectGuid;
use wow_database::group_loot_money_adapter::MariaDbGroupLootMoneyPersistenceAdapterLikeCpp;
use wow_loot::{
    CreatureLoot, LootClaimError, LootClaimLease, LootClaimPayload, OwnedLootAuthority,
    OwnedLootAuthorityLifecycle,
};
use wow_world_core::{
    loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp,
    session::mailbox::{ApplyLootMoneyLikeCppCommand, KickLikeCppCommand, SessionCommand},
};
use wow_world_lifecycle::{
    LootMoneyPersistenceErrorLikeCpp, SessionLifecycleState,
    loot_delivery_contracts::{LootMoneyDeliveryAddressLikeCpp, LootMoneyViewerFanoutLikeCpp},
    loot_money_persistence::spawn_group_loot_money_persistence_like_cpp,
};

#[path = "loot_money_quarantine/database.rs"]
mod database;

use database::{BEFORE, COINS, Database, LostAcknowledgement, RECIPIENT};

const BOUND: Duration = Duration::from_secs(15);

fn money_source(loot_obj: ObjectGuid, allowed_looters: Vec<ObjectGuid>) -> CreatureLoot {
    CreatureLoot {
        loot_guid: loot_obj,
        coins: COINS as u32,
        unlooted_count: 0,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters,
        items: Vec::new(),
        looted_by_player: false,
    }
}

fn apply_money_command(
    recipient: ObjectGuid,
    loot_owner: ObjectGuid,
    loot_obj: ObjectGuid,
    authority: &OwnedLootAuthority,
    generation: u64,
    tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    authority_committed: Arc<AtomicBool>,
) -> ApplyLootMoneyLikeCppCommand {
    ApplyLootMoneyLikeCppCommand {
        recipient,
        loot_owner,
        loot_obj,
        amount: COINS,
        money_mod: 0,
        durable_applied_amount: Arc::new(AtomicU64::new(0)),
        durable_persistence_tracker: tracker,
        sole_looter: true,
        authority: authority.clone(),
        authority_generation: generation,
        authority_committed,
        send_coin_removed: Arc::new(AtomicBool::new(false)),
        applied: Arc::new(AtomicBool::new(false)),
        published: Arc::new(AtomicBool::new(false)),
    }
}

fn viewer_fanout(
    player: ObjectGuid,
    source_tx: flume::Sender<SessionCommand>,
    loot_owner: ObjectGuid,
    loot_obj: ObjectGuid,
    authority: &OwnedLootAuthority,
    generation: u64,
) -> LootMoneyViewerFanoutLikeCpp {
    LootMoneyViewerFanoutLikeCpp {
        scope_player: player,
        source_player: player,
        source_command_tx: source_tx,
        player_registry: None,
        map_id: 1,
        instance_id: 0,
        loot_owner,
        loot_obj,
        authority: authority.clone(),
        authority_generation: generation,
        payout_recipients: HashSet::from([player]),
    }
}

fn spawn_waiter(
    authority: OwnedLootAuthority,
    player: ObjectGuid,
    generation: u64,
) -> (
    tokio::task::JoinHandle<Result<LootClaimLease, LootClaimError>>,
    oneshot::Receiver<()>,
) {
    let (started_tx, started_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let reservation = authority.reserve_money_for_generation_like_cpp(player, generation);
        tokio::pin!(reservation);
        let mut started_tx = Some(started_tx);
        poll_fn(|context| match reservation.as_mut().poll(context) {
            Poll::Pending => {
                if let Some(started_tx) = started_tx.take() {
                    let _ = started_tx.send(());
                }
                Poll::Pending
            }
            Poll::Ready(result) => Poll::Ready(result),
        })
        .await
    });
    (task, started_rx)
}

async fn run_case(rollback: bool) -> Result<()> {
    let database = Database::create(rollback).await?;
    let scenario = exercise_case(&database, rollback).await;
    // Run cleanup even when an assertion represented by `ensure!` fails.
    let cleanup = database.finish().await;
    scenario.and(cleanup)
}

async fn exercise_case(database: &Database, rollback: bool) -> Result<()> {
    let recipient = ObjectGuid::create_player(1, RECIPIENT as i64);
    let waiter_a = ObjectGuid::create_player(1, 55);
    let waiter_b = ObjectGuid::create_player(1, 56);
    let loot_owner = ObjectGuid::create_uniq(501);
    let loot_obj = ObjectGuid::create_uniq(502);
    let authority = OwnedLootAuthority::new();
    let generation = authority.replace_like_cpp(
        Some(money_source(loot_obj, vec![recipient, waiter_a, waiter_b])),
        HashMap::new(),
    );
    ensure!(generation != 0, "owned money source was not installed");
    authority
        .add_viewer_like_cpp(recipient)
        .map_err(|error| anyhow::anyhow!("failed to open the owned source: {error:?}"))?;

    let claim = authority
        .reserve_money_for_generation_like_cpp(recipient, generation)
        .await
        .map_err(|error| anyhow::anyhow!("failed to reserve owned money: {error:?}"))?;
    ensure!(
        matches!(claim.payload_like_cpp(), LootClaimPayload::Money(amount) if u64::from(*amount) == COINS),
        "the owned claim did not reserve the source's full money pool"
    );

    let lost_writer = Arc::new(LostAcknowledgement::new(Arc::clone(&database.db), rollback));
    let mut lifecycle = SessionLifecycleState::new_like_cpp();
    lifecycle.set_group_loot_money_persistence_port_like_cpp(lost_writer.clone());
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let authority_committed = Arc::new(AtomicBool::new(false));
    let (command_tx, command_rx) = flume::unbounded();
    let delivery = apply_money_command(
        recipient,
        loot_owner,
        loot_obj,
        &authority,
        generation,
        Arc::clone(&tracker),
        Arc::clone(&authority_committed),
    );
    let attempt_started = lost_writer.started.notified();
    tokio::pin!(attempt_started);
    let worker = spawn_group_loot_money_persistence_like_cpp(
        &lifecycle,
        true,
        vec![(recipient, COINS)],
        claim,
        vec![(
            LootMoneyDeliveryAddressLikeCpp::Source(command_tx.clone()),
            SessionCommand::ApplyLootMoneyLikeCpp(delivery),
        )],
        Arc::clone(&authority_committed),
        viewer_fanout(
            recipient,
            command_tx.clone(),
            loot_owner,
            loot_obj,
            &authority,
            generation,
        ),
    )
    .context("failed to start the production shared-loot persistence worker")?;

    timeout(BOUND, attempt_started.as_mut())
        .await
        .context("the real database attempt did not reach its pause point")?;

    // Both claimants must have polled through the occupied source reservation
    // and become pending before the writer loses its acknowledgement.
    let (waiter_a_task, waiter_a_started) = spawn_waiter(authority.clone(), waiter_a, generation);
    let (waiter_b_task, waiter_b_started) = spawn_waiter(authority.clone(), waiter_b, generation);
    timeout(BOUND, async {
        waiter_a_started.await?;
        waiter_b_started.await?;
        Ok::<_, oneshot::error::RecvError>(())
    })
    .await
    .context("both source waiters did not reach the pending reservation")?
    .context("a source waiter exited before it became pending")?;

    assert_eq!(lost_writer.attempts.load(Ordering::SeqCst), 1);
    lost_writer.resume.notify_one();
    let worker_result = timeout(BOUND, worker)
        .await
        .context("the production persistence worker did not complete")?
        .context("the production persistence worker panicked")?;
    ensure!(
        matches!(
            &worker_result,
            Err(LootMoneyPersistenceErrorLikeCpp::CommitOutcomeUnknownPersistence(_))
        ),
        "the dropped acknowledgement must quarantine instead of publishing: {worker_result:?}"
    );
    ensure!(
        authority.lifecycle_like_cpp() == OwnedLootAuthorityLifecycle::Quarantined,
        "the exact owned source was not quarantined"
    );
    ensure!(
        !authority_committed.load(Ordering::Acquire),
        "uncertain SQL outcome must not commit the source authority"
    );
    assert_eq!(lost_writer.indeterminate_reads.load(Ordering::SeqCst), 1);
    assert_eq!(lost_writer.settled_reads.load(Ordering::SeqCst), 0);
    assert_eq!(lost_writer.attempts.load(Ordering::SeqCst), 1);

    let first_command = command_rx
        .try_recv()
        .context("unknown COMMIT must deliver a kick to the affected recipient")?;
    match first_command {
        SessionCommand::KickLikeCpp(KickLikeCppCommand { reason }) => ensure!(
            reason.contains("outcome is unknown"),
            "unexpected quarantine kick reason: {reason}"
        ),
        other => {
            anyhow::bail!("unknown COMMIT published a paid command instead of a kick: {other:?}")
        }
    }
    ensure!(
        matches!(command_rx.try_recv(), Err(flume::TryRecvError::Empty)),
        "unknown COMMIT must not queue a money application or viewer removal"
    );

    let waiter_a_result = timeout(BOUND, waiter_a_task)
        .await
        .context("first source waiter stayed blocked after quarantine")?
        .context("first source waiter task panicked")?;
    let waiter_b_result = timeout(BOUND, waiter_b_task)
        .await
        .context("second source waiter stayed blocked after quarantine")?
        .context("second source waiter task panicked")?;
    ensure!(
        matches!(waiter_a_result, Err(LootClaimError::Retired)),
        "first pre-quarantine waiter was not rejected once with Retired"
    );
    ensure!(
        matches!(waiter_b_result, Err(LootClaimError::Retired)),
        "second pre-quarantine waiter was not rejected once with Retired"
    );

    let while_quarantined = timeout(
        BOUND,
        authority.reserve_money_for_generation_like_cpp(waiter_a, generation),
    )
    .await
    .context("new claimant blocked instead of failing closed during quarantine")?;
    ensure!(
        matches!(while_quarantined, Err(LootClaimError::Retired)),
        "new source grant was admitted while the receipt remained indeterminate"
    );

    let receipt_settled = lost_writer.settled.notified();
    tokio::pin!(receipt_settled);
    database.restore_receipt(rollback).await?;
    timeout(BOUND, receipt_settled.as_mut())
        .await
        .context("late database receipt read did not settle the quarantined source")?;
    assert_eq!(lost_writer.settled_reads.load(Ordering::SeqCst), 1);
    assert_eq!(lost_writer.attempts.load(Ordering::SeqCst), 1);
    ensure!(
        matches!(command_rx.try_recv(), Err(flume::TryRecvError::Empty)),
        "late receipt settlement must not replay paid session commands"
    );

    let (money, receipts) = database.counts().await?;
    if !rollback {
        assert_eq!((money, receipts), (BEFORE + COINS, 1));
        let snapshot = authority
            .shared_snapshot_like_cpp()
            .context("committed authority snapshot disappeared")?;
        assert_eq!(u64::from(snapshot.loot.coins), 0);
        return Ok(());
    }

    assert_eq!((money, receipts), (BEFORE, 0));
    ensure!(
        authority.lifecycle_like_cpp() == OwnedLootAuthorityLifecycle::Active,
        "rolled-back receipt did not reopen the exact source"
    );
    let snapshot = authority
        .shared_snapshot_like_cpp()
        .context("rolled-back authority snapshot disappeared")?;
    assert_eq!(u64::from(snapshot.loot.coins), COINS);

    // The unknown first attempt quarantined its old session tracker. A legal
    // retry uses a fresh session lifetime, as it would after the required kick.
    let retry_writer = Arc::new(MariaDbGroupLootMoneyPersistenceAdapterLikeCpp::new(
        Arc::clone(&database.db),
    ));
    let mut retry_lifecycle = SessionLifecycleState::new_like_cpp();
    retry_lifecycle.set_group_loot_money_persistence_port_like_cpp(retry_writer.clone());
    let retry_tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let retry_claim = authority
        .reserve_money_for_generation_like_cpp(recipient, generation)
        .await
        .map_err(|error| {
            anyhow::anyhow!("fresh legal claim failed after receipt rollback: {error:?}")
        })?;
    ensure!(
        matches!(retry_claim.payload_like_cpp(), LootClaimPayload::Money(amount) if u64::from(*amount) == COINS),
        "fresh legal retry did not claim the restored source amount"
    );

    let retry_authority_committed = Arc::new(AtomicBool::new(false));
    let (retry_tx, retry_rx) = flume::unbounded();
    let retry_delivery = apply_money_command(
        recipient,
        loot_owner,
        loot_obj,
        &authority,
        generation,
        Arc::clone(&retry_tracker),
        Arc::clone(&retry_authority_committed),
    );
    let retry_worker = spawn_group_loot_money_persistence_like_cpp(
        &retry_lifecycle,
        true,
        vec![(recipient, COINS)],
        retry_claim,
        vec![(
            LootMoneyDeliveryAddressLikeCpp::Source(retry_tx.clone()),
            SessionCommand::ApplyLootMoneyLikeCpp(retry_delivery),
        )],
        Arc::clone(&retry_authority_committed),
        viewer_fanout(
            recipient, retry_tx, loot_owner, loot_obj, &authority, generation,
        ),
    )
    .context("failed to start fresh post-rollback money retry")?;
    timeout(BOUND, retry_worker)
        .await
        .context("fresh legal retry did not complete")?
        .context("fresh legal retry worker panicked")?
        .context("fresh legal retry did not commit")?;

    let retry_command = retry_rx
        .try_recv()
        .context("fresh legal retry did not publish its committed money command")?;
    match retry_command {
        SessionCommand::ApplyLootMoneyLikeCpp(command) => {
            assert_eq!(command.recipient, recipient);
            assert_eq!(command.amount, COINS);
            assert_eq!(
                command.durable_applied_amount.load(Ordering::Acquire),
                COINS
            );
            ensure!(
                command.authority.shares_storage_like_cpp(&authority),
                "retry command addressed a different loot authority"
            );
            assert_eq!(command.authority_generation, generation);
            ensure!(command.authority_committed.load(Ordering::Acquire));
            ensure!(command.send_coin_removed.load(Ordering::Acquire));
        }
        other => anyhow::bail!("fresh legal retry published the wrong command: {other:?}"),
    }
    ensure!(
        retry_rx.try_recv().is_err(),
        "fresh legal retry published duplicate payout or removal commands"
    );
    assert_eq!(database.counts().await?, (BEFORE + COINS, 1));
    assert_eq!(
        u64::from(
            authority
                .shared_snapshot_like_cpp()
                .context("retried authority snapshot disappeared")?
                .loot
                .coins
        ),
        0
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires an isolated MariaDB Unix socket in RUSTYCORE_LOOT_TEST_SOCKET"]
async fn uncertain_group_money_receipt_quarantines_waiters_and_recovers_once() -> Result<()> {
    run_case(false).await?;
    run_case(true).await?;
    Ok(())
}
