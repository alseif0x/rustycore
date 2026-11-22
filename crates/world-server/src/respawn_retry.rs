use super::*;

/// Keeps the latest respawn DB operation per spawn for the shared DB writer.
///
/// C++ submits each respawn statement once from `Map::SaveRespawnInfoDB` /
/// `Map::DeleteRespawnInfoFromDB` and executes it on the CharacterDatabase
/// worker. RustyCore mirrors that ownership with one writer for canonical and
/// legacy producers, avoiding both map-tick stalls and cross-runtime REP/DEL
/// reordering. Retry cadence is independent per spawn key.
#[derive(Debug)]
pub(super) struct PendingRespawnDbMutationLikeCpp {
    pub(super) mutation: RespawnPersistenceMutationLikeCpp,
    consecutive_failures: u32,
    retry_not_before: Instant,
}

#[derive(Debug, Default)]
pub(super) struct RespawnDbRetryQueueLikeCpp {
    pub(super) pending: BTreeMap<
        (RespawnDbMutationKeyLikeCpp, Option<(u16, u64)>),
        PendingRespawnDbMutationLikeCpp,
    >,
}

#[derive(Debug)]
pub(super) struct RespawnDbAttemptLikeCpp {
    pub(super) key: RespawnDbMutationKeyLikeCpp,
    pub(super) pending: PendingRespawnDbMutationLikeCpp,
}

impl RespawnDbRetryQueueLikeCpp {
    fn mailbox_key(
        mutation: RespawnPersistenceMutationLikeCpp,
    ) -> (RespawnDbMutationKeyLikeCpp, Option<(u16, u64)>) {
        let generation = match mutation {
            RespawnPersistenceMutationLikeCpp::ArmChestMoney {
                realm_id,
                generation,
                ..
            } => Some((realm_id, generation)),
            _ => None,
        };
        (mutation.key(), generation)
    }
    pub(super) fn enqueue_latest(
        &mut self,
        mutation: RespawnPersistenceMutationLikeCpp,
        now: Instant,
    ) -> RespawnDbMutationKeyLikeCpp {
        let key = mutation.key();
        self.pending.insert(
            Self::mailbox_key(mutation),
            PendingRespawnDbMutationLikeCpp {
                mutation,
                consecutive_failures: 0,
                retry_not_before: now,
            },
        );
        key
    }

    pub(super) fn next_deadline(&self) -> Option<Instant> {
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
            .map(|pending| RespawnDbAttemptLikeCpp {
                key: key.0,
                pending,
            })
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
        self.pending
            .entry(Self::mailbox_key(attempt.pending.mutation))
            .or_insert(attempt.pending);
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
