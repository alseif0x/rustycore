// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::player_directory::PlayerRegistry;
use crate::session::state::SessionCore;
use std::sync::Arc;
use wow_constants::UnitState;
use wow_social::group::PendingInvites;

impl SessionCore {
    /// Get a reference to the shared player registry.
    pub fn player_registry(&self) -> Option<&Arc<PlayerRegistry>> {
        self.player_registry.as_ref()
    }

    /// Get a reference to the shared pending invites map.
    pub fn pending_invites(&self) -> Option<&Arc<PendingInvites>> {
        self.directory.pending_invites.as_ref()
    }

    pub fn player_is_in_world_for_registry_like_cpp(&self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        if let Some(manager) = &self.canonical_map_manager
            && let Ok(manager) = manager.lock()
        {
            let mut canonical_in_world = None;
            manager.do_for_all_maps(|managed| {
                if canonical_in_world.is_none()
                    && let Some(player) = managed.map().get_typed_player(guid)
                {
                    canonical_in_world = Some(player.unit().world().object().is_in_world());
                }
            });
            if let Some(is_in_world) = canonical_in_world {
                return is_in_world;
            }
        }

        // Registry insertion happens only after successful character login; logout/disconnect
        // unregisters instead of leaving a false/stale row behind.
        true
    }

    pub fn player_is_strictly_in_world_like_cpp(&self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };
        let Some(manager) = self
            .canonical_map_manager
            .as_ref()
            .and_then(|manager| manager.lock().ok())
        else {
            return false;
        };
        let mut is_in_world = None;
        manager.do_for_all_maps(|managed| {
            if is_in_world.is_none()
                && let Some(player) = managed.map().get_typed_player(guid)
            {
                is_in_world = Some(player.unit().world().object().is_in_world());
            }
        });
        is_in_world.unwrap_or(false)
    }

    pub fn player_has_unit_state_like_cpp(&self, state: UnitState) -> bool {
        self.canonical_player_snapshot_like_cpp(|player| player.unit().unit_state())
            .is_some_and(|unit_state| unit_state & state.bits() != 0)
    }
}
