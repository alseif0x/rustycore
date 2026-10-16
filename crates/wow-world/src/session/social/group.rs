//! Represented group and party membership at the Session boundary.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_phase_group_store(&mut self, store: Arc<PhaseGroupStore>) {
        self.catalogs.phase_group_store = Some(store);
    }
    /// Set the shared group registry and pending invites.
    pub fn set_group_registry(&mut self, reg: Arc<GroupRegistry>, invites: Arc<PendingInvites>) {
        self.core.directory.group_registry = Some(reg);
        self.core.directory.pending_invites = Some(invites);
    }
    pub(crate) fn group_registry(&self) -> Option<&Arc<GroupRegistry>> {
        self.core.group_registry()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/social/group/f3_shims.rs"]
mod f3_shims;
