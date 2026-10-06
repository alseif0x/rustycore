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
    pub(in crate::session) fn check_instance_count_probe_like_cpp(&self, instance_id: u32) -> bool {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.check_instance_count_probe_like_cpp(hub, instance_id)
    }
    pub(crate) fn add_instance_enter_time_like_cpp(&mut self, instance_id: u32, enter_time: u64) {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.add_instance_enter_time_like_cpp(&mut hub, instance_id, enter_time)
    }
    pub(in crate::session) fn create_map_instance_owner_guid_like_cpp(
        &self,
        map_id: u32,
    ) -> Option<ObjectGuid> {
        // #743: C++ reads `Player::GetGroup()`, which `Group::RemoveMember`
        // and `Group::Disband` clear in the same operation. Resolve instance
        // ownership through the authority so a member removed while a
        // notification is still queued cannot keep the group's instance.
        self.authoritative_group_membership_like_cpp()
            .and_then(|group_guid| {
                self.core
                    .directory
                    .group_registry
                    .as_ref()?
                    .get(&group_guid)
            })
            .map(|group| group.recent_instance_owner_like_cpp(map_id))
            .or(self.core.player_guid)
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
    pub(crate) fn resolved_player_recent_instance_id_like_cpp(&self, map_id: u32) -> Option<u32> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.resolved_player_recent_instance_id_like_cpp(hub, map_id)
    }
    pub(crate) fn set_represented_player_recent_instance_like_cpp(
        &mut self,
        map_id: u32,
        instance_id: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.set_represented_player_recent_instance_like_cpp(&mut hub, map_id, instance_id)
    }
    pub(in crate::session) fn current_map_instanceable_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.current_map_instanceable_like_cpp(hub)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/instance/f3_shims.rs"]
mod f3_shims;
