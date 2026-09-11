// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The group transitions C++ keeps on the Player itself.
//!
//! `Player::SetGroup` (Player.cpp:23440) owns the member's own
//! `GroupReference`, and the two sequence transitions beside it own
//! `Player::m_groupUpdateSequences` (Player.h:3034). Group membership stays
//! authoritative in the group registry, exactly as C++ keeps it in `Group`;
//! what the Player owns is its own snapshot of that membership and its own
//! party-update counters.
//!
//! Departures kept explicit:
//!
//! * C++ `Player::SetGroup` ends with `UpdateObjectVisibility(false)`. Here the
//!   visibility and registry publication remain with the session callers that
//!   already perform them after the assignment; this module does not add a
//!   second publication path.
//! * `Player::NextGroupUpdateSequenceNumber` (Player.cpp:25235) is a plain
//!   post-increment in C++. The saturating increment below is the existing
//!   RustyCore behavior and is preserved rather than changed here.

use crate::{Player, PlayerGroupState};

impl Player {
    /// C++ `Player::SetGroup` (Player.cpp:23440): replace the Player-owned
    /// group reference snapshot, `None` for `m_group.unlink()`.
    pub fn set_group_like_cpp(&mut self, group: Option<PlayerGroupState>) {
        self.gameplay_state_mut().group = group;
    }

    /// C++ `Player::ResetGroupUpdateSequenceIfNeeded` (Player.cpp:25224):
    /// rejoining the last group must not reset the sequence, so the counter is
    /// restarted only when the category's group guid actually changed. Returns
    /// whether it was reset.
    pub fn reset_group_update_sequence_if_needed_like_cpp(
        &mut self,
        category: usize,
        group_guid: u64,
    ) -> bool {
        let sequence = &mut self.gameplay_state_mut().group_update_sequences[category];
        if sequence.group_guid == Some(group_guid) {
            return false;
        }
        sequence.group_guid = Some(group_guid);
        sequence.update_sequence_number = 1;
        true
    }

    /// C++ `Player::NextGroupUpdateSequenceNumber` (Player.cpp:25235): return
    /// the category's current sequence number and then advance it.
    pub fn next_group_update_sequence_number_like_cpp(&mut self, category: usize) -> i32 {
        let sequence = &mut self.gameplay_state_mut().group_update_sequences[category];
        let current = sequence.update_sequence_number;
        sequence.update_sequence_number = sequence.update_sequence_number.saturating_add(1);
        current
    }
}
