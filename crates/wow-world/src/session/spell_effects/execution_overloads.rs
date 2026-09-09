//! Thin overload wrappers over the represented spell execution entry point.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.

use super::*;

impl WorldSession {
    /// Execute a spell — apply effects, set cooldown, send SMSG_SPELL_GO.
    ///
    /// Called for instant-cast spells. Delegates to execute_spell_with_visual
    /// with default cast_id and visual.
    pub async fn execute_spell_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_id: i32,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        use wow_packet::packets::spell::SpellCastVisual;

        self.execute_spell_with_visual_and_generator_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            target_guid,
            ObjectGuid::EMPTY,
            SpellCastVisual {
                spell_visual_id: 0,
                script_visual_id: 0,
            },
        )
        .await
    }
    #[cfg(test)]
    pub async fn execute_spell(
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
    pub async fn execute_spell_with_target_data_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_id: i32,
        target_guid: ObjectGuid,
        target_data: SpellTargetData,
    ) -> Result<(), &'static str> {
        use wow_packet::packets::spell::SpellCastVisual;

        self.execute_spell_with_visual_and_target_data_and_generator_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            target_guid,
            ObjectGuid::EMPTY,
            SpellCastVisual {
                spell_visual_id: 0,
                script_visual_id: 0,
            },
            target_data,
        )
        .await
    }
    #[cfg(test)]
    pub async fn execute_spell_with_target_data(
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
    /// Execute a spell with full visual/cast info — apply effects, set cooldown, send SMSG_SPELL_GO.
    ///
    /// Called after cast time completes or for instant-cast spells.
    /// Supports: instakill (type 1), damage (type 2), aura application (type 6), heal (type 10).
    pub async fn execute_spell_with_visual_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_id: i32,
        target_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual: wow_packet::packets::spell::SpellCastVisual,
    ) -> Result<(), &'static str> {
        self.execute_spell_with_visual_and_target_data_and_generator_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            target_guid,
            cast_id,
            spell_visual,
            SpellTargetData {
                flags: 0x2,
                unit: target_guid,
                item: ObjectGuid::EMPTY,
                ..SpellTargetData::default()
            },
        )
        .await
    }
    #[cfg(test)]
    pub async fn execute_spell_with_visual(
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
    pub async fn execute_spell_with_visual_and_target_data_and_generator_like_cpp(
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
    pub async fn execute_spell_with_visual_and_target_data(
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
    pub async fn execute_spell_with_visual_and_target_data_with_metadata(
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
