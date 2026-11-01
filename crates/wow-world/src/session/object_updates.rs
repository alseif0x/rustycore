// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session consumption of map-owned `Map::SendObjectUpdates` snapshots.
//!
//! The map owns update-field mutation and clears masks while it still owns the
//! map guard.  This module consumes the resulting owned snapshots afterwards,
//! applies the receiver's committed visibility, and writes packets without
//! retaining a map or directory guard across the send.

use super::WorldSession;

impl WorldSession {
    pub(crate) fn send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
        &mut self,
    ) -> usize {
        let (state, mut hub) = crate::session::split_visibility_mut(self);
        state
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
                &mut hub,
            )
    }
}
