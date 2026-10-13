// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Nearby creature and gameobject delivery on world entry and transfer
//! (#1363 slice 8a; moved from the World
//! `handlers/character/visibility/{creatures,gameobjects}.rs`).

use std::collections::HashSet;

use tracing::{debug, info, warn};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_entities::{GAMEOBJECT_TYPE_FISHING_HOLE, GameObjectTemplateData};
use wow_packet::packets::update::{GameObjectCreateData, UpdateBlock, UpdateObject};
use wow_world_core::creature_projection::creature_create_data_like_cpp;

use super::VisibilitySessionHostLikeCpp;
use super::spawn::{
    DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP, is_within_2d_visibility_range_like_cpp,
    materialize_creature_spawn_row_with_catalogs_like_cpp, represented_go_state_from_i8_like_cpp,
};
use crate::{
    CreatureSpawnCatalogsLikeCpp, MaterializedCreatureSpawnLikeCpp, RepresentedGameObjectUseState,
};

pub(super) fn viewer_creature_create_block_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    spawn: &MaterializedCreatureSpawnLikeCpp,
) -> UpdateBlock {
    let mut viewer_create_data = spawn.create_data.clone();
    viewer_create_data.npc_flags = host.represented_viewer_dependent_creature_npc_flags_like_cpp(
        spawn.guid,
        viewer_create_data.npc_flags,
    );
    UpdateObject::create_creature_block(viewer_create_data, &spawn.position)
}

/// Send nearby creatures to the client as UpdateObject packets.
///
/// Queries the world database for creatures within visibility range
/// on the player's map, builds CreatureCreateData for each, and sends
/// a batched UpdateObject.
pub async fn send_nearby_creatures_with_catalogs_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    catalogs: &CreatureSpawnCatalogsLikeCpp,
    map_id: u16,
    position: &Position,
    _zone_id: u32,
) {
    let map_creatures = host.visible_world_creatures_from_map_like_cpp(map_id, position);
    if host.core().has_world_map_manager_like_cpp() {
        let mut blocks = Vec::with_capacity(map_creatures.len());
        let mut visible_guids = Vec::with_capacity(map_creatures.len());
        for creature in &map_creatures {
            let guid = creature.guid();
            // C++ `Player::UpdateVisibilityOf` (Player.cpp) only builds a CREATE block
            // when the object is NOT already in `m_clientGUIDs` (`!HaveAtClient`).
            // Re-creating an object the client already knows sends a duplicate CREATE,
            // which the Wrath client rejects by resetting the connection. This function
            // runs on world-port/spawn and must not re-create already-known creatures.
            visible_guids.push(guid);
            if host.core().client_visible_guids_like_cpp.contains(&guid) {
                continue;
            }
            let mut create_data = creature_create_data_like_cpp(creature);
            create_data.health = i64::from(creature.current_hp());
            create_data.max_health = i64::from(creature.max_hp());
            create_data.level = creature.level();
            create_data.npc_flags = creature.npc_flags_mask_like_cpp();
            create_data.npc_flags = host.represented_viewer_dependent_creature_npc_flags_like_cpp(
                guid,
                create_data.npc_flags,
            );
            create_data.current_area_id = 0;
            blocks.push(UpdateObject::create_creature_block_with_spline(
                create_data,
                &creature.position(),
                creature.active_move_spline_like_cpp().cloned(),
            ));
        }

        // Publish the creature membership and its create blocks as one step;
        // see `publish_transition_like_cpp`.
        let visibility_like_cpp = host.core().client_visible_guids_like_cpp.clone();
        visibility_like_cpp.publish_transition_like_cpp(
            |guid| !guid.is_any_type_creature(),
            visible_guids.iter().copied(),
            || {
                if blocks.is_empty() {
                    return;
                }
                let update = UpdateObject::create_creatures(blocks, map_id);
                if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
                    for line in update.debug_create_summary_like_cpp() {
                        info!("RUST_UPDATEOBJECT map_owned_creatures {line}");
                    }
                }
                host.send_packet(&update);
            },
        );
        host.visibility_mut()
            .set_last_visibility_pos_like_cpp(*position);
        debug!(
            "Sent {} map-owned creatures to account {} on map {}",
            visible_guids.len(),
            host.core().account_id,
            map_id
        );
        return;
    }

    let port = match host
        .lifecycle()
        .visibility_spawn_catalog_persistence_port_like_cpp()
    {
        Some(port) => port,
        None => {
            host.core()
                .client_visible_guids_like_cpp
                .retain(|guid| !guid.is_any_type_creature());
            host.visibility_mut()
                .set_last_visibility_pos_like_cpp(*position);
            warn!("No world database — skipping creature spawn");
            return;
        }
    };

    let x_min = position.x - DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
    let x_max = position.x + DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
    let y_min = position.y - DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
    let y_max = position.y + DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;

    let rows = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        port.load_creatures_in_bounds_like_cpp(
            wow_persistence::VisibilitySpawnCatalogRequestLikeCpp {
                map_id,
                x_min,
                x_max,
                y_min,
                y_max,
            },
        ),
    )
    .await
    {
        Ok(wow_persistence::VisibilitySpawnCatalogOutcomeLikeCpp::Loaded(rows)) => rows,
        Ok(wow_persistence::VisibilitySpawnCatalogOutcomeLikeCpp::Failed { reason }) => {
            warn!("Failed to query creatures for map {map_id}: {reason}");
            return;
        }
        Err(_) => {
            warn!("Creature query timed out for map {map_id}");
            return;
        }
    };

    if rows.is_empty() {
        host.core()
            .client_visible_guids_like_cpp
            .retain(|guid| !guid.is_any_type_creature());
        host.visibility_mut()
            .set_last_visibility_pos_like_cpp(*position);
        return;
    }

    let mut blocks = Vec::new();
    let mut visible_guids = Vec::new();
    for row in &rows {
        let Some(spawn) = materialize_creature_spawn_row_with_catalogs_like_cpp(
            host,
            catalogs,
            map_id,
            row,
            position,
            DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP,
        ) else {
            continue;
        };

        {
            let (s, mut h) = host.split_world_entities_mut();
            s.register_materialized_creature_spawn_like_cpp(&mut h, map_id, &spawn)
        };
        blocks.push(viewer_creature_create_block_like_cpp(host, &spawn));
        visible_guids.push(spawn.guid);
    }

    if blocks.is_empty() {
        return;
    }

    let count = blocks.len();
    let update = UpdateObject::create_creatures(blocks, map_id);
    if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
        for line in update.debug_create_summary_like_cpp() {
            info!("RUST_UPDATEOBJECT nearby_creatures {line}");
        }
    }
    // Mirror C++ Player::m_clientGUIDs semantics: this is the exact set
    // of creatures sent to this client, not every creature loaded on map.
    // The membership and its packet are published as one step; see
    // `publish_transition_like_cpp`.
    let visibility_like_cpp = host.core().client_visible_guids_like_cpp.clone();
    visibility_like_cpp.publish_transition_like_cpp(
        |guid| !guid.is_any_type_creature(),
        visible_guids.iter().copied(),
        || host.send_packet(&update),
    );
    host.visibility_mut()
        .set_last_visibility_pos_like_cpp(*position);
    let mob_count = visible_guids
        .iter()
        .filter(|g| {
            host.core_mut()
                .mutate_world_creature(**g, |creature| creature.npc_flags() == 0)
                .unwrap_or(false)
        })
        .count();
    let npc_count = visible_guids.len().saturating_sub(mob_count);
    info!(
        "Sent {} creatures ({} mobs / {} npcs) to account {} on map {}",
        count,
        mob_count,
        npc_count,
        host.core().account_id,
        map_id
    );
}

/// Send nearby gameobjects to the client as UpdateObject packets.
pub async fn send_nearby_gameobjects<H: VisibilitySessionHostLikeCpp>(
    host: &mut H,
    map_id: u16,
    position: &Position,
    _zone_id: u32,
) {
    if let Some(gameobjects) = host.visible_gameobjects_from_canonical_map_like_cpp(
        map_id,
        position,
        DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP,
    ) {
        if gameobjects.is_empty() {
            host.core()
                .client_visible_guids_like_cpp
                .retain(|guid| !guid.is_game_object());
            return;
        }

        let go_guids: HashSet<_> = gameobjects.iter().map(|go| go.guid).collect();
        // C++ `Player::UpdateVisibilityOf` (Player.cpp) only CREATES gameobjects NOT
        // already in `m_clientGUIDs` (`!HaveAtClient`). Re-creating a known gameobject
        // sends a duplicate CREATE, which the Wrath client rejects by resetting the
        // connection. This function runs on world-port/spawn and must skip known GOs.
        let known_guids = &host.core().client_visible_guids_like_cpp;
        let blocks = gameobjects
            .into_iter()
            .filter(|go| !known_guids.contains(&go.guid))
            .map(UpdateObject::create_gameobject_block)
            .collect::<Vec<_>>();
        let count = blocks.len();
        host.core()
            .client_visible_guids_like_cpp
            .retain(|guid| !guid.is_game_object());
        host.core()
            .client_visible_guids_like_cpp
            .extend(go_guids.iter().copied());
        if !blocks.is_empty() {
            host.send_packet(&UpdateObject::create_world_objects(blocks, map_id));
        }
        debug!(
            "Sent {} new canonical gameobjects to account {} on map {}",
            count,
            host.core().account_id,
            map_id
        );
        return;
    }

    let port = match host
        .lifecycle()
        .visibility_spawn_catalog_persistence_port_like_cpp()
    {
        Some(port) => port,
        None => return,
    };

    let x_min = position.x - DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
    let x_max = position.x + DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
    let y_min = position.y - DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
    let y_max = position.y + DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;

    let gameobjects = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        port.load_gameobjects_in_bounds_like_cpp(
            wow_persistence::VisibilitySpawnCatalogRequestLikeCpp {
                map_id,
                x_min,
                x_max,
                y_min,
                y_max,
            },
        ),
    )
    .await
    {
        Ok(wow_persistence::VisibilitySpawnCatalogOutcomeLikeCpp::Loaded(rows)) => rows,
        Ok(wow_persistence::VisibilitySpawnCatalogOutcomeLikeCpp::Failed { reason }) => {
            warn!("Failed to query gameobjects for map {map_id}: {reason}");
            return;
        }
        Err(_) => {
            warn!("Gameobject query timed out for map {map_id}");
            return;
        }
    };

    if gameobjects.is_empty() {
        return;
    }

    let realm_id = host.core().realm_id();
    let mut blocks = Vec::new();
    let mut go_guids: Vec<wow_core::ObjectGuid> = Vec::new();
    for row in &gameobjects {
        let spawn_guid = row.spawn_guid;
        let entry = row.entry;
        let [pos_x, pos_y, pos_z, orientation] = row.position;
        if !is_within_2d_visibility_range_like_cpp(
            position,
            pos_x,
            pos_y,
            DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP,
        ) {
            continue;
        }
        let [rot0, rot1, rot2, rot3] = row.rotation;
        // #NEXT.R8.ENTITIES.1216: GameObjectData.ParentRotation from per-spawn
        // gameobject_addon (cols 58-61); NULL (no addon) -> identity (0,0,0,1).
        let [parent_rot0, parent_rot1, parent_rot2, parent_rot3] = row.parent_rotation;
        // C++ GameObject::Create defaults animProgress to 255 (GameObject.cpp:1068,1089);
        // match the other GO paths (canonical ~6920) instead of 0. #NEXT.R8.ENTITIES.1218.
        let anim_progress = row.anim_progress;
        let state = row.state;
        let go_type = row.go_type;
        let display_id = row.display_id;
        let scale = row.scale;
        let template_data = row.template_data.map(|raw| u32::try_from(raw).unwrap_or(0));
        let data2 = template_data[2];
        let data3 = template_data[3];
        let template = GameObjectTemplateData::new(u32::from(go_type), template_data);
        let phase_use_flags = row.phase_use_flags;
        let phase_id = row.phase_id;
        let phase_group_id = row.phase_group_id;
        let terrain_swap_map = row.terrain_swap_map;
        let effective_flags = row.effective_flags;
        let effective_faction = row.effective_faction;
        let override_source_known = row.override_source_known;

        // Skip gameobjects with no display
        if display_id == 0 {
            continue;
        }

        let (target_phase_shift, _) = host.hub_ref().catalogs.db_spawn_phase_shift_like_cpp(
            map_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
        );
        if !host.can_see_phase_shift_like_cpp(&target_phase_shift) {
            continue;
        }

        let guid = ObjectGuid::create_world_object(
            HighGuid::GameObject,
            0,
            realm_id,
            map_id,
            1,
            entry,
            spawn_guid as i64,
        );

        let go_pos = Position::new(pos_x, pos_y, pos_z, orientation);
        let dynamic_flags = host.represented_gameobject_dynamic_flags_for_player_like_cpp(
            entry,
            &RepresentedGameObjectUseState {
                go_type: Some(go_type),
                go_state: represented_go_state_from_i8_like_cpp(state),
                ..Default::default()
            },
        );
        let create_data = GameObjectCreateData {
            guid,
            entry,
            dynamic_flags,
            display_id,
            go_type,
            position: go_pos,
            rotation: [rot0, rot1, rot2, rot3],
            anim_progress,
            state,
            // C++ ObjectMgr initializes SQL `GameObjectData::artKit` to zero.
            art_kit: 0,
            created_by: ObjectGuid::EMPTY,
            faction_template: effective_faction as i32,
            gameobject_flags: effective_flags,
            world_effect_id: 0,
            scale,
            level: 0, // non-transport GameObject: Level unused (period via AnimationData)
            parent_rotation: [parent_rot0, parent_rot1, parent_rot2, parent_rot3],
        };

        blocks.push(UpdateObject::create_gameobject_block(create_data));
        go_guids.push(guid);
        {
            let (state, mut hub) = host.split_world_entities_mut();
            state.record_represented_gameobject_db_phase_shift_like_cpp(
                &mut hub,
                guid,
                map_id,
                phase_use_flags,
                phase_id,
                phase_group_id,
                terrain_swap_map,
            )
        };
        {
            let (state, mut hub) = host.split_world_entities_mut();
            state.record_represented_gameobject_runtime_state_like_cpp(
                &mut hub, map_id, guid, entry, go_pos, go_type,
            )
        };
        host.world_entities_mut()
            .record_represented_gameobject_override_like_cpp(
                guid,
                effective_flags,
                effective_faction,
                override_source_known,
            );
        if u32::from(go_type) == GAMEOBJECT_TYPE_FISHING_HOLE {
            let max_opens = if data2 <= data3 {
                host.core_mut().represented_urand_u32_like_cpp(data2, data3)
            } else {
                data2
            };
            host.world_entities_mut()
                .record_represented_fishing_hole_max_opens_like_cpp(guid, max_opens);
            host.world_entities_mut()
                .record_represented_fishing_hole_radius_like_cpp(guid, template_data[0]);
        }
        host.world_entities_mut()
            .record_represented_gameobject_interact_radius_override_like_cpp(
                guid,
                template.get_interact_radius_override_like_cpp(),
            );
        host.world_entities_mut()
            .record_represented_gameobject_lock_id_like_cpp(guid, template.get_lock_id_like_cpp());
        host.world_entities_mut()
            .record_represented_gameobject_display_model_like_cpp(
                guid,
                display_id,
                scale,
                [rot0, rot1, rot2, rot3],
            );
        host.world_entities_mut()
            .record_represented_gameobject_anim_progress_like_cpp(guid, anim_progress);
    }

    if blocks.is_empty() {
        return;
    }

    host.core()
        .client_visible_guids_like_cpp
        .retain(|guid| !guid.is_game_object());
    host.core()
        .client_visible_guids_like_cpp
        .extend(go_guids.iter().copied());
    let count = blocks.len();
    let update = UpdateObject::create_world_objects(blocks, map_id);
    host.send_packet(&update);
    debug!(
        "Sent {} gameobjects to account {} on map {}",
        count,
        host.core().account_id,
        map_id
    );
}
