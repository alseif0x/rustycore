//! Logout admission. Persistence and retirement run through the finalization executor.
use super::super::{SessionState, WorldSession};
use crate::finalization::{FinalizationMode, SessionFinalization};
use tracing::info;

impl WorldSession {
    #[cfg(test)]
    pub async fn save_disconnect_player_to_db_like_cpp(&mut self) -> crate::FinalizationReport {
        let generators = self.id_generators_for_test_like_cpp();
        self.finalize_session_with_generator_like_cpp(
            FinalizationMode::Disconnect,
            generators.item.as_ref(),
        )
        .await
    }

    pub(crate) fn set_player_logout_like_cpp(&mut self, player_logout: bool) {
        self.lifecycle.player_logout_like_cpp = player_logout;
        if player_logout {
            self.lifecycle
                .durable_loot_money_persistence_like_cpp
                .close_admission_permanently_like_cpp();
        }
        self.sync_current_player_session_visibility_detection_like_cpp();
    }

    pub(crate) fn player_logout_like_cpp(&self) -> bool {
        self.lifecycle.player_logout_like_cpp
    }

    /// Admit timed finalization, without publishing completion before save.
    /// The session supervisor executes the same obligation coordinator; the
    /// timed route retains disconnect semantics after its logout publication.
    pub(in crate::session) fn complete_logout(&mut self) {
        info!("Timed logout admitted for account {}", self.core.account_id);
        if self.lifecycle.finalization.is_none() {
            self.lifecycle.finalization = Some(SessionFinalization::new(
                FinalizationMode::TimedLogout,
                self.player_guid().is_some(),
                self.core.player_handle_like_cpp,
            ));
        }
        self.core.state = SessionState::Disconnecting;
    }
}
