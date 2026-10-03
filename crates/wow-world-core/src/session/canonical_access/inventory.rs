// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::PlayerInventoryRuntime;

use crate::session::SessionCore;

/// Borrowed access to the session's canonical inventory owner.
pub struct OwnedInventoryAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build a borrowed capability for the canonical inventory owner.
    pub fn owned_inventory_access_like_cpp(&self) -> OwnedInventoryAccessLikeCpp<'_> {
        OwnedInventoryAccessLikeCpp { core: self }
    }
}

impl OwnedInventoryAccessLikeCpp<'_> {
    /// Clone the current canonical inventory runtime while its owner is held.
    pub fn inventory_runtime_snapshot_like_cpp(&self) -> Option<PlayerInventoryRuntime> {
        self.core
            .with_owned_player_like_cpp(|player| player.inventory_runtime_like_cpp().clone())
    }

    /// Whether this fixture session has no canonical Player handle installed.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}
