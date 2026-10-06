// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::PlayerEquipmentSetsLikeCpp;

use crate::session::SessionCore;

/// Borrowed canonical-player capability for equipment-set mutations.
///
/// It keeps the session's generation-checked owner resolution inside Core and
/// never owns or exposes the session or its map-manager guard.
pub struct OwnedEquipmentSetsAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build a borrowed capability for the canonical equipment-set owner.
    pub fn owned_equipment_sets_access_like_cpp(&self) -> OwnedEquipmentSetsAccessLikeCpp<'_> {
        OwnedEquipmentSetsAccessLikeCpp { core: self }
    }
}

impl OwnedEquipmentSetsAccessLikeCpp<'_> {
    /// Read the equipment sets on this session's current canonical Player.
    pub fn with_equipment_sets_like_cpp<R>(
        &self,
        f: impl FnOnce(&PlayerEquipmentSetsLikeCpp) -> R,
    ) -> Option<R> {
        self.core
            .with_owned_player_like_cpp(|player| f(&player.gameplay_state().equipment_sets))
    }

    /// Mutate the equipment sets on this session's current canonical Player.
    pub fn with_equipment_sets_mut_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut PlayerEquipmentSetsLikeCpp) -> R,
    ) -> Option<R> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            f(&mut player.gameplay_state_mut().equipment_sets)
        })
    }

    /// Whether this fixture session has no canonical Player handle installed.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}
