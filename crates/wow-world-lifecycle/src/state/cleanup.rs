use tracing::{debug, warn};
use wow_world_core::session::{HubMut, HubRef};

use super::SessionLifecycleState;
use crate::FinalizationOutcome;

impl SessionLifecycleState {
    pub fn unregister_canonical_player_from_map_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> FinalizationOutcome {
        let Some(handle) = hub.core.player_handle_like_cpp else {
            return if hub.core.player_guid().is_none() {
                FinalizationOutcome::NoWork
            } else {
                FinalizationOutcome::Unavailable
            };
        };
        let Some(manager) = hub.core.canonical_map_manager.as_ref() else {
            return FinalizationOutcome::Unavailable;
        };
        let Ok(mut manager) = manager.lock() else {
            return FinalizationOutcome::Unavailable;
        };

        // C++ WorldSession::LogoutPlayer retires its exact Player*, not a fresh
        // GUID lookup (WorldSession.cpp:660-672). Without an incarnation token
        // this Session has no authority to remove any current map resident.
        if manager.retire_player_like_cpp(handle).is_some() {
            hub.core.player_handle_like_cpp = None;
            FinalizationOutcome::Applied
        } else {
            // Retain the exact token on failure. Any later attempt must still
            // target this incarnation, never adopt or search for a replacement.
            warn!(
                "Failed to retire canonical Player {:?}: stale handle or missing owner value",
                handle.guid()
            );
            FinalizationOutcome::RetirementFailed
        }
    }

    /// Remove this session from the player registry.
    /// Called on logout or disconnect.
    pub fn unregister_from_player_registry(&self, hub: HubRef<'_>) {
        let (Some(guid), Some(reg)) = (hub.core.player_guid(), &hub.core.player_registry) else {
            return;
        };
        if reg.unregister_control_channel(guid, &hub.core.session_command_tx) {
            debug!("Unregistered player {:?} from broadcast registry", guid);
        }
    }
}
