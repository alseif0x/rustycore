// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player's own recent-instance map.
//!
//! C++ keeps `m_recentInstances` private to the Player and reaches it only
//! through `Player::SetRecentInstance` (Player.h:2518), the lookup beside it
//! (Player.h:2514) and the expiry erase in `Player::UpdateInstanceLockTimers`
//! (Player.cpp:20698). The same three operations live here so no caller has to
//! open the Player's gameplay state to reach the map.

use crate::Player;

const INSTANCE_RESET_WINDOW_SECS_LIKE_CPP: u64 = 60 * 60;

impl Player {
    /// C++ `Player::SetRecentInstance` (Player.h:2518).
    pub fn set_recent_instance_like_cpp(&mut self, map_id: u32, instance_id: u32) {
        self.gameplay_state_mut()
            .recent_instances
            .insert(map_id, instance_id);
    }

    /// The erase C++ performs while expiring recent instances
    /// (Player.cpp:20698). Returns whether the map had an entry.
    pub fn forget_recent_instance_like_cpp(&mut self, map_id: u32) -> bool {
        self.gameplay_state_mut()
            .recent_instances
            .remove(&map_id)
            .is_some()
    }

    /// C++ `Player::_instanceResetTimes` is Player-owned state. The Session
    /// may borrow this snapshot for admission and persistence, but cannot
    /// retain a second production map.
    pub fn instance_reset_times_like_cpp(&self) -> &std::collections::BTreeMap<u32, u64> {
        &self.gameplay_state().instance_reset_times
    }

    pub fn clear_instance_reset_times_like_cpp(&mut self) {
        self.gameplay_state_mut().instance_reset_times.clear();
    }

    pub fn replace_instance_reset_times_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = (u32, u64)>,
    ) {
        let times = &mut self.gameplay_state_mut().instance_reset_times;
        times.clear();
        for (instance_id, release_time) in rows {
            times.entry(instance_id).or_insert(release_time);
        }
    }

    pub fn prune_instance_reset_times_like_cpp(&mut self, now_secs: u64) {
        self.gameplay_state_mut()
            .instance_reset_times
            .retain(|_, release_time| *release_time > now_secs);
    }

    pub fn check_instance_count_like_cpp(
        &mut self,
        instance_id: u32,
        now_secs: u64,
        max_instances_per_hour: u32,
    ) -> bool {
        self.prune_instance_reset_times_like_cpp(now_secs);
        let times = self.instance_reset_times_like_cpp();
        times.len() < max_instances_per_hour as usize || times.contains_key(&instance_id)
    }

    pub fn check_instance_count_probe_like_cpp(
        &self,
        instance_id: u32,
        now_secs: u64,
        max_instances_per_hour: u32,
    ) -> bool {
        let times = self.instance_reset_times_like_cpp();
        let active_count = times
            .values()
            .filter(|release_time| **release_time > now_secs)
            .count();
        active_count < max_instances_per_hour as usize
            || times
                .get(&instance_id)
                .is_some_and(|release_time| *release_time > now_secs)
    }

    pub fn add_instance_enter_time_like_cpp(&mut self, instance_id: u32, enter_time: u64) {
        self.gameplay_state_mut()
            .instance_reset_times
            .entry(instance_id)
            .or_insert(enter_time.saturating_add(INSTANCE_RESET_WINDOW_SECS_LIKE_CPP));
    }
}
