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
        let creature_spawns = self.creature_spawn_catalogs_for_test_like_cpp();
        let graveyards = self.catalogs.graveyard_store().cloned();
        wow_world_application::finalize_session_with_resurrection_catalogs(
            self,
            FinalizationMode::Disconnect,
            generators.item.as_ref(),
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creature_spawns,
                graveyards: graveyards.as_deref(),
            },
        )
        .await
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/lifecycle/logout/f3_shims.rs"]
mod f3_shims;
