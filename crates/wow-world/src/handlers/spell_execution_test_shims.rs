// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the spell execution moved to
//! `wow_world_spell` (#1363 slice 9d).
//!
//! The first impl block holds the `#[cfg(test)]` overloads of
//! `session/spell_effects/execution_overloads.rs`, verbatim except that their
//! calls to the moved execution entry points are free calls: they were already
//! test-only. The second holds one-call method forwards for the moved
//! functions the scenarios and the `execution_overloads` test fixture call,
//! with this session as the host, so no scenario call site changes. It also
//! mounts the `execution_overloads` test fixture, whose parent module left
//! with the production overload.

#![cfg(test)]

use crate::session::*;
use wow_core::{ObjectGuid, Position};
use wow_entities::SpellCastMetadata;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;
use wow_world_spell::PlayerCastPublicationPhaseLikeCpp;

#[path = "../../unit_tests/session/spell_effects/execution_overloads/test_only.rs"]
mod execution_overloads_test_only;

impl WorldSession {
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
        wow_world_spell::execute_spell_with_visual_and_target_data_and_generator_like_cpp(
            self,
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
        wow_world_spell::execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
            self,
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

impl WorldSession {
    pub(crate) fn represented_transport_destination_world_position_like_cpp(
        &self,
        transport_guid: ObjectGuid,
        transport_offset: Position,
    ) -> Option<Position> {
        wow_world_spell::represented_transport_destination_world_position_like_cpp(
            self,
            transport_guid,
            transport_offset,
        )
    }
    pub(crate) fn sync_represented_creature_threat_to_canonical_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
        threat_value: f32,
    ) {
        wow_world_spell::sync_represented_creature_threat_to_canonical_like_cpp(
            self,
            creature_guid,
            attacker_guid,
            threat_value,
        )
    }
    pub(crate) fn apply_modify_spell_charges_effect_like_cpp(
        &mut self,
        damage: i32,
        charge_category_id: i32,
        target_guid: ObjectGuid,
    ) -> u32 {
        wow_world_spell::apply_modify_spell_charges_effect_like_cpp(
            self,
            damage,
            charge_category_id,
            target_guid,
        )
    }
    pub(crate) fn player_cast_publication_like_cpp(
        &self,
        spell: &wow_data::SpellInfo,
        metadata: &SpellCastMetadata,
        phase: PlayerCastPublicationPhaseLikeCpp,
    ) -> (wow_packet::packets::spell::SpellCastData, u32) {
        wow_world_spell::player_cast_publication_like_cpp(self, spell, metadata, phase)
    }
    pub(crate) fn publish_player_cast_frame_like_cpp(
        &mut self,
        metadata: SpellCastMetadata,
        bytes: Vec<u8>,
    ) {
        wow_world_spell::publish_player_cast_frame_like_cpp(self, metadata, bytes)
    }
    pub(crate) fn take_spell_power_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        cast_id: ObjectGuid,
        spell_id: i32,
        visual: &SpellCastVisual,
    ) -> bool {
        wow_world_spell::take_spell_power_like_cpp(self, spell_info, cast_id, spell_id, visual)
    }
}
