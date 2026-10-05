use std::time::Instant;

use wow_core::ObjectGuid;
use wow_world_core::session::HubMut;

use crate::WorldEntitiesState;

/// C++ ref: `GameObject::GetInteractionDistance()`.
/// Spell-lock range remains with the typed GameObject/SpellInfo port.
pub fn represented_gameobject_interaction_distance_like_cpp(
    go_type: Option<u8>,
    interact_radius_override: Option<u32>,
) -> f32 {
    if let Some(override_hundredths) = interact_radius_override.filter(|value| *value != 0) {
        return override_hundredths as f32 / 100.0;
    }

    match go_type.map(u32::from) {
        Some(wow_entities::GAMEOBJECT_TYPE_AREADAMAGE) => 0.0,
        Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER)
        | Some(wow_entities::GAMEOBJECT_TYPE_TEXT)
        | Some(wow_entities::GAMEOBJECT_TYPE_FLAGSTAND)
        | Some(wow_entities::GAMEOBJECT_TYPE_FLAGDROP)
        | Some(wow_entities::GAMEOBJECT_TYPE_MINI_GAME) => 5.5555553,
        Some(wow_entities::GAMEOBJECT_TYPE_BINDER) => 10.0,
        Some(wow_entities::GAMEOBJECT_TYPE_CHAIR)
        | Some(wow_entities::GAMEOBJECT_TYPE_BARBER_CHAIR) => 3.0,
        Some(wow_entities::GAMEOBJECT_TYPE_FISHING_NODE) => 100.0,
        Some(wow_entities::GAMEOBJECT_TYPE_FISHING_HOLE) => {
            20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP
        }
        Some(wow_entities::GAMEOBJECT_TYPE_CAMERA)
        | Some(wow_entities::GAMEOBJECT_TYPE_MAP_OBJECT)
        | Some(wow_entities::GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY)
        | Some(wow_entities::GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING)
        | Some(wow_entities::GAMEOBJECT_TYPE_DOOR) => 5.0,
        Some(wow_entities::GAMEOBJECT_TYPE_GUILD_BANK)
        | Some(wow_entities::GAMEOBJECT_TYPE_MAILBOX) => 10.0,
        _ => 5.0,
    }
}

/// C++ ref: `GameObject::IsWithinDistInDisplayBox()`-style display-box
/// containment used by the loot authority's interactable check.
pub fn represented_gameobject_display_box_contains_like_cpp(
    go_position: wow_core::Position,
    player_position: wow_core::Position,
    display_info: &wow_data::GameObjectDisplayInfoEntry,
    scale: f32,
    rotation: [f32; 4],
    radius: f32,
) -> bool {
    let min_x = display_info.geo_box_min.x * scale - radius;
    let min_y = display_info.geo_box_min.y * scale - radius;
    let min_z = display_info.geo_box_min.z * scale - radius;
    let max_x = display_info.geo_box_max.x * scale + radius;
    let max_y = display_info.geo_box_max.y * scale + radius;
    let max_z = display_info.geo_box_max.z * scale + radius;

    let dx = player_position.x - go_position.x;
    let dy = player_position.y - go_position.y;
    let dz = player_position.z - go_position.z;
    let [qx, qy, qz, qw] = rotation;
    let iqx = -qx;
    let iqy = -qy;
    let iqz = -qz;

    let tx = 2.0 * (iqy * dz - iqz * dy);
    let ty = 2.0 * (iqz * dx - iqx * dz);
    let tz = 2.0 * (iqx * dy - iqy * dx);
    let local_x = dx + qw * tx + (iqy * tz - iqz * ty);
    let local_y = dy + qw * ty + (iqz * tx - iqx * tz);
    let local_z = dz + qw * tz + (iqx * ty - iqy * tx);

    local_x >= min_x
        && local_x <= max_x
        && local_y >= min_y
        && local_y <= max_y
        && local_z >= min_z
        && local_z <= max_z
}

impl WorldEntitiesState {
    /// C++ fishing-hole release performs AddUse, MaxOpens comparison, and
    /// SetLootState on one world thread. Keep all three under one map lock so
    /// two concurrent personal releases cannot finish in `Ready` after max.
    pub fn release_canonical_fishing_hole_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        max_opens: Option<u32>,
    ) -> Option<(
        u32,
        wow_entities::LootState,
        wow_map::map::GameObjectSetLootStateOutcomeLikeCpp,
    )> {
        hub.core
            .loot_release_access_like_cpp()
            .release_canonical_fishing_hole_like_cpp(guid, max_opens)
    }

    pub fn record_represented_fishing_hole_max_opens_like_cpp(
        &mut self,
        guid: ObjectGuid,
        max_opens: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .fishing_hole_max_opens = Some(max_opens);
    }

    pub fn record_represented_fishing_hole_radius_like_cpp(
        &mut self,
        guid: ObjectGuid,
        radius: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .fishing_hole_radius = Some(radius as f32);
    }

    pub fn lookup_represented_fishing_hole_around_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        const FISHING_HOLE_SEARCH_RANGE_LIKE_CPP: f32 =
            20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP;

        let source = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        let source_position = source.position?;
        let source_map_id = source.map_id;
        let now = Instant::now();
        let mut nearest: Option<(ObjectGuid, f32)> = None;

        for (candidate_guid, candidate) in &self.represented_gameobject_use_states {
            if *candidate_guid == gameobject_guid {
                continue;
            }
            if candidate.go_type != Some(wow_entities::GAMEOBJECT_TYPE_FISHING_HOLE as u8) {
                continue;
            }
            if source_map_id.is_some() && candidate.map_id != source_map_id {
                continue;
            }
            if candidate
                .per_player_despawn_until
                .is_some_and(|until| until > now)
            {
                continue;
            }
            let Some(candidate_position) = candidate.position else {
                continue;
            };
            let Some(fishing_hole_radius) = candidate.fishing_hole_radius else {
                continue;
            };
            if !source_position
                .is_within_dist(&candidate_position, FISHING_HOLE_SEARCH_RANGE_LIKE_CPP)
                || !source_position.is_within_dist(&candidate_position, fishing_hole_radius)
            {
                continue;
            }
            let distance = source_position.distance(&candidate_position);
            if nearest.is_none_or(|(_, nearest_distance)| distance < nearest_distance) {
                nearest = Some((*candidate_guid, distance));
            }
        }

        nearest.map(|(guid, _)| guid)
    }
}
