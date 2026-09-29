//! Shared imports and fixtures for the map manager test scenarios.
//!
//! Extracted from `map_manager_tests.rs` when the module moved to `wow-map`:
//! the explicit imports are the whole inherited surface, and keeping them beside
//! the fixtures lets the scenario modules keep importing everything through
//! `use super::*` without depending on the test root's size.

pub use crate::map_manager::{
    ChaseTargetSnapshotLikeCpp, ChaseTickOutcomeLikeCpp, CreatureAnimKitSlotLikeCpp, GRID_SIZE,
    Grid, LiveTerrainHeights,
    MapInstance, MapManager, PendingRespawn, PersistedRespawnRowLikeCpp, RecipientRule,
    RuntimeOutput,
    TerrainGridFileIndexLikeCpp, TerrainGridFilesLikeCpp, VISIBILITY_RADIUS, WorldCreature,
    WorldMMapPathRequestLikeCpp, WorldMMapPathfinderLikeCpp, WorldMMapPathfinderWorkerLikeCpp,
    calculate_creature_detour_path_like_cpp, detour_path_without_navmesh_like_cpp, grid_to_world,
    instant_from_respawn_time_like_cpp, path_generator_from_detour_like_cpp,
    path_type_from_detour_like_cpp, pending_respawn_from_world_creature_like_cpp,
    snap_respawn_creature_to_ground_like_cpp,
    terrain_grid_area_id_for_position_like_cpp, terrain_grid_coords_for_wow_position_like_cpp,
    terrain_map_id_for_phase_shift_like_cpp,
    world_creature_from_pending_respawn_like_cpp, world_to_grid_coords, world_to_grid_x,
    zone_and_area_for_position_like_cpp,
};
pub(crate) use crate::map_manager::{
    MAP_AREA_CELLS_PER_GRID_LIKE_CPP, MAP_AREA_HEADER_FLAG_NO_AREA_LIKE_CPP,
    MAP_AREA_HEADER_SIZE_LIKE_CPP, MAP_AREA_MAGIC_LIKE_CPP, MAP_FILE_HEADER_SIZE_LIKE_CPP,
    MAP_MAGIC_LIKE_CPP, MAP_VERSION_MAGIC_LIKE_CPP, TERRAIN_GRID_COUNT_LIKE_CPP,
};
pub use crate::SpawnObjectType;
pub use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
pub use std::collections::{HashMap, HashSet};
pub use std::fs;
pub use std::path::PathBuf;
pub use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
pub use wow_constants::movement::MovementFlag;
pub use wow_constants::{
    Class, CreatureFlagsExtra, DeathState, PowerType, UnitStandStateType, UnitState,
    WeaponAttackType,
};
pub use wow_core::guid::HighGuid;
pub use wow_core::{ObjectGuid, Position};
pub use wow_entities::{
    Creature, CreatureAddonLifecycleRecordLikeCpp, CreatureAiState, CreatureCombatLogStatsLikeCpp,
    DistractMovementAction, EVENT_CHARGE_PREPATH, GenericMovementInform, INVALID_HEIGHT,
    MovementGeneratorKind, MovementGeneratorType, PhaseShift, PointMovementAction,
    PointMovementInform, Z_OFFSET_FIND_HEIGHT,
};
pub use wow_movement::{
    MoveSpline, MoveSplineFlag, MovementGenerator as RuntimeMovementGenerator,
    MovementGeneratorFlags as RuntimeMovementGeneratorFlags,
    MovementGeneratorType as RuntimeMovementGeneratorType, PathType, RandomPathResult,
    WaypointMovementAction, WaypointMovementGenerator, WaypointPath, WaypointRandomAtPathEnd,
};
pub use wow_persistence::{RespawnPersistenceKeyLikeCpp, RespawnPersistenceMutationLikeCpp};
pub use wow_recastdetour::{
    DetourOwnerCapabilitiesLikeCpp, DetourPathType, DetourPointPath, DetourPolyPath,
    MAX_POINT_PATH_LENGTH_LIKE_CPP, MMapData, PathQueryFilterContext, SIZE_OF_GRIDS_LIKE_CPP,
    create_path_query_filter_like_cpp,
};

pub fn unique_temp_data_dir(test_name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!("rustycore-{test_name}-{unique}"));
    fs::create_dir_all(data_dir.join("maps")).expect("create maps test dir");
    data_dir
}

pub fn map_file_header_like_cpp() -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(MAP_MAGIC_LIKE_CPP);
    header.extend_from_slice(&MAP_VERSION_MAGIC_LIKE_CPP.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    assert_eq!(header.len(), MAP_FILE_HEADER_SIZE_LIKE_CPP);
    header
}

pub fn map_file_header_with_area_like_cpp(area_offset: u32, area_size: u32) -> Vec<u8> {
    let mut header = map_file_header_like_cpp();
    header[12..16].copy_from_slice(&area_offset.to_le_bytes());
    header[16..20].copy_from_slice(&area_size.to_le_bytes());
    header
}

pub fn test_area_entry(id: u32, parent_area_id: u16, flags: u32) -> wow_data::AreaTableEntry {
    wow_data::AreaTableEntry {
        id,
        continent_id: 571,
        parent_area_id,
        area_bit: -1,
        exploration_level: 0,
        mount_flags: 0,
        flags,
    }
}

pub fn test_creature(guid: ObjectGuid) -> WorldCreature {
    WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    )
}

pub fn test_chase_target(victim: ObjectGuid, x: f32) -> ChaseTargetSnapshotLikeCpp {
    ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(x, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    }
}

pub fn test_chase_corridor(poly_refs: Vec<u64>, end_x: f32) -> DetourPolyPath {
    let points = vec![
        [10.0, 10.0, 0.0],
        [(10.0 + end_x) * 0.5, 10.0, 0.0],
        [end_x, 10.0, 0.0],
    ];
    DetourPolyPath {
        poly_refs,
        point_path: DetourPointPath {
            actual_end: *points.last().expect("test path"),
            points,
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    }
}

pub fn tilelist_like_cpp(grid_indices: impl IntoIterator<Item = usize>) -> Vec<u8> {
    let mut bitset_string = vec![b'0'; TERRAIN_GRID_COUNT_LIKE_CPP];
    for grid_idx in grid_indices {
        bitset_string[TERRAIN_GRID_COUNT_LIKE_CPP - 1 - grid_idx] = b'1';
    }

    let mut tilelist = Vec::new();
    tilelist.extend_from_slice(MAP_MAGIC_LIKE_CPP);
    tilelist.extend_from_slice(&MAP_VERSION_MAGIC_LIKE_CPP.to_le_bytes());
    tilelist.extend_from_slice(&0_u32.to_le_bytes());
    tilelist.extend_from_slice(&bitset_string);
    tilelist
}

pub fn unique_test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rustycore-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ))
}

// ── Slice 4A.1a tests ────────────────────────────────────────────────────

// ── Slice 4A.2a: respawn queue tests ──────────────────────────────────────

pub fn make_pending_respawn(respawn_at: Instant) -> PendingRespawn {
    use wow_packet::packets::update::CreatureCreateData;
    static NEXT_TEST_SPAWN_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let spawn_id = NEXT_TEST_SPAWN_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        0,
        0,
        1,
        spawn_id as i64,
    );
    PendingRespawn {
        respawn_at,
        spawn_id,
        persistent_spawn: true,
        home_pos: Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: 0.0,
        },
        create_data: CreatureCreateData {
            guid,
            entry: 1,
            display_id: 1,
            native_display_id: 1,
            display_scale: 1.0,
            native_x_display_scale: 1.0,
            bounding_radius: 0.389,
            combat_reach: 1.5,
            health: 100,
            max_health: 100,
            level: 1,
            faction_template: 1,
            npc_flags: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            aura_state: WorldCreature::health_aura_state_like_cpp(100, 100, true),
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            display_power: 1,
            power: [0; 10],
            max_power: [0; 10],
            base_mana: 0,
            virtual_items: [(0, 0, 0); 3],
            base_attack_time: 2000,
            ranged_attack_time: 0,
            movement_flags: 0,
            vehicle_id: 0,
            play_hover_anim: false,
            hover_height: 1.0,
            mount_display_id: 0,
            stand_state: 0,
            vis_flags: 0,
            anim_tier: 0,
            emote_state: 0,
            sheathe_state: wow_constants::unit::SheathState::Melee as u8,
            pvp_flags: 0,
            current_area_id: 0,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        },
        max_hp: 100,
        level: 1,
        min_dmg: 1,
        max_dmg: 5,
        combat_log_stats: CreatureCombatLogStatsLikeCpp::default(),
        spell_hit_aura_source_authority_like_cpp: false,
        spell_cast_log_aura_source_authority_like_cpp: false,
        aggro_radius: 10.0,
        wander_distance: 0.0,
        flags_extra: 0,
        static_flags: [0; 8],
        ai_name: String::new(),
        script_name: String::new(),
        string_id: None,
        addon: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        default_movement_type: MovementGeneratorType::Idle,
        waypoint_path_id: 0,
        npc_flags: 0,
        unit_flags: 0,
        map_id: 0,
        loot_id: 0,
        skin_loot_id: 0,
        gold_min: 0,
        gold_max: 0,
        respawn_delay_secs: 30,
        selected_equipment_id: 0,
        original_equipment_id: 0,
        boss_id: None,
        dungeon_encounter_id: 0,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        terrain_swap_map: -1,
        phase_shift: PhaseShift::default(),
    }
}

/// Unique temp `maps/` dir holding one synthetic constant-height tile.
pub fn temp_dir_with_constant_tile(map_id: u32, gx: i32, gy: i32, height: f32) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("rustycore_live_terrain_{}_{n}", std::process::id()));
    std::fs::create_dir_all(dir.join("maps")).expect("create temp maps dir");

    // Minimal float `.map`: fileheader(44) + MHGT header(16) + V9 + V8, all = height.
    const V9: usize = 129 * 129;
    const V8: usize = 128 * 128;
    let mut b = Vec::new();
    b.extend_from_slice(b"MAPS");
    b.extend_from_slice(&10u32.to_le_bytes()); // version
    b.extend_from_slice(&0u32.to_le_bytes()); // build
    b.extend_from_slice(&0u32.to_le_bytes()); // areaMapOffset
    b.extend_from_slice(&0u32.to_le_bytes()); // areaMapSize
    b.extend_from_slice(&44u32.to_le_bytes()); // heightMapOffset
    for _ in 0..5 {
        b.extend_from_slice(&0u32.to_le_bytes());
    }
    b.extend_from_slice(b"MHGT");
    b.extend_from_slice(&0u32.to_le_bytes()); // flags = float
    b.extend_from_slice(&height.to_le_bytes()); // gridHeight
    b.extend_from_slice(&height.to_le_bytes()); // gridMaxHeight
    for _ in 0..(V9 + V8) {
        b.extend_from_slice(&height.to_le_bytes());
    }
    std::fs::write(
        dir.join("maps")
            .join(format!("{map_id:04}_{gx:02}_{gy:02}.map")),
        &b,
    )
    .expect("write tile");
    dir
}
