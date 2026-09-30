//! Owned-loot authority, claims and leases state definitions, part 1 of 1.
//!
//! Separated from the authority.rs root under #642. Behaviour is preserved.

use super::*;

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

    pub(super) fn mark_item_looted_for_player_like_cpp(
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
pub(super) enum ReservationKey {
    Item(LootItemClaimKey),
    Money(OwnedLootScope),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AuthorityState {
    /// Whole-object lifecycle counter retained for first-install/retire
    /// arbitration. Claims use the selected pool's epoch below.
    pub(super) generation: u64,
    pub(super) retired: bool,
    pub(super) detached: bool,
    pub(super) quarantined: bool,
    pub(super) shared: Option<CreatureLoot>,
    pub(super) personal: HashMap<ObjectGuid, CreatureLoot>,
    /// Unique identity of each concrete C++ `Loot` pool. Replacing one
    /// personal pool advances only that scope and leaves peer claims valid.
    pub(super) scope_epochs: HashMap<OwnedLootScope, u64>,
    pub(super) next_scope_epoch: u64,
    pub(super) lifecycle_revision: u64,
    pub(super) reservations: HashMap<ReservationKey, u64>,
    /// Claims that crossed the durable-persistence boundary. Lifecycle close
    /// may reject every new claim, but must retain these reservations until
    /// their SQL result commits or rolls back.
    pub(super) persisting: HashMap<ReservationKey, u64>,
    pub(super) next_token: u64,
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

pub(super) fn retain_only_persisting_reservations(state: &mut AuthorityState) {
    let persisting = &state.persisting;
    state
        .reservations
        .retain(|key, token| persisting.get(key) == Some(token));
}

pub(super) fn finalize_closed_state_if_drained(state: &mut AuthorityState) {
    if !state.retired || !state.persisting.is_empty() {
        return;
    }
    state.shared = None;
    state.personal.clear();
    state.scope_epochs.clear();
    state.reservations.clear();
}

pub(super) fn scope_has_persisting_claim(state: &AuthorityState, scope: OwnedLootScope) -> bool {
    state
        .persisting
        .keys()
        .any(|key| reservation_scope(*key) == scope)
}

pub(super) struct OwnedLootAuthorityInner {
    pub(super) state: Mutex<AuthorityState>,
    pub(super) changed: watch::Sender<u64>,
}

/// Shared, object-owned authority for one creature or game-object loot lifetime.
#[derive(Clone)]
pub struct OwnedLootAuthority {
    pub(super) inner: Arc<OwnedLootAuthorityInner>,
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

pub(super) enum ReserveAttempt {
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
pub(super) enum LootItemClaimMode {
    Direct,
    Award,
}

pub(super) fn reserve_item_once(
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

pub(super) fn reserve_money_once(
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

pub(super) fn next_token(state: &mut AuthorityState) -> u64 {
    let token = state.next_token;
    state.next_token = state.next_token.wrapping_add(1).max(1);
    token
}

pub(super) fn next_scope_epoch(state: &mut AuthorityState) -> u64 {
    let epoch = state.next_scope_epoch;
    state.next_scope_epoch = state.next_scope_epoch.wrapping_add(1).max(1);
    epoch
}

pub(super) fn bump_lifecycle_revision(state: &mut AuthorityState) {
    // A retired authority must never wrap back to a lifecycle token retained
    // by a detached worker.
    state.lifecycle_revision = state.lifecycle_revision.saturating_add(1).max(1);
}

pub(super) fn install_all_scope_epochs(state: &mut AuthorityState, epoch: u64) {
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

pub(super) fn any_scope_epoch(state: &AuthorityState) -> Option<u64> {
    state.scope_epochs.values().copied().min()
}

pub(super) fn scope_epoch(state: &AuthorityState, scope: OwnedLootScope) -> Option<u64> {
    state.scope_epochs.get(&scope).copied()
}

pub(super) const fn reservation_scope(key: ReservationKey) -> OwnedLootScope {
    match key {
        ReservationKey::Item(item) => item.scope,
        ReservationKey::Money(scope) => scope,
    }
}

pub(super) fn selected_scope_like_cpp(
    state: &AuthorityState,
    player: ObjectGuid,
) -> Option<OwnedLootScope> {
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

pub(super) fn active_loot_pools_fully_looted_like_cpp(state: &AuthorityState) -> bool {
    state
        .shared
        .as_ref()
        .is_none_or(CreatureLoot::is_looted_like_cpp)
        && state
            .personal
            .values()
            .all(CreatureLoot::is_looted_like_cpp)
}

pub(super) fn active_loot_pools_have_no_viewers_like_cpp(state: &AuthorityState) -> bool {
    state
        .shared
        .as_ref()
        .is_none_or(|loot| loot.players_looting.is_empty())
        && state
            .personal
            .values()
            .all(|loot| loot.players_looting.is_empty())
}

pub(super) fn active_loot_pools_fully_skinned_like_cpp(state: &AuthorityState) -> bool {
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

pub(super) fn loot_for_scope(
    state: &AuthorityState,
    scope: OwnedLootScope,
) -> Option<&CreatureLoot> {
    match scope {
        OwnedLootScope::Shared => state.shared.as_ref(),
        OwnedLootScope::Personal(player) => state.personal.get(&player),
    }
}

pub(super) fn loot_for_scope_mut(
    state: &mut AuthorityState,
    scope: OwnedLootScope,
) -> Option<&mut CreatureLoot> {
    match scope {
        OwnedLootScope::Shared => state.shared.as_mut(),
        OwnedLootScope::Personal(player) => state.personal.get_mut(&player),
    }
}

pub(super) fn snapshot_for_scope(
    state: &AuthorityState,
    scope: OwnedLootScope,
) -> Option<OwnedLootSnapshot> {
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

pub(super) const LEASE_ACTIVE: u8 = 0;

pub(super) const LEASE_COMMITTED: u8 = 1;

pub(super) const LEASE_ROLLED_BACK: u8 = 2;

pub(super) const LEASE_QUARANTINED: u8 = 3;

pub(super) struct LootClaimCommitOutcome {
    pub(super) first_commit: bool,
    pub(super) snapshot: Option<OwnedLootSnapshot>,
}

pub(super) struct LootClaimLeaseInner {
    pub(super) authority: OwnedLootAuthority,
    pub(super) generation: u64,
    pub(super) token: u64,
    pub(super) key: ReservationKey,
    pub(super) player: ObjectGuid,
    pub(super) payload: LootClaimPayload,
    pub(super) status: AtomicU8,
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
    pub(super) inner: Arc<LootClaimLeaseInner>,
}

/// RAII owner of the durable phase of one claim. Dropping it before a
/// successful commit is the only operation allowed to reopen a persisting
/// reservation; ordinary clones cannot roll it back concurrently.
pub struct LootClaimPersistenceGuard {
    pub(super) claim: LootClaimLease,
    pub(super) resolved: bool,
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
    pub(super) fn new(
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
