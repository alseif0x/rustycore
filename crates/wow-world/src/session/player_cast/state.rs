//! Canonical Player/Unit cast state adapters and existing driver entry points.

use super::*;

impl WorldSession {
    pub(crate) fn mutate_cast_execution_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::CastExecutionStateLikeCpp) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        wow_world_application::mutate_cast_execution_like_cpp(&mut hub, state, f)
    }

    pub(crate) fn spell_last_cast_time_like_cpp(&self, spell_id: i32) -> Option<Option<Instant>> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.spell_last_cast_time_like_cpp(hub, spell_id)
    }

    /// The driver's entry to the pending-request tick, moved to
    /// `wow_world_application` (#1363 slice 12b7).
    pub(crate) async fn tick_pending_spell_cast_request_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) {
        wow_world_application::tick_pending_spell_cast_request_with_generator_like_cpp(
            self,
            item_guid_generator,
            creature_spawn_catalogs,
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn tick_pending_spell_cast_request_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.tick_pending_spell_cast_request_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
        )
        .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_cast/state/f3_shims.rs"]
mod f3_shims;
