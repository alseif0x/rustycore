// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session consumption of map-owned `Map::SendObjectUpdates` snapshots.
//!
//! The map owns update-field mutation and clears masks while it still owns the
//! map guard.  This module consumes the resulting owned snapshots afterwards,
//! applies the receiver's committed visibility, and writes packets without
//! retaining a map or directory guard across the send.

use super::WorldSession;

pub(crate) fn dynamic_object_create_data_from_canonical_like_cpp(
    guid: wow_core::ObjectGuid,
    dynamic_object: &wow_entities::DynamicObject,
) -> wow_packet::packets::update::DynamicObjectCreateData {
    let object = dynamic_object.world();
    let object_data = object.object().object_data_values();
    let data = dynamic_object.data();
    wow_packet::packets::update::DynamicObjectCreateData {
        guid,
        entry_id: u32::try_from(object_data.entry_id).unwrap_or(0),
        dynamic_flags: object_data.dynamic_flags,
        scale: object_data.scale,
        position: object.position(),
        caster: data.caster,
        dynamic_object_type: data.dynamic_object_type,
        spell_visual_id: data.spell_visual_id,
        spell_id: data.spell_id,
        radius: data.radius,
        cast_time_ms: data.cast_time_ms,
    }
}

impl WorldSession {
    pub(crate) fn send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
        &mut self,
    ) -> usize {
        let (state, mut hub) = crate::session::split_visibility_mut(self);
        state
            .send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
                &mut hub,
            )
    }
}
