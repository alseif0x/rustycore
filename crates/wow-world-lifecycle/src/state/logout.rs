use tracing::info;
use wow_world_core::session::{HubMut, SessionState};

use super::SessionLifecycleState;
use crate::{FinalizationMode, SessionFinalization};

impl SessionLifecycleState {
    pub fn clear_logout_time(&mut self) {
        self.logout_time = None;
    }

    pub fn set_player_logout_like_cpp(&mut self, player_logout: bool) {
        self.player_logout_like_cpp = player_logout;
        if player_logout {
            self.durable_loot_money_persistence_like_cpp
                .close_admission_permanently_like_cpp();
        }
    }

    pub fn player_logout_like_cpp(&self) -> bool {
        self.player_logout_like_cpp
    }

    /// Admit timed finalization, without publishing completion before save.
    /// The session supervisor executes the same obligation coordinator; the
    /// timed route retains disconnect semantics after its logout publication.
    pub fn complete_logout(&mut self, hub: &mut HubMut<'_>) {
        info!("Timed logout admitted for account {}", hub.core.account_id);
        if self.finalization.is_none() {
            self.finalization = Some(SessionFinalization::new(
                FinalizationMode::TimedLogout,
                hub.core.player_guid().is_some(),
                hub.core.player_handle_like_cpp,
            ));
        }
        hub.core.state = SessionState::Disconnecting;
    }
}
