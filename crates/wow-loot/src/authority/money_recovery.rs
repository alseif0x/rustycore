//! A receipt can resolve only the exact pool and persisting reservation that
//! requested it. A delayed database read never owns the map or a session.

use super::*;
use std::sync::Weak;

pub(super) fn consume_pool_money(state: &mut AuthorityState, scope: OwnedLootScope) -> bool {
    let Some(pool) = loot_for_scope_mut(state, scope) else {
        return false;
    };
    if std::mem::take(&mut pool.coins) > 0
        && let Some(spent) = state
            .scope_epochs
            .get_mut(&scope)
            .and_then(|epoch| epoch.recovered_world_money.as_mut())
    {
        *spent = true;
    }
    true
}

impl OwnedLootAuthority {
    pub fn recovered_world_money(&self, scope: OwnedLootScope, generation: u64) -> Option<bool> {
        let state = self.lock_state();
        let pool = state.scope_epochs.get(&scope)?;
        if state.retired || pool.generation != generation {
            return None;
        }
        pool.recovered_world_money
    }

    /// Reattach the stored grant before admitting a view of this exact pool.
    pub fn bind_world_money(
        &self,
        scope: OwnedLootScope,
        generation: u64,
        id: [u8; 16],
        spent: bool,
    ) -> bool {
        let mut state = self.lock_state();
        if state.retired || scope_epoch(&state, scope) != Some(generation) {
            return false;
        }
        let epoch = &state.scope_epochs[&scope];
        if epoch.recovered_world_money.is_some() {
            return epoch.money_claim_id == id;
        }
        if state
            .reservations
            .contains_key(&ReservationKey::Money(scope))
        {
            return false;
        }
        if spent {
            let Some(loot) = loot_for_scope_mut(&mut state, scope) else {
                return false;
            };
            loot.coins = 0;
        }
        let epoch = state.scope_epochs.get_mut(&scope).unwrap();
        epoch.money_claim_id = id;
        epoch.recovered_world_money = Some(spent);
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoneyReceiptDecision {
    Committed,
    RolledBack,
}

pub struct QuarantinedMoneyClaim {
    authority: Weak<OwnedLootAuthorityInner>,
    scope: OwnedLootScope,
    generation: u64,
    token: u64,
}

impl LootClaimPersistenceGuard {
    pub fn quarantine_money_for_receipt(&mut self) -> Option<QuarantinedMoneyClaim> {
        let lease = &self.claim.inner;
        let ReservationKey::Money(scope) = lease.key else {
            return None;
        };
        let mut state = lease.authority.lock_state();
        if state.detached
            || (state.retired && state.money_recovery.is_empty())
            || state.persisting.get(&lease.key) != Some(&lease.token)
            || scope_epoch(&state, scope) != Some(lease.generation)
            || lease.status.load(Ordering::Acquire) != LEASE_ACTIVE
        {
            return None;
        }
        state.persisting.remove(&lease.key);
        retain_only_persisting_reservations(&mut state);
        state.retired = true;
        state.quarantined = true;
        state.last_quarantine_token = lease.token;
        state.money_recovery.insert(scope, lease.token);
        bump_lifecycle_revision(&mut state);
        lease.status.store(LEASE_QUARANTINED, Ordering::Release);
        self.resolved = true;
        drop(state);
        lease.authority.notify_changed();
        Some(QuarantinedMoneyClaim {
            authority: Arc::downgrade(&lease.authority.inner),
            scope,
            generation: lease.generation,
            token: lease.token,
        })
    }
}

impl QuarantinedMoneyClaim {
    fn matches(&self, state: &AuthorityState) -> bool {
        !state.detached
            && state.quarantined
            && state.money_recovery.get(&self.scope) == Some(&self.token)
            && scope_epoch(state, self.scope) == Some(self.generation)
    }

    pub fn is_current(&self) -> bool {
        self.authority.upgrade().is_some_and(|inner| {
            let authority = OwnedLootAuthority { inner };
            self.matches(&authority.lock_state())
        })
    }

    pub fn settle(&self, decision: MoneyReceiptDecision) -> bool {
        let Some(inner) = self.authority.upgrade() else {
            return false;
        };
        let authority = OwnedLootAuthority { inner };
        let mut state = authority.lock_state();
        if !self.matches(&state) {
            return false;
        }
        if decision == MoneyReceiptDecision::Committed
            && !consume_pool_money(&mut state, self.scope)
        {
            return false;
        }
        state.money_recovery.remove(&self.scope);
        if state.money_recovery.is_empty() {
            state.retired = false;
            state.quarantined = false;
        }
        bump_lifecycle_revision(&mut state);
        drop(state);
        authority.notify_changed();
        true
    }
}
