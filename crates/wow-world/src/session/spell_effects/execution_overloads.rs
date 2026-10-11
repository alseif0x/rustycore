//! Thin overload wrappers over the represented spell execution entry point.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn execute_spell(
        &mut self,
        spell_id: i32,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.execute_spell_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            spell_id,
            target_guid,
        )
        .await
    }
    #[cfg(test)]
    pub(crate) async fn execute_spell_with_target_data(
        &mut self,
        spell_id: i32,
        target_guid: ObjectGuid,
        target_data: SpellTargetData,
    ) -> Result<(), &'static str> {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.execute_spell_with_target_data_and_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            spell_id,
            target_guid,
            target_data,
        )
        .await
    }
    #[cfg(test)]
    pub(crate) async fn execute_spell_with_visual(
        &mut self,
        spell_id: i32,
        target_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual: wow_packet::packets::spell::SpellCastVisual,
    ) -> Result<(), &'static str> {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.execute_spell_with_visual_and_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            spell_id,
            target_guid,
            cast_id,
            spell_visual,
        )
        .await
    }
    pub(crate) async fn execute_spell_with_visual_and_target_data_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_id: i32,
        target_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual: wow_packet::packets::spell::SpellCastVisual,
        target_data: SpellTargetData,
    ) -> Result<(), &'static str> {
        self.execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            target_guid,
            cast_id,
            spell_visual,
            target_data,
            SpellCastMetadata::default(),
        )
        .await
    }
    #[cfg(test)]
    pub(crate) async fn execute_spell_with_visual_and_target_data(
        &mut self,
        spell_id: i32,
        target_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual: wow_packet::packets::spell::SpellCastVisual,
        target_data: SpellTargetData,
    ) -> Result<(), &'static str> {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.execute_spell_with_visual_and_target_data_and_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            spell_id,
            target_guid,
            cast_id,
            spell_visual,
            target_data,
        )
        .await
    }
    #[cfg(test)]
    pub(crate) async fn execute_spell_with_visual_and_target_data_with_metadata(
        &mut self,
        spell_id: i32,
        target_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual: wow_packet::packets::spell::SpellCastVisual,
        target_data: SpellTargetData,
        metadata: SpellCastMetadata,
    ) -> Result<(), &'static str> {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            spell_id,
            target_guid,
            cast_id,
            spell_visual,
            target_data,
            metadata,
        )
        .await
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_effects/execution_overloads/test_only.rs"]
mod test_only;
