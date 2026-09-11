//! Represented instance identity and its Session-side state.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn check_instance_count_like_cpp(&mut self, instance_id: u32) -> bool {
        let now_secs = u64::try_from(unix_now()).unwrap_or(0);
        self.check_instance_count_at_like_cpp(instance_id, now_secs)
    }
    pub(in crate::session) fn check_instance_count_probe_like_cpp(&self, instance_id: u32) -> bool {
        let now_secs = u64::try_from(unix_now()).unwrap_or(0);
        let active_count = self
            .represented_instance_reset_times_like_cpp
            .values()
            .filter(|release_time| **release_time > now_secs)
            .count();
        if active_count < self.max_instances_per_hour_like_cpp as usize {
            return true;
        }

        self.represented_instance_reset_times_like_cpp
            .get(&instance_id)
            .is_some_and(|release_time| *release_time > now_secs)
    }
    pub(in crate::session) fn check_instance_count_at_like_cpp(
        &mut self,
        instance_id: u32,
        now_secs: u64,
    ) -> bool {
        self.prune_expired_instance_reset_times_like_cpp(now_secs);
        if self.represented_instance_reset_times_like_cpp.len()
            < self.max_instances_per_hour_like_cpp as usize
        {
            return true;
        }

        self.represented_instance_reset_times_like_cpp
            .contains_key(&instance_id)
    }
    pub(crate) fn add_instance_enter_time_like_cpp(&mut self, instance_id: u32, enter_time: u64) {
        self.represented_instance_reset_times_like_cpp
            .entry(instance_id)
            .or_insert(enter_time.saturating_add(HOUR_SECS_LIKE_CPP));
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
            .and_then(|group_guid| self.group_registry.as_ref()?.get(&group_guid))
            .map(|group| group.recent_instance_owner_like_cpp(map_id))
            .or(self.player_guid)
    }
    pub fn set_instance_ignore_raid_like_cpp(&mut self, ignore: bool) {
        self.instance_ignore_raid_like_cpp = ignore;
    }
    pub fn set_instance_ignore_level_like_cpp(&mut self, ignore: bool) {
        self.instance_ignore_level_like_cpp = ignore;
    }
    pub fn set_max_instances_per_hour_like_cpp(&mut self, max_instances: u32) {
        self.max_instances_per_hour_like_cpp = max_instances;
    }
    /// C++ `Player::GetRecentInstanceId`.
    pub(crate) fn resolved_player_recent_instance_id_like_cpp(&self, map_id: u32) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .recent_instances
                .get(&map_id)
                .copied()
                .unwrap_or(0)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                self.represented_player_recent_instances_like_cpp
                    .get(&map_id)
                    .copied()
                    .unwrap_or(0),
            );
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn represented_player_recent_instance_id_like_cpp(&self, map_id: u32) -> u32 {
        self.resolved_player_recent_instance_id_like_cpp(map_id)
            .expect("test Player recent-instance owner must resolve")
    }
    /// C++ `Player::SetRecentInstance`.
    pub(crate) fn set_represented_player_recent_instance_like_cpp(
        &mut self,
        map_id: u32,
        instance_id: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_recent_instance_like_cpp(map_id, instance_id);
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_player_recent_instances_like_cpp
                .insert(map_id, instance_id);
            return true;
        }
        canonical
    }
    pub(crate) fn forget_represented_player_recent_instance_like_cpp(
        &mut self,
        map_id: u32,
    ) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.forget_recent_instance_like_cpp(map_id)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self
                .represented_player_recent_instances_like_cpp
                .remove(&map_id)
                .is_some();
        }
        canonical.unwrap_or(false)
    }
    pub(in crate::session) fn current_map_instanceable_like_cpp(&self) -> bool {
        let map_id = u32::from(self.player_map_id_like_cpp());
        self.map_store()
            .and_then(|store| store.get(map_id))
            .is_some_and(|entry| {
                matches!(
                    entry.instance_type,
                    wow_data::map::MAP_INSTANCE
                        | wow_data::map::MAP_RAID
                        | wow_data::map::MAP_BATTLEGROUND
                        | wow_data::map::MAP_ARENA
                        | wow_data::map::MAP_SCENARIO
                )
            })
    }
}
