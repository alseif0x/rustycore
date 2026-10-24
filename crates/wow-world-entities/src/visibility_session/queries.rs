// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The visible-object queries behind the visibility bodies (#1363 slice 10a;
//! moved from the World `session/world_entities/{gameobject_query,gameobject,creature_query,aggro}.rs`
//! and, for the per-viewer GoState projection, from
//! `wow_world_quest::quest::visibility::gameobject_flags`): the nearby
//! gameobject and creature candidates of `Player::UpdateVisibilityForPlayer`
//! with their canonical create data, and the seer's combat reach that widens
//! the `WorldObject::CanSeeOrDetect` range.

use std::time::Instant;

use wow_core::ObjectGuid;
use wow_entities::GoState;

use super::VisibilitySessionHostLikeCpp;
use crate::RepresentedGameObjectUseState;
use crate::world_entities_host::WorldEntitiesHostLikeCpp;

pub fn visible_gameobjects_from_canonical_map_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    map_id: u16,
    position: &wow_core::Position,
    visibility_radius: f32,
) -> Option<Vec<wow_packet::packets::update::GameObjectCreateData>> {
    let requested_map_id = u32::from(map_id);
    let player_map_key = host.core().current_canonical_player_map_key_like_cpp();
    let source_combat_reach = represented_visibility_source_combat_reach_like_cpp(host);
    // Resolve Player-owned phase state before entering the map. The
    // canonical Player handle uses this same manager mutex, so consulting
    // it from the entity read scope would recursively lock the manager.
    let viewer_phase_shift = host.represented_player_phase_shift_like_cpp();
    let candidates = {
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
        nearby
            .grid
            .gameobjects
            .into_iter()
            .filter_map(|guid| {
                map.map()
                    .with_game_object_like_cpp(guid, |gameobject| (guid, gameobject.clone()))
            })
            .collect::<Vec<_>>()
    };
    let now = Instant::now();
    let mut gameobjects = Vec::new();

    for (guid, gameobject) in candidates {
        let object = gameobject.world();
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
        if let Some(phase_shift) = host
            .world_entities()
            .represented_gameobject_phase_shift_like_cpp(guid)
            && !viewer_phase_shift
                .as_ref()
                .is_some_and(|viewer| viewer.can_see(phase_shift))
        {
            continue;
        }
        let state = host
            .world_entities()
            .represented_gameobject_use_state_like_cpp(guid);
        if state.is_some_and(|state| {
            state
                .per_player_despawn_until
                .is_some_and(|until| until > now)
        }) {
            continue;
        }

        let create_data = if let Some(state) = state {
            let Some(display_id) = state.display_id else {
                continue;
            };
            let Some(go_type) = state.go_type else {
                continue;
            };
            let state_for_viewer =
                represented_gameobject_go_state_for_viewer_like_cpp(state, host.player_guid(), now);

            wow_packet::packets::update::GameObjectCreateData {
                guid,
                entry: object.object().entry(),
                dynamic_flags: host.represented_gameobject_dynamic_flags_for_player_like_cpp(
                    object.object().entry(),
                    state,
                ),
                display_id,
                go_type,
                position: object.position(),
                rotation: state.rotation,
                anim_progress: 255,
                state: state_for_viewer as i8,
                art_kit: gameobject.data().art_kit,
                created_by: state.owner_guid.unwrap_or(ObjectGuid::EMPTY),
                faction_template: state.faction_template.unwrap_or(0) as i32,
                gameobject_flags: state.gameobject_flags,
                world_effect_id: 0,
                scale: state.scale,
                level: 0, // non-transport GameObject: Level unused (period via AnimationData)
                // Canonical state path does not yet record per-spawn ParentRotation;
                // identity until state population is added (#NEXT.R8.ENTITIES.1216 follow-up).
                parent_rotation: [0.0, 0.0, 0.0, 1.0],
            }
        } else {
            let Some(create_data) =
                gameobject_create_data_from_canonical_like_cpp(host, guid, &gameobject)
            else {
                continue;
            };
            create_data
        };

        gameobjects.push(create_data);
    }

    Some(gameobjects)
}

fn gameobject_create_data_from_canonical_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
    guid: ObjectGuid,
    gameobject: &wow_entities::GameObject,
) -> Option<wow_packet::packets::update::GameObjectCreateData> {
    let object = gameobject.world();
    let data = gameobject.data();
    let display_id = u32::try_from(data.display_id).ok()?;
    if display_id == 0 {
        return None;
    }
    let go_type = u8::try_from(data.type_id).ok()?;
    let go_state = match data.state {
        0 => Some(wow_entities::GoState::Active),
        1 => Some(wow_entities::GoState::Ready),
        2 => Some(wow_entities::GoState::Destroyed),
        24 => Some(wow_entities::GoState::TransportActive),
        25 => Some(wow_entities::GoState::TransportStopped),
        _ => None,
    };

    Some(wow_packet::packets::update::GameObjectCreateData {
        guid,
        entry: object.object().entry(),
        dynamic_flags: host.represented_gameobject_dynamic_flags_for_player_like_cpp(
            object.object().entry(),
            &RepresentedGameObjectUseState {
                go_type: Some(go_type),
                go_state,
                dynamic_flags: object.object().dynamic_flags(),
                ..Default::default()
            },
        ),
        display_id,
        go_type,
        position: object.position(),
        rotation: gameobject.local_rotation_like_cpp(),
        anim_progress: gameobject.go_anim_progress_like_cpp(),
        state: data.state,
        art_kit: data.art_kit,
        created_by: data.created_by,
        faction_template: data.faction_template,
        gameobject_flags: data.flags,
        world_effect_id: 0,
        scale: object.object().scale(),
        level: 0, // non-transport GameObject: Level unused (period via AnimationData)
        // Canonical GameObject entity does not yet carry per-spawn ParentRotation;
        // identity until the entity stores it (#NEXT.R8.ENTITIES.1216 follow-up).
        parent_rotation: [0.0, 0.0, 0.0, 1.0],
    })
}

/// Nearby creature candidates for one represented player position.
///
/// #1263 F6-8B/F6-8E. The canonical creature on the canonical map is the
/// one authority for the movement/visibility transition, so the
/// spline-bearing candidate set is selected from it, with its own phase and
/// `CanSeeOrDetect` filtering.
pub fn visible_world_creatures_from_map_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    map_id: u16,
    _position: &wow_core::Position,
) -> Vec<wow_entities::Creature> {
    // These candidates carry the canonical runtime state, including the
    // live `Unit::movespline` a create block projects. The caller's position
    // is retained for API compatibility; spatial ownership comes from the
    // generation-stamped canonical Player and its actual viewpoint.
    visible_creatures_from_canonical_map_like_cpp(host, map_id, _position).unwrap_or_default()
}

pub fn visible_creatures_from_canonical_map_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    map_id: u16,
    _position: &wow_core::Position,
) -> Option<Vec<wow_entities::Creature>> {
    let requested_map_id = u32::from(map_id);
    let handle = host.core().player_handle_like_cpp?;
    if host.player_guid()? != handle.guid() {
        return None;
    }
    let client_visible = host.core().client_visible_guids_like_cpp.snapshot_like_cpp();
    let manager = host.core().canonical_map_manager.as_ref()?;
    let Ok(manager) = manager.lock() else {
        return None;
    };
    let (player_map_key, _residence_revision) =
        manager.player_active_residence_revision_like_cpp(handle)?;
    if player_map_key.map_id != requested_map_id {
        return Some(Vec::new());
    }
    let map = manager.find_map(player_map_key.map_id, player_map_key.instance_id)?;
    let map = map.map();
    let selected = manager.with_player_like_cpp(handle, |player| {
        let player_world = player.unit().world();
        if !player_world.object().is_in_world()
            || player_world.map_id() != player_map_key.map_id
            || player_world.instance_id() != player_map_key.instance_id
        {
            return Vec::new();
        }

        let viewpoint = super::ghost_visibility::canonical_viewpoint(player, map);
        // Map::WorldObjectEnvironment intentionally uses the map's normal
        // visibility range. CinematicMgr::IsOnCinematic (which resolves
        // FlyByCamera rows) is not represented by this map-owned query.
        // Cell::Visit adds the center object's combat reach itself.
        let search_radius = player_world.get_sight_range(None, map) + viewpoint.combat_reach();
        let nearby = map.nearby_cell_guids_like_cpp(
            viewpoint.position().x,
            viewpoint.position().y,
            search_radius,
        );
        // Map::corpse_guid_by_player_like_cpp scans the map's entity records.
        // Resolve the current Player corpse once for this query, then reuse
        // the borrowed canonical Corpse while checking each candidate.
        let corpse = {
            let seer = player.unit();
            (seer.is_dead() && seer.data().health > 0)
                .then(|| {
                    map.corpse_guid_by_player_like_cpp(player.guid())
                        .and_then(|guid| map.get_typed_corpse(guid))
                })
                .flatten()
        };

        let mut candidates = Vec::new();
        for guid in nearby
            .world
            .creatures
            .into_iter()
            .chain(nearby.grid.creatures)
        {
            let Some(creature) = map
                .with_creature_or_pet_like_cpp(guid, |creature, _owner| {
                    let unit = creature.unit();
                    let world = unit.world();
                    if !world.object().is_in_world()
                        || world.map_id() != player_map_key.map_id
                        || world.instance_id() != player_map_key.instance_id
                    {
                        return None;
                    }

                    let context = super::ghost_visibility::unit_context(
                        player, unit, viewpoint, map, corpse,
                    );
                    let vehicle_visible = unit
                        .subsystems()
                        .vehicle
                        .vehicle_guid
                        .is_none_or(|vehicle| client_visible.contains(&vehicle));
                    let context = wow_entities::UnitVisibilityContext {
                        distance_allowed: context.distance_allowed && vehicle_visible,
                        ..context
                    };
                    player.unit().can_see_or_detect_unit_with_context(
                        unit, false, true, false, context,
                    )
                    .then(|| creature.clone())
                })
                .flatten()
            else {
                continue;
            };
            candidates.push(creature);
        }
        candidates
    })?;
    drop(manager);

    Some(
        selected
            .into_iter()
            .map(|creature| {
                // #1263 F6-8B: selected candidates carry the canonical runtime
                // state, including the live MoveSpline used by CREATE.
                let create_data =
                    wow_world_core::creature_projection::creature_create_data_like_cpp(&creature);
                wow_world_core::creature_projection::projected_creature_preserving_runtime_like_cpp(
                    creature,
                    &create_data,
                )
            })
            .collect(),
    )
}

pub fn represented_visibility_source_combat_reach_like_cpp<H: VisibilitySessionHostLikeCpp>(
    host: &H,
) -> f32 {
    let Some(player_guid) = host.player_guid() else {
        return 0.0;
    };
    let seer_guid = host
        .current_seer_guid_like_cpp()
        .filter(|guid| !guid.is_empty())
        .unwrap_or(player_guid);

    if let (Some(key), Some(manager)) = (
        host.core().current_canonical_player_map_key_like_cpp(),
        host.core().canonical_map_manager.as_ref(),
    ) && let Ok(manager) = manager.lock()
        && let Some(reach) = manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|managed| {
                managed.map().with_world_object_by_kinds_like_cpp(
                    seer_guid,
                    wow_entities::represented_seer_kinds_like_cpp(),
                    |object| object.combat_reach(),
                )
            })
    {
        return reach.max(0.0);
    }

    host.core()
        .canonical_player_combat_reach_snapshot_like_cpp()
        .max(0.0)
}

/// Resolve per-player state with the original viewer identity and expiry checks.
pub fn represented_gameobject_go_state_for_viewer_like_cpp(
    state: &RepresentedGameObjectUseState,
    player_guid: Option<ObjectGuid>,
    now: Instant,
) -> GoState {
    if state.per_player_state_player_guid == player_guid
        && state
            .per_player_go_state_until
            .is_none_or(|until| until > now)
        && let Some(per_player_go_state) = state.per_player_go_state
    {
        return per_player_go_state;
    }

    state.go_state.unwrap_or(GoState::Ready)
}
