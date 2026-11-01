//! Canonical residence identity reads for Player-origin casts.

use crate::SessionSpellState;
use wow_world_core::session::HubRef;

impl SessionSpellState {
    /// The admitted residence revision for the logged-in player.
    ///
    /// A prepared cast is fenced against residence reentry by stamping this
    /// value. Server-triggered timed casts need the same fence as normal
    /// client requests: without it a cast prepared before a map transfer would
    /// still launch after the player returns.
    pub fn current_player_residence_revision_like_cpp(&self, hub: HubRef<'_>) -> Option<u64> {
        let handle = hub.core.player_handle_like_cpp?;
        if Some(handle.guid()) != hub.core.player_guid() {
            return None;
        }
        let manager = hub.core.canonical_map_manager.as_ref()?.lock().ok()?;
        manager
            .player_active_residence_revision_like_cpp(handle)
            .map(|(_, revision)| revision)
    }
}
