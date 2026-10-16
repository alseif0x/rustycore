//! Represented group and party membership at the Session boundary.
//!
//! Moved out of the Session root under #615. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Resolve C++ `Player::m_group` through this session incarnation's
    /// generation-checked canonical Player handle. An unresolved owner never
    /// falls back in production.
    pub(crate) fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        let (state, hub) = crate::session::split_social_ref(self);
        wow_world_instances::resolved_group_guid_like_cpp(hub, state, cfg!(test))
    }
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
