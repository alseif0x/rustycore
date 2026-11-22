//! A receipt can resolve only the exact pool and persisting reservation that
//! requested it. A delayed database read never owns the map or a session.

use super::*;
use std::sync::Weak;

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
        if decision == MoneyReceiptDecision::Committed {
            let Some(pool) = loot_for_scope_mut(&mut state, self.scope) else {
                return false;
            };
            pool.coins = 0;
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
