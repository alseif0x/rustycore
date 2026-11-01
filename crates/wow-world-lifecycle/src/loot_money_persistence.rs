// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Detached durable loot-money persistence workers (#1263 F4).
//!
//! The complete detached workers of the shared-pool payout and of the
//! stored-Item money transaction live here, next to the loot-money delivery
//! contracts and the durable money tracker they already own. Their bodies moved
//! verbatim out of `crates/wow-world/src/session/money/operations.rs` and
//! `crates/wow-world/src/handlers/loot/money.rs`; only the access path changed
//! (the lifecycle owner is a parameter instead of a World session field).
//!
//! Boundary (unchanged): the shared-pool worker is an in-process guarantee, not
//! a durable claim journal. Aborting the detached task at the database commit
//! await boundary can still lose the continuation between durable SQL and the
//! synchronous authority commit.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::warn;
use wow_core::ObjectGuid;
use wow_entities::MAX_MONEY_AMOUNT;
use wow_loot::LootClaimLease;
use wow_world_core::loot_persistence::{
    DurableLootMoneyCompletionLikeCpp, DurableLootMoneyPersistenceGuardLikeCpp,
    DurableLootMoneyPersistenceTrackerLikeCpp,
};
use wow_world_core::session::mailbox::{
    ApplyLootMoneyLikeCppCommand, KickLikeCppCommand, NotifyLootMoneyRemovedLikeCppCommand,
    SessionCommand,
};

use crate::loot_delivery_contracts::{
    LootMoneyDeliveryAddressLikeCpp, LootMoneyViewerFanoutLikeCpp,
    loot_money_durable_outcome_like_cpp,
};
use crate::{
    DurableItemLootCompletionLikeCpp, LootMoneyPersistenceErrorLikeCpp, SessionLifecycleState,
};

/// Detached-worker handle of the shared-pool payout. The alias exists so the
/// application owner can name the handle without a Tokio dependency of its own;
/// the worker itself and its spawn live here.
pub type LootMoneyPersistenceWorkerHandleLikeCpp =
    tokio::task::JoinHandle<Result<(), LootMoneyPersistenceErrorLikeCpp>>;

/// Start the complete durable half of one shared money claim in a detached
/// task.  The task owns the lease across `COMMIT`, commits the authority in
/// the same task immediately after SQL success, then schedules the
/// already-durable session-local applications.
///
/// Dropping or aborting the packet-handler future only drops its
/// `JoinHandle`; Tokio keeps this worker alive.  This closes the duplicate
/// window where SQL could commit after the handler was cancelled while the
/// lease's `Drop` reopened the object pool.
///
/// Boundary: this is an in-process guarantee, not a durable claim journal.
/// Aborting this detached task (including runtime/process shutdown) at the
/// database commit await boundary can still lose the continuation between
/// durable SQL and the synchronous authority commit. Recovery across that
/// boundary requires persisting claim identity in the same transaction and
/// replaying it at startup; #106 does not yet provide such a journal.
/// Delivery tasks may likewise outlive a target session, but they carry
/// only runtime publication: durable player money reloads from SQL and the
/// already-committed authority prevents a second in-process payout.
///
/// Moved verbatim from `crates/wow-world/src/session/money/operations.rs`
/// under #1263 F4: admission, the sorted/deduplicated payouts, the
/// per-recipient persistence guards and mutation locks, the SQL attempt with
/// its rollback/unknown-COMMIT reconciliation, the authority commit and the
/// delivery scheduling keep their original order, refusal semantics and
/// durability fences.
pub fn spawn_group_loot_money_persistence_like_cpp(
    lifecycle: &SessionLifecycleState,
    player_present: bool,
    mut payouts: Vec<(ObjectGuid, u64)>,
    claim: LootClaimLease,
    mut deliveries: Vec<(LootMoneyDeliveryAddressLikeCpp, SessionCommand)>,
    authority_committed: Arc<AtomicBool>,
    viewer_fanout: LootMoneyViewerFanoutLikeCpp,
) -> Result<LootMoneyPersistenceWorkerHandleLikeCpp, LootMoneyPersistenceErrorLikeCpp> {
    if payouts.is_empty() {
        return Err(LootMoneyPersistenceErrorLikeCpp::MissingPlayer);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    let test_result = lifecycle.loot_money_persistence_test_result_like_cpp();
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let test_result: Option<bool> = None;

    payouts.sort_unstable_by_key(|(recipient, _)| (recipient.high_value(), recipient.low_value()));
    payouts.dedup_by_key(|(recipient, _)| *recipient);

    // Register every recipient directly before any SQL begins. Commands
    // are publication-only; waiting for target acknowledgements here would
    // create self/A↔B deadlocks between concurrent group looters.
    let payout_recipients = payouts
        .iter()
        .map(|(recipient, _)| *recipient)
        .collect::<HashSet<_>>();
    let mut money_persistence_guards =
        HashMap::<ObjectGuid, DurableLootMoneyPersistenceGuardLikeCpp>::with_capacity(
            payouts.len(),
        );
    let mut money_mutation_trackers = HashMap::<
        ObjectGuid,
        Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    >::with_capacity(payouts.len());
    for (_, command) in &deliveries {
        let SessionCommand::ApplyLootMoneyLikeCpp(command) = command else {
            continue;
        };
        if payout_recipients.contains(&command.recipient)
            && !money_persistence_guards.contains_key(&command.recipient)
        {
            let guard = command
                .durable_persistence_tracker
                .begin_like_cpp()
                .map_err(|_| LootMoneyPersistenceErrorLikeCpp::MissingPlayer)?;
            money_persistence_guards.insert(command.recipient, guard);
            money_mutation_trackers.insert(
                command.recipient,
                Arc::clone(&command.durable_persistence_tracker),
            );
        }
    }
    if money_persistence_guards.len() != payouts.len() {
        return Err(LootMoneyPersistenceErrorLikeCpp::MissingPlayer);
    }

    let persistence_port = if test_result.is_some() {
        None
    } else {
        Some(
            lifecycle
                .group_loot_money_persistence_port_like_cpp()
                .cloned()
                .ok_or(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase)?,
        )
    };

    let mut persistence_guard = claim
        .begin_persistence_guard_like_cpp()
        .map_err(LootMoneyPersistenceErrorLikeCpp::Claim)?;
    drop(claim);

    Ok(tokio::spawn(async move {
        // Match the sorted character-row lock order below. Stored-item
        // money uses the same per-character lock before it takes the row,
        // so COMMIT reconciliation cannot be confused by a later local
        // payout interleaving between the failed reply and our reads.
        let mut _money_mutation_locks = Vec::with_capacity(payouts.len());
        for (recipient, _) in &payouts {
            _money_mutation_locks.push(
                money_mutation_trackers
                    .get(recipient)
                    .expect("every payout retained its target mutation lock")
                    .lock_money_mutation_like_cpp()
                    .await,
            );
        }

        let durable_outcomes = if let Some(success) = test_result {
            // Give cancellation regressions a deterministic opportunity to
            // drop the outer waiter while this detached task owns `claim`.
            tokio::task::yield_now().await;
            if !success {
                return Err(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase);
            }
            payouts
                .iter()
                .map(|(recipient, amount)| {
                    (
                        recipient.counter() as u64,
                        wow_persistence::GroupLootMoneyPersistenceOutcomeLikeCpp {
                            recipient_guid: recipient.counter() as u64,
                            before: 0,
                            after: *amount,
                            applied_delta: *amount,
                        },
                    )
                })
                .collect::<HashMap<_, _>>()
        } else {
            let persistence_port =
                persistence_port.expect("production loot-money worker must own a persistence port");
            let request = wow_persistence::GroupLootMoneyPersistenceRequestLikeCpp {
                payouts: payouts
                    .iter()
                    .map(
                        |(recipient, amount)| wow_persistence::GroupLootMoneyPayoutLikeCpp {
                            recipient_guid: recipient.counter() as u64,
                            requested_delta: *amount,
                        },
                    )
                    .collect(),
                max_money: MAX_MONEY_AMOUNT,
            };
            let durable_outcomes = match persistence_port
                .attempt_group_loot_money_like_cpp(request)
                .await
            {
                wow_persistence::GroupLootMoneyPersistenceAttemptLikeCpp::Applied(outcomes) => {
                    outcomes
                        .into_iter()
                        .map(|outcome| (outcome.recipient_guid, outcome))
                        .collect::<HashMap<_, _>>()
                }
                wow_persistence::GroupLootMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                    kind,
                    reason,
                    ..
                } => {
                    return Err(match kind {
                        wow_persistence::GroupLootMoneyRollbackKindLikeCpp::MissingPlayer { .. } => {
                            LootMoneyPersistenceErrorLikeCpp::MissingPlayer
                        }
                        wow_persistence::GroupLootMoneyRollbackKindLikeCpp::Database => {
                            LootMoneyPersistenceErrorLikeCpp::Persistence(reason)
                        }
                    });
                }
                wow_persistence::GroupLootMoneyPersistenceAttemptLikeCpp::CommitOutcomeUnknown {
                    reason,
                    outcomes: durable_outcomes,
                } => match persistence_port
                    .reconcile_group_loot_money_like_cpp(durable_outcomes.clone())
                    .await
                {
                    wow_persistence::GroupLootMoneyReconciliationLikeCpp::RolledBack => {
                        return Err(LootMoneyPersistenceErrorLikeCpp::Persistence(
                            "loot-money COMMIT was reconciled as rolled back".to_owned(),
                        ));
                    }
                    wow_persistence::GroupLootMoneyReconciliationLikeCpp::Indeterminate { .. } => {
                        for guard in money_persistence_guards.values_mut() {
                            guard.mark_indeterminate_like_cpp();
                        }
                        let _ = persistence_guard.quarantine_commit_unknown_like_cpp();
                        for (delivery, _) in &deliveries {
                            let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
                                reason: "loot-money COMMIT outcome is unknown; relog required"
                                    .to_string(),
                            });
                            delivery.clone().queue_reliably_like_cpp(kick);
                        }
                        return Err(LootMoneyPersistenceErrorLikeCpp::CommitOutcomeUnknownPersistence(
                            reason,
                        ));
                    }
                    wow_persistence::GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop => {
                        durable_outcomes
                            .into_iter()
                            .map(|outcome| (outcome.recipient_guid, outcome))
                            .collect::<HashMap<_, _>>()
                    }
                },
            };
            durable_outcomes
        };

        for (_, command) in &mut deliveries {
            if let SessionCommand::ApplyLootMoneyLikeCpp(command) = command {
                let outcome = durable_outcomes
                    .get(&(command.recipient.counter() as u64))
                    .copied()
                    .expect("every admitted payout retains its locked DB outcome");
                command
                    .durable_applied_amount
                    .store(outcome.applied_delta, Ordering::Release);
                money_persistence_guards
                    .get_mut(&command.recipient)
                    .expect("every payout registered its target money fence")
                    .commit_like_cpp(DurableLootMoneyCompletionLikeCpp {
                        durable_money_before: outcome.before,
                        durable_money_after: outcome.after,
                        durable_applied_amount: outcome.applied_delta,
                        applied: Arc::clone(&command.applied),
                    });
            }
        }

        let committed_snapshot = match persistence_guard.commit_with_snapshot_like_cpp() {
            Ok((_, committed_snapshot)) => {
                authority_committed.store(true, Ordering::Release);
                committed_snapshot
            }
            Err(error) => {
                // SQL is already durable. Never let a lifecycle race reopen
                // this old allocation; payout publication still proceeds,
                // but replacement loot is not touched.
                warn!(?error, "durable loot-money authority commit failed closed");
                let _ = persistence_guard.quarantine_commit_unknown_like_cpp();
                None
            }
        };

        // `Loot::PlayersLooting` can grow while the detached SQL task is
        // running. The snapshot above was captured by the commit while
        // holding the same authority mutex: every included opener saw
        // non-zero money, and every later opener sees zero directly.
        let mut viewers = committed_snapshot
            .filter(|snapshot| {
                snapshot.generation == viewer_fanout.authority_generation
                    && snapshot.loot.loot_guid == viewer_fanout.loot_obj
            })
            .map(|snapshot| {
                snapshot
                    .loot
                    .players_looting
                    .into_iter()
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();

        for (_, command) in &mut deliveries {
            if let SessionCommand::ApplyLootMoneyLikeCpp(command) = command {
                command
                    .send_coin_removed
                    .store(viewers.remove(&command.recipient), Ordering::Release);
            }
        }

        for viewer in viewers {
            if viewer_fanout.payout_recipients.contains(&viewer) {
                continue;
            }
            let delivery = if viewer == viewer_fanout.source_player {
                Some(LootMoneyDeliveryAddressLikeCpp::Source(
                    viewer_fanout.source_command_tx.clone(),
                ))
            } else {
                viewer_fanout.player_registry.as_ref().and_then(|registry| {
                    let registration = registry.in_world_loot_delivery_recipient(
                        viewer,
                        viewer_fanout.map_id,
                        viewer_fanout.instance_id,
                    )?;
                    Some(LootMoneyDeliveryAddressLikeCpp::Directory {
                        registry: Arc::clone(registry),
                        registration,
                    })
                })
            };
            let Some(delivery) = delivery else {
                continue;
            };
            deliveries.push((
                delivery,
                SessionCommand::NotifyLootMoneyRemovedLikeCpp(
                    NotifyLootMoneyRemovedLikeCppCommand {
                        recipient: viewer,
                        loot_owner: viewer_fanout.loot_owner,
                        loot_obj: viewer_fanout.loot_obj,
                        authority: viewer_fanout.authority.clone(),
                        authority_generation: viewer_fanout.authority_generation,
                        authority_committed: Arc::clone(&authority_committed),
                    },
                ),
            ));
        }

        // Do not make persistence wait for another session's bounded
        // command queue. Each delivery task owns its command until the
        // target drains capacity or disconnects.
        for (delivery, command) in deliveries {
            delivery.queue_reliably_like_cpp(command);
        }
        Ok(())
    }))
}

/// C++ `HandleLootMoneyOpcode`'s stored-Item half
/// (`LootHandler.cpp`): await the complete atomic character/source-row worker
/// and report both completion flags unchanged.
///
/// Moved verbatim from `crates/wow-world/src/handlers/loot/money.rs` under
/// #1263 F4, together with its detached worker.
pub async fn persist_and_consume_stored_item_money_like_cpp(
    lifecycle: &SessionLifecycleState,
    player_guid: Option<ObjectGuid>,
    test_current_money: Option<u64>,
    command_tx: flume::Sender<SessionCommand>,
    item_guid: ObjectGuid,
    cached_notified_amount: u64,
) -> Option<(Arc<AtomicBool>, Arc<AtomicBool>, u64, u64)> {
    let (worker, balance_applied, publication_applied) =
        spawn_stored_item_money_persistence_worker_like_cpp(
            lifecycle,
            player_guid,
            test_current_money,
            command_tx,
            item_guid,
            cached_notified_amount,
        )?;
    match worker.await {
        Ok(Ok((applied_delta, notified_amount))) => Some((
            balance_applied,
            publication_applied,
            applied_delta,
            notified_amount,
        )),
        Ok(Err(error)) => {
            warn!(
                item_guid = item_guid.counter(),
                ?error,
                "failed to atomically persist and consume stored item loot money"
            );
            None
        }
        Err(error) => {
            warn!(
                item_guid = item_guid.counter(),
                ?error,
                "stored item loot-money persistence worker terminated"
            );
            None
        }
    }
}

fn spawn_stored_item_money_persistence_worker_like_cpp(
    lifecycle: &SessionLifecycleState,
    player_guid: Option<ObjectGuid>,
    test_current_money: Option<u64>,
    command_tx: flume::Sender<SessionCommand>,
    item_guid: ObjectGuid,
    cached_notified_amount: u64,
) -> Option<(
    tokio::task::JoinHandle<Result<(u64, u64), LootMoneyPersistenceErrorLikeCpp>>,
    Arc<AtomicBool>,
    Arc<AtomicBool>,
)> {
    let Some(player_guid) = player_guid else {
        return None;
    };
    #[cfg(any(test, feature = "test-fixtures"))]
    let test_result = lifecycle.loot_money_persistence_test_result_like_cpp();
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let test_result: Option<bool> = None;
    let persistence_port = if test_result.is_some() {
        None
    } else {
        Some(
            lifecycle
                .stored_item_money_persistence_port_like_cpp()
                .cloned()?,
        )
    };
    // C++ `Player::GetMoney` through the Inventory owner's selected-owner
    // operation (#1263 F4 dependency 5); the World wrapper is not on this path.
    let test_current_money = test_current_money?;
    let balance_applied = Arc::new(AtomicBool::new(false));
    let publication_applied = Arc::new(AtomicBool::new(false));
    let mut item_persistence_guard = lifecycle.begin_durable_item_loot_persistence_like_cpp();
    let money_persistence_tracker =
        Arc::clone(lifecycle.durable_loot_money_persistence_tracker_like_cpp());
    let mut money_persistence_guard = money_persistence_tracker.begin_like_cpp().ok()?;
    let worker_balance_applied = Arc::clone(&balance_applied);
    let worker_publication_applied = Arc::clone(&publication_applied);
    let worker = tokio::spawn(async move {
        let _money_mutation_lock = money_persistence_tracker
            .lock_money_mutation_like_cpp()
            .await;
        let (before, after, applied_delta, notified_amount) = if let Some(success) = test_result {
            tokio::task::yield_now().await;
            if !success {
                return Err(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase);
            }
            let (after, applied_delta) =
                loot_money_durable_outcome_like_cpp(test_current_money, cached_notified_amount);
            (
                test_current_money,
                after,
                applied_delta,
                cached_notified_amount,
            )
        } else {
            let persistence_port =
                persistence_port.expect("production stored-money worker has a persistence port");
            let request = wow_persistence::StoredItemMoneyPersistenceRequestLikeCpp {
                player_guid: player_guid.counter() as u64,
                item_guid: item_guid.counter() as u64,
                cached_notified_amount,
                max_money: MAX_MONEY_AMOUNT,
            };
            let outcome = match persistence_port
                .attempt_stored_item_money_like_cpp(request)
                .await
            {
                wow_persistence::StoredItemMoneyPersistenceAttemptLikeCpp::Applied(outcome) => {
                    outcome
                }
                wow_persistence::StoredItemMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                    kind,
                    reason,
                    ..
                } => {
                    return Err(match kind {
                        wow_persistence::StoredItemMoneyRollbackKindLikeCpp::MissingPlayer => {
                            LootMoneyPersistenceErrorLikeCpp::MissingPlayer
                        }
                        wow_persistence::StoredItemMoneyRollbackKindLikeCpp::SourceAlreadyConsumed
                        | wow_persistence::StoredItemMoneyRollbackKindLikeCpp::Database => {
                            LootMoneyPersistenceErrorLikeCpp::Persistence(reason)
                        }
                    });
                }
                wow_persistence::StoredItemMoneyPersistenceAttemptLikeCpp::CommitOutcomeUnknown {
                    reason,
                    outcome,
                } => match persistence_port
                    .reconcile_stored_item_money_like_cpp(request, outcome)
                    .await
                {
                    wow_persistence::StoredItemMoneyReconciliationLikeCpp::Committed => outcome,
                    wow_persistence::StoredItemMoneyReconciliationLikeCpp::RolledBack => {
                        return Err(LootMoneyPersistenceErrorLikeCpp::Persistence(
                            "stored Item money COMMIT was reconciled as rolled back".to_owned(),
                        ));
                    }
                    wow_persistence::StoredItemMoneyReconciliationLikeCpp::Indeterminate { .. } => {
                        money_persistence_guard.mark_indeterminate_like_cpp();
                        let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
                            reason: "stored Item money COMMIT outcome is unknown; relog required"
                                .to_string(),
                        });
                        if let Err(error) = command_tx.try_send(kick) {
                            let kick = error.into_inner();
                            let command_tx = command_tx.clone();
                            tokio::spawn(async move {
                                let _ = command_tx.send_async(kick).await;
                            });
                        }
                        return Err(
                            LootMoneyPersistenceErrorLikeCpp::CommitOutcomeUnknownPersistence(reason),
                        );
                    }
                },
            };
            (
                outcome.before,
                outcome.after,
                outcome.applied_delta,
                outcome.notified_amount,
            )
        };

        money_persistence_guard.commit_like_cpp(
            wow_world_core::loot_persistence::DurableLootMoneyCompletionLikeCpp {
                durable_money_before: before,
                durable_money_after: after,
                durable_applied_amount: applied_delta,
                applied: Arc::clone(&worker_balance_applied),
            },
        );
        item_persistence_guard.mark_committed_like_cpp(DurableItemLootCompletionLikeCpp {
            owner_guid: item_guid,
            loot_list_id: 0,
            player_guid,
            item_owner_auto_release: false,
            durable_item_money_applied_amount: Some(applied_delta),
            durable_item_money_notified_amount: Some(notified_amount),
            durable_item_money_balance_applied: Some(Arc::clone(&worker_balance_applied)),
            item_fanout: None,
            runtime_inventory_applied: Arc::clone(&worker_publication_applied),
        });
        Ok((applied_delta, notified_amount))
    });
    Some((worker, balance_applied, publication_applied))
}
