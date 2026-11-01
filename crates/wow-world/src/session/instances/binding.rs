//! Represented instance binding, occupancy and resets.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Inject the shared C++ `InstanceLockMgr` analogue.
    pub fn set_instance_lock_mgr(
        &mut self,
        mgr: Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>,
    ) {
        self.core.instance_lock_mgr = Some(mgr);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/binding/f3_shims.rs"]
mod f3_shims;
