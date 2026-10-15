//! Logout admission. Persistence and retirement run through the finalization executor.
use super::super::WorldSession;
#[cfg(test)]
use crate::finalization::FinalizationMode;

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn save_disconnect_player_to_db_like_cpp(
        &mut self,
    ) -> crate::FinalizationReport {
        let generators = self.id_generators_for_test_like_cpp();
        self.finalize_session_with_generator_like_cpp(
            FinalizationMode::Disconnect,
            generators.item.as_ref(),
        )
        .await
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/lifecycle/logout/f3_shims.rs"]
mod f3_shims;
