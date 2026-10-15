//! Represented instance identity and its Session-side state.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn check_instance_count_like_cpp(&mut self, instance_id: u32) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.check_instance_count_like_cpp(&mut hub, instance_id)
    }
    pub(crate) fn add_instance_enter_time_like_cpp(&mut self, instance_id: u32, enter_time: u64) {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.add_instance_enter_time_like_cpp(&mut hub, instance_id, enter_time)
    }
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
