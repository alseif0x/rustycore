//! Owned-loot authority, claims and leases operations, part 2 of 2.
//!
//! The inherent `OwnedLootAuthority` impl is divided by responsibility under
//! #642; every method keeps its original body.

use super::*;

impl OwnedLootAuthority {
    pub async fn reserve_item_for_generation_like_cpp(
        &self,
        player: ObjectGuid,
        loot_list_id: u8,
        expected_generation: u64,
    ) -> Result<LootClaimLease, LootClaimError> {
        self.reserve_item_with_mode_like_cpp(
            player,
            loot_list_id,
            LootItemClaimMode::Direct,
            Some(expected_generation),
        )
        .await
    }
    /// Uses the same atomic claim boundary for master-loot and completed-roll awards.
    /// Award paths may consume a group-roll-blocked slot, but still enforce the selected winner.
    pub async fn reserve_item_for_award_like_cpp(
        &self,
        player: ObjectGuid,
        loot_list_id: u8,
    ) -> Result<LootClaimLease, LootClaimError> {
        self.reserve_item_with_mode_like_cpp(player, loot_list_id, LootItemClaimMode::Award, None)
            .await
    }
    pub async fn reserve_item_for_award_generation_like_cpp(
        &self,
        player: ObjectGuid,
        loot_list_id: u8,
        expected_generation: u64,
    ) -> Result<LootClaimLease, LootClaimError> {
        self.reserve_item_with_mode_like_cpp(
            player,
            loot_list_id,
            LootItemClaimMode::Award,
            Some(expected_generation),
        )
        .await
    }
    pub(super) async fn reserve_item_with_mode_like_cpp(
        &self,
        player: ObjectGuid,
        loot_list_id: u8,
        mode: LootItemClaimMode,
        expected_generation: Option<u64>,
    ) -> Result<LootClaimLease, LootClaimError> {
        let mut changed = self.inner.changed.subscribe();
        loop {
            let attempt = {
                let mut state = self.lock_state();
                reserve_item_once(&mut state, player, loot_list_id, mode, expected_generation)
            };
            match attempt {
                ReserveAttempt::Acquired {
                    generation,
                    token,
                    key,
                    payload,
                } => {
                    return Ok(LootClaimLease::new(
                        self.clone(),
                        generation,
                        token,
                        key,
                        player,
                        payload,
                    ));
                }
                ReserveAttempt::Wait => {
                    // The sender is owned by the authority, so closure only occurs when the
                    // authority itself is gone (which this method's `&self` prevents).
                    let _ = changed.changed().await;
                }
                ReserveAttempt::Rejected(error) => return Err(error),
            }
        }
    }
    /// Waits until the selected money pool is unreserved, then reserves it atomically.
    pub async fn reserve_money_like_cpp(
        &self,
        player: ObjectGuid,
    ) -> Result<LootClaimLease, LootClaimError> {
        self.reserve_money_for_optional_generation_like_cpp(player, None)
            .await
    }
    pub async fn reserve_money_for_generation_like_cpp(
        &self,
        player: ObjectGuid,
        expected_generation: u64,
    ) -> Result<LootClaimLease, LootClaimError> {
        self.reserve_money_for_optional_generation_like_cpp(player, Some(expected_generation))
            .await
    }
    pub(super) async fn reserve_money_for_optional_generation_like_cpp(
        &self,
        player: ObjectGuid,
        expected_generation: Option<u64>,
    ) -> Result<LootClaimLease, LootClaimError> {
        let mut changed = self.inner.changed.subscribe();
        loop {
            let attempt = {
                let mut state = self.lock_state();
                reserve_money_once(&mut state, player, expected_generation)
            };
            match attempt {
                ReserveAttempt::Acquired {
                    generation,
                    token,
                    key,
                    payload,
                } => {
                    return Ok(LootClaimLease::new(
                        self.clone(),
                        generation,
                        token,
                        key,
                        player,
                        payload,
                    ));
                }
                ReserveAttempt::Wait => {
                    let _ = changed.changed().await;
                }
                ReserveAttempt::Rejected(error) => return Err(error),
            }
        }
    }
    pub(super) fn lock_state(&self) -> MutexGuard<'_, AuthorityState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(super) fn state_snapshot(&self) -> AuthorityState {
        self.lock_state().clone()
    }
    pub(super) fn notify_changed(&self) {
        self.inner
            .changed
            .send_modify(|version| *version = version.wrapping_add(1));
    }
    pub(super) fn begin_claim_persistence(
        &self,
        lease: &LootClaimLeaseInner,
    ) -> Result<(), LootClaimCommitError> {
        let mut state = self.lock_state();
        // The status check belongs under the same authority lock as the
        // reservation transition. Otherwise a cloned lease can commit after
        // an optimistic check and before this lock is acquired, letting a
        // stale persistence begin overwrite a terminal status (ABA).
        match lease.status.load(Ordering::Acquire) {
            LEASE_COMMITTED => return Err(LootClaimCommitError::StateChanged),
            LEASE_ROLLED_BACK => return Err(LootClaimCommitError::RolledBack),
            LEASE_ACTIVE => {}
            _ => return Err(LootClaimCommitError::StateChanged),
        }
        if state.persisting.get(&lease.key) == Some(&lease.token) {
            return Err(LootClaimCommitError::StateChanged);
        }
        if state.retired
            || scope_epoch(&state, reservation_scope(lease.key)) != Some(lease.generation)
        {
            lease.status.store(LEASE_ROLLED_BACK, Ordering::Release);
            return Err(LootClaimCommitError::StaleGeneration);
        }
        if state.reservations.get(&lease.key) != Some(&lease.token) {
            lease.status.store(LEASE_ROLLED_BACK, Ordering::Release);
            return Err(LootClaimCommitError::StateChanged);
        }
        state.persisting.insert(lease.key, lease.token);
        Ok(())
    }
    pub(super) fn abort_claim_persistence(&self, lease: &LootClaimLeaseInner) -> bool {
        let aborted = {
            let mut state = self.lock_state();
            if lease.status.load(Ordering::Acquire) != LEASE_ACTIVE
                || state.persisting.get(&lease.key) != Some(&lease.token)
            {
                return false;
            }
            state.persisting.remove(&lease.key);
            state.reservations.remove(&lease.key);
            lease.status.store(LEASE_ROLLED_BACK, Ordering::Release);
            finalize_closed_state_if_drained(&mut state);
            true
        };
        if aborted {
            self.notify_changed();
        }
        aborted
    }
    pub(super) fn quarantine_claim_persistence_commit_unknown(
        &self,
        lease: &LootClaimLeaseInner,
    ) -> bool {
        let quarantined = {
            let mut state = self.lock_state();
            if lease.status.load(Ordering::Acquire) != LEASE_ACTIVE
                || state.persisting.get(&lease.key) != Some(&lease.token)
            {
                return false;
            }

            // A transport error after COMMIT was submitted cannot prove
            // whether SQL applied the durable side effect. Remove the local
            // reservation so persistence waiters drain, but never make it
            // claimable again: the whole authority becomes an attached,
            // retired quarantine until DB reconciliation decides the result.
            state.persisting.remove(&lease.key);
            state.reservations.remove(&lease.key);
            lease.status.store(LEASE_QUARANTINED, Ordering::Release);
            state.retired = true;
            state.quarantined = true;
            finalize_closed_state_if_drained(&mut state);
            true
        };
        if quarantined {
            self.notify_changed();
        }
        quarantined
    }
    pub(super) fn commit_claim(
        &self,
        lease: &LootClaimLeaseInner,
        persistence_owner: bool,
    ) -> Result<bool, LootClaimCommitError> {
        self.commit_claim_with_snapshot(lease, persistence_owner)
            .map(|outcome| outcome.first_commit)
    }
    pub(super) fn commit_claim_with_snapshot(
        &self,
        lease: &LootClaimLeaseInner,
        persistence_owner: bool,
    ) -> Result<LootClaimCommitOutcome, LootClaimCommitError> {
        if lease.status.load(Ordering::Acquire) == LEASE_COMMITTED {
            return Ok(LootClaimCommitOutcome {
                first_commit: false,
                snapshot: None,
            });
        }
        if lease.status.load(Ordering::Acquire) == LEASE_QUARANTINED {
            return Err(LootClaimCommitError::StateChanged);
        }

        let result = {
            let mut state = self.lock_state();
            match lease.status.load(Ordering::Acquire) {
                LEASE_COMMITTED => {
                    return Ok(LootClaimCommitOutcome {
                        first_commit: false,
                        snapshot: None,
                    });
                }
                LEASE_ROLLED_BACK => return Err(LootClaimCommitError::RolledBack),
                LEASE_QUARANTINED => return Err(LootClaimCommitError::StateChanged),
                _ => {}
            }

            let scope = reservation_scope(lease.key);
            let persistence_protected = state.persisting.get(&lease.key) == Some(&lease.token);
            if persistence_protected && !persistence_owner {
                return Err(LootClaimCommitError::StateChanged);
            }
            if (state.retired && !persistence_protected)
                || scope_epoch(&state, scope) != Some(lease.generation)
            {
                lease.status.store(LEASE_ROLLED_BACK, Ordering::Release);
                if persistence_protected {
                    state.persisting.remove(&lease.key);
                    finalize_closed_state_if_drained(&mut state);
                }
                Err(LootClaimCommitError::StaleGeneration)
            } else if state.reservations.get(&lease.key) != Some(&lease.token) {
                lease.status.store(LEASE_ROLLED_BACK, Ordering::Release);
                if persistence_protected {
                    state.persisting.remove(&lease.key);
                    finalize_closed_state_if_drained(&mut state);
                }
                Err(LootClaimCommitError::StateChanged)
            } else {
                let committed = match lease.key {
                    ReservationKey::Item(key) => loot_for_scope_mut(&mut state, key.scope)
                        .is_some_and(|loot| {
                            loot.mark_item_looted_for_player_like_cpp(
                                key.loot_list_id,
                                lease.player,
                            )
                        }),
                    ReservationKey::Money(scope) => loot_for_scope_mut(&mut state, scope)
                        .is_some_and(|loot| {
                            loot.coins = 0;
                            true
                        }),
                };

                // Capture the exact post-mutation pool while still holding
                // the authority mutex. This is the C++ serialization point
                // used by money fanout: a viewer captured here necessarily
                // opened before coins became zero; a later opener observes
                // zero directly and must not receive a spurious removal.
                let committed_snapshot = committed.then(|| {
                    scope_epoch(&state, scope).and_then(|generation| {
                        loot_for_scope(&state, scope)
                            .cloned()
                            .map(|loot| OwnedLootSnapshot {
                                generation,
                                scope,
                                loot,
                            })
                    })
                });

                state.reservations.remove(&lease.key);
                state.persisting.remove(&lease.key);
                if committed {
                    lease.status.store(LEASE_COMMITTED, Ordering::Release);
                    finalize_closed_state_if_drained(&mut state);
                    Ok(LootClaimCommitOutcome {
                        first_commit: true,
                        snapshot: committed_snapshot.flatten(),
                    })
                } else {
                    lease.status.store(LEASE_ROLLED_BACK, Ordering::Release);
                    finalize_closed_state_if_drained(&mut state);
                    Err(LootClaimCommitError::StateChanged)
                }
            }
        };
        self.notify_changed();
        result
    }
    pub(super) fn rollback_claim(&self, lease: &LootClaimLeaseInner) -> bool {
        let removed = {
            let mut state = self.lock_state();
            if lease.status.load(Ordering::Acquire) != LEASE_ACTIVE {
                return false;
            }
            if state.persisting.get(&lease.key) == Some(&lease.token) {
                return false;
            }
            let removed = scope_epoch(&state, reservation_scope(lease.key))
                == Some(lease.generation)
                && state.reservations.get(&lease.key) == Some(&lease.token)
                && state.reservations.remove(&lease.key).is_some();
            if state.persisting.get(&lease.key) == Some(&lease.token) {
                state.persisting.remove(&lease.key);
            }
            lease.status.store(LEASE_ROLLED_BACK, Ordering::Release);
            finalize_closed_state_if_drained(&mut state);
            removed
        };
        if removed {
            self.notify_changed();
        }
        removed
    }
}
