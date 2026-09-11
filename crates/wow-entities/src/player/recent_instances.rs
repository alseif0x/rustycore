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
}
