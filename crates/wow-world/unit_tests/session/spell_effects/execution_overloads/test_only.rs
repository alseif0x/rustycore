// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `execution_overloads` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    /// Execute a spell — apply effects, set cooldown, send SMSG_SPELL_GO.
    ///
    /// Called for instant-cast spells. Delegates to execute_spell_with_visual
    /// with default cast_id and visual.
    pub(crate) async fn execute_spell_with_generator_like_cpp(
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
    pub(crate) async fn execute_spell_with_target_data_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_id: i32,
        target_guid: ObjectGuid,
        target_data: SpellTargetData,
    ) -> Result<(), &'static str> {
        use wow_packet::packets::spell::SpellCastVisual;

        wow_world_spell::execute_spell_with_visual_and_target_data_and_generator_like_cpp(
            self,
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
    /// Execute a spell with full visual/cast info — apply effects, set cooldown, send SMSG_SPELL_GO.
    ///
    /// Called after cast time completes or for instant-cast spells.
    /// Supports: instakill (type 1), damage (type 2), aura application (type 6), heal (type 10).
    pub(crate) async fn execute_spell_with_visual_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_id: i32,
        target_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual: wow_packet::packets::spell::SpellCastVisual,
    ) -> Result<(), &'static str> {
        wow_world_spell::execute_spell_with_visual_and_target_data_and_generator_like_cpp(
            self,
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
}
