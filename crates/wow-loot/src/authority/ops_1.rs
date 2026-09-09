//! Owned-loot authority, claims and leases operations, part 1 of 2.
//!
//! The inherent `OwnedLootAuthority` impl is divided by responsibility under
//! #642; every method keeps its original body.

use super::*;

impl OwnedLootAuthority {
    #[must_use]
    pub fn new() -> Self {
        let (changed, _) = watch::channel(0);
        Self {
            inner: Arc::new(OwnedLootAuthorityInner {
                state: Mutex::new(AuthorityState::default()),
                changed,
            }),
        }
    }
    /// Whether both handles address the same object-owned loot state.
    ///
    /// State equality is insufficient here: two independently allocated
    /// authorities can contain identical loot while still allowing separate
    /// claims. Runtime mirror reconciliation must compare the backing `Arc`.
    #[must_use]
    pub fn shares_storage_like_cpp(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
    /// A newly constructed authority that has never owned a loot lifetime.
    #[must_use]
    pub fn is_pristine_like_cpp(&self) -> bool {
        self.lifecycle_like_cpp() == OwnedLootAuthorityLifecycle::Pristine
    }
    /// Classifies this backing allocation without racing two separate state
    /// reads. `Pristine` means it has never owned a C++ `Loot` lifetime;
    /// `Retired` means an earlier lifetime was explicitly invalidated.
    #[must_use]
    pub fn lifecycle_like_cpp(&self) -> OwnedLootAuthorityLifecycle {
        let state = self.lock_state();
        if state.detached {
            OwnedLootAuthorityLifecycle::Detached
        } else if state.quarantined {
            OwnedLootAuthorityLifecycle::Quarantined
        } else if !state.retired {
            OwnedLootAuthorityLifecycle::Active
        } else if state.generation == 0 {
            OwnedLootAuthorityLifecycle::Pristine
        } else {
            OwnedLootAuthorityLifecycle::Retired
        }
    }
    #[must_use]
    pub fn stamp_like_cpp(&self) -> OwnedLootAuthorityStamp {
        let state = self.lock_state();
        let lifecycle = if state.detached {
            OwnedLootAuthorityLifecycle::Detached
        } else if state.quarantined {
            OwnedLootAuthorityLifecycle::Quarantined
        } else if !state.retired {
            OwnedLootAuthorityLifecycle::Active
        } else if state.generation == 0 {
            OwnedLootAuthorityLifecycle::Pristine
        } else {
            OwnedLootAuthorityLifecycle::Retired
        };
        OwnedLootAuthorityStamp {
            lifecycle,
            object_generation: state.generation,
        }
    }
    /// Creates an attached fail-closed tombstone for mirror conflicts. Unlike
    /// a pristine authority, this cannot be initialized through the legacy
    /// first-generation bridge.
    #[must_use]
    pub fn new_retired_tombstone_like_cpp() -> Self {
        let authority = Self::new();
        {
            let mut state = authority.lock_state();
            state.generation = 1;
            state.quarantined = true;
        }
        authority
    }
    /// Permanently invalidates a displaced backing allocation. Retiring alone
    /// is intentionally reversible for respawn/restock; detaching is not.
    pub fn detach_like_cpp(&self) -> u64 {
        let generation = {
            let mut state = self.lock_state();
            if state.detached {
                return state.generation;
            }
            state.generation = state.generation.wrapping_add(1).max(1);
            state.retired = true;
            state.detached = true;
            state.quarantined = false;
            retain_only_persisting_reservations(&mut state);
            finalize_closed_state_if_drained(&mut state);
            state.generation
        };
        self.notify_changed();
        generation
    }
    /// Conditional detach used by entity-local compare/exchange. A stale
    /// observer cannot detach a replacement generation on the same `Arc`.
    pub fn detach_if_stamp_like_cpp(&self, expected: OwnedLootAuthorityStamp) -> bool {
        let detached = {
            let mut state = self.lock_state();
            let current_lifecycle = if state.detached {
                OwnedLootAuthorityLifecycle::Detached
            } else if state.quarantined {
                OwnedLootAuthorityLifecycle::Quarantined
            } else if !state.retired {
                OwnedLootAuthorityLifecycle::Active
            } else if state.generation == 0 {
                OwnedLootAuthorityLifecycle::Pristine
            } else {
                OwnedLootAuthorityLifecycle::Retired
            };
            if current_lifecycle != expected.lifecycle
                || state.generation != expected.object_generation
            {
                return false;
            }
            if state.detached {
                return true;
            }
            state.generation = state.generation.wrapping_add(1).max(1);
            state.retired = true;
            state.detached = true;
            state.quarantined = false;
            retain_only_persisting_reservations(&mut state);
            finalize_closed_state_if_drained(&mut state);
            true
        };
        if detached {
            self.notify_changed();
        }
        detached
    }
    /// Reads lifecycle decisions directly from the authoritative pools.
    #[must_use]
    pub fn is_fully_looted_like_cpp(&self) -> bool {
        let state = self.lock_state();
        state.retired || active_loot_pools_fully_looted_like_cpp(&state)
    }
    /// Waits until every claim that crossed its durable boundary has resolved.
    /// The authority mutex is never held across the await. Disconnect/logout
    /// uses this before C++ `DoLootReleaseAll`, so a cancelled packet waiter
    /// cannot leave a post-commit owner without its release lifecycle.
    pub async fn wait_for_persisting_claims_like_cpp(&self) {
        let mut changed = self.inner.changed.subscribe();
        loop {
            if self.lock_state().persisting.is_empty() {
                return;
            }
            if changed.changed().await.is_err() {
                return;
            }
        }
    }
    /// Runs one map-object lifecycle mutation only while the exact whole-owner
    /// generation and pool topology observed by `close_viewer...` are still
    /// current and fully looted. Callers must acquire the owning map lock first;
    /// the callback must not re-enter this authority.
    pub fn with_fully_looted_lifecycle_observation_like_cpp<R>(
        &self,
        expected_object_generation: u64,
        expected_lifecycle_revision: u64,
        apply_before_unlock: impl FnOnce() -> R,
    ) -> Option<R> {
        let state = self.lock_state();
        if state.retired
            || state.generation != expected_object_generation
            || state.lifecycle_revision != expected_lifecycle_revision
            || !active_loot_pools_fully_looted_like_cpp(&state)
        {
            return None;
        }
        Some(apply_before_unlock())
    }
    /// Runs lifecycle work for a detached durable claim only while every
    /// authoritative `Loot::PlayersLooting` set is empty.  Unlike the normal
    /// `DoLootRelease` path, the worker is not itself closing a client view;
    /// it must therefore leave the object active for any other open viewer.
    /// The no-viewer check belongs in the same critical section as the map
    /// mutation so a concurrent open cannot slip between observation and use.
    pub fn with_unviewed_fully_looted_lifecycle_observation_like_cpp<R>(
        &self,
        expected_object_generation: u64,
        expected_lifecycle_revision: u64,
        apply_before_unlock: impl FnOnce() -> R,
    ) -> Option<R> {
        let state = self.lock_state();
        if state.retired
            || state.generation != expected_object_generation
            || state.lifecycle_revision != expected_lifecycle_revision
            || !active_loot_pools_fully_looted_like_cpp(&state)
            || !active_loot_pools_have_no_viewers_like_cpp(&state)
        {
            return None;
        }
        Some(apply_before_unlock())
    }
    /// Captures the exact fully-looted, globally unviewed generation for
    /// lifecycle work triggered after a detached durable claim commits.
    /// Unlike `close_viewer_if_generation_like_cpp`, this does not mutate
    /// `players_looting`; any open shared or personal view rejects the work.
    #[must_use]
    pub fn fully_looted_unviewed_lifecycle_observation_like_cpp(
        &self,
    ) -> Option<LootFullyLootedLifecycleObservation> {
        let state = self.lock_state();
        if state.retired
            || !active_loot_pools_fully_looted_like_cpp(&state)
            || !active_loot_pools_have_no_viewers_like_cpp(&state)
        {
            return None;
        }
        Some(LootFullyLootedLifecycleObservation {
            object_generation: state.generation,
            lifecycle_revision: state.lifecycle_revision,
            whole_object_fully_skinned: active_loot_pools_fully_skinned_like_cpp(&state),
        })
    }
    /// Installs the first loot generation, or returns the active generation unchanged.
    pub fn initialize_like_cpp(
        &self,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> LootInstallOutcome {
        self.initialize_pristine_like_cpp(shared, personal)
    }
    /// Bridges only a newly constructed object whose authority has never
    /// owned loot. The pristine check and installation share one lock, so a
    /// concurrent retirement cannot be followed by resurrection of a stale
    /// session cache.
    pub fn initialize_pristine_like_cpp(
        &self,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> LootInstallOutcome {
        let outcome = {
            let mut state = self.lock_state();
            if state.detached
                || state.quarantined
                || !state.persisting.is_empty()
                || !state.retired
                || state.generation != 0
            {
                LootInstallOutcome::AlreadyInitialized {
                    generation: any_scope_epoch(&state).unwrap_or(0),
                }
            } else {
                state.generation = 1;
                state.retired = false;
                state.shared = shared;
                state.personal = personal;
                state.reservations.clear();
                state.persisting.clear();
                let epoch = next_scope_epoch(&mut state);
                install_all_scope_epochs(&mut state, epoch);
                bump_lifecycle_revision(&mut state);
                LootInstallOutcome::Installed { generation: epoch }
            }
        };
        if outcome.installed() {
            self.notify_changed();
        }
        outcome
    }
    /// Starts an explicitly generated new object lifetime only if the caller
    /// still observes the same retired lifetime it inspected before any async
    /// loot-template/database work.
    pub fn replace_retired_generation_like_cpp(
        &self,
        expected_object_generation: u64,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> Option<u64> {
        let epoch = {
            let mut state = self.lock_state();
            if state.detached
                || state.quarantined
                || !state.persisting.is_empty()
                || !state.retired
                || state.generation != expected_object_generation
            {
                return None;
            }
            state.generation = state.generation.wrapping_add(1).max(1);
            state.retired = false;
            state.shared = shared;
            state.personal = personal;
            state.reservations.clear();
            state.persisting.clear();
            let epoch = next_scope_epoch(&mut state);
            install_all_scope_epochs(&mut state, epoch);
            bump_lifecycle_revision(&mut state);
            epoch
        };
        self.notify_changed();
        Some(epoch)
    }
    /// Replaces all pools and starts a new generation.
    pub fn replace_like_cpp(
        &self,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> u64 {
        let epoch = {
            let mut state = self.lock_state();
            if state.detached || state.quarantined || !state.persisting.is_empty() {
                return 0;
            }
            state.generation = state.generation.wrapping_add(1).max(1);
            state.retired = false;
            state.shared = shared;
            state.personal = personal;
            state.reservations.clear();
            state.persisting.clear();
            let epoch = next_scope_epoch(&mut state);
            install_all_scope_epochs(&mut state, epoch);
            bump_lifecycle_revision(&mut state);
            epoch
        };
        self.notify_changed();
        epoch
    }
    /// Installs shared loot without replacing already-installed personal pools.
    pub fn initialize_shared_like_cpp(&self, loot: CreatureLoot) -> LootInstallOutcome {
        let (outcome, changed) = {
            let mut state = self.lock_state();
            if state.detached
                || state.quarantined
                || !state.persisting.is_empty()
                || (state.retired && state.generation != 0)
            {
                (
                    LootInstallOutcome::AlreadyInitialized { generation: 0 },
                    false,
                )
            } else if !state.retired && state.shared.is_some() {
                (
                    LootInstallOutcome::AlreadyInitialized {
                        generation: scope_epoch(&state, OwnedLootScope::Shared).unwrap_or(0),
                    },
                    false,
                )
            } else {
                if state.retired {
                    state.generation = state.generation.wrapping_add(1).max(1);
                    state.retired = false;
                    state.personal.clear();
                    state.scope_epochs.clear();
                    state.reservations.clear();
                    state.persisting.clear();
                }
                let epoch = next_scope_epoch(&mut state);
                state.shared = Some(loot);
                state.scope_epochs.insert(OwnedLootScope::Shared, epoch);
                bump_lifecycle_revision(&mut state);
                (LootInstallOutcome::Installed { generation: epoch }, true)
            }
        };
        if changed {
            self.notify_changed();
        }
        outcome
    }
    /// Adds a personal pool to the active generation.
    ///
    /// With `replace == false`, an existing pool is first-writer-wins. Replacing an existing
    /// pool invalidates only that player's leases; independent personal pools keep their claims.
    pub fn upsert_personal_like_cpp(
        &self,
        player: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
    ) -> LootInstallOutcome {
        let (outcome, changed) = {
            let mut state = self.lock_state();
            if state.detached || state.quarantined || (state.retired && state.generation != 0) {
                (
                    LootInstallOutcome::AlreadyInitialized { generation: 0 },
                    false,
                )
            } else if scope_has_persisting_claim(&state, OwnedLootScope::Personal(player))
                || (state.personal.is_empty()
                    && state.shared.is_some()
                    && !state.persisting.is_empty())
            {
                (
                    LootInstallOutcome::AlreadyInitialized {
                        generation: scope_epoch(&state, OwnedLootScope::Personal(player))
                            .or_else(|| scope_epoch(&state, OwnedLootScope::Shared))
                            .unwrap_or(0),
                    },
                    false,
                )
            } else if !state.retired && state.personal.contains_key(&player) && !replace {
                (
                    LootInstallOutcome::AlreadyInitialized {
                        generation: scope_epoch(&state, OwnedLootScope::Personal(player))
                            .unwrap_or(0),
                    },
                    false,
                )
            } else {
                if state.retired {
                    state.generation = state.generation.wrapping_add(1).max(1);
                    state.retired = false;
                    state.shared = None;
                    state.scope_epochs.clear();
                    state.reservations.clear();
                    state.persisting.clear();
                } else if state.personal.is_empty() && state.shared.is_some() {
                    // Switching the C++ owner from shared selection to a
                    // personal-loot map changes the scope for every player.
                    state.generation = state.generation.wrapping_add(1).max(1);
                    state.scope_epochs.remove(&OwnedLootScope::Shared);
                    state.reservations.clear();
                } else if state.personal.contains_key(&player) {
                    // Replacing P1's independent Loot must not invalidate an
                    // in-flight P2 claim. Token removal makes only P1's old
                    // leases stale without advancing the object generation.
                    state.reservations.retain(|key, _| match key {
                        ReservationKey::Item(item) => {
                            item.scope != OwnedLootScope::Personal(player)
                        }
                        ReservationKey::Money(scope) => *scope != OwnedLootScope::Personal(player),
                    });
                }
                let epoch = next_scope_epoch(&mut state);
                state.personal.insert(player, loot);
                state
                    .scope_epochs
                    .insert(OwnedLootScope::Personal(player), epoch);
                bump_lifecycle_revision(&mut state);
                (LootInstallOutcome::Installed { generation: epoch }, true)
            }
        };
        if changed {
            self.notify_changed();
        }
        outcome
    }
    /// Invalidates all leases and removes every pool. Retiring a pristine
    /// allocation advances it to a real tombstone so an async first-generation
    /// installer captured before the clear cannot install afterwards.
    pub fn retire_like_cpp(&self) -> u64 {
        let (generation, changed) = {
            let mut state = self.lock_state();
            if state.retired && state.generation != 0 {
                (state.generation, false)
            } else {
                state.generation = state.generation.wrapping_add(1).max(1);
                state.retired = true;
                retain_only_persisting_reservations(&mut state);
                finalize_closed_state_if_drained(&mut state);
                (state.generation, true)
            }
        };
        if changed {
            self.notify_changed();
        }
        generation
    }
    #[must_use]
    pub fn generation_like_cpp(&self) -> u64 {
        self.lock_state().generation
    }
    #[must_use]
    pub fn scope_generation_like_cpp(&self, scope: OwnedLootScope) -> Option<u64> {
        let state = self.lock_state();
        (!state.retired)
            .then(|| scope_epoch(&state, scope))
            .flatten()
    }
    #[must_use]
    pub fn is_retired_like_cpp(&self) -> bool {
        self.lock_state().retired
    }
    #[must_use]
    pub fn snapshot_for_player_like_cpp(&self, player: ObjectGuid) -> Option<OwnedLootSnapshot> {
        let state = self.lock_state();
        let scope = selected_scope_like_cpp(&state, player)?;
        snapshot_for_scope(&state, scope)
    }
    #[must_use]
    pub fn shared_snapshot_like_cpp(&self) -> Option<OwnedLootSnapshot> {
        let state = self.lock_state();
        snapshot_for_scope(&state, OwnedLootScope::Shared)
    }
    #[must_use]
    pub fn personal_snapshot_like_cpp(&self, player: ObjectGuid) -> Option<OwnedLootSnapshot> {
        let state = self.lock_state();
        snapshot_for_scope(&state, OwnedLootScope::Personal(player))
    }
    #[must_use]
    pub fn personal_snapshots_like_cpp(&self) -> HashMap<ObjectGuid, OwnedLootSnapshot> {
        let state = self.lock_state();
        if state.retired {
            return HashMap::new();
        }
        state
            .personal
            .iter()
            .filter_map(|(player, loot)| {
                let scope = OwnedLootScope::Personal(*player);
                Some((
                    *player,
                    OwnedLootSnapshot {
                        generation: scope_epoch(&state, scope)?,
                        scope,
                        loot: loot.clone(),
                    },
                ))
            })
            .collect()
    }
    pub fn add_viewer_like_cpp(
        &self,
        player: ObjectGuid,
    ) -> Result<LootViewerOpenOutcome, LootClaimError> {
        self.open_view_with_snapshot_like_cpp(player, |_, _| ())
            .map(|(outcome, ())| outcome)
    }
    /// Registers a viewer, snapshots the exact pool, and executes one
    /// synchronous response-enqueue callback before claims may commit again.
    ///
    /// The callback must not re-enter this authority. This explicit critical
    /// section replaces C++'s globally serialized world-session scheduler: a
    /// client cannot receive a stale `LootResponse` while being omitted from
    /// the corresponding removal fanout.
    pub fn open_view_with_snapshot_like_cpp<R>(
        &self,
        player: ObjectGuid,
        observe_before_unlock: impl FnOnce(&OwnedLootSnapshot, &LootViewerOpenOutcome) -> R,
    ) -> Result<(LootViewerOpenOutcome, R), LootClaimError> {
        self.try_open_view_with_snapshot_like_cpp(player, |snapshot, outcome| {
            Some(observe_before_unlock(snapshot, outcome))
        })
    }
    /// Transactionally opens one client view and attempts its response
    /// publication while the authority remains locked.
    ///
    /// C++ writes `LootResponse` before `Loot::OnLootOpened` and processes the
    /// thread-unsafe loot handlers serially (`Player.cpp:8747-8773`,
    /// `Opcodes.cpp:587-590`). Rust sessions are concurrent, so the enqueue
    /// must share this critical section with `players_looting`: a successful
    /// enqueue is ordered before any claim-removal fanout. If the nonblocking
    /// callback rejects publication (for example, a full/disconnected socket
    /// queue), both tentative open mutations are rolled back before unlock.
    ///
    /// The callback must not re-enter this authority or perform blocking work.
    pub fn try_open_view_with_snapshot_like_cpp<R>(
        &self,
        player: ObjectGuid,
        try_observe_before_unlock: impl FnOnce(&OwnedLootSnapshot, &LootViewerOpenOutcome) -> Option<R>,
    ) -> Result<(LootViewerOpenOutcome, R), LootClaimError> {
        let (outcome, observed) = {
            let mut state = self.lock_state();
            if state.retired {
                return Err(LootClaimError::Retired);
            }
            let scope =
                selected_scope_like_cpp(&state, player).ok_or(LootClaimError::NoLootForPlayer)?;
            let generation = scope_epoch(&state, scope).ok_or(LootClaimError::StaleGeneration)?;
            let loot =
                loot_for_scope_mut(&mut state, scope).ok_or(LootClaimError::NoLootForPlayer)?;
            let first_viewer = !loot.looted_by_player;
            if first_viewer {
                loot.looted_by_player = true;
            }
            let inserted = if loot.players_looting.contains(&player) {
                false
            } else {
                loot.players_looting.push(player);
                true
            };
            let outcome = LootViewerOpenOutcome {
                generation,
                scope,
                inserted,
                first_viewer,
            };
            let snapshot = OwnedLootSnapshot {
                generation,
                scope,
                loot: loot.clone(),
            };
            let Some(observed) = try_observe_before_unlock(&snapshot, &outcome) else {
                // No other authority mutation can interleave while the
                // callback runs. Restore exactly the fields tentatively
                // changed by this open and fail closed.
                let loot = loot_for_scope_mut(&mut state, scope)
                    .expect("selected loot scope remains present under its authority lock");
                if inserted {
                    loot.players_looting.retain(|viewer| *viewer != player);
                }
                if first_viewer {
                    loot.looted_by_player = false;
                }
                return Err(LootClaimError::ResponseEnqueueFailed);
            };
            (outcome, observed)
        };
        if outcome.inserted || outcome.first_viewer {
            self.notify_changed();
        }
        Ok((outcome, observed))
    }
    pub fn remove_viewer_like_cpp(&self, player: ObjectGuid) -> bool {
        let removed = {
            let mut state = self.lock_state();
            let Some(scope) = selected_scope_like_cpp(&state, player) else {
                return false;
            };
            let Some(loot) = loot_for_scope_mut(&mut state, scope) else {
                return false;
            };
            let old_len = loot.players_looting.len();
            loot.players_looting.retain(|viewer| *viewer != player);
            old_len != loot.players_looting.len()
        };
        if removed {
            self.notify_changed();
        }
        removed
    }
    /// Removes a viewer only from the exact object lifetime that the session
    /// opened. `None` means the authority was retired or replaced before the
    /// release reached it; callers must then avoid touching replacement
    /// lifecycle state.
    pub fn remove_viewer_if_generation_like_cpp(
        &self,
        expected_generation: u64,
        player: ObjectGuid,
    ) -> Option<bool> {
        let removed = {
            let mut state = self.lock_state();
            if state.retired {
                return None;
            }
            let scope = selected_scope_like_cpp(&state, player)?;
            if scope_epoch(&state, scope) != Some(expected_generation) {
                return None;
            }
            let loot = loot_for_scope_mut(&mut state, scope)?;
            let old_len = loot.players_looting.len();
            loot.players_looting.retain(|viewer| *viewer != player);
            old_len != loot.players_looting.len()
        };
        if removed {
            self.notify_changed();
        }
        Some(removed)
    }
    /// Closes one exact viewer generation and observes both the selected pool
    /// and whole-object completion in the same critical section.
    ///
    /// C++ `WorldSession::DoLootRelease` first tests the selected
    /// `Loot::isLooted()`, then separately gates global creature/gameobject
    /// lifecycle changes on `IsFullyLooted()`, which walks every personal pool.
    pub fn close_viewer_if_generation_like_cpp(
        &self,
        expected_generation: u64,
        player: ObjectGuid,
    ) -> Option<LootViewerCloseOutcome> {
        let outcome = {
            let mut state = self.lock_state();
            if state.retired {
                return None;
            }
            let scope = selected_scope_like_cpp(&state, player)?;
            if scope_epoch(&state, scope) != Some(expected_generation) {
                return None;
            }
            let (removed, snapshot) = {
                let loot = loot_for_scope_mut(&mut state, scope)?;
                let old_len = loot.players_looting.len();
                loot.players_looting.retain(|viewer| *viewer != player);
                (
                    old_len != loot.players_looting.len(),
                    OwnedLootSnapshot {
                        generation: expected_generation,
                        scope,
                        loot: loot.clone(),
                    },
                )
            };
            LootViewerCloseOutcome {
                snapshot,
                removed,
                whole_object_fully_looted: active_loot_pools_fully_looted_like_cpp(&state),
                object_generation: state.generation,
                lifecycle_revision: state.lifecycle_revision,
                whole_object_fully_skinned: active_loot_pools_fully_skinned_like_cpp(&state),
            }
        };
        if outcome.removed {
            self.notify_changed();
        }
        Some(outcome)
    }
    /// Clears C++ `Loot::roundRobinPlayer` on the exact selected pool that the
    /// session opened. A stale release cannot mutate a replacement pool.
    pub fn clear_round_robin_if_generation_like_cpp(
        &self,
        expected_generation: u64,
        player: ObjectGuid,
    ) -> Option<LootRoundRobinReleaseOutcome> {
        let outcome = {
            let mut state = self.lock_state();
            if state.retired {
                return None;
            }
            let scope = selected_scope_like_cpp(&state, player)?;
            if scope_epoch(&state, scope) != Some(expected_generation) {
                return None;
            }
            let loot = loot_for_scope_mut(&mut state, scope)?;
            let cleared = loot.round_robin_player == player;
            if cleared {
                loot.round_robin_player = ObjectGuid::EMPTY;
            }
            LootRoundRobinReleaseOutcome {
                snapshot: OwnedLootSnapshot {
                    generation: expected_generation,
                    scope,
                    loot: loot.clone(),
                },
                cleared,
            }
        };
        if outcome.cleared {
            self.notify_changed();
        }
        Some(outcome)
    }
    #[must_use]
    pub fn viewers_for_player_like_cpp(&self, player: ObjectGuid) -> Vec<ObjectGuid> {
        self.snapshot_for_player_like_cpp(player)
            .map_or_else(Vec::new, |snapshot| snapshot.loot.players_looting)
    }
    /// Publishes the final group-roll state on the object-owned item.
    ///
    /// C++ mutates the same `LootItem` that later reaches `StoreLootItem`: a
    /// one-candidate roll becomes under-threshold/unblocked, while a completed
    /// roll becomes unblocked and records its winner. Session-local packet
    /// views must therefore not be the owner of these fields.
    pub fn finish_item_roll_like_cpp(
        &self,
        player: ObjectGuid,
        expected_generation: u64,
        loot_list_id: u8,
        under_threshold: bool,
        winner: Option<ObjectGuid>,
    ) -> Result<bool, LootClaimError> {
        let changed = {
            let mut state = self.lock_state();
            if state.retired {
                return Err(LootClaimError::Retired);
            }
            let scope =
                selected_scope_like_cpp(&state, player).ok_or(LootClaimError::NoLootForPlayer)?;
            if scope_epoch(&state, scope) != Some(expected_generation) {
                return Err(LootClaimError::StaleGeneration);
            }
            let loot =
                loot_for_scope_mut(&mut state, scope).ok_or(LootClaimError::NoLootForPlayer)?;
            let entry = loot
                .items
                .iter_mut()
                .find(|entry| entry.loot_list_id == loot_list_id)
                .ok_or(LootClaimError::ItemNotFound)?;
            let winner = winner.unwrap_or(ObjectGuid::EMPTY);
            let changed = entry.flags.blocked
                || entry.flags.under_threshold != under_threshold
                || entry.roll_winner != winner;
            entry.flags.blocked = false;
            entry.flags.under_threshold = under_threshold;
            entry.roll_winner = winner;
            changed
        };
        if changed {
            self.notify_changed();
        }
        Ok(changed)
    }
    /// Publishes a winning roll and acquires its item claim in the same
    /// authority critical section. C++ gets this serialization from the world
    /// update thread; Rust session tasks need an explicit atomic boundary so a
    /// respawn cannot land between `LootRoll::Finish` and winner storage.
    pub fn finish_item_roll_and_reserve_award_like_cpp(
        &self,
        scope_player: ObjectGuid,
        expected_generation: u64,
        loot_list_id: u8,
        winner: ObjectGuid,
    ) -> Result<LootClaimLease, LootClaimError> {
        let acquired = {
            let mut state = self.lock_state();
            if state.retired {
                return Err(LootClaimError::Retired);
            }
            let scope = selected_scope_like_cpp(&state, scope_player)
                .ok_or(LootClaimError::NoLootForPlayer)?;
            if selected_scope_like_cpp(&state, winner) != Some(scope) {
                return Err(LootClaimError::NoLootForPlayer);
            }
            if scope_epoch(&state, scope) != Some(expected_generation) {
                return Err(LootClaimError::StaleGeneration);
            }

            let previous = {
                let loot =
                    loot_for_scope_mut(&mut state, scope).ok_or(LootClaimError::NoLootForPlayer)?;
                let entry = loot
                    .items
                    .iter_mut()
                    .find(|entry| entry.loot_list_id == loot_list_id)
                    .ok_or(LootClaimError::ItemNotFound)?;
                let previous = (
                    entry.flags.blocked,
                    entry.flags.under_threshold,
                    entry.roll_winner,
                );
                entry.flags.blocked = false;
                entry.flags.under_threshold = false;
                entry.roll_winner = winner;
                previous
            };

            match reserve_item_once(
                &mut state,
                winner,
                loot_list_id,
                LootItemClaimMode::Award,
                Some(expected_generation),
            ) {
                ReserveAttempt::Acquired {
                    generation,
                    token,
                    key,
                    payload,
                } => Ok((generation, token, key, payload)),
                ReserveAttempt::Wait | ReserveAttempt::Rejected(_) => {
                    if let Some(entry) = loot_for_scope_mut(&mut state, scope).and_then(|loot| {
                        loot.items
                            .iter_mut()
                            .find(|entry| entry.loot_list_id == loot_list_id)
                    }) {
                        entry.flags.blocked = previous.0;
                        entry.flags.under_threshold = previous.1;
                        entry.roll_winner = previous.2;
                    }
                    Err(LootClaimError::ItemAlreadyLooted)
                }
            }
        }?;

        self.notify_changed();
        Ok(LootClaimLease::new(
            self.clone(),
            acquired.0,
            acquired.1,
            acquired.2,
            winner,
            acquired.3,
        ))
    }
    /// Waits until the selected slot is unreserved, then reserves it atomically.
    pub async fn reserve_item_like_cpp(
        &self,
        player: ObjectGuid,
        loot_list_id: u8,
    ) -> Result<LootClaimLease, LootClaimError> {
        self.reserve_item_with_mode_like_cpp(player, loot_list_id, LootItemClaimMode::Direct, None)
            .await
    }
}
