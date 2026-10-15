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
    position: &wow_core::Position,
) -> Vec<wow_entities::Creature> {
    if host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .is_some_and(|key| key.map_id != u32::from(map_id))
    {
        return Vec::new();
    }

    // These candidates carry the canonical runtime state, including the
    // live `Unit::movespline` a create block projects.
    visible_creatures_from_canonical_map_like_cpp(host, map_id, position).unwrap_or_default()
}

pub fn visible_creatures_from_canonical_map_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    map_id: u16,
    position: &wow_core::Position,
) -> Option<Vec<wow_entities::Creature>> {
    let requested_map_id = u32::from(map_id);
    let player_map_key = host.core().current_canonical_player_map_key_like_cpp();
    let player_map_key = player_map_key?;
    if player_map_key.map_id != requested_map_id {
        return Some(Vec::new());
    }
    let source_combat_reach = host
        .core()
        .with_owned_player_like_cpp(|player| player.unit().world().combat_reach())?;
    let player_phase_shift = host.represented_player_phase_shift_like_cpp()?;
    let manager = host.core().canonical_map_manager.as_ref()?;
    let Ok(manager) = manager.lock() else {
        return None;
    };
    let map = manager.find_map(player_map_key.map_id, player_map_key.instance_id)?;
    let visibility_range = host
        .hub_ref()
        .config
        .player_map_visibility_range_like_cpp(map_id);
    let nearby = map.map().nearby_cell_guids_like_cpp(
        position.x,
        position.y,
        visibility_range + source_combat_reach,
    );

    let mut candidates = Vec::new();
    for guid in nearby
        .world
        .creatures
        .into_iter()
        .chain(nearby.grid.creatures)
    {
        let Some(creature) = map
            .map()
            .with_creature_or_pet_like_cpp(guid, |creature, _owner| creature.clone())
        else {
            continue;
        };
        let world = creature.unit().world();
        if !world.object().is_in_world()
            || world.map_id() != requested_map_id
            // C++ visibility is a horizontal (XY) range test: CanSeeOrDetect ->
            // IsWithinDist(obj, GetSightRange, is3D=false) (Object.cpp:1587-1609).
            // #NEXT.R8.ENTITIES.1223 — use 2D so vertically-separated objects (e.g. ICC
            // layered floors) within horizontal range are not dropped.
            || !wow_core::visibility_distance_allows_like_cpp(
                position,
                source_combat_reach,
                &world.position(),
                world.combat_reach(),
                visibility_range,
            )
            || !player_phase_shift.can_see(world.phase_shift())
        {
            continue;
        }
        candidates.push(creature.clone());
    }
    drop(manager);

    host.core().with_owned_player_like_cpp(move |player| {
        candidates
            .into_iter()
            .filter(|creature| {
                player
                    .unit()
                    .can_see_or_detect_unit_like_cpp(creature.unit(), false, true, false)
            })
            .map(|creature| {
                // #1263 F6-8B: the visibility candidate must carry the
                // canonical runtime state, because the create projection at
                // the call site reads the live `Unit::movespline` from it.
                // The caller computes that projection on demand (the
                // runtime installation changes no field it reads).
                let create_data =
                    wow_world_core::creature_projection::creature_create_data_like_cpp(&creature);
                wow_world_core::creature_projection::projected_creature_preserving_runtime_like_cpp(
                    creature,
                    &create_data,
                )
            })
            .collect()
    })
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
