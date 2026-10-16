//! The World session as the host of the player cast moved to
//! `wow_world_spell` (#1363 slice 9e). Each operation names the inherent
//! `WorldSession` method by path.

use super::*;
use std::future::Future;

impl wow_world_spell::PlayerCastHostLikeCpp for WorldSession {
    type PlayerConditionContext = RepresentedPlayerConditionContextLikeCpp;
    fn tick_pending_spell_cast_request_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send {
        wow_world_application::tick_pending_spell_cast_request_with_generator_like_cpp(
            self,
            item_guid_generator,
            creature_spawn_catalogs,
        )
    }
    fn has_represented_self_res_spell_like_cpp(&self, spell_id: i32) -> bool {
        wow_world_spell::has_represented_self_res_spell_like_cpp(self, spell_id)
    }
    fn represented_spell_click_creature_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedSpellClickCreatureSnapshotLikeCpp> {
        self.world_entities
            .represented_spell_click_creature_snapshot_like_cpp(
                &self.core.quest_objective_access_like_cpp(),
                guid,
            )
    }
    fn represented_player_condition_context_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerConditionContextLikeCpp> {
        WorldSession::represented_player_condition_context_like_cpp(self)
    }
}
