//! Spawn object and group model regression scenarios.
//!
//! Separated from the spawn.rs root under #644.

use super::*;

fn spawn(object_type: SpawnObjectType, spawn_id: SpawnId, x: f32, y: f32) -> SpawnData {
    SpawnData {
        object_type,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: 42,
        spawn_point: SpawnPosition::new(x, y, 1.0, 2.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 120,
        spawn_difficulties: vec![0, 1],
        script_id: 0,
        string_id: String::new(),
    }
}

fn respawn_info(
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
    respawn_time: i64,
) -> RespawnInfoLikeCpp {
    RespawnInfoLikeCpp {
        object_type,
        spawn_id,
        entry: 42,
        respawn_time,
        grid_id: 7,
    }
}

fn template(group_id: u32, map_id: u32, flags: SpawnGroupFlags) -> SpawnGroupTemplateData {
    SpawnGroupTemplateData {
        group_id,
        name: format!("group-{group_id}"),
        map_id,
        flags,
    }
}

mod scenarios_1;
