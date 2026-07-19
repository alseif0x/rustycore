// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map-object-owned runtime loot and cancellation-safe claims.
//!
//! TrinityCore owns one shared `Loot` plus an optional GUID-keyed personal-loot map on
//! `Creature` and `GameObject`. `GetLootForPlayer` uses shared loot only while the personal
//! map is empty (`Creature.cpp:1377-1386`, `GameObject.cpp:3898-3907`). The C++ world-session
//! scheduler serializes loot handlers globally. Rust sessions run concurrently, so this module
//! preserves the same single-owner result with short synchronous critical sections and async
//! waiters. No authority lock is held across an `.await`.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::watch;
use wow_core::ObjectGuid;

use crate::LOOT_SLOT_TYPE_OWNER_LIKE_CPP;

/// Server-side loot state for one loot object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureLoot {
    pub loot_guid: ObjectGuid,
    pub coins: u32,
    pub unlooted_count: u8,
    pub loot_type: u8,
    pub dungeon_encounter_id: u32,
    pub loot_method: u8,
    pub loot_master: ObjectGuid,
    pub round_robin_player: ObjectGuid,
    pub player_ffa_items: Vec<(ObjectGuid, Vec<NotNormalLootItem>)>,
    pub players_looting: Vec<ObjectGuid>,
    pub allowed_looters: Vec<ObjectGuid>,
    pub items: Vec<LootEntry>,
    pub looted_by_player: bool,
}

impl CreatureLoot {
    #[must_use]
    pub const fn is_looted_like_cpp(&self) -> bool {
        self.coins == 0 && self.unlooted_count == 0
    }

    #[must_use]
    pub fn item_like_cpp(&self, loot_list_id: u8) -> Option<&LootEntry> {
        self.items
            .iter()
            .find(|entry| entry.loot_list_id == loot_list_id)
    }

    #[must_use]
    pub fn player_has_unlooted_ffa_item_like_cpp(
        &self,
        player: ObjectGuid,
        loot_list_id: u8,
    ) -> bool {
        self.player_ffa_items
            .iter()
            .find(|(looter, _)| *looter == player)
            .is_some_and(|(_, items)| {
                items
                    .iter()
                    .any(|item| item.loot_list_id == loot_list_id && !item.is_looted)
            })
    }

    #[must_use]
    pub fn item_is_looted_for_player_like_cpp(&self, loot_list_id: u8, player: ObjectGuid) -> bool {
        self.item_like_cpp(loot_list_id)
            .is_none_or(|entry| entry.is_looted_for_player_like_cpp(player))
    }

    fn mark_item_looted_for_player_like_cpp(
        &mut self,
        loot_list_id: u8,
        player: ObjectGuid,
    ) -> bool {
        let Some(index) = self
            .items
            .iter()
            .position(|entry| entry.loot_list_id == loot_list_id)
        else {
            return false;
        };

        if self.items[index].is_looted_for_player_like_cpp(player) {
            return false;
        }

        let free_for_all = self.items[index].flags.freeforall;
        self.items[index].mark_looted_for_player_like_cpp(player);
        if free_for_all
            && let Some((_, items)) = self
                .player_ffa_items
                .iter_mut()
                .find(|(looter, _)| *looter == player)
            && let Some(item) = items
                .iter_mut()
                .find(|item| item.loot_list_id == loot_list_id)
        {
            item.is_looted = true;
        }

        self.unlooted_count = self.unlooted_count.saturating_sub(1);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotNormalLootItem {
    pub loot_list_id: u8,
    pub is_looted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootEntry {
    pub loot_list_id: u8,
    pub item_id: u32,
    pub quantity: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub item_context: u8,
    pub flags: LootEntryFlags,
    pub allowed_looters: Vec<ObjectGuid>,
    pub roll_winner: ObjectGuid,
    pub ffa_looted_by: Vec<ObjectGuid>,
    pub taken: bool,
}

impl LootEntry {
    #[must_use]
    pub const fn free_for_all_ui_type_like_cpp(&self) -> u8 {
        LOOT_SLOT_TYPE_OWNER_LIKE_CPP
    }

    #[must_use]
    pub const fn is_over_threshold_like_cpp(&self) -> bool {
        !self.flags.under_threshold && !self.flags.freeforall
    }

    #[must_use]
    pub fn visible_in_represented_free_for_all_view_like_cpp(&self, player: ObjectGuid) -> bool {
        !self.is_looted_for_player_like_cpp(player) && self.has_allowed_looter_like_cpp(player)
    }

    pub fn add_allowed_looter_like_cpp(&mut self, player: ObjectGuid) {
        if !player.is_empty() && !self.allowed_looters.contains(&player) {
            self.allowed_looters.push(player);
        }
    }

    #[must_use]
    pub fn has_allowed_looter_like_cpp(&self, player: ObjectGuid) -> bool {
        self.allowed_looters.contains(&player)
    }

    #[must_use]
    pub fn roll_winner_allows_like_cpp(&self, player: ObjectGuid) -> bool {
        self.roll_winner.is_empty() || self.roll_winner == player
    }

    #[must_use]
    pub fn is_looted_for_player_like_cpp(&self, player: ObjectGuid) -> bool {
        if self.flags.freeforall {
            self.ffa_looted_by.contains(&player)
        } else {
            self.taken
        }
    }

    pub fn mark_looted_for_player_like_cpp(&mut self, player: ObjectGuid) {
        if self.flags.freeforall {
            if !player.is_empty() && !self.ffa_looted_by.contains(&player) {
                self.ffa_looted_by.push(player);
            }
        } else {
            self.taken = true;
        }
    }

    #[must_use]
    pub fn fully_looted_like_cpp(&self) -> bool {
        if self.flags.freeforall {
            !self.allowed_looters.is_empty()
                && self
                    .allowed_looters
                    .iter()
                    .all(|player| self.ffa_looted_by.contains(player))
        } else {
            self.taken
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LootEntryFlags {
    pub follow_loot_rules: bool,
    pub freeforall: bool,
    pub blocked: bool,
    pub counted: bool,
    pub under_threshold: bool,
    pub needs_quest: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnedLootScope {
    Shared,
    Personal(ObjectGuid),
}

/// Observable lifetime state of one object-owned loot authority.
///
/// Keeping this classification behind one mutex acquisition matters during
/// mirror reconciliation: a retired authority from an older object lifetime
/// must not be treated like a newly constructed authority merely because both
/// currently expose no loot pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedLootAuthorityLifecycle {
    Pristine,
    Active,
    Retired,
    /// Conflicting live mirrors were observed for one C++ object. This
    /// attached tombstone is terminal until the object is destroyed.
    Quarantined,
    /// This allocation was displaced from its owning entity mirror. It may
    /// still be held by an async task, but can never own loot again.
    Detached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedLootAuthorityStamp {
    pub lifecycle: OwnedLootAuthorityLifecycle,
    pub object_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedLootSnapshot {
    pub generation: u64,
    pub scope: OwnedLootScope,
    pub loot: CreatureLoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootInstallOutcome {
    Installed { generation: u64 },
    AlreadyInitialized { generation: u64 },
}

impl LootInstallOutcome {
    #[must_use]
    pub const fn generation(self) -> u64 {
        match self {
            Self::Installed { generation } | Self::AlreadyInitialized { generation } => generation,
        }
    }

    #[must_use]
    pub const fn installed(self) -> bool {
        matches!(self, Self::Installed { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootViewerOpenOutcome {
    pub generation: u64,
    pub scope: OwnedLootScope,
    pub inserted: bool,
    pub first_viewer: bool,
}

/// Coherent close-time view of one selected `Loot` and its whole C++ owner.
///
/// `Loot::isLooted()` controls the releasing player's branch, while
/// `Creature::IsFullyLooted()` / `GameObject::IsFullyLooted()` inspect every
/// shared and personal pool before a global lifecycle transition. Keeping both
/// observations under the authority mutex prevents mixing different pool
/// states when concurrent sessions release personal loot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootViewerCloseOutcome {
    pub snapshot: OwnedLootSnapshot,
    pub removed: bool,
    pub whole_object_fully_looted: bool,
    /// Whole-owner generation observed with the selected pool.  Lifecycle
    /// callers must revalidate this together with `lifecycle_revision` before
    /// mutating the map object.
    pub object_generation: u64,
    /// Advances whenever an install, replacement, or personal-pool upsert can
    /// make a previously complete owner incomplete again.
    pub lifecycle_revision: u64,
    /// C++ `Creature::AllLootRemovedFromCorpse` scans the shared pool and every
    /// personal pool rather than inferring skinning from the releasing pool.
    pub whole_object_fully_skinned: bool,
}

/// Whole-owner completion observed without requiring an open client view.
/// Detached durable workers use this after COMMIT when the player already
/// released the window while the claim was still `Persisting`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootFullyLootedLifecycleObservation {
    pub object_generation: u64,
    pub lifecycle_revision: u64,
    pub whole_object_fully_skinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootRoundRobinReleaseOutcome {
    pub snapshot: OwnedLootSnapshot,
    pub cleared: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LootItemClaimKey {
    pub scope: OwnedLootScope,
    pub loot_list_id: u8,
    /// `Some(player)` for FFA items; `None` for globally unique items.
    pub claimant: Option<ObjectGuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LootClaimPayload {
    Item(LootEntry),
    Money(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootClaimError {
    Retired,
    StaleGeneration,
    NoLootForPlayer,
    /// The selected session could not enqueue its opening response.  Rust
    /// must not retain a C++ `PlayersLooting`/`_wasOpened` transition for a
    /// window the client never had a chance to observe.
    ResponseEnqueueFailed,
    ItemNotFound,
    ItemAlreadyLooted,
    PlayerNotAllowed,
    ItemBlocked,
    WrongRollWinner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootClaimCommitError {
    RolledBack,
    StaleGeneration,
    StateChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ReservationKey {
    Item(LootItemClaimKey),
    Money(OwnedLootScope),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthorityState {
    /// Whole-object lifecycle counter retained for first-install/retire
    /// arbitration. Claims use the selected pool's epoch below.
    generation: u64,
    retired: bool,
    detached: bool,
    quarantined: bool,
    shared: Option<CreatureLoot>,
    personal: HashMap<ObjectGuid, CreatureLoot>,
    /// Unique identity of each concrete C++ `Loot` pool. Replacing one
    /// personal pool advances only that scope and leaves peer claims valid.
    scope_epochs: HashMap<OwnedLootScope, u64>,
    next_scope_epoch: u64,
    lifecycle_revision: u64,
    reservations: HashMap<ReservationKey, u64>,
    /// Claims that crossed the durable-persistence boundary. Lifecycle close
    /// may reject every new claim, but must retain these reservations until
    /// their SQL result commits or rolls back.
    persisting: HashMap<ReservationKey, u64>,
    next_token: u64,
}

impl Default for AuthorityState {
    fn default() -> Self {
        Self {
            generation: 0,
            retired: true,
            detached: false,
            quarantined: false,
            shared: None,
            personal: HashMap::new(),
            scope_epochs: HashMap::new(),
            next_scope_epoch: 1,
            lifecycle_revision: 0,
            reservations: HashMap::new(),
            persisting: HashMap::new(),
            next_token: 1,
        }
    }
}

fn retain_only_persisting_reservations(state: &mut AuthorityState) {
    let persisting = &state.persisting;
    state
        .reservations
        .retain(|key, token| persisting.get(key) == Some(token));
}

fn finalize_closed_state_if_drained(state: &mut AuthorityState) {
    if !state.retired || !state.persisting.is_empty() {
        return;
    }
    state.shared = None;
    state.personal.clear();
    state.scope_epochs.clear();
    state.reservations.clear();
}

fn scope_has_persisting_claim(state: &AuthorityState, scope: OwnedLootScope) -> bool {
    state
        .persisting
        .keys()
        .any(|key| reservation_scope(*key) == scope)
}

struct OwnedLootAuthorityInner {
    state: Mutex<AuthorityState>,
    changed: watch::Sender<u64>,
}

/// Shared, object-owned authority for one creature or game-object loot lifetime.
#[derive(Clone)]
pub struct OwnedLootAuthority {
    inner: Arc<OwnedLootAuthorityInner>,
}

impl Default for OwnedLootAuthority {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for OwnedLootAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnedLootAuthority")
            .field("state", &self.state_snapshot())
            .finish()
    }
}

impl PartialEq for OwnedLootAuthority {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner) || self.state_snapshot() == other.state_snapshot()
    }
}

impl Eq for OwnedLootAuthority {}

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

    /// Captures the exact fully-looted generation for lifecycle work that is
    /// triggered after a detached durable claim commits. Unlike
    /// `close_viewer_if_generation_like_cpp`, this does not require or mutate
    /// `players_looting`.
    #[must_use]
    pub fn fully_looted_lifecycle_observation_like_cpp(
        &self,
    ) -> Option<LootFullyLootedLifecycleObservation> {
        let state = self.lock_state();
        if state.retired || !active_loot_pools_fully_looted_like_cpp(&state) {
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

    async fn reserve_item_with_mode_like_cpp(
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

    async fn reserve_money_for_optional_generation_like_cpp(
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

    fn lock_state(&self) -> MutexGuard<'_, AuthorityState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn state_snapshot(&self) -> AuthorityState {
        self.lock_state().clone()
    }

    fn notify_changed(&self) {
        self.inner
            .changed
            .send_modify(|version| *version = version.wrapping_add(1));
    }

    fn begin_claim_persistence(
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

    fn abort_claim_persistence(&self, lease: &LootClaimLeaseInner) -> bool {
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

    fn quarantine_claim_persistence_commit_unknown(&self, lease: &LootClaimLeaseInner) -> bool {
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

    fn commit_claim(
        &self,
        lease: &LootClaimLeaseInner,
        persistence_owner: bool,
    ) -> Result<bool, LootClaimCommitError> {
        self.commit_claim_with_snapshot(lease, persistence_owner)
            .map(|outcome| outcome.first_commit)
    }

    fn commit_claim_with_snapshot(
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

    fn rollback_claim(&self, lease: &LootClaimLeaseInner) -> bool {
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

enum ReserveAttempt {
    Acquired {
        generation: u64,
        token: u64,
        key: ReservationKey,
        payload: LootClaimPayload,
    },
    Wait,
    Rejected(LootClaimError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LootItemClaimMode {
    Direct,
    Award,
}

fn reserve_item_once(
    state: &mut AuthorityState,
    player: ObjectGuid,
    loot_list_id: u8,
    mode: LootItemClaimMode,
    expected_generation: Option<u64>,
) -> ReserveAttempt {
    if state.retired {
        return ReserveAttempt::Rejected(LootClaimError::Retired);
    }
    let Some(scope) = selected_scope_like_cpp(state, player) else {
        return ReserveAttempt::Rejected(LootClaimError::NoLootForPlayer);
    };
    let Some(generation) = scope_epoch(state, scope) else {
        return ReserveAttempt::Rejected(LootClaimError::StaleGeneration);
    };
    if expected_generation.is_some_and(|expected| expected != generation) {
        return ReserveAttempt::Rejected(LootClaimError::StaleGeneration);
    }
    let Some(loot) = loot_for_scope(state, scope) else {
        return ReserveAttempt::Rejected(LootClaimError::NoLootForPlayer);
    };
    let Some(entry) = loot.item_like_cpp(loot_list_id).cloned() else {
        return ReserveAttempt::Rejected(LootClaimError::ItemNotFound);
    };
    if entry.is_looted_for_player_like_cpp(player) {
        return ReserveAttempt::Rejected(LootClaimError::ItemAlreadyLooted);
    }
    if !entry.has_allowed_looter_like_cpp(player) {
        return ReserveAttempt::Rejected(LootClaimError::PlayerNotAllowed);
    }
    if mode == LootItemClaimMode::Direct && entry.flags.blocked {
        return ReserveAttempt::Rejected(LootClaimError::ItemBlocked);
    }
    if !entry.roll_winner_allows_like_cpp(player) {
        return ReserveAttempt::Rejected(LootClaimError::WrongRollWinner);
    }

    let key = ReservationKey::Item(LootItemClaimKey {
        scope,
        loot_list_id,
        claimant: entry.flags.freeforall.then_some(player),
    });
    if state.reservations.contains_key(&key) {
        return ReserveAttempt::Wait;
    }

    let token = next_token(state);
    state.reservations.insert(key, token);
    ReserveAttempt::Acquired {
        generation,
        token,
        key,
        payload: LootClaimPayload::Item(entry),
    }
}

fn reserve_money_once(
    state: &mut AuthorityState,
    player: ObjectGuid,
    expected_generation: Option<u64>,
) -> ReserveAttempt {
    if state.retired {
        return ReserveAttempt::Rejected(LootClaimError::Retired);
    }
    let Some(scope) = selected_scope_like_cpp(state, player) else {
        return ReserveAttempt::Rejected(LootClaimError::NoLootForPlayer);
    };
    let Some(generation) = scope_epoch(state, scope) else {
        return ReserveAttempt::Rejected(LootClaimError::StaleGeneration);
    };
    if expected_generation.is_some_and(|expected| expected != generation) {
        return ReserveAttempt::Rejected(LootClaimError::StaleGeneration);
    }
    let Some(loot) = loot_for_scope(state, scope) else {
        return ReserveAttempt::Rejected(LootClaimError::NoLootForPlayer);
    };
    if !loot.allowed_looters.contains(&player) {
        return ReserveAttempt::Rejected(LootClaimError::PlayerNotAllowed);
    }
    let key = ReservationKey::Money(scope);
    if state.reservations.contains_key(&key) {
        return ReserveAttempt::Wait;
    }

    let payload = LootClaimPayload::Money(loot.coins);
    let token = next_token(state);
    state.reservations.insert(key, token);
    ReserveAttempt::Acquired {
        generation,
        token,
        key,
        payload,
    }
}

fn next_token(state: &mut AuthorityState) -> u64 {
    let token = state.next_token;
    state.next_token = state.next_token.wrapping_add(1).max(1);
    token
}

fn next_scope_epoch(state: &mut AuthorityState) -> u64 {
    let epoch = state.next_scope_epoch;
    state.next_scope_epoch = state.next_scope_epoch.wrapping_add(1).max(1);
    epoch
}

fn bump_lifecycle_revision(state: &mut AuthorityState) {
    // A retired authority must never wrap back to a lifecycle token retained
    // by a detached worker.
    state.lifecycle_revision = state.lifecycle_revision.saturating_add(1).max(1);
}

fn install_all_scope_epochs(state: &mut AuthorityState, epoch: u64) {
    state.scope_epochs.clear();
    if state.shared.is_some() {
        state.scope_epochs.insert(OwnedLootScope::Shared, epoch);
    }
    let players = state.personal.keys().copied().collect::<Vec<_>>();
    for player in players {
        state
            .scope_epochs
            .insert(OwnedLootScope::Personal(player), epoch);
    }
}

fn any_scope_epoch(state: &AuthorityState) -> Option<u64> {
    state.scope_epochs.values().copied().min()
}

fn scope_epoch(state: &AuthorityState, scope: OwnedLootScope) -> Option<u64> {
    state.scope_epochs.get(&scope).copied()
}

const fn reservation_scope(key: ReservationKey) -> OwnedLootScope {
    match key {
        ReservationKey::Item(item) => item.scope,
        ReservationKey::Money(scope) => scope,
    }
}

fn selected_scope_like_cpp(state: &AuthorityState, player: ObjectGuid) -> Option<OwnedLootScope> {
    if state.retired {
        None
    } else if state.personal.is_empty() {
        state.shared.as_ref().map(|_| OwnedLootScope::Shared)
    } else {
        state
            .personal
            .contains_key(&player)
            .then_some(OwnedLootScope::Personal(player))
    }
}

fn active_loot_pools_fully_looted_like_cpp(state: &AuthorityState) -> bool {
    state
        .shared
        .as_ref()
        .is_none_or(CreatureLoot::is_looted_like_cpp)
        && state
            .personal
            .values()
            .all(CreatureLoot::is_looted_like_cpp)
}

fn active_loot_pools_fully_skinned_like_cpp(state: &AuthorityState) -> bool {
    let skinning = wow_constants::LootType::Skinning as u8;
    if state
        .shared
        .as_ref()
        .is_some_and(|loot| loot.loot_type == skinning && loot.is_looted_like_cpp())
    {
        return true;
    }

    let mut has_personal_skinning_loot = false;
    for loot in state.personal.values() {
        if loot.loot_type != skinning {
            continue;
        }
        if !loot.is_looted_like_cpp() {
            return false;
        }
        has_personal_skinning_loot = true;
    }
    has_personal_skinning_loot
}

fn loot_for_scope(state: &AuthorityState, scope: OwnedLootScope) -> Option<&CreatureLoot> {
    match scope {
        OwnedLootScope::Shared => state.shared.as_ref(),
        OwnedLootScope::Personal(player) => state.personal.get(&player),
    }
}

fn loot_for_scope_mut(
    state: &mut AuthorityState,
    scope: OwnedLootScope,
) -> Option<&mut CreatureLoot> {
    match scope {
        OwnedLootScope::Shared => state.shared.as_mut(),
        OwnedLootScope::Personal(player) => state.personal.get_mut(&player),
    }
}

fn snapshot_for_scope(state: &AuthorityState, scope: OwnedLootScope) -> Option<OwnedLootSnapshot> {
    if state.retired {
        return None;
    }
    let generation = scope_epoch(state, scope)?;
    loot_for_scope(state, scope)
        .cloned()
        .map(|loot| OwnedLootSnapshot {
            generation,
            scope,
            loot,
        })
}

const LEASE_ACTIVE: u8 = 0;
const LEASE_COMMITTED: u8 = 1;
const LEASE_ROLLED_BACK: u8 = 2;
const LEASE_QUARANTINED: u8 = 3;

struct LootClaimCommitOutcome {
    first_commit: bool,
    snapshot: Option<OwnedLootSnapshot>,
}

struct LootClaimLeaseInner {
    authority: OwnedLootAuthority,
    generation: u64,
    token: u64,
    key: ReservationKey,
    player: ObjectGuid,
    payload: LootClaimPayload,
    status: AtomicU8,
}

impl Drop for LootClaimLeaseInner {
    fn drop(&mut self) {
        let authority = self.authority.clone();
        authority.rollback_claim(self);
    }
}

/// A cloneable claim lease. Only dropping the final clone rolls an active claim back.
#[derive(Clone)]
pub struct LootClaimLease {
    inner: Arc<LootClaimLeaseInner>,
}

/// RAII owner of the durable phase of one claim. Dropping it before a
/// successful commit is the only operation allowed to reopen a persisting
/// reservation; ordinary clones cannot roll it back concurrently.
pub struct LootClaimPersistenceGuard {
    claim: LootClaimLease,
    resolved: bool,
}

impl fmt::Debug for LootClaimPersistenceGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LootClaimPersistenceGuard")
            .field("claim", &self.claim)
            .field("resolved", &self.resolved)
            .finish()
    }
}

impl LootClaimPersistenceGuard {
    pub fn commit_like_cpp(&mut self) -> Result<bool, LootClaimCommitError> {
        self.commit_with_snapshot_like_cpp()
            .map(|(first_commit, _)| first_commit)
    }

    /// Commits and captures the post-mutation pool under the same authority
    /// mutex. Consumers use this to distinguish viewers that opened before a
    /// money transition from those that opened afterwards and already saw
    /// zero in their response.
    pub fn commit_with_snapshot_like_cpp(
        &mut self,
    ) -> Result<(bool, Option<OwnedLootSnapshot>), LootClaimCommitError> {
        let result = self
            .claim
            .inner
            .authority
            .commit_claim_with_snapshot(&self.claim.inner, true);
        if result.is_ok() {
            self.resolved = true;
        }
        result.map(|outcome| (outcome.first_commit, outcome.snapshot))
    }

    /// Terminal fail-closed outcome for an indeterminate database COMMIT.
    /// The pending claim is removed so shutdown waits can drain, but neither
    /// this lease nor its authority can be reserved or reinitialized until an
    /// external durable reconciliation resolves the ambiguity.
    #[must_use]
    pub fn quarantine_commit_unknown_like_cpp(&mut self) -> bool {
        let quarantined = self
            .claim
            .inner
            .authority
            .quarantine_claim_persistence_commit_unknown(&self.claim.inner);
        if quarantined {
            self.resolved = true;
        }
        quarantined
    }
}

impl Drop for LootClaimPersistenceGuard {
    fn drop(&mut self) {
        if !self.resolved {
            self.claim
                .inner
                .authority
                .abort_claim_persistence(&self.claim.inner);
        }
    }
}

impl fmt::Debug for LootClaimLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LootClaimLease")
            .field("generation", &self.inner.generation)
            .field("player", &self.inner.player)
            .field("payload", &self.inner.payload)
            .field("status", &self.inner.status.load(Ordering::Acquire))
            .finish()
    }
}

impl LootClaimLease {
    fn new(
        authority: OwnedLootAuthority,
        generation: u64,
        token: u64,
        key: ReservationKey,
        player: ObjectGuid,
        payload: LootClaimPayload,
    ) -> Self {
        Self {
            inner: Arc::new(LootClaimLeaseInner {
                authority,
                generation,
                token,
                key,
                player,
                payload,
                status: AtomicU8::new(LEASE_ACTIVE),
            }),
        }
    }

    #[must_use]
    pub fn generation_like_cpp(&self) -> u64 {
        self.inner.generation
    }

    /// Whether this lease was reserved from the same object-owned authority.
    /// Epochs restart in independently allocated authorities, so comparing the
    /// numeric generation alone is vulnerable to an ABA across respawns.
    #[must_use]
    pub fn shares_authority_like_cpp(&self, authority: &OwnedLootAuthority) -> bool {
        self.inner.authority.shares_storage_like_cpp(authority)
    }

    #[must_use]
    pub fn player_like_cpp(&self) -> ObjectGuid {
        self.inner.player
    }

    #[must_use]
    pub fn payload_like_cpp(&self) -> &LootClaimPayload {
        &self.inner.payload
    }

    /// Protects this reservation across the following durable database
    /// operation. Once this succeeds, lifecycle retirement rejects new claims
    /// but keeps this exact lease commit-capable until SQL resolves. The raw
    /// transition is intentionally not exposed: this RAII guard guarantees
    /// cancellation and early-return paths release the reservation.
    pub fn begin_persistence_guard_like_cpp(
        &self,
    ) -> Result<LootClaimPersistenceGuard, LootClaimCommitError> {
        self.inner.authority.begin_claim_persistence(&self.inner)?;
        Ok(LootClaimPersistenceGuard {
            claim: self.clone(),
            resolved: false,
        })
    }

    /// Applies the claim exactly once. Later commits from clones are successful no-ops.
    pub fn commit_like_cpp(&self) -> Result<bool, LootClaimCommitError> {
        self.inner.authority.commit_claim(&self.inner, false)
    }

    /// Applies the claim and captures the exact post-mutation pool while the
    /// authority mutex is still held. Durable fanout must use this cut rather
    /// than sampling the authority after commit, when a later viewer may have
    /// opened a window that already reflects the consumed item or money.
    pub fn commit_with_snapshot_like_cpp(
        &self,
    ) -> Result<(bool, Option<OwnedLootSnapshot>), LootClaimCommitError> {
        self.inner
            .authority
            .commit_claim_with_snapshot(&self.inner, false)
            .map(|outcome| (outcome.first_commit, outcome.snapshot))
    }

    /// Releases an active claim. Later rollback calls are no-ops.
    pub fn rollback_like_cpp(&self) -> bool {
        self.inner.authority.rollback_claim(&self.inner)
    }

    #[must_use]
    pub fn is_committed_like_cpp(&self) -> bool {
        self.inner.status.load(Ordering::Acquire) == LEASE_COMMITTED
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::Barrier;
    use wow_core::ObjectGuid;

    use super::{
        CreatureLoot, LootClaimCommitError, LootClaimError, LootClaimPayload, LootEntry,
        LootEntryFlags, LootInstallOutcome, LootItemClaimMode, NotNormalLootItem,
        OwnedLootAuthority, OwnedLootAuthorityLifecycle, OwnedLootScope, ReserveAttempt,
        reserve_item_once,
    };

    fn player(counter: i64) -> ObjectGuid {
        ObjectGuid::create_player(1, counter)
    }

    fn owner(counter: u32) -> ObjectGuid {
        ObjectGuid::create_creature_like_cpp(1, 1, counter, i64::from(counter))
    }

    fn entry(list_id: u8, free_for_all: bool, allowed: Vec<ObjectGuid>) -> LootEntry {
        LootEntry {
            loot_list_id: list_id,
            item_id: 1000 + u32::from(list_id),
            quantity: 1,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags: LootEntryFlags {
                freeforall: free_for_all,
                counted: true,
                ..LootEntryFlags::default()
            },
            allowed_looters: allowed,
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        }
    }

    fn loot(owner_guid: ObjectGuid, coins: u32, items: Vec<LootEntry>) -> CreatureLoot {
        let unlooted_count = items.len() as u8;
        CreatureLoot {
            loot_guid: owner_guid,
            coins,
            unlooted_count,
            loot_type: 1,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items,
            looted_by_player: false,
        }
    }

    fn money_loot(
        owner_guid: ObjectGuid,
        coins: u32,
        allowed_looters: Vec<ObjectGuid>,
    ) -> CreatureLoot {
        let mut loot = loot(owner_guid, coins, Vec::new());
        loot.allowed_looters = allowed_looters;
        loot
    }

    #[test]
    fn equal_authority_state_does_not_imply_shared_storage_like_cpp() {
        let first = OwnedLootAuthority::new();
        let second = OwnedLootAuthority::new();
        let shared_loot = loot(owner(1), 17, Vec::new());

        first.replace_like_cpp(Some(shared_loot.clone()), HashMap::new());
        second.replace_like_cpp(Some(shared_loot), HashMap::new());

        assert_eq!(first, second, "the independent authority states match");
        assert!(
            !first.shares_storage_like_cpp(&second),
            "equal state must not hide two independently claimable Arc owners"
        );
        assert!(first.shares_storage_like_cpp(&first.clone()));
    }

    #[test]
    fn personal_map_suppresses_shared_pool_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        let mut personal = HashMap::new();
        personal.insert(first, loot(owner(2), 9, Vec::new()));
        authority.replace_like_cpp(Some(loot(owner(1), 5, Vec::new())), personal);

        let first_snapshot = authority
            .snapshot_for_player_like_cpp(first)
            .expect("personal owner has loot");
        assert_eq!(first_snapshot.scope, OwnedLootScope::Personal(first));
        assert_eq!(first_snapshot.loot.coins, 9);
        assert!(authority.snapshot_for_player_like_cpp(second).is_none());
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 5);
    }

    #[test]
    fn initialize_is_first_writer_wins_and_retire_invalidates_generation() {
        let authority = OwnedLootAuthority::new();
        let first =
            authority.initialize_like_cpp(Some(loot(owner(1), 5, Vec::new())), HashMap::new());
        let second =
            authority.initialize_like_cpp(Some(loot(owner(1), 9, Vec::new())), HashMap::new());
        assert!(matches!(first, LootInstallOutcome::Installed { .. }));
        assert_eq!(first.generation(), second.generation());
        assert!(!second.installed());
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 5);

        let retired_generation = authority.retire_like_cpp();
        assert!(retired_generation > first.generation());
        assert!(authority.shared_snapshot_like_cpp().is_none());
        assert_eq!(authority.retire_like_cpp(), retired_generation);
    }

    #[tokio::test]
    async fn normal_item_has_one_winner_and_waiter_observes_commit() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        authority.replace_like_cpp(
            Some(loot(
                owner(1),
                0,
                vec![entry(1, false, vec![first, second])],
            )),
            HashMap::new(),
        );

        let first_lease = authority.reserve_item_like_cpp(first, 1).await.unwrap();
        let waiting_authority = authority.clone();
        let waiter =
            tokio::spawn(async move { waiting_authority.reserve_item_like_cpp(second, 1).await });
        tokio::task::yield_now().await;
        first_lease.commit_like_cpp().unwrap();

        assert_eq!(
            waiter.await.unwrap().unwrap_err(),
            LootClaimError::ItemAlreadyLooted
        );
        assert!(authority.shared_snapshot_like_cpp().unwrap().loot.items[0].taken);
    }

    #[tokio::test]
    async fn item_claim_requires_allowed_looter_and_direct_policy_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(1, false, Vec::new())])),
            HashMap::new(),
        );
        assert_eq!(
            authority
                .reserve_item_like_cpp(looter, 1)
                .await
                .unwrap_err(),
            LootClaimError::PlayerNotAllowed
        );

        let mut blocked = entry(1, false, vec![looter]);
        blocked.flags.blocked = true;
        authority.replace_like_cpp(Some(loot(owner(1), 0, vec![blocked])), HashMap::new());
        assert_eq!(
            authority
                .reserve_item_like_cpp(looter, 1)
                .await
                .unwrap_err(),
            LootClaimError::ItemBlocked
        );
        let award = authority
            .reserve_item_for_award_like_cpp(looter, 1)
            .await
            .expect("completed award may claim the blocked slot");
        assert!(award.commit_like_cpp().unwrap());
        assert!(!award.commit_like_cpp().unwrap(), "commit is idempotent");

        let other = player(2);
        let mut won = entry(2, false, vec![looter, other]);
        won.roll_winner = other;
        authority.replace_like_cpp(Some(loot(owner(1), 0, vec![won])), HashMap::new());
        assert_eq!(
            authority
                .reserve_item_for_award_like_cpp(looter, 2)
                .await
                .unwrap_err(),
            LootClaimError::WrongRollWinner
        );
        authority
            .reserve_item_for_award_like_cpp(other, 2)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap();
    }

    #[tokio::test]
    async fn finished_roll_state_is_published_before_later_direct_claims() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        let mut rolled = entry(1, false, vec![first, second]);
        rolled.flags.blocked = true;
        authority.replace_like_cpp(Some(loot(owner(1), 0, vec![rolled])), HashMap::new());
        let roll_generation = authority.shared_snapshot_like_cpp().unwrap().generation;

        assert!(
            authority
                .finish_item_roll_like_cpp(first, roll_generation, 1, false, Some(second))
                .unwrap()
        );
        assert_eq!(
            authority.reserve_item_like_cpp(first, 1).await.unwrap_err(),
            LootClaimError::WrongRollWinner
        );
        authority
            .reserve_item_like_cpp(second, 1)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap();

        let mut single_candidate = entry(2, false, vec![first]);
        single_candidate.flags.blocked = true;
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![single_candidate])),
            HashMap::new(),
        );
        let roll_generation = authority.shared_snapshot_like_cpp().unwrap().generation;
        authority
            .finish_item_roll_like_cpp(first, roll_generation, 2, true, None)
            .unwrap();
        let snapshot = authority.shared_snapshot_like_cpp().unwrap();
        assert!(snapshot.loot.items[0].flags.under_threshold);
        assert!(!snapshot.loot.items[0].flags.blocked);
        authority
            .reserve_item_like_cpp(first, 2)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap();
    }

    #[tokio::test]
    async fn final_clone_drop_rolls_back_and_wakes_waiter() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
            HashMap::new(),
        );

        let lease = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
        let clone = lease.clone();
        let waiting_authority = authority.clone();
        let waiter =
            tokio::spawn(async move { waiting_authority.reserve_item_like_cpp(looter, 1).await });
        drop(lease);
        tokio::task::yield_now().await;
        assert!(
            !waiter.is_finished(),
            "one clone still owns the reservation"
        );
        drop(clone);

        let retry = waiter.await.unwrap().unwrap();
        assert!(matches!(
            retry.payload_like_cpp(),
            LootClaimPayload::Item(_)
        ));
        retry.commit_like_cpp().unwrap();
    }

    #[tokio::test]
    async fn change_between_busy_check_and_wait_poll_is_not_lost() {
        use std::time::Duration;

        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
            HashMap::new(),
        );
        let first = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
        let mut changed = authority.inner.changed.subscribe();
        let busy = {
            let mut state = authority.lock_state();
            reserve_item_once(&mut state, looter, 1, LootItemClaimMode::Direct, None)
        };
        assert!(matches!(busy, ReserveAttempt::Wait));

        // This transition is deliberately between the locked Busy result and the first poll of
        // `changed()`. A watch version remembers it; Notify::notify_waiters would lose it.
        first.rollback_like_cpp();
        tokio::time::timeout(Duration::from_secs(1), changed.changed())
            .await
            .expect("reservation wake must not be lost")
            .unwrap();
        let retry = {
            let mut state = authority.lock_state();
            reserve_item_once(&mut state, looter, 1, LootItemClaimMode::Direct, None)
        };
        assert!(matches!(retry, ReserveAttempt::Acquired { .. }));
        authority.retire_like_cpp();
    }

    #[tokio::test]
    async fn ffa_item_is_reserved_and_consumed_once_per_player() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        let mut shared = loot(owner(1), 0, vec![entry(1, true, vec![first, second])]);
        shared.unlooted_count = 2;
        shared.player_ffa_items = vec![
            (
                first,
                vec![NotNormalLootItem {
                    loot_list_id: 1,
                    is_looted: false,
                }],
            ),
            (
                second,
                vec![NotNormalLootItem {
                    loot_list_id: 1,
                    is_looted: false,
                }],
            ),
        ];
        authority.replace_like_cpp(Some(shared), HashMap::new());

        let barrier = Arc::new(Barrier::new(3));
        let mut tasks = Vec::new();
        for looter in [first, second] {
            let authority = authority.clone();
            let barrier = barrier.clone();
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                let lease = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
                lease.commit_like_cpp().unwrap();
            }));
        }
        barrier.wait().await;
        for task in tasks {
            task.await.unwrap();
        }

        let shared = authority.shared_snapshot_like_cpp().unwrap().loot;
        assert_eq!(shared.unlooted_count, 0);
        assert!(shared.items[0].fully_looted_like_cpp());
        assert_eq!(
            authority.reserve_item_like_cpp(first, 1).await.unwrap_err(),
            LootClaimError::ItemAlreadyLooted
        );
        assert_eq!(
            authority
                .reserve_item_like_cpp(second, 1)
                .await
                .unwrap_err(),
            LootClaimError::ItemAlreadyLooted
        );
    }

    #[tokio::test]
    async fn personal_claims_and_ae_owners_are_independent() {
        let first = player(1);
        let second = player(2);
        let authority = OwnedLootAuthority::new();
        let mut personal = HashMap::new();
        personal.insert(first, loot(owner(1), 0, vec![entry(1, false, vec![first])]));
        personal.insert(
            second,
            loot(owner(1), 0, vec![entry(1, false, vec![second])]),
        );
        authority.replace_like_cpp(None, personal);

        let first_lease = authority.reserve_item_like_cpp(first, 1).await.unwrap();
        let second_lease = authority.reserve_item_like_cpp(second, 1).await.unwrap();
        first_lease.commit_like_cpp().unwrap();
        second_lease.commit_like_cpp().unwrap();

        let first_ae = OwnedLootAuthority::new();
        let second_ae = OwnedLootAuthority::new();
        first_ae.replace_like_cpp(Some(money_loot(owner(10), 7, vec![first])), HashMap::new());
        second_ae.replace_like_cpp(
            Some(money_loot(owner(11), 11, vec![second])),
            HashMap::new(),
        );
        first_ae
            .reserve_money_like_cpp(first)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap();
        assert_eq!(first_ae.shared_snapshot_like_cpp().unwrap().loot.coins, 0);
        assert_eq!(second_ae.shared_snapshot_like_cpp().unwrap().loot.coins, 11);
    }

    #[tokio::test]
    async fn stale_lease_cannot_touch_replacement_generation() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(Some(money_loot(owner(1), 5, vec![looter])), HashMap::new());
        let stale = authority.reserve_money_like_cpp(looter).await.unwrap();

        let retired_generation = authority.retire_like_cpp();
        authority
            .replace_retired_generation_like_cpp(
                retired_generation,
                Some(money_loot(owner(1), 9, vec![looter])),
                HashMap::new(),
            )
            .unwrap();
        assert!(stale.commit_like_cpp().is_err());
        drop(stale);
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 9);
    }

    #[tokio::test]
    async fn allowed_player_can_reserve_and_commit_money_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(Some(money_loot(owner(1), 17, vec![looter])), HashMap::new());

        let claim = authority.reserve_money_like_cpp(looter).await.unwrap();
        assert_eq!(claim.payload_like_cpp(), &LootClaimPayload::Money(17));
        assert!(claim.commit_like_cpp().unwrap());
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 0);
    }

    #[tokio::test]
    async fn money_claim_rejects_player_not_in_allowed_looters_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let allowed = player(1);
        let denied = player(2);
        authority.replace_like_cpp(
            Some(money_loot(owner(1), 19, vec![allowed])),
            HashMap::new(),
        );

        assert_eq!(
            authority.reserve_money_like_cpp(denied).await.unwrap_err(),
            LootClaimError::PlayerNotAllowed
        );
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 19);
    }

    #[tokio::test]
    async fn stale_player_cannot_claim_replacement_generation_money_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let stale_looter = player(1);
        let current_looter = player(2);
        authority.replace_like_cpp(
            Some(money_loot(owner(1), 5, vec![stale_looter])),
            HashMap::new(),
        );
        let stale_generation = authority.generation_like_cpp();

        let retired_generation = authority.retire_like_cpp();
        authority
            .replace_retired_generation_like_cpp(
                retired_generation,
                Some(money_loot(owner(1), 23, vec![current_looter])),
                HashMap::new(),
            )
            .unwrap();
        assert!(authority.generation_like_cpp() > stale_generation);
        assert_eq!(
            authority
                .reserve_money_like_cpp(stale_looter)
                .await
                .unwrap_err(),
            LootClaimError::PlayerNotAllowed
        );
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 23);

        let current_claim = authority
            .reserve_money_like_cpp(current_looter)
            .await
            .unwrap();
        assert!(current_claim.commit_like_cpp().unwrap());
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 0);
    }

    #[tokio::test]
    async fn money_claim_rolls_back_and_later_cpp_request_observes_zero() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        authority.replace_like_cpp(
            Some(money_loot(owner(1), 25, vec![first, second])),
            HashMap::new(),
        );

        let abandoned = authority.reserve_money_like_cpp(first).await.unwrap();
        assert!(abandoned.rollback_like_cpp());
        let winner = authority.reserve_money_like_cpp(second).await.unwrap();
        assert_eq!(winner.payload_like_cpp(), &LootClaimPayload::Money(25));
        assert!(winner.commit_like_cpp().unwrap());
        assert!(!winner.commit_like_cpp().unwrap());
        let zero = authority.reserve_money_like_cpp(first).await.unwrap();
        assert_eq!(zero.payload_like_cpp(), &LootClaimPayload::Money(0));
        assert!(zero.commit_like_cpp().unwrap());
    }

    #[tokio::test]
    async fn concurrent_money_waiter_observes_the_single_committed_winner() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        authority.replace_like_cpp(
            Some(money_loot(owner(1), 31, vec![first, second])),
            HashMap::new(),
        );

        let winner = authority.reserve_money_like_cpp(first).await.unwrap();
        let waiting_authority = authority.clone();
        let waiter =
            tokio::spawn(async move { waiting_authority.reserve_money_like_cpp(second).await });
        tokio::task::yield_now().await;
        assert!(winner.commit_like_cpp().unwrap());
        let zero = waiter.await.unwrap().unwrap();
        assert_eq!(zero.payload_like_cpp(), &LootClaimPayload::Money(0));
        assert!(zero.commit_like_cpp().unwrap());
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 0);
    }

    #[test]
    fn personal_pools_can_be_installed_incrementally_without_replacing_peers() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        authority.upsert_personal_like_cpp(first, loot(owner(1), 7, Vec::new()), false);
        let generation = authority.generation_like_cpp();
        authority.upsert_personal_like_cpp(second, loot(owner(1), 11, Vec::new()), false);

        assert_eq!(authority.generation_like_cpp(), generation);
        assert_eq!(
            authority
                .personal_snapshot_like_cpp(first)
                .unwrap()
                .loot
                .coins,
            7
        );
        assert_eq!(
            authority
                .personal_snapshot_like_cpp(second)
                .unwrap()
                .loot
                .coins,
            11
        );
        let duplicate =
            authority.upsert_personal_like_cpp(first, loot(owner(1), 99, Vec::new()), false);
        assert!(!duplicate.installed());
        assert_eq!(
            authority
                .personal_snapshot_like_cpp(first)
                .unwrap()
                .loot
                .coins,
            7
        );
    }

    #[tokio::test]
    async fn replacing_one_personal_pool_does_not_stale_another_players_claim() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        let mut first_loot = loot(owner(1), 0, vec![entry(1, false, vec![first])]);
        first_loot.allowed_looters = vec![first];
        authority.upsert_personal_like_cpp(first, first_loot, false);
        let mut second_loot = loot(owner(1), 0, vec![entry(1, false, vec![second])]);
        second_loot.allowed_looters = vec![second];
        authority.upsert_personal_like_cpp(second, second_loot, false);

        let stale_first_item = authority.reserve_item_like_cpp(first, 1).await.unwrap();
        let stale_first_money = authority.reserve_money_like_cpp(first).await.unwrap();
        let second_claim = authority.reserve_item_like_cpp(second, 1).await.unwrap();
        let first_epoch = authority
            .personal_snapshot_like_cpp(first)
            .unwrap()
            .generation;
        let second_epoch = authority
            .personal_snapshot_like_cpp(second)
            .unwrap()
            .generation;
        let generation = authority.generation_like_cpp();
        let mut replacement = loot(owner(1), 9, vec![entry(2, false, vec![first])]);
        replacement.allowed_looters = vec![first];
        authority.upsert_personal_like_cpp(first, replacement, true);

        assert_eq!(authority.generation_like_cpp(), generation);
        assert_ne!(
            authority
                .personal_snapshot_like_cpp(first)
                .unwrap()
                .generation,
            first_epoch
        );
        assert_eq!(
            authority
                .personal_snapshot_like_cpp(second)
                .unwrap()
                .generation,
            second_epoch
        );
        assert_eq!(
            stale_first_item.commit_like_cpp(),
            Err(LootClaimCommitError::StaleGeneration)
        );
        assert_eq!(
            stale_first_money.commit_like_cpp(),
            Err(LootClaimCommitError::StaleGeneration)
        );
        assert!(second_claim.commit_like_cpp().unwrap());
        assert!(
            authority
                .personal_snapshot_like_cpp(second)
                .unwrap()
                .loot
                .items[0]
                .taken
        );
        assert_eq!(
            authority
                .personal_snapshot_like_cpp(first)
                .unwrap()
                .loot
                .coins,
            9
        );
        assert!(
            authority
                .reserve_item_like_cpp(first, 2)
                .await
                .unwrap()
                .commit_like_cpp()
                .unwrap()
        );
        assert!(
            authority
                .reserve_money_like_cpp(first)
                .await
                .unwrap()
                .commit_like_cpp()
                .unwrap()
        );
    }

    #[tokio::test]
    async fn stale_personal_waiter_rejects_replacement_epoch_without_reserving_it() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.upsert_personal_like_cpp(
            looter,
            loot(owner(1), 0, vec![entry(1, false, vec![looter])]),
            false,
        );
        let old_epoch = authority
            .personal_snapshot_like_cpp(looter)
            .unwrap()
            .generation;
        let held = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
        let waiting_authority = authority.clone();
        let waiter = tokio::spawn(async move {
            waiting_authority
                .reserve_item_for_generation_like_cpp(looter, 1, old_epoch)
                .await
        });
        tokio::task::yield_now().await;

        authority.upsert_personal_like_cpp(
            looter,
            loot(owner(1), 0, vec![entry(1, false, vec![looter])]),
            true,
        );
        assert_eq!(
            waiter.await.unwrap().unwrap_err(),
            LootClaimError::StaleGeneration
        );
        drop(held);
        assert!(
            authority
                .reserve_item_like_cpp(looter, 1)
                .await
                .unwrap()
                .commit_like_cpp()
                .unwrap()
        );
    }

    #[test]
    fn pristine_bridge_cannot_resurrect_a_retired_nonzero_generation() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        let first = authority.initialize_pristine_like_cpp(
            Some(money_loot(owner(1), 5, vec![looter])),
            HashMap::new(),
        );
        assert!(first.installed());
        authority.retire_like_cpp();

        let stale = authority.initialize_pristine_like_cpp(
            Some(money_loot(owner(1), 99, vec![looter])),
            HashMap::new(),
        );
        assert!(!stale.installed());
        assert!(
            !authority
                .initialize_like_cpp(Some(money_loot(owner(1), 98, vec![looter])), HashMap::new(),)
                .installed()
        );
        assert!(
            !authority
                .initialize_shared_like_cpp(money_loot(owner(1), 97, vec![looter]))
                .installed()
        );
        assert!(
            !authority
                .upsert_personal_like_cpp(looter, money_loot(owner(1), 96, vec![looter]), true,)
                .installed()
        );
        assert!(authority.is_retired_like_cpp());
        assert!(authority.shared_snapshot_like_cpp().is_none());
    }

    #[test]
    fn viewer_first_open_is_atomic_and_retire_clears_it() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        authority.replace_like_cpp(Some(loot(owner(1), 1, Vec::new())), HashMap::new());

        let first_open = authority.add_viewer_like_cpp(first).unwrap();
        let duplicate = authority.add_viewer_like_cpp(first).unwrap();
        let second_open = authority.add_viewer_like_cpp(second).unwrap();
        assert!(first_open.first_viewer);
        assert!(!duplicate.inserted);
        assert!(!second_open.first_viewer);
        assert_eq!(
            authority.viewers_for_player_like_cpp(first),
            vec![first, second]
        );
        assert!(authority.remove_viewer_like_cpp(first));
        assert!(!authority.remove_viewer_like_cpp(first));
        assert!(authority.remove_viewer_like_cpp(second));
        let reopened = authority.add_viewer_like_cpp(first).unwrap();
        assert!(reopened.inserted);
        assert!(
            !reopened.first_viewer,
            "C++ Loot::_wasOpened survives an empty active-looter set"
        );
        authority.retire_like_cpp();
        assert!(authority.viewers_for_player_like_cpp(second).is_empty());
    }

    #[test]
    fn rejected_view_response_rolls_back_viewer_and_first_open_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let viewer = player(1);
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(1, false, vec![viewer])])),
            HashMap::new(),
        );

        assert_eq!(
            authority.try_open_view_with_snapshot_like_cpp(viewer, |_, _| None::<()>),
            Err(LootClaimError::ResponseEnqueueFailed)
        );
        let rejected = authority.shared_snapshot_like_cpp().unwrap();
        assert!(rejected.loot.players_looting.is_empty());
        assert!(!rejected.loot.looted_by_player);

        let retry = authority.add_viewer_like_cpp(viewer).unwrap();
        assert!(retry.inserted);
        assert!(
            retry.first_viewer,
            "a response the client never observed must not consume C++ Loot::_wasOpened"
        );
    }

    #[test]
    fn exact_generation_release_removes_only_that_viewer() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        let generation = authority.replace_like_cpp(
            Some(money_loot(owner(1), 17, vec![first, second])),
            HashMap::new(),
        );
        authority.add_viewer_like_cpp(first).unwrap();
        authority.add_viewer_like_cpp(second).unwrap();

        assert_eq!(
            authority.remove_viewer_if_generation_like_cpp(generation, first),
            Some(true)
        );
        assert_eq!(authority.viewers_for_player_like_cpp(second), vec![second]);
        assert_eq!(
            authority.remove_viewer_if_generation_like_cpp(generation, first),
            Some(false)
        );
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 17);
    }

    #[test]
    fn retired_generation_release_cannot_touch_replacement_viewer_or_pool() {
        let authority = OwnedLootAuthority::new();
        let viewer = player(1);
        let old_generation =
            authority.replace_like_cpp(Some(money_loot(owner(1), 7, vec![viewer])), HashMap::new());
        authority.add_viewer_like_cpp(viewer).unwrap();

        authority.retire_like_cpp();
        let replacement_generation = authority
            .replace_like_cpp(Some(money_loot(owner(1), 29, vec![viewer])), HashMap::new());
        authority.add_viewer_like_cpp(viewer).unwrap();

        assert_ne!(replacement_generation, old_generation);
        assert_eq!(
            authority.remove_viewer_if_generation_like_cpp(old_generation, viewer),
            None
        );
        let replacement = authority.shared_snapshot_like_cpp().unwrap();
        assert_eq!(replacement.generation, replacement_generation);
        assert_eq!(replacement.loot.coins, 29);
        assert_eq!(replacement.loot.players_looting, vec![viewer]);
        assert_eq!(replacement.loot.allowed_looters, vec![viewer]);
    }

    #[tokio::test]
    async fn persistence_guard_survives_retire_and_closes_after_commit_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
            HashMap::new(),
        );
        let claim = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
        let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();

        let retired_generation = authority.retire_like_cpp();
        assert_eq!(
            authority.lifecycle_like_cpp(),
            OwnedLootAuthorityLifecycle::Retired
        );
        assert_eq!(
            claim.commit_like_cpp(),
            Err(LootClaimCommitError::StateChanged),
            "only the durable-phase owner may resolve a protected reservation"
        );
        assert!(!claim.rollback_like_cpp());
        assert!(persistence.commit_like_cpp().unwrap());

        assert!(authority.is_retired_like_cpp());
        assert!(authority.shared_snapshot_like_cpp().is_none());
        assert!(
            authority
                .replace_retired_generation_like_cpp(
                    retired_generation,
                    Some(money_loot(owner(1), 9, vec![looter])),
                    HashMap::new(),
                )
                .is_some(),
            "the closed lifetime may respawn only after durable completion"
        );
    }

    #[tokio::test]
    async fn persistence_guard_survives_detach_but_detached_owner_never_reopens_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
            HashMap::new(),
        );
        let claim = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
        let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();

        let detached_generation = authority.detach_like_cpp();
        assert_eq!(
            authority.lifecycle_like_cpp(),
            OwnedLootAuthorityLifecycle::Detached
        );
        assert!(persistence.commit_like_cpp().unwrap());
        assert_eq!(
            authority
                .replace_like_cpp(Some(money_loot(owner(1), 9, vec![looter])), HashMap::new(),),
            0
        );
        assert!(
            authority
                .replace_retired_generation_like_cpp(
                    detached_generation,
                    Some(money_loot(owner(1), 9, vec![looter])),
                    HashMap::new(),
                )
                .is_none()
        );
    }

    #[tokio::test]
    async fn persistence_guard_is_unique_and_external_clones_cannot_resolve_it_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(Some(money_loot(owner(1), 17, vec![looter])), HashMap::new());
        let claim = authority.reserve_money_like_cpp(looter).await.unwrap();
        let clone = claim.clone();
        let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();

        assert_eq!(
            clone.begin_persistence_guard_like_cpp().unwrap_err(),
            LootClaimCommitError::StateChanged
        );
        assert_eq!(
            clone.commit_like_cpp(),
            Err(LootClaimCommitError::StateChanged)
        );
        assert!(!clone.rollback_like_cpp());
        assert!(persistence.commit_like_cpp().unwrap());
        assert_eq!(
            claim.begin_persistence_guard_like_cpp().unwrap_err(),
            LootClaimCommitError::StateChanged
        );
    }

    #[tokio::test]
    async fn dropped_persistence_guard_reopens_claim_after_failure_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
            HashMap::new(),
        );
        let failed = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
        let persistence = failed.begin_persistence_guard_like_cpp().unwrap();
        drop(persistence);

        assert_eq!(
            failed.commit_like_cpp(),
            Err(LootClaimCommitError::RolledBack)
        );
        let retry = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
        assert!(retry.commit_like_cpp().unwrap());
    }

    #[tokio::test]
    async fn persistence_commit_snapshot_excludes_viewers_opened_after_money_transition_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let late = player(2);
        authority.replace_like_cpp(
            Some(money_loot(owner(1), 17, vec![first, late])),
            HashMap::new(),
        );
        authority.add_viewer_like_cpp(first).unwrap();

        let claim = authority.reserve_money_like_cpp(first).await.unwrap();
        let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();
        let (first_commit, committed_snapshot) =
            persistence.commit_with_snapshot_like_cpp().unwrap();
        assert!(first_commit);
        assert_eq!(
            committed_snapshot.unwrap().loot.players_looting,
            vec![first],
            "the durable fanout snapshot is captured at the same serialized transition as coins=0"
        );

        authority.add_viewer_like_cpp(late).unwrap();
        assert_eq!(
            authority
                .shared_snapshot_like_cpp()
                .unwrap()
                .loot
                .players_looting,
            vec![first, late],
            "a later opener observes zero directly and is not retroactively part of the commit fanout"
        );
    }

    #[tokio::test]
    async fn persistence_commit_snapshot_excludes_viewers_opened_after_item_transition_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let late = player(2);
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(1, false, vec![first, late])])),
            HashMap::new(),
        );
        authority.add_viewer_like_cpp(first).unwrap();

        let claim = authority.reserve_item_like_cpp(first, 1).await.unwrap();
        let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();
        let (first_commit, committed_snapshot) =
            persistence.commit_with_snapshot_like_cpp().unwrap();
        assert!(first_commit);
        let committed_snapshot = committed_snapshot.unwrap();
        assert_eq!(committed_snapshot.loot.players_looting, vec![first]);
        assert!(committed_snapshot.loot.items[0].taken);

        let (_, late_snapshot) = authority
            .open_view_with_snapshot_like_cpp(late, |snapshot, _| snapshot.clone())
            .unwrap();
        assert_eq!(late_snapshot.loot.players_looting, vec![first, late]);
        assert!(late_snapshot.loot.items[0].taken);
        assert_eq!(
            committed_snapshot.loot.players_looting,
            vec![first],
            "a viewer that first observes the consumed item is outside the commit fanout cut"
        );
    }

    #[tokio::test]
    async fn one_personal_persistence_does_not_block_peer_scope_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        authority.upsert_personal_like_cpp(
            first,
            loot(owner(1), 0, vec![entry(1, false, vec![first])]),
            false,
        );
        let first_claim = authority.reserve_item_like_cpp(first, 1).await.unwrap();
        let mut first_persistence = first_claim.begin_persistence_guard_like_cpp().unwrap();

        assert!(
            authority
                .upsert_personal_like_cpp(
                    second,
                    loot(owner(1), 0, vec![entry(1, false, vec![second])]),
                    false,
                )
                .installed(),
            "P1 durable work must not serialize an independent P2 pool"
        );
        let second_claim = authority.reserve_item_like_cpp(second, 1).await.unwrap();
        assert!(second_claim.commit_like_cpp().unwrap());

        assert!(
            !authority
                .upsert_personal_like_cpp(
                    first,
                    loot(owner(1), 0, vec![entry(2, false, vec![first])]),
                    true,
                )
                .installed(),
            "the exact persisting P1 pool cannot be replaced"
        );
        assert!(first_persistence.commit_like_cpp().unwrap());
    }

    #[test]
    fn lifecycle_observation_is_invalidated_by_late_personal_upsert_like_cpp() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let late = player(2);
        authority.replace_like_cpp(Some(loot(owner(1), 0, vec![])), HashMap::new());
        authority.add_viewer_like_cpp(first).unwrap();
        let generation = authority
            .snapshot_for_player_like_cpp(first)
            .unwrap()
            .generation;
        let close = authority
            .close_viewer_if_generation_like_cpp(generation, first)
            .unwrap();
        assert!(close.whole_object_fully_looted);

        let applications = AtomicUsize::new(0);
        assert!(
            authority
                .with_fully_looted_lifecycle_observation_like_cpp(
                    close.object_generation,
                    close.lifecycle_revision,
                    || applications.fetch_add(1, Ordering::SeqCst),
                )
                .is_some()
        );
        assert!(
            authority
                .upsert_personal_like_cpp(
                    late,
                    loot(owner(1), 0, vec![entry(1, false, vec![late])]),
                    false,
                )
                .installed()
        );
        assert!(
            authority
                .with_fully_looted_lifecycle_observation_like_cpp(
                    close.object_generation,
                    close.lifecycle_revision,
                    || applications.fetch_add(1, Ordering::SeqCst),
                )
                .is_none(),
            "an upsert after close must invalidate the pre-upsert lifecycle observation"
        );
        assert_eq!(applications.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn round_robin_clear_is_generation_guarded_and_returns_authoritative_snapshot_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        let mut shared = loot(owner(1), 0, vec![entry(1, false, vec![first, second])]);
        shared.round_robin_player = first;
        authority.replace_like_cpp(Some(shared), HashMap::new());
        let generation = authority
            .snapshot_for_player_like_cpp(first)
            .unwrap()
            .generation;

        let cleared = authority
            .clear_round_robin_if_generation_like_cpp(generation, first)
            .unwrap();
        assert!(cleared.cleared);
        assert!(cleared.snapshot.loot.round_robin_player.is_empty());

        let mut replacement = loot(owner(1), 0, vec![entry(1, false, vec![first, second])]);
        replacement.round_robin_player = second;
        authority.replace_like_cpp(Some(replacement), HashMap::new());
        assert!(
            authority
                .clear_round_robin_if_generation_like_cpp(generation, second)
                .is_none(),
            "a stale release cannot clear round robin on a replacement pool"
        );
        assert_eq!(
            authority
                .snapshot_for_player_like_cpp(second)
                .unwrap()
                .loot
                .round_robin_player,
            second
        );
    }

    #[tokio::test]
    async fn wait_for_persisting_claims_observes_commit_and_failure_boundaries_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
            HashMap::new(),
        );
        let claim = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
        let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();
        let waiting = authority.clone();
        let waiter = tokio::spawn(async move {
            waiting.wait_for_persisting_claims_like_cpp().await;
        });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        assert!(persistence.commit_like_cpp().unwrap());
        waiter.await.unwrap();

        authority.replace_like_cpp(
            Some(loot(owner(1), 0, vec![entry(2, false, vec![looter])])),
            HashMap::new(),
        );
        let failed = authority.reserve_item_like_cpp(looter, 2).await.unwrap();
        let persistence = failed.begin_persistence_guard_like_cpp().unwrap();
        drop(persistence);
        authority.wait_for_persisting_claims_like_cpp().await;
        assert!(authority.reserve_item_like_cpp(looter, 2).await.is_ok());
    }

    #[tokio::test]
    async fn commit_unknown_quarantine_is_terminal_fail_closed_and_drains_waiters_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let looter = player(1);
        let original = loot(owner(1), 0, vec![entry(1, false, vec![looter])]);
        authority.replace_like_cpp(Some(original.clone()), HashMap::new());
        let claim = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
        let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();
        let waiting = authority.clone();
        let waiter = tokio::spawn(async move {
            waiting.wait_for_persisting_claims_like_cpp().await;
        });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        assert!(persistence.quarantine_commit_unknown_like_cpp());
        drop(persistence);
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("quarantine removes the persisting token")
            .unwrap();

        assert_eq!(
            authority.lifecycle_like_cpp(),
            OwnedLootAuthorityLifecycle::Quarantined
        );
        assert_eq!(
            claim.commit_like_cpp(),
            Err(LootClaimCommitError::StateChanged)
        );
        assert!(matches!(
            authority.reserve_item_like_cpp(looter, 1).await,
            Err(LootClaimError::Retired)
        ));
        assert_eq!(
            authority.replace_like_cpp(Some(original.clone()), HashMap::new()),
            0
        );
        assert!(
            !authority
                .initialize_pristine_like_cpp(Some(original), HashMap::new())
                .installed()
        );
    }

    #[tokio::test]
    async fn whole_object_skinned_requires_every_active_skinning_pool_like_cpp() {
        let authority = OwnedLootAuthority::new();
        let first = player(1);
        let second = player(2);
        let mut first_pool = loot(owner(1), 0, vec![]);
        first_pool.loot_type = wow_constants::LootType::Skinning as u8;
        let mut second_pool = loot(owner(1), 0, vec![entry(1, false, vec![second])]);
        second_pool.loot_type = wow_constants::LootType::Skinning as u8;
        authority.replace_like_cpp(
            None,
            [(first, first_pool), (second, second_pool)]
                .into_iter()
                .collect(),
        );
        authority.add_viewer_like_cpp(first).unwrap();
        let first_generation = authority
            .snapshot_for_player_like_cpp(first)
            .unwrap()
            .generation;
        let first_close = authority
            .close_viewer_if_generation_like_cpp(first_generation, first)
            .unwrap();
        assert!(first_close.snapshot.loot.is_looted_like_cpp());
        assert!(!first_close.whole_object_fully_looted);
        assert!(!first_close.whole_object_fully_skinned);

        authority
            .reserve_item_like_cpp(second, 1)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap();
        authority.add_viewer_like_cpp(second).unwrap();
        let second_generation = authority
            .snapshot_for_player_like_cpp(second)
            .unwrap()
            .generation;
        let second_close = authority
            .close_viewer_if_generation_like_cpp(second_generation, second)
            .unwrap();
        assert!(second_close.whole_object_fully_looted);
        assert!(second_close.whole_object_fully_skinned);
    }
}
