// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Coalesced respawn persistence writer and shutdown lifecycle.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::{debug, warn};
use wow_persistence::{
    RespawnPersistenceMutationLikeCpp, RespawnPersistenceMutationOutcomeLikeCpp,
    RespawnPersistencePortLikeCpp,
};

use super::{
    PendingRespawnDbMutationLikeCpp, RESPAWN_DB_PRODUCER_STOP_TIMEOUT,
    RESPAWN_DB_RETRY_INITIAL_DELAY, RESPAWN_DB_RETRY_MAX_DELAY, RESPAWN_DB_WRITER_DRAIN_TIMEOUT,
    RespawnDbAttemptLikeCpp, RespawnDbMailboxLikeCpp, RespawnDbMutationKeyLikeCpp,
    RespawnDbRetryQueueLikeCpp, RespawnDbSubmitErrorLikeCpp, RespawnDbWriterPollLikeCpp,
    RespawnDbWriterSenderLikeCpp, RespawnDbWriterTaskGuardLikeCpp,
};

impl RespawnDbRetryQueueLikeCpp {
    pub(super) fn enqueue_latest(
        &mut self,
        mutation: RespawnPersistenceMutationLikeCpp,
        now: Instant,
    ) -> RespawnDbMutationKeyLikeCpp {
        let key = mutation.key();
        self.pending.insert(
            key,
            PendingRespawnDbMutationLikeCpp {
                mutation,
                consecutive_failures: 0,
                retry_not_before: now,
            },
        );
        key
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.pending
            .values()
            .map(|pending| pending.retry_not_before)
            .min()
    }

    pub(super) fn take_due(&mut self, now: Instant) -> Option<RespawnDbAttemptLikeCpp> {
        let key = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.retry_not_before <= now)
            .min_by_key(|(key, pending)| (pending.retry_not_before, **key))
            .map(|(key, _)| *key)?;
        self.pending
            .remove(&key)
            .map(|pending| RespawnDbAttemptLikeCpp { key, pending })
    }

    pub(super) fn retry_failed(
        &mut self,
        mut attempt: RespawnDbAttemptLikeCpp,
        observed_at: Instant,
    ) -> (Duration, u32) {
        attempt.pending.consecutive_failures =
            attempt.pending.consecutive_failures.saturating_add(1);
        let delay = respawn_db_retry_delay(attempt.pending.consecutive_failures);
        attempt.pending.retry_not_before = observed_at.checked_add(delay).unwrap_or(observed_at);
        let failed_count = attempt.pending.consecutive_failures;
        // Keep the failed operation only until the writer receives a newer
        // operation for the same key; `enqueue_latest` then replaces it.
        self.pending.entry(attempt.key).or_insert(attempt.pending);
        (delay, failed_count)
    }

    pub(super) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub(super) fn make_all_due(&mut self, now: Instant) {
        for pending in self.pending.values_mut() {
            pending.retry_not_before = now;
        }
    }
}

impl RespawnDbMailboxLikeCpp {
    fn close_like_cpp(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.closed = true;
        state.queue.make_all_due(Instant::now());
        drop(state);
        self.notify.notify_one();
    }
}

impl RespawnDbWriterSenderLikeCpp {
    pub(super) fn new_like_cpp() -> Self {
        Self {
            mailbox: Arc::new(RespawnDbMailboxLikeCpp::default()),
        }
    }

    pub(super) fn send(
        &self,
        mutation: RespawnPersistenceMutationLikeCpp,
    ) -> std::result::Result<(), RespawnDbSubmitErrorLikeCpp> {
        let mut state = self
            .mailbox
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed {
            return Err(RespawnDbSubmitErrorLikeCpp::Closed);
        }
        state.queue.enqueue_latest(mutation, Instant::now());
        drop(state);
        self.mailbox.notify.notify_one();
        Ok(())
    }

    pub(super) fn close_like_cpp(&self) {
        self.mailbox.close_like_cpp();
    }
}

impl Drop for RespawnDbWriterTaskGuardLikeCpp {
    fn drop(&mut self) {
        // Reject further producer submissions if the supervised writer exits,
        // panics, or is aborted unexpectedly.
        self.mailbox.close_like_cpp();
    }
}

pub(super) fn respawn_db_retry_delay(consecutive_failed_flushes: u32) -> Duration {
    let exponent = consecutive_failed_flushes.saturating_sub(1).min(31);
    let multiplier = 1_u32.checked_shl(exponent).unwrap_or(u32::MAX);
    RESPAWN_DB_RETRY_INITIAL_DELAY
        .saturating_mul(multiplier)
        .min(RESPAWN_DB_RETRY_MAX_DELAY)
}

pub(super) async fn execute_respawn_db_attempt_like_cpp(
    attempt: RespawnDbAttemptLikeCpp,
    mailbox: &RespawnDbMailboxLikeCpp,
    respawn_persistence: &dyn RespawnPersistencePortLikeCpp,
) {
    if let RespawnPersistenceMutationOutcomeLikeCpp::Failed { reason } = respawn_persistence
        .execute_mutation_like_cpp(attempt.pending.mutation)
        .await
    {
        let key = attempt.key;
        let (retry_delay, failed_attempts) = mailbox
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .queue
            .retry_failed(attempt, Instant::now());
        warn!(
            error = %reason,
            object_type = key.object_type_raw,
            spawn_id = key.spawn_id,
            map_id = key.map_id,
            instance_id = key.instance_id,
            retry_in_ms = retry_delay.as_millis(),
            failed_attempts,
            "Failed to persist respawn operation like C++; shared DB-writer retry deferred"
        );
    }
}

pub(super) fn spawn_respawn_db_writer_like_cpp(
    respawn_persistence: Arc<dyn RespawnPersistencePortLikeCpp>,
) -> (RespawnDbWriterSenderLikeCpp, tokio::task::JoinHandle<()>) {
    let sender = RespawnDbWriterSenderLikeCpp::new_like_cpp();
    let mailbox = Arc::clone(&sender.mailbox);
    let handle = tokio::spawn(async move {
        let _writer_guard = RespawnDbWriterTaskGuardLikeCpp {
            mailbox: Arc::clone(&mailbox),
        };

        loop {
            // Arm the notification before inspecting the mailbox. A producer
            // racing this check therefore leaves a permit instead of losing
            // the idle-writer wakeup.
            let notified = mailbox.notify.notified();
            let poll = {
                let mut state = mailbox
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let now = Instant::now();
                if let Some(attempt) = state.queue.take_due(now) {
                    RespawnDbWriterPollLikeCpp::Attempt(attempt)
                } else if state.closed && state.queue.pending_len() == 0 {
                    RespawnDbWriterPollLikeCpp::Finished
                } else if let Some(deadline) = state.queue.next_deadline() {
                    RespawnDbWriterPollLikeCpp::WaitUntil(deadline)
                } else {
                    RespawnDbWriterPollLikeCpp::WaitForNotification
                }
            };

            match poll {
                RespawnDbWriterPollLikeCpp::Attempt(attempt) => {
                    // Never hold the mailbox mutex across a database await.
                    // If a newer same-key operation arrives while this SQL is
                    // in flight, `retry_failed(...).or_insert(...)` preserves
                    // that newer operation instead of restoring stale state.
                    execute_respawn_db_attempt_like_cpp(
                        attempt,
                        mailbox.as_ref(),
                        respawn_persistence.as_ref(),
                    )
                    .await;
                }
                RespawnDbWriterPollLikeCpp::WaitForNotification => notified.await,
                RespawnDbWriterPollLikeCpp::WaitUntil(deadline) => {
                    tokio::select! {
                        _ = notified => {}
                        _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {}
                    }
                }
                RespawnDbWriterPollLikeCpp::Finished => break,
            }
        }
    });
    (sender, handle)
}

pub(super) async fn stop_respawn_db_producer_like_cpp(
    task_name: &'static str,
    handle: &mut tokio::task::JoinHandle<()>,
    already_finished: bool,
) -> bool {
    if already_finished {
        return true;
    }

    match tokio::time::timeout(RESPAWN_DB_PRODUCER_STOP_TIMEOUT, &mut *handle).await {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            tracing::error!(task_name, %error, "Respawn DB producer task failed during shutdown");
            false
        }
        Err(_) => {
            // `spawn_blocking` cannot be force-cancelled. Aborting the outer
            // task prevents further async iterations; a still-running closure
            // may attempt a late mailbox submission, which explicit writer
            // close rejects before the separately bounded drain.
            handle.abort();
            tracing::error!(
                task_name,
                timeout_ms = RESPAWN_DB_PRODUCER_STOP_TIMEOUT.as_millis(),
                "Respawn DB producer shutdown timed out; abort requested and terminal error status required"
            );
            false
        }
    }
}

pub(super) async fn drain_respawn_db_writer_like_cpp(
    handle: &mut tokio::task::JoinHandle<()>,
    already_finished: bool,
) -> bool {
    if already_finished {
        return false;
    }

    match tokio::time::timeout(RESPAWN_DB_WRITER_DRAIN_TIMEOUT, &mut *handle).await {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            tracing::error!(%error, "Shared respawn DB writer failed while draining");
            false
        }
        Err(_) => {
            tracing::error!(
                timeout_ms = RESPAWN_DB_WRITER_DRAIN_TIMEOUT.as_millis(),
                "Shared respawn DB writer drain timed out; aborting with persistence work still pending"
            );
            handle.abort();
            let _ = (&mut *handle).await;
            false
        }
    }
}
