// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the world-entity bodies moved to
//! `wow_world_entities` (#1363 slice 10a): one-call method forwards for the
//! moved functions the scenarios and the unit_tests F3 shims call, with this
//! session as the host, so no scenario call site changes.

#![cfg(test)]

use crate::session::*;
use wow_core::ObjectGuid;

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
