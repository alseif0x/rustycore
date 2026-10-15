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
    /// The login and battleground-fallback map entry and the cross-crate
    /// fixture call it as a session method; the body moved to
    /// `wow_world_instances` (#1363 slice 13a).
    pub(crate) fn ensure_canonical_world_map_for_current_player_like_cpp(
        &mut self,
    ) -> Option<wow_map::CreateMapDecision> {
        wow_world_instances::ensure_canonical_world_map_for_current_player_like_cpp(self)
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
