// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player group membership query for the difficulty application.

use crate::session::state::SessionCore;

pub struct PlayerGroupOwnerAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    pub fn player_group_owner_access_like_cpp(&self) -> PlayerGroupOwnerAccessLikeCpp<'_> {
        PlayerGroupOwnerAccessLikeCpp { core: self }
    }
}

impl PlayerGroupOwnerAccessLikeCpp<'_> {
    /// `None` means the canonical Player did not resolve; `Some(None)` means
    /// it resolved and has no group membership.
    pub fn canonical_group_guid_like_cpp(&self) -> Option<Option<u64>> {
        self.core.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .group
                .as_ref()
                .map(|group| group.group_guid.counter() as u64)
        })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}
