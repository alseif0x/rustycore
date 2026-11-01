//! Represented instance identity and its Session-side state.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_instance_ignore_raid_like_cpp(&mut self, ignore: bool) {
        self.core.realm_policy.instance_ignore_raid_like_cpp = ignore;
    }
    pub fn set_instance_ignore_level_like_cpp(&mut self, ignore: bool) {
        self.core.realm_policy.instance_ignore_level_like_cpp = ignore;
    }
    pub fn set_max_instances_per_hour_like_cpp(&mut self, max_instances: u32) {
        self.core.realm_policy.max_instances_per_hour_like_cpp = max_instances;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/instance/f3_shims.rs"]
mod f3_shims;
