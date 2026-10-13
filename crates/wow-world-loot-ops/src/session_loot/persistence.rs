// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable loot persistence and its worker (#1363 slice 5b; moved from the
//! World `handlers/loot/persistence.rs`).

use super::*;

/// Own a loot lease in the same detached task that crosses the durable
/// persistence boundary.  Tokio does not cancel a spawned task when the
/// caller drops its `JoinHandle`, so packet/session cancellation cannot turn a
/// successful SQL commit back into an available object-owned claim.
pub enum LootClaimPersistenceWorkerError<E> {
    Persistence(E),
    Claim(LootClaimCommitError),
}

impl<E: std::fmt::Debug> std::fmt::Debug for LootClaimPersistenceWorkerError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Persistence(error) => formatter.debug_tuple("Persistence").field(error).finish(),
            Self::Claim(error) => formatter.debug_tuple("Claim").field(error).finish(),
        }
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub fn spawn_loot_claim_persistence_worker_like_cpp<F, E>(
    persistence: F,
    claim: Option<LootClaimLease>,
    durable_item_completion: Option<(
        DurableItemLootPersistenceGuardLikeCpp,
        DurableItemLootCompletionLikeCpp,
    )>,
) -> Result<
    tokio::task::JoinHandle<Result<(), LootClaimPersistenceWorkerError<E>>>,
    LootClaimCommitError,
>
where
    F: std::future::Future<Output = Result<(), E>> + Send + 'static,
    E: Send + 'static,
{
    let persistence_guard = claim
        .as_ref()
        .map(LootClaimLease::begin_persistence_guard_like_cpp)
        .transpose()?;
    drop(claim);
    Ok(tokio::spawn(async move {
        let mut durable_item_completion = durable_item_completion;
        persistence
            .await
            .map_err(LootClaimPersistenceWorkerError::Persistence)?;
        if let Some(mut guard) = persistence_guard {
            let (_, committed_snapshot) = guard
                .commit_with_snapshot_like_cpp()
                .map_err(LootClaimPersistenceWorkerError::Claim)?;
            if let (Some(snapshot), Some((_, completion))) =
                (committed_snapshot, durable_item_completion.as_ref())
                && let Some(fanout) = completion.item_fanout.as_ref()
            {
                // Publish the serialization cut before exposing the durable
                // completion to the session. Sampling the authority later can
                // include an opener that already saw the consumed slot.
                let _ = fanout.committed_snapshot.set(snapshot);
            }
        }
        if let Some((guard, completion)) = durable_item_completion.as_mut() {
            guard.mark_committed_like_cpp(completion.clone());
        }
        Ok(())
    }))
}

/// Outcome-aware persistence worker for consume-and-grant item transactions.
/// An unknown COMMIT cannot be treated as rollback: the old object allocation
/// is quarantined permanently and the player is kicked to reload whichever
/// durable state the concrete adapter ultimately kept.
pub fn spawn_loot_item_persistence_worker_like_cpp<F>(
    persistence: F,
    claim: Option<LootClaimLease>,
    durable_item_completion: Option<(
        DurableItemLootPersistenceGuardLikeCpp,
        DurableItemLootCompletionLikeCpp,
    )>,
    command_tx: flume::Sender<SessionCommand>,
) -> Result<
    tokio::task::JoinHandle<Result<(), LootClaimPersistenceWorkerError<String>>>,
    LootClaimCommitError,
>
where
    F: std::future::Future<Output = PersistenceOutcomeLikeCpp> + Send + 'static,
{
    let mut persistence_guard = claim
        .as_ref()
        .map(LootClaimLease::begin_persistence_guard_like_cpp)
        .transpose()?;
    drop(claim);
    Ok(tokio::spawn(async move {
        let mut durable_item_completion = durable_item_completion;
        match persistence.await {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                return Err(LootClaimPersistenceWorkerError::Persistence(reason));
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                if let Some(guard) = persistence_guard.as_mut() {
                    let _ = guard.quarantine_commit_unknown_like_cpp();
                }
                let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
                    reason: "loot item COMMIT outcome is unknown; relog required".to_string(),
                });
                if let Err(send_error) = command_tx.try_send(kick) {
                    let kick = send_error.into_inner();
                    tokio::spawn(async move {
                        let _ = command_tx.send_async(kick).await;
                    });
                }
                return Err(LootClaimPersistenceWorkerError::Persistence(reason));
            }
        }
        if let Some(mut guard) = persistence_guard {
            let (_, committed_snapshot) = guard
                .commit_with_snapshot_like_cpp()
                .map_err(LootClaimPersistenceWorkerError::Claim)?;
            if let (Some(snapshot), Some((_, completion))) =
                (committed_snapshot, durable_item_completion.as_ref())
                && let Some(fanout) = completion.item_fanout.as_ref()
            {
                let _ = fanout.committed_snapshot.set(snapshot);
            }
        }
        if let Some((guard, completion)) = durable_item_completion.as_mut() {
            guard.mark_committed_like_cpp(completion.clone());
        }
        Ok(())
    }))
}

pub fn prepare_durable_loot_item_fanout_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    claim: &LootClaimLease,
    context: LootItemClaimCommitContextLikeCpp,
) -> Option<DurableLootItemFanoutLikeCpp> {
    // Both non-found facts refuse the durable fanout: an unreadable
    // designated owner must not publish a claim it could not verify.
    let OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority) =
        represented_owned_loot_authority_outcome_like_cpp(host, context.owner_guid)
    else {
        return None;
    };
    if !claim.shares_authority_like_cpp(&authority) {
        return None;
    }
    let precommit_snapshot = authority
        .snapshot_for_player_like_cpp(context.player_guid)
        .filter(|snapshot| {
            snapshot.generation == claim.generation_like_cpp()
                && snapshot.loot.loot_guid == context.loot_obj
                && snapshot
                    .loot
                    .items
                    .iter()
                    .any(|entry| entry.loot_list_id == context.loot_list_id)
        })?;
    let instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    Some(DurableLootItemFanoutLikeCpp {
        owner_guid: context.owner_guid,
        loot_obj: context.loot_obj,
        loot_list_id: context.loot_list_id,
        player_guid: context.player_guid,
        free_for_all: context.free_for_all,
        authority,
        authority_generation: claim.generation_like_cpp(),
        precommit_snapshot,
        committed_snapshot: Arc::new(std::sync::OnceLock::new()),
        source_send_tx: host.core().send_tx().clone(),
        player_registry: host.player_registry().cloned(),
        map_id: host.core().player_map_id_like_cpp(),
        instance_id,
        published: Arc::new(AtomicBool::new(false)),
    })
}

fn commit_represented_loot_item_claim_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    claim: &LootClaimLease,
    context: LootItemClaimCommitContextLikeCpp,
    fanout: Option<&DurableLootItemFanoutLikeCpp>,
) -> bool {
    if !claim.is_committed_like_cpp() {
        match claim.commit_with_snapshot_like_cpp() {
            Ok((_, Some(snapshot))) => {
                if let Some(fanout) = fanout {
                    let _ = fanout.committed_snapshot.set(snapshot);
                }
            }
            Ok((_, None)) => {
                warn!(
                    owner = ?context.owner_guid,
                    loot_list_id = context.loot_list_id,
                    "durable loot item committed without an exact authority snapshot"
                );
                return false;
            }
            Err(error) => {
                warn!(
                    owner = ?context.owner_guid,
                    loot_list_id = context.loot_list_id,
                    ?error,
                    "durable loot item could not commit its object-owned claim"
                );
                return false;
            }
        }
    }
    let Some(fanout) = fanout.filter(|fanout| {
        fanout.owner_guid == context.owner_guid
            && fanout.loot_obj == context.loot_obj
            && fanout.loot_list_id == context.loot_list_id
            && fanout.player_guid == context.player_guid
            && fanout.free_for_all == context.free_for_all
            && claim.shares_authority_like_cpp(&fanout.authority)
            && claim.generation_like_cpp() == fanout.authority_generation
    }) else {
        warn!(
            owner = ?context.owner_guid,
            loot_list_id = context.loot_list_id,
            "durable loot item committed without its retained fanout route"
        );
        return false;
    };
    host.loot_release_cx_like_cpp()
        .publish_durable_loot_item_fanout_like_cpp(fanout)
}

pub fn publish_persisted_loot_item_removal_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    claim: Option<&LootClaimLease>,
    context: Option<LootItemClaimCommitContextLikeCpp>,
    fanout: Option<&DurableLootItemFanoutLikeCpp>,
) -> bool {
    match (claim, context, fanout) {
        (None, None, None) => true,
        (Some(claim), Some(context), fanout) => {
            commit_represented_loot_item_claim_like_cpp(host, claim, context, fanout)
        }
        _ => {
            warn!("durable loot item claim/context mismatch before removal publication");
            false
        }
    }
}

/// Publishes successful durable loot transactions that outlived their
/// packet waiter. This replays committed Item-owned money or item grants
/// into runtime state before disconnect can persist a stale snapshot. C++
/// auto-releases only when `Loot::isLooted()` (zero coins and no visible
/// items) and the owner GUID is an Item.
pub async fn apply_pending_durable_item_loot_completions_with_generator_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) {
    apply_pending_durable_item_loot_completions_with_objective_drain_like_cpp(
        host,
        item_guid_generator,
        true,
    )
    .await;
}

pub async fn apply_pending_durable_item_loot_completions_with_objective_drain_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    drain_money_objectives: bool,
) {
    let completions = host.take_durable_item_loot_completions_like_cpp();
    for completion in completions {
        if let Some(fanout) = completion.item_fanout.as_ref() {
            let _ = host
                .loot_release_cx_like_cpp()
                .publish_durable_loot_item_fanout_like_cpp(fanout);
        }
        let requires_runtime_recovery =
            !completion.runtime_inventory_applied.load(Ordering::Acquire);
        let targets_current_player = host.player_guid() == Some(completion.player_guid);

        if let Some(applied_delta) = completion.durable_item_money_applied_amount {
            let apply_balance = targets_current_player
                && completion
                    .durable_item_money_balance_applied
                    .as_ref()
                    .is_some_and(|applied| {
                        applied
                            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                            .is_ok()
                    });
            let publish = targets_current_player
                && completion
                    .runtime_inventory_applied
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok();
            let Some(old_money) = host.resolved_player_money_like_cpp() else {
                continue;
            };
            if apply_balance {
                let new_money = old_money
                    .checked_add(applied_delta)
                    .filter(|money| *money <= MAX_MONEY_AMOUNT)
                    .unwrap_or(old_money);
                if !host.set_player_gold_like_cpp(new_money) {
                    continue;
                }
                if old_money != new_money {
                    host.quest_state_mut()
                        .enqueue_represented_quest_objective_progress_like_cpp(
                            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                                old_money,
                                new_money,
                            },
                        );
                }
            }
            if publish {
                host.represented_notify_money_removed_like_cpp(completion.owner_guid);
                host.send_packet(&LootMoneyNotify {
                    money: completion
                        .durable_item_money_notified_amount
                        .expect("durable Item money completion retains notification amount"),
                    money_mod: 0,
                    sole_looter: true,
                });

                let fully_looted = host
                    .loot_mut()
                    .cached_loot_for_owner_mut_like_cpp(completion.owner_guid)
                    .is_some_and(|loot| {
                        loot.coins = 0;
                        loot_is_looted_like_cpp(loot)
                    });
                if fully_looted {
                    // Source Item destruction remains owned by the normal
                    // C++ release phase; the completion only makes the
                    // already-durable money mutation visible first.
                    host.do_loot_release_owner_like_cpp(
                        completion.owner_guid,
                        completion.player_guid,
                    )
                    .await;
                }
            }
            if drain_money_objectives && (apply_balance || publish) {
                host.drain_represented_quest_objective_progress_with_generator_like_cpp(
                    item_guid_generator,
                )
                .await;
            }
            continue;
        }

        if targets_current_player && completion.item_owner_auto_release {
            debug_assert!(completion.owner_guid.is_item());
            let removal = host
                .loot_mut()
                .cached_loot_for_owner_mut_like_cpp(completion.owner_guid)
                .and_then(|loot| {
                    let entry = loot
                        .items
                        .iter()
                        .find(|entry| entry.loot_list_id == completion.loot_list_id)?;
                    let free_for_all = entry.flags.freeforall;
                    let newly_removed = !loot_item_is_looted_for_player_like_cpp(
                        loot,
                        entry,
                        completion.player_guid,
                    );
                    let loot_obj = loot.loot_guid;
                    mark_loot_item_looted_for_player_like_cpp(
                        loot,
                        completion.loot_list_id,
                        completion.player_guid,
                    );
                    Some((
                        loot_obj,
                        free_for_all,
                        newly_removed,
                        loot_is_looted_like_cpp(loot),
                    ))
                });

            if let Some((loot_obj, free_for_all, newly_removed, fully_looted)) = removal {
                if newly_removed {
                    if free_for_all {
                        host.send_packet(&LootRemoved {
                            owner: completion.owner_guid,
                            loot_obj,
                            loot_list_id: completion.loot_list_id,
                        });
                    } else {
                        represented_notify_loot_item_removed_like_cpp(
                            host,
                            completion.owner_guid,
                            completion.loot_list_id,
                        );
                    }
                }

                if fully_looted {
                    host.do_loot_release_owner_like_cpp(
                        completion.owner_guid,
                        completion.player_guid,
                    )
                    .await;
                }
            }
        } else if targets_current_player && requires_runtime_recovery {
            // The detached worker already committed the authority. Refresh
            // the packet cache so disconnect's DoLootReleaseAll observes
            // the consumed claim rather than the pre-commit session copy.
            refresh_owned_loot_summary_like_cpp(host, completion.owner_guid);
            let _ = reconcile_represented_loot_cache_like_cpp(
                host,
                completion.owner_guid,
                completion.player_guid,
            );
        }

        if requires_runtime_recovery {
            // SQL committed after the packet waiter disappeared, before
            // its synchronous runtime inventory publication. Do not let
            // the player operate on a stale slot; the persisted grant and
            // source consumption are reconstructed on the next login.
            host.core_mut()
                .kick("durable loot item completed after handler cancellation; relog required");
        }
    }
}

pub async fn wait_for_active_loot_persistence_with_generator_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) {
    let mut authorities = Vec::<OwnedLootAuthority>::new();
    for authority in host.loot().active_loot_view_authorities_iter_like_cpp() {
        if authorities
            .iter()
            .any(|existing| existing.shares_storage_like_cpp(authority))
        {
            continue;
        }
        authorities.push(authority.clone());
    }
    for authority in authorities {
        authority.wait_for_persisting_claims_like_cpp().await;
    }
    host.wait_for_durable_item_loot_persistence_like_cpp().await;
    apply_pending_durable_item_loot_completions_with_generator_like_cpp(host, item_guid_generator)
        .await;
}
