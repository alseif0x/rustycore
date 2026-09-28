//! Persistence workers and the errors they report back to the world loop.
//!
//! Split out of `loot/mod.rs` under #584 (B5); items are unchanged.

use super::*;

/// Own a loot lease in the same detached task that crosses the durable
/// persistence boundary.  Tokio does not cancel a spawned task when the
/// caller drops its `JoinHandle`, so packet/session cancellation cannot turn a
/// successful SQL commit back into an available object-owned claim.
pub(in crate::handlers::loot) enum LootClaimPersistenceWorkerError<E> {
    Persistence(E),
    Claim(LootClaimCommitError),
}

pub(in crate::handlers::loot) fn queue_stored_item_money_indeterminate_kick_like_cpp(
    command_tx: &flume::Sender<SessionCommand>,
) {
    let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
        reason: "stored Item money COMMIT outcome is unknown; relog required".to_string(),
    });
    if let Err(error) = command_tx.try_send(kick) {
        let kick = error.into_inner();
        let command_tx = command_tx.clone();
        tokio::spawn(async move {
            let _ = command_tx.send_async(kick).await;
        });
    }
}

pub(in crate::handlers::loot) fn spawn_loot_claim_persistence_worker_like_cpp<F, E>(
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
pub(in crate::handlers::loot) fn spawn_loot_item_persistence_worker_like_cpp<F>(
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
