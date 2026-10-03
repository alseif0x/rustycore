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

    /// Invalidate aura authority through the existing GUID/map mutation path.
    /// A missing manager or matching Player is ignored; the manager guard is
    /// released before this call returns.
    pub fn invalidate_spell_hit_aura_authority_for_inventory_mutation_like_cpp(&self) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }

    /// Mutate inventory only through this session's generation-checked Player
    /// handle. A missing or stale owner returns `None` without GUID/map fallback.
    /// The synchronous callback runs under the manager guard, which is released
    /// before this method returns.
    pub fn with_inventory_runtime_mut_like_cpp<R>(
        &self,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            update(player.inventory_runtime_mut_like_cpp())
        })
    }

    /// Whether this fixture session has no canonical Player handle installed.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}
