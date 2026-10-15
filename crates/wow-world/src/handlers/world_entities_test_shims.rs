// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the world-entity bodies moved to
//! `wow_world_entities` (#1363 slice 10a): one-call method forwards for the
//! moved functions the scenarios and the unit_tests F3 shims call, with this
//! session as the host, so no scenario call site changes, and the scripted
//! goober-preamble test entry (slice 4a), moved verbatim from the
//! cfg(test)-mounted `session/world_entities/gameobject_use_scripted.rs`.

#![cfg(test)]

use crate::session::*;
use wow_core::{ObjectGuid, Position};

/// The #1241 F3 entry point of the creature publication (whose bodies moved
/// here in #1363 slice 10a), mounted here instead of from
/// `session/world_entities/creature_publication.rs`.
#[path = "../../unit_tests/session/world_entities/creature_publication/f3_shims.rs"]
mod creature_publication_f3_shims;
/// The #1241 F3 entry points of the per-type gameobject uses (whose bodies
/// moved to `wow_world_entities::gameobject_use_kinds` in #1363 slice 4a),
/// mounted here instead of from the emptied
/// `session/world_entities/gameobject_use_types.rs`.
#[path = "../../unit_tests/session/world_entities/gameobject_use_types/f3_shims.rs"]
mod gameobject_use_types_f3_shims;

impl WorldSession {
    pub(crate) fn represented_can_receive_creature_message_to_set_like_cpp(
        &self,
        guid: ObjectGuid,
        creature: &wow_entities::Creature,
        required_3d: bool,
    ) -> bool {
        wow_world_entities::represented_can_receive_creature_message_to_set_like_cpp(
            self,
            guid,
            creature,
            required_3d,
        )
    }
    pub(crate) fn apply_creature_aura_with_provenance_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        effect_mask: u32,
        duration_ms: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str> {
        wow_world_entities::apply_creature_aura_with_provenance_like_cpp(
            self,
            spell_id,
            caster_guid,
            target_guid,
            effect_mask,
            duration_ms,
            provenance,
        )
    }
}

impl WorldSession {
    pub(crate) fn visible_gameobjects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::GameObjectCreateData>> {
        wow_world_entities::visible_gameobjects_from_canonical_map_like_cpp(
            self,
            map_id,
            position,
            visibility_radius,
        )
    }
    pub(crate) fn visible_world_creatures_from_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
    ) -> Vec<wow_entities::Creature> {
        wow_world_entities::visible_world_creatures_from_map_like_cpp(self, map_id, position)
    }
    pub(crate) fn visible_creatures_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
    ) -> Option<Vec<wow_entities::Creature>> {
        wow_world_entities::visible_creatures_from_canonical_map_like_cpp(self, map_id, position)
    }
}

impl WorldSession {
    pub(crate) fn send_represented_gameobject_visual_despawn_from_last_update_like_cpp(
        &mut self,
    ) -> usize {
        wow_world_entities::send_represented_gameobject_visual_despawn_from_last_update_like_cpp(
            self,
        )
    }
}

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
