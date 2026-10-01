// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn broadcast_creature_packet_from_position_to_visible_set_realm_like_cpp(
        &self,
        source_guid: ObjectGuid,
        source_position: Position,
        bytes: Vec<u8>,
    ) {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.broadcast_creature_packet_from_position_to_visible_set_realm_like_cpp(
            hub,
            source_guid,
            source_position,
            bytes,
        )
    }
}
