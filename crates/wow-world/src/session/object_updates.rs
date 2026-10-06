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

pub(crate) fn represented_gameobject_dynamic_flags_update_like_cpp(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    dynamic_flags: u32,
) -> Option<wow_packet::packets::update::UpdateObject> {
    let mut mask = wow_entities::UpdateMask::new(wow_entities::OBJECT_DATA_BITS);
    mask.set(wow_entities::OBJECT_DATA_PARENT_BIT);
    mask.set(wow_entities::OBJECT_DATA_DYNAMIC_FLAGS_BIT);
    let values_update = wow_entities::GameObjectValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
        object_data: Some(wow_entities::ObjectDataUpdate {
            mask,
            values: wow_entities::ObjectDataValues {
                entry_id: 0,
                dynamic_flags,
                scale: 0.0,
            },
        }),
        game_object_data: None,
    };
    crate::entity_update_bridge::game_object_values_update_to_update_object(
        guid,
        map_id,
        &values_update,
    )
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
