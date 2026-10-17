// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the canonical-map object queries and the
//! dynamic-object VALUES publication moved to
//! `wow_world_entities::visibility_session` (#1363 slice 11a): one-call method
//! forwards with this session as the host, so no scenario call site changes.

#![cfg(test)]

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn visible_dynamic_objects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::DynamicObjectCreateData>> {
        wow_world_entities::visible_dynamic_objects_from_canonical_map_like_cpp(
            self,
            map_id,
            position,
            visibility_radius,
        )
    }
}
