//! Test entry point for the scripted gameobject use whose bodies moved to
//! `wow_world_entities::gameobject_use_scripted` (#1363 slice 4a).

use super::*;

impl WorldSession {
    pub(crate) async fn use_represented_gameobject_goober_preamble_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        gameobject_position: Position,
        player_guid: ObjectGuid,
        source: wow_entities::GooberUseSource,
    ) -> bool {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_entities::use_represented_gameobject_goober_preamble_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            gameobject_guid,
            gameobject_entry,
            gameobject_position,
            player_guid,
            source,
        )
        .await
    }
}
