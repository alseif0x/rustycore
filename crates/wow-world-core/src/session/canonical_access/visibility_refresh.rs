// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow canonical read view for the visibility refresh scans (#1440).
//!
//! C++ `Player::UpdateVisibleGameobjectsOrSpellClicks` (`Player.cpp:24433`)
//! walks the client-visible set, reads each object's quest/visibility state,
//! and publishes a values update. The Rust scans borrow exactly those reads
//! through this view instead of the whole `SessionCore`.

use std::collections::HashSet;

use wow_core::ObjectGuid;

use crate::session::state::SessionCore;
use crate::session::{QuestObjectiveAccessLikeCpp, RepresentedGameObjectAccessLikeCpp};

/// Borrowed reads and publication used by the visibility refresh scans.
#[derive(Clone, Copy)]
pub struct VisibilityRefreshAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build the visibility-refresh view without resolving or copying the Player.
    pub fn visibility_refresh_access_like_cpp(&self) -> VisibilityRefreshAccessLikeCpp<'_> {
        VisibilityRefreshAccessLikeCpp { core: self }
    }
}

impl<'a> VisibilityRefreshAccessLikeCpp<'a> {
    /// Snapshot of the client-visible GUID set the scans walk.
    pub fn client_visible_guids_snapshot_like_cpp(&self) -> HashSet<ObjectGuid> {
        self.core.client_visible_guids_like_cpp.snapshot_like_cpp()
    }

    pub fn canonical_gameobject_access_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        self.core.canonical_gameobject_access_like_cpp(guid)
    }

    pub fn quest_objective_access_like_cpp(&self) -> QuestObjectiveAccessLikeCpp<'a> {
        self.core.quest_objective_access_like_cpp()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    pub fn send_packet<P: wow_packet::ServerPacket>(&self, packet: &P) -> bool {
        self.core.send_packet(packet)
    }
}
