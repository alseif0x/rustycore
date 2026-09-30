//! Pure interaction distance and represented model bounds.
//! C++ anchors: GameObject::GetInteractionDistance and IsAtInteractDistance.
//! Quaternion and inclusive-bound behavior is retained from the represented adapter.
//! Catalog lookup, phase/map admission and spell-lock resolution remain in World.
use super::*;

pub fn gameobject_interaction_distance(
    go_type: Option<u8>,
    interact_radius_override: Option<u32>,
    contact_distance: f32,
) -> f32 {
    // C++ ref: GameObject.cpp GetInteractionDistance().
    // Spell-lock range remains with the typed GameObject/SpellInfo port.
    if let Some(override_hundredths) = interact_radius_override.filter(|value| *value != 0) {
        return override_hundredths as f32 / 100.0;
    }

    match go_type.map(u32::from) {
        Some(GAMEOBJECT_TYPE_AREADAMAGE) => 0.0,
        Some(GAMEOBJECT_TYPE_QUESTGIVER)
        | Some(GAMEOBJECT_TYPE_TEXT)
        | Some(GAMEOBJECT_TYPE_FLAGSTAND)
        | Some(GAMEOBJECT_TYPE_FLAGDROP)
        | Some(GAMEOBJECT_TYPE_MINI_GAME) => 5.5555553,
        Some(GAMEOBJECT_TYPE_BINDER) => 10.0,
        Some(GAMEOBJECT_TYPE_CHAIR) | Some(GAMEOBJECT_TYPE_BARBER_CHAIR) => 3.0,
        Some(GAMEOBJECT_TYPE_FISHING_NODE) => 100.0,
        Some(GAMEOBJECT_TYPE_FISHING_HOLE) => 20.0 + contact_distance,
        Some(GAMEOBJECT_TYPE_CAMERA)
        | Some(GAMEOBJECT_TYPE_MAP_OBJECT)
        | Some(GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY)
        | Some(GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING)
        | Some(GAMEOBJECT_TYPE_DOOR) => 5.0,
        Some(GAMEOBJECT_TYPE_GUILD_BANK) | Some(GAMEOBJECT_TYPE_MAILBOX) => 10.0,
        _ => 5.0,
    }
}

pub fn gameobject_display_box_contains(
    go_position: wow_core::Position,
    player_position: wow_core::Position,
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    scale: f32,
    rotation: [f32; 4],
    radius: f32,
) -> bool {
    let min_x = bounds_min[0] * scale - radius;
    let min_y = bounds_min[1] * scale - radius;
    let min_z = bounds_min[2] * scale - radius;
    let max_x = bounds_max[0] * scale + radius;
    let max_y = bounds_max[1] * scale + radius;
    let max_z = bounds_max[2] * scale + radius;

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

#[cfg(test)]
mod tests;
