//! Viewer membership, first-open state and round-robin release for one Loot.
//!
//! C++ anchors: Loot.h::AddLooter/RemoveLooter (313-314),
//! Loot.cpp::Loot::OnLootOpened (695), Player.cpp::Player::SendLoot (8723),
//! and LootHandler.cpp::WorldSession::DoLootRelease (270, 384).
//! Response-enqueue rollback is the existing Rust authority contract. Guards,
//! snapshots, publication and notifications stay with their current callers.

use super::CreatureLoot;
use wow_core::ObjectGuid;

impl CreatureLoot {
    pub fn add_viewer(&mut self, player: ObjectGuid) -> bool {
        if self.players_looting.contains(&player) {
            false
        } else {
            self.players_looting.push(player);
            true
        }
    }

    pub fn mark_first_open(&mut self) -> bool {
        let first_viewer = !self.looted_by_player;
        if first_viewer {
            self.looted_by_player = true;
        }
        first_viewer
    }

    pub fn remove_viewer(&mut self, player: ObjectGuid) -> bool {
        let old_len = self.players_looting.len();
        self.players_looting.retain(|viewer| *viewer != player);
        old_len != self.players_looting.len()
    }

    pub fn remove_viewers(&mut self, players: &[ObjectGuid]) -> bool {
        let old_len = self.players_looting.len();
        self.players_looting.retain(|viewer| !players.contains(viewer));
        old_len != self.players_looting.len()
    }

    pub fn release_round_robin(&mut self, player: ObjectGuid) -> bool {
        let cleared = self.round_robin_player == player;
        if cleared {
            self.round_robin_player = ObjectGuid::EMPTY;
        }
        cleared
    }

    /// Authority opens mark first-open before inserting the viewer. Application
    /// fallbacks call the two public transitions at their separate old phases.
    pub(super) fn prepare_viewer_open(&mut self, player: ObjectGuid) -> (bool, bool) {
        let first_viewer = self.mark_first_open();
        let inserted = self.add_viewer(player);
        (inserted, first_viewer)
    }

    pub(super) fn rollback_viewer_open(
        &mut self,
        player: ObjectGuid,
        inserted: bool,
        first_viewer: bool,
    ) {
        if inserted {
            self.players_looting.retain(|viewer| *viewer != player);
        }
        if first_viewer {
            self.looted_by_player = false;
        }
    }
}

#[cfg(test)]
mod tests;
