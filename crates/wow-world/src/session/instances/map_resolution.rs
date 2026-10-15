//! Resolving the canonical map and manager the represented Session is
//! currently attached to.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Inject the shared, file-backed terrain heights (`DataDir`). Call once
    /// at session creation, before login.
    pub fn set_live_terrain_like_cpp(&mut self, terrain: Arc<crate::terrain::LiveTerrainHeights>) {
        self.core.live_terrain = Some(terrain);
    }
    pub fn set_canonical_map_manager(&mut self, mgr: SharedCanonicalMapManager) {
        if let Some(registry) = &self.core.player_registry {
            let _ = registry.bind_canonical_map_manager(Arc::clone(&mgr));
        }
        self.core.canonical_map_manager = Some(mgr);
    }
    /// Resolve the one canonical Player identity and move that exact value to
    /// the selected map. Existing map records predate Player handles, so the
    /// transition must adopt them before considering a new initial value.
    pub(in crate::session) fn ensure_canonical_player_owner_for_map_like_cpp(
        &mut self,
        key: wow_map::MapKey,
        position: Position,
    ) -> bool {
        if !self.ensure_canonical_player_owner_exists_like_cpp(key) {
            return false;
        }
        let Some(manager) = self.core.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };

        let Some(handle) = self.core.player_handle_like_cpp else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Active(current)) if current == key => manager
                .relocate_player_like_cpp(handle, position)
                .is_ok_and(|outcome| outcome.relocated),
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) => {
                manager.detach_player_like_cpp(handle).is_ok()
                    && manager
                        .attach_player_like_cpp(handle, key, position)
                        .is_ok()
            }
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => manager
                .attach_player_like_cpp(handle, key, position)
                .is_ok(),
            None => false,
        }
    }
    /// Inject the dedicated Detour worker handle. The session only sends
    /// path requests; it never owns raw mmap/navmesh state.
    pub fn set_mmap_pathfinder_like_cpp(
        &mut self,
        pathfinder: Arc<WorldMMapPathfinderWorkerLikeCpp>,
    ) {
        self.core.mmap_pathfinder_like_cpp = Some(pathfinder);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/map_resolution/f3_shims.rs"]
mod f3_shims;
