//! Logout admission. Persistence and retirement run through the finalization executor.
use super::super::WorldSession;
#[cfg(test)]
use crate::finalization::FinalizationMode;

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
        self.lifecycle.set_player_logout_like_cpp(player_logout);
        self.sync_current_player_session_visibility_detection_like_cpp();
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/lifecycle/logout/f3_shims.rs"]
mod f3_shims;
