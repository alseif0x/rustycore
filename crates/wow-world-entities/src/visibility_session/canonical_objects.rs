// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The canonical-map object queries behind the visibility update (#1363 slice
//! 11a; moved from the World `session/instances/map_resolution.rs`, with the
//! dynamic-object create data from `session/object_updates.rs`): the
//! transports, dynamic objects, area triggers, corpses, scene objects and
//! conversations a viewer may see on its canonical map, read under one map
//! manager lock each.

use super::VisibilitySessionHostLikeCpp;

pub fn dynamic_object_create_data_from_canonical_like_cpp(
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

/// Snapshot every in-world transport on the player's canonical map that
/// survives the viewer phase check.  C++ `Map::SendInitTransports` and the
/// transport add/remove fan-out use the map reference list rather than a
/// distance radius, so this is intentionally a whole-map typed scan.
pub(crate) fn visible_transports_from_canonical_map_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    map_id: u16,
) -> Option<Vec<wow_packet::packets::update::GameObjectCreateData>> {
    let requested_map_id = u32::from(map_id);
    let player_map_key = host.core().current_canonical_player_map_key_like_cpp();
    let viewer_phase_shift = host.represented_player_phase_shift_like_cpp();
    let manager = host.core().canonical_map_manager.as_ref()?;
    let Ok(manager) = manager.lock() else {
        return None;
    };
    let map = match player_map_key {
        Some(key) if key.map_id == requested_map_id => {
            manager.find_map(key.map_id, key.instance_id)?
        }
        Some(_) => return None,
        None => manager.find_map(requested_map_id, 0)?,
    };

    let mut transports = Vec::new();
    for guid in map.map().typed_transport_guids_like_cpp() {
        let Some(transport) = map.map().get_typed_transport_like_cpp(guid) else {
            continue;
        };
        let world = transport.world();
        if !world.object().is_in_world()
            || world.map_id() != requested_map_id
            || !viewer_phase_shift
                .as_ref()
                .is_some_and(|viewer| viewer.can_see(world.phase_shift()))
        {
            continue;
        }
        transports.push(
            wow_world_core::entity_update_bridge::transport_create_data_from_entity_like_cpp(
                transport,
            ),
        );
    }
    Some(transports)
}
pub fn visible_dynamic_objects_from_canonical_map_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    map_id: u16,
    position: &wow_core::Position,
    visibility_radius: f32,
) -> Option<Vec<wow_packet::packets::update::DynamicObjectCreateData>> {
    let requested_map_id = u32::from(map_id);
    let player_map_key = host.core().current_canonical_player_map_key_like_cpp();
    let source_combat_reach =
        super::queries::represented_visibility_source_combat_reach_like_cpp(host);
    let manager = host.core().canonical_map_manager.as_ref()?;
    let Ok(manager) = manager.lock() else {
        return None;
    };
    let map = match player_map_key {
        Some(key) if key.map_id == requested_map_id => {
            manager.find_map(key.map_id, key.instance_id)?
        }
        Some(_) => return None,
        None => manager.find_map(requested_map_id, 0)?,
    };
    let nearby = map.map().nearby_cell_guids_like_cpp(
        position.x,
        position.y,
        visibility_radius + source_combat_reach,
    );
    let mut dynamic_objects = Vec::new();

    for guid in nearby
        .world
        .dynamic_objects
        .into_iter()
        .chain(nearby.grid.dynamic_objects)
    {
        let Some(dynamic_object) = map.map().get_typed_dynamic_object(guid) else {
            continue;
        };
        let object = dynamic_object.world();
        if !object.object().is_in_world()
            || object.map_id() != u32::from(map_id)
            || !wow_core::visibility_distance_allows_like_cpp(
                position,
                source_combat_reach,
                &object.position(),
                object.combat_reach(),
                visibility_radius,
            )
        {
            continue;
        }
        dynamic_objects.push(dynamic_object_create_data_from_canonical_like_cpp(
            guid,
            dynamic_object,
        ));
    }

    Some(dynamic_objects)
}
pub(crate) fn visible_area_triggers_from_canonical_map_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    map_id: u16,
    position: &wow_core::Position,
    visibility_radius: f32,
) -> Option<Vec<wow_packet::packets::update::AreaTriggerCreateData>> {
    let requested_map_id = u32::from(map_id);
    let player_map_key = host.core().current_canonical_player_map_key_like_cpp();
    let source_combat_reach =
        super::queries::represented_visibility_source_combat_reach_like_cpp(host);
    let viewer_phase_shift = host.represented_player_phase_shift_like_cpp();
    let manager = host.core().canonical_map_manager.as_ref()?;
    let Ok(manager) = manager.lock() else {
        return None;
    };
    let map = match player_map_key {
        Some(key) if key.map_id == requested_map_id => {
            manager.find_map(key.map_id, key.instance_id)?
        }
        Some(_) => return None,
        None => manager.find_map(requested_map_id, 0)?,
    };
    let nearby = map.map().nearby_cell_guids_like_cpp(
        position.x,
        position.y,
        visibility_radius + source_combat_reach,
    );
    let mut area_triggers = Vec::new();

    for guid in nearby.grid.area_triggers {
        let Some(create_data) = map
            .map()
            .with_area_trigger_like_cpp(guid, |area_trigger| {
                let object = area_trigger.world();
                if !object.object().is_in_world()
                    || object.map_id() != u32::from(map_id)
                    || !wow_core::visibility_distance_allows_like_cpp(
                        position,
                        source_combat_reach,
                        &object.position(),
                        object.combat_reach(),
                        visibility_radius,
                    )
                    || !viewer_phase_shift
                        .as_ref()
                        .is_some_and(|viewer| viewer.can_see(object.phase_shift()))
                    || area_trigger.is_server_side()
                    || area_trigger.is_removed()
                {
                    return None;
                }
                Some(
                    wow_world_core::entity_update_bridge::area_trigger_create_data_from_entity_like_cpp(
                        area_trigger,
                    ),
                )
            })
            .flatten()
        else {
            continue;
        };
        area_triggers.push(create_data);
    }

    Some(area_triggers)
}
pub(crate) fn visible_misc_objects_from_canonical_map_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    map_id: u16,
    position: &wow_core::Position,
    visibility_radius: f32,
) -> Option<(
    Vec<wow_packet::packets::update::CorpseCreateData>,
    Vec<wow_packet::packets::update::SceneObjectCreateData>,
    Vec<wow_packet::packets::update::ConversationCreateData>,
)> {
    let requested_map_id = u32::from(map_id);
    let player_map_key = host.core().current_canonical_player_map_key_like_cpp();
    let source_combat_reach =
        super::queries::represented_visibility_source_combat_reach_like_cpp(host);
    let viewer_phase_shift = host.represented_player_phase_shift_like_cpp();
    let manager = host.core().canonical_map_manager.as_ref()?;
    let Ok(manager) = manager.lock() else {
        return None;
    };
    let map = match player_map_key {
        Some(key) if key.map_id == requested_map_id => {
            manager.find_map(key.map_id, key.instance_id)?
        }
        Some(_) => return None,
        None => manager.find_map(requested_map_id, 0)?,
    };
    let nearby = map.map().nearby_cell_guids_like_cpp(
        position.x,
        position.y,
        visibility_radius + source_combat_reach,
    );

    let allows_world_object = |world: &wow_entities::WorldObject| {
        world.object().is_in_world()
            && world.map_id() == requested_map_id
            && viewer_phase_shift
                .as_ref()
                .is_some_and(|viewer| viewer.can_see(world.phase_shift()))
            && wow_core::visibility_distance_allows_like_cpp(
                position,
                source_combat_reach,
                &world.position(),
                world.combat_reach(),
                visibility_radius,
            )
    };

    let mut corpses = Vec::new();
    for guid in nearby.world.corpses.into_iter().chain(nearby.grid.corpses) {
        let Some(corpse) = map.map().get_typed_corpse(guid) else {
            continue;
        };
        if allows_world_object(corpse.world()) {
            corpses.push(
                wow_world_core::entity_update_bridge::corpse_create_data_from_entity_like_cpp(
                    corpse,
                ),
            );
        }
    }

    let mut scene_objects = Vec::new();
    for guid in nearby.grid.scene_objects {
        let Some(create_data) = map
            .map()
            .with_scene_object_like_cpp(guid, |scene_object| {
                allows_world_object(scene_object.world()).then(|| {
                    wow_world_core::entity_update_bridge::scene_object_create_data_from_entity_like_cpp(
                        scene_object,
                    )
                })
            })
            .flatten()
        else {
            continue;
        };
        scene_objects.push(create_data);
    }

    let mut conversations = Vec::new();
    for guid in nearby.grid.conversations {
        let Some(create_data) = map
            .map()
            .with_conversation_like_cpp(guid, |conversation| {
                (!conversation.is_removed() && allows_world_object(conversation.world())).then(
                || {
                    wow_world_core::entity_update_bridge::conversation_create_data_from_entity_like_cpp(
                        conversation,
                        &host.core().locale,
                    )
                },
            )
            })
            .flatten()
        else {
            continue;
        };
        conversations.push(create_data);
    }

    Some((corpses, scene_objects, conversations))
}
