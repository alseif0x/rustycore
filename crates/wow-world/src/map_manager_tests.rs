//! Behaviour tests for [`super`].
//!
//! Extracted from `map_manager.rs`, which was 12,935 lines of which
//! 6,328 — 49% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use wow_constants::{Class, CreatureFlagsExtra, DeathState, PhaseFlags, PowerType};
use wow_core::guid::HighGuid;
use wow_map::map::MapWorldObjectEnvironment;

fn unique_temp_data_dir(test_name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!("rustycore-{test_name}-{unique}"));
    fs::create_dir_all(data_dir.join("maps")).expect("create maps test dir");
    data_dir
}

fn map_file_header_like_cpp() -> Vec<u8> {
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

fn map_file_header_with_area_like_cpp(area_offset: u32, area_size: u32) -> Vec<u8> {
    let mut header = map_file_header_like_cpp();
    header[12..16].copy_from_slice(&area_offset.to_le_bytes());
    header[16..20].copy_from_slice(&area_size.to_le_bytes());
    header
}

fn test_area_entry(id: u32, parent_area_id: u16, flags: u32) -> wow_data::AreaTableEntry {
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

fn test_creature(guid: ObjectGuid) -> WorldCreature {
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

#[test]
fn only_loaded_grid_creature_bridge_completes_spell_aura_authorities_like_cpp() {
    let generic = test_creature(ObjectGuid::new(0, 90_001));
    assert!(
        !generic
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()
    );
    assert!(
        !generic
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    );

    let mut previously_authorized = generic.creature.clone();
    previously_authorized
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_hit_aura_authority_inert_like_cpp(true);
    previously_authorized
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_cast_log_aura_authority_inert_like_cpp(true);
    let generic_bridge =
        WorldCreature::from_canonical(previously_authorized, generic.create_data.clone());
    assert!(
        !generic_bridge
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()
    );
    assert!(
        !generic_bridge
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    );

    let canonical = test_creature(ObjectGuid::new(0, 90_002)).creature;
    let loaded_grid = WorldCreature::from_loaded_grid_canonical_like_cpp(canonical, |_| None);
    assert!(
        loaded_grid
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()
    );
    assert!(
        loaded_grid
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    );
}

fn test_chase_target(victim: ObjectGuid, x: f32) -> ChaseTargetSnapshotLikeCpp {
    ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(x, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    }
}

fn test_chase_corridor(poly_refs: Vec<u64>, end_x: f32) -> DetourPolyPath {
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

#[test]
fn world_creature_motion_master_chase_interrupts_random_and_resumes_default_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_009);
    let target = ObjectGuid::create_player(1, 7009);
    let mut creature = test_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Random);

    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Random)
    );
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), 1);

    creature.enter_combat(target);
    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(RuntimeMovementGeneratorType::Chase),
        "C++ MotionMaster::Add places normal-priority active chase above the default random generator"
    );
    let represented_chase = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(represented_chase.kind, MovementGeneratorKind::Chase);
    assert_eq!(represented_chase.target_guid, Some(target));
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Chase)
    );
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), 2);

    creature.reset_combat();
    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(RuntimeMovementGeneratorType::Random),
        "removing active chase must expose and reset the deactivated default generator"
    );
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Random)
    );
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), 3);
}

#[test]
fn world_creature_motion_master_preserves_high_priority_point_above_chase_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_011);
    let target = ObjectGuid::create_player(1, 7011);
    let mut creature = test_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Random);
    creature
        .begin_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
        .expect("launch point spline");
    creature
        .creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .move_charge(42);

    creature.enter_combat(target);

    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(RuntimeMovementGeneratorType::Point),
        "C++ keeps highest-priority charge/point above normal-priority chase"
    );
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Point)
    );
    assert!(
        creature.active_move_spline_like_cpp().is_some(),
        "selecting chase must not stop the higher-priority point spline"
    );

    creature.finish_move();
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Chase),
        "finishing the higher-priority point spline pops its represented generator and exposes chase"
    );
}

#[test]
fn world_creature_motion_master_expires_finite_distract_and_exposes_chase_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_012);
    let target = ObjectGuid::create_player(1, 7012);
    let mut creature = test_creature(guid);
    creature
        .begin_distract_movement_like_cpp(10, 1.25)
        .expect("launch finite distract");
    creature.enter_combat(target);

    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(10),
        Some(RuntimeMovementGeneratorType::Distract),
        "C++ Distract remains selected while its timer has not expired"
    );
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(1),
        Some(RuntimeMovementGeneratorType::Chase),
        "the represented finite generator must pop and remove its runtime proxy"
    );
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Chase
    );
}

#[test]
fn world_creature_death_and_loot_keep_game_time_and_monotonic_deadlines_separate_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_010);
    let mut creature = test_creature(guid);
    creature.creature.set_respawn_compatibility_mode(false);
    creature.creature.set_corpse_delay(60, false);

    let clock_started_at = Instant::now();
    let death_game_time_secs = 1_700_000_000;
    creature.clock_started_at = clock_started_at;
    assert!(
        creature
            .creature
            .apply_ai_damage_before_death_state_at_game_time_like_cpp(50, 0, death_game_time_secs,)
    );
    creature.complete_death_state_after_kill_hooks_at_game_time_like_cpp(death_game_time_secs);

    let completion_ms = creature
        .creature
        .ai_ownership()
        .death_time_ms
        .expect("death completion must record the monotonic mirror");
    let completion_now = clock_started_at + Duration::from_millis(completion_ms);
    assert_eq!(
        creature.creature.corpse_remove_time(),
        death_game_time_secs + 60
    );
    assert_eq!(creature.creature.respawn_time(), death_game_time_secs + 30);
    assert_eq!(
        creature.respawn_at_from_death_at_game_time_like_cpp(completion_now, death_game_time_secs,),
        completion_now + Duration::from_secs(30)
    );

    let loot_now = completion_now + Duration::from_secs(3);
    let loot_game_time_secs = death_game_time_secs + 3;
    assert!(creature.all_loot_removed_from_corpse_at_game_time_like_cpp(
        loot_now,
        loot_game_time_secs,
        0.5,
        false,
    ));
    assert_eq!(
        creature.creature.corpse_remove_time(),
        loot_game_time_secs + 30
    );
    assert_eq!(creature.creature.respawn_time(), loot_game_time_secs + 60);
    assert_eq!(
        creature.corpse_despawn_at(),
        Some(loot_now + Duration::from_secs(30))
    );
    assert_eq!(
        creature.respawn_at_from_death_at_game_time_like_cpp(loot_now, loot_game_time_secs,),
        loot_now + Duration::from_secs(60)
    );
}

#[test]
fn never_respawn_time_saturates_to_future_instant_instead_of_ready_now_like_cpp() {
    let now = Instant::now();
    let deadline = instant_from_respawn_time_like_cpp(i64::MAX, now, 1_700_000_000);

    assert!(deadline > now);
}

#[derive(Debug)]
struct RecordingLiveStaticVMapLos {
    result: bool,
    calls: std::sync::Mutex<Vec<wow_map::VMapLineOfSightQuery>>,
}

impl RecordingLiveStaticVMapLos {
    fn new(result: bool) -> Self {
        Self {
            result,
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl wow_map::StaticVMapLineOfSightProvider for RecordingLiveStaticVMapLos {
    fn is_in_line_of_sight(&self, query: wow_map::VMapLineOfSightQuery) -> bool {
        self.calls
            .lock()
            .expect("recording live vmap LOS calls poisoned")
            .push(query);
        self.result
    }
}

#[test]
fn live_terrain_wires_static_vmap_los_provider_into_map_cache_like_cpp() {
    let dir = unique_temp_data_dir("live-vmap-los-provider");
    let provider = Arc::new(RecordingLiveStaticVMapLos::new(false));
    let shared_provider: SharedStaticVMapLineOfSightProvider = provider.clone();
    let terrain_cache =
        LiveTerrainHeights::new_with_static_vmap_line_of_sight(&dir, shared_provider);

    let terrain = terrain_cache.terrain_for_map(1);
    let mut source = wow_entities::WorldObject::new(
        false,
        wow_constants::TypeId::Unit,
        wow_constants::TypeMask::UNIT,
    );
    source.relocate(Position::new(10.0, 10.0, 1.0, 0.0));
    let query = wow_entities::LineOfSightQuery::to_position_like_cpp(
        &source,
        Position::new(20.0, 10.0, 1.0, 0.0),
        wow_entities::LineOfSightOptions::default(),
    );

    assert!(
        !terrain.line_of_sight(query),
        "live terrain must not bypass an installed static VMAP LOS provider"
    );
    let calls = provider
        .calls
        .lock()
        .expect("recording live vmap LOS calls poisoned");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].map_id, 1);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn health_aura_state_like_cpp_matches_cpp_modify_aura_state() {
    // Regression for the world-entry ERROR #132 client crash: every creature
    // CREATE block must carry UNIT_FIELD_AURASTATE matching C++ Unit::Update ->
    // ModifyAuraState (Unit.cpp:469-476). A full-HP alive creature yields
    // 0x00D00000 (bits 20|22|23 = WOUND_HEALTH_20_80 | HEALTHY_75 | WOUND_HEALTH_35_80).
    // The client tests bit 0x100000 of this field on a per-frame tick; 0 crashed it.
    assert_eq!(
        WorldCreature::health_aura_state_like_cpp(100, 100, true),
        0x00D0_0000,
        "full-HP alive creature must match C++ 0x00D00000"
    );
    // Dead unit / zero max: no aura state (C++ only runs ModifyAuraState if IsAlive).
    assert_eq!(WorldCreature::health_aura_state_like_cpp(0, 100, false), 0);
    assert_eq!(WorldCreature::health_aura_state_like_cpp(50, 0, true), 0);
    // Low health (<=20%): WOUNDED_20/25/35 + WOUND_HEALTH_20_80 + WOUND_HEALTH_35_80
    // bits set, HEALTHY_75 clear. Must include the crash bit 0x100000.
    let low = WorldCreature::health_aura_state_like_cpp(10, 100, true);
    assert_ne!(
        low & 0x0010_0000,
        0,
        "WOUND_HEALTH_20_80 (0x100000) set at low HP"
    );
    assert_eq!(low & 0x0040_0000, 0, "HEALTHY_75 clear at low HP");
    // Mid health (50%): none of the threshold states (not <35, not >75, not <20/>80).
    assert_eq!(WorldCreature::health_aura_state_like_cpp(50, 100, true), 0);
}

#[test]
fn create_data_from_canonical_clamps_zero_base_attack_time_like_cpp() {
    // Regression for the world-entry client crash: a creature CREATE block with
    // UnitData.AttackRoundBaseTime == 0 makes the 3.4.3 client divide-by-zero in its
    // swing-timer math on the first post-spawn tick (crash ~5s after the visibility
    // burst). C++ guarantees this is never 0 (ObjectMgr.cpp:1100-1104 clamps
    // creature_template BaseAttackTime/RangeAttackTime 0 -> BASE_ATTACK_TIME=2000).
    // A bare canonical Creature leaves Unit::base_attack_speed at its [0; MAX_ATTACK]
    // default, reproducing the bug; create_data_from_canonical_like_cpp must clamp it.
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 9001);
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(9001);
    // Sanity: the underlying base attack speed really is the uninitialized 0 here.
    assert_eq!(
        creature.unit().base_attack_speed()[WeaponAttackType::BaseAttack as usize],
        0,
        "precondition: bare canonical creature has 0 base attack speed"
    );

    let create_data = WorldCreature::create_data_from_canonical_like_cpp(&creature);

    assert_eq!(
        create_data.base_attack_time, BASE_ATTACK_TIME_LIKE_CPP,
        "0 base attack time must be clamped to BASE_ATTACK_TIME (2000), never shipped as 0"
    );
    assert_eq!(
        create_data.ranged_attack_time, BASE_ATTACK_TIME_LIKE_CPP,
        "0 ranged attack time must be clamped to BASE_ATTACK_TIME (2000), never shipped as 0"
    );
}

#[test]
fn create_data_from_canonical_preserves_nonzero_base_attack_time_like_cpp() {
    // The clamp must only replace 0; a real attack time must pass through unchanged.
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 9002);
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(9002);
    creature
        .unit_mut()
        .set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 1500);
    creature
        .unit_mut()
        .set_base_attack_time_like_cpp(WeaponAttackType::RangedAttack, 1800);

    let create_data = WorldCreature::create_data_from_canonical_like_cpp(&creature);

    assert_eq!(create_data.base_attack_time, 1500);
    assert_eq!(create_data.ranged_attack_time, 1800);
}

#[test]
fn create_data_from_canonical_keeps_base_mana_distinct_from_non_mana_power_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 9003);
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(9003);
    creature.set_power_type(PowerType::Focus);
    creature.unit_mut().set_create_mana_like_cpp(600);
    creature.unit_mut().set_max_power(PowerType::Focus, 100);
    creature.unit_mut().set_power(PowerType::Focus, 25);

    let create_data = WorldCreature::create_data_from_canonical_like_cpp(&creature);

    assert_eq!(create_data.display_power, PowerType::Focus as u8);
    assert_eq!(create_data.base_mana, 600);
    assert_eq!(create_data.max_power[0], 100);
    assert_eq!(create_data.power[0], 25);
}

#[test]
fn terrain_grid_area_map_decodes_cpp_area_cell_and_zone_parent() {
    let data_dir = unique_temp_data_dir("terrain-area-map");
    let map_id = 571;
    let x = 0.0;
    let y = 0.0;
    let (gx, gy) = terrain_grid_coords_for_wow_position_like_cpp(x, y);
    let area_offset = MAP_FILE_HEADER_SIZE_LIKE_CPP as u32;
    let area_size = (MAP_AREA_HEADER_SIZE_LIKE_CPP
        + MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * std::mem::size_of::<u16>()) as u32;

    let mut bytes = map_file_header_with_area_like_cpp(area_offset, area_size);
    bytes.extend_from_slice(MAP_AREA_MAGIC_LIKE_CPP);
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&4395_u16.to_le_bytes());
    let mut cells = [0_u16; MAP_AREA_CELLS_PER_GRID_LIKE_CPP * MAP_AREA_CELLS_PER_GRID_LIKE_CPP];
    cells[0] = 4613;
    for cell in cells {
        bytes.extend_from_slice(&cell.to_le_bytes());
    }
    fs::write(
        data_dir
            .join("maps")
            .join(format!("{map_id:04}_{gx:02}_{gy:02}.map")),
        bytes,
    )
    .expect("write test map");

    let area_store = wow_data::AreaTableStore::from_entries([
        test_area_entry(4395, 0, 0),
        test_area_entry(4613, 4395, 0x4000_0000),
    ]);

    assert_eq!(
        zone_and_area_for_position_like_cpp(&data_dir, map_id, x, y, Some(&area_store), |_| {
            9999
        },)
        .expect("resolve terrain zone area"),
        (4395, 4613)
    );
}

#[test]
fn terrain_area_file_uses_cpp_reversed_grid_coordinates_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-area-grid-reversal");
    let map_id = 530;
    let x = 2_933.0;
    let y = -5_600.0;
    let (file_x, file_y) = terrain_grid_coords_for_wow_position_like_cpp(x, y);
    assert_eq!((file_x, file_y), (26, 42));

    let area_offset = MAP_FILE_HEADER_SIZE_LIKE_CPP as u32;
    let mut bytes =
        map_file_header_with_area_like_cpp(area_offset, MAP_AREA_HEADER_SIZE_LIKE_CPP as u32);
    bytes.extend_from_slice(MAP_AREA_MAGIC_LIKE_CPP);
    bytes.extend_from_slice(&MAP_AREA_HEADER_FLAG_NO_AREA_LIKE_CPP.to_le_bytes());
    bytes.extend_from_slice(&3_697_u16.to_le_bytes());
    fs::write(
        data_dir
            .join("maps")
            .join(format!("{map_id:04}_{file_x:02}_{file_y:02}.map")),
        bytes,
    )
    .expect("write reversed C++ terrain tile");

    assert_eq!(
        terrain_grid_area_id_for_position_like_cpp(&data_dir, map_id, x, y)
            .expect("read reversed C++ terrain tile"),
        Some(3_697)
    );
}

#[test]
fn terrain_zone_area_falls_back_to_map_area_when_grid_missing_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-area-fallback");
    assert_eq!(
        zone_and_area_for_position_like_cpp(&data_dir, 571, 0.0, 0.0, None, |_| 4395)
            .expect("resolve fallback terrain zone area"),
        (4395, 4395)
    );
}

#[test]
fn world_creature_runtime_rng_replaces_timer_seeded_damage_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70001);
    let mut creature = test_creature(guid);
    creature.seed_runtime_rng_like_cpp(0xA141_BEEF);

    let rolls: Vec<u32> = (0..16)
        .map(|_| creature.roll_damage().expect("authoritative damage roll"))
        .collect();

    assert!(rolls.iter().all(|roll| (5..=10).contains(roll)));
    assert!(
        rolls.iter().any(|roll| *roll != rolls[0]),
        "damage rolls should come from owned RNG, not now_ms/spline_id: {rolls:?}"
    );
}

#[test]
fn creature_spell_hit_roll_consumes_owned_runtime_rng_sequence_like_cpp() {
    let seed = 0x5E11_117_u64;
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70002);
    let mut creature = test_creature(guid);
    creature.seed_runtime_rng_like_cpp(seed);
    let mut expected_rng = StdRng::seed_from_u64(seed);

    let actual: Vec<u32> = (0..16)
        .map(|_| {
            creature
                .random_creature_spell_hit_roll_like_cpp()
                .expect("authoritative creature-spell hit roll")
        })
        .collect();
    let expected: Vec<u32> = (0..16).map(|_| expected_rng.gen_range(0..=9_999)).collect();

    assert_eq!(actual, expected);
}

#[test]
fn equal_rng_bounds_still_consume_shared_runtime_draws_like_cpp() {
    let seed = 0xE011_A1_u64;
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70008);
    let mut creature = test_creature(guid);
    creature.seed_runtime_rng_like_cpp(seed);
    creature.creature.ai_ownership_mut().min_damage = 7;
    creature.creature.ai_ownership_mut().max_damage = 7;
    let mut expected_rng = StdRng::seed_from_u64(seed);

    assert_eq!(
        creature.random_creature_spell_delay_like_cpp(5_000, 5_000),
        Some(5_000)
    );
    let _ = expected_rng.next_u32();
    assert_eq!(creature.roll_damage(), Some(7));
    let _ = expected_rng.next_u32();
    assert_eq!(
        creature.random_creature_spell_hit_roll_like_cpp(),
        Some(expected_rng.gen_range(0..=9_999))
    );
}

#[test]
fn world_creature_spell_rng_tombstone_preserves_legacy_melee_and_movement() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70007);
    let mut creature = test_creature(guid);
    creature.seed_runtime_rng_like_cpp(0x7007);
    assert!(creature.runtime_rng_authority_complete_like_cpp());
    assert!(creature.random_creature_spell_hit_roll_like_cpp().is_some());

    creature.invalidate_runtime_rng_authority_like_cpp();
    creature.reset_creature_spell_schedule_like_cpp();
    creature.seed_runtime_rng_like_cpp(0x7008);

    assert!(!creature.runtime_rng_authority_complete_like_cpp());
    assert_eq!(creature.random_creature_spell_hit_roll_like_cpp(), None);
    assert_eq!(
        creature.random_creature_spell_delay_like_cpp(5_000, 10_000),
        None
    );
    assert_eq!(
        creature.random_creature_spell_delay_like_cpp(5_000, 5_000),
        None,
        "even equal C++ bounds consume the permanently lost RNG stream"
    );
    assert!(creature.roll_damage().is_some());
    assert!(creature.pick_wander_destination().is_some());
    assert!(
        creature
            .pick_random_destination_from_current_position_like_cpp(12.0)
            .is_some()
    );
    assert!(creature.reset_wander_timer());
    creature.creature.ai_ownership_mut().wander_steps_remaining = 0;
    assert!(creature.record_random_movement_launch_like_cpp());
    assert!(creature.schedule_after_random_movement_like_cpp());
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 12.0;
    assert!(creature.initialize_default_random_movement_like_cpp());

    let cloned = creature.clone();
    assert!(!cloned.runtime_rng_authority_complete_like_cpp());
}

#[test]
fn reversed_damage_bounds_reject_and_tombstone_only_spell_rng_authority() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70009);
    let mut creature = test_creature(guid);
    creature.creature.ai_ownership_mut().min_damage = 10;
    creature.creature.ai_ownership_mut().max_damage = 5;

    assert_eq!(creature.roll_damage(), None);
    assert!(!creature.runtime_rng_authority_complete_like_cpp());
    assert_eq!(creature.random_creature_spell_hit_roll_like_cpp(), None);
    assert!(
        creature
            .pick_random_destination_from_current_position_like_cpp(12.0)
            .is_some(),
        "the conservative spell-RNG tombstone must not freeze legacy movement"
    );
}

#[test]
fn world_creature_random_movement_walk_rule_matches_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70003);
    let mut creature = test_creature(guid);

    creature.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::Walk as u8,
    );
    assert!(creature.random_movement_walk_like_cpp());

    creature.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
    );
    assert!(!creature.random_movement_walk_like_cpp());

    creature.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::CanRun as u8,
    );
    creature
        .creature
        .set_movement_flags_runtime_like_cpp(MovementFlag::NONE);
    assert!(!creature.random_movement_walk_like_cpp());
    creature
        .creature
        .set_movement_flags_runtime_like_cpp(MovementFlag::WALKING);
    assert!(creature.random_movement_walk_like_cpp());
}

#[test]
fn world_creature_random_spline_uses_walk_or_run_speed_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70004);
    let mut walker = test_creature(guid);
    walker.create_data.speed_walk_rate = 1.0;
    walker.create_data.speed_run_rate = 1.0;
    walker.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::Walk as u8,
    );
    let (_, walk_spline) = walker
        .begin_random_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
        .expect("walk random spline");

    let mut runner = test_creature(guid);
    runner.create_data.speed_walk_rate = 1.0;
    runner.create_data.speed_run_rate = 1.0;
    runner.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
    );
    let (_, run_spline) = runner
        .begin_random_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
        .expect("run random spline");

    assert!((walk_spline.duration_ms() - 4_000).abs() <= 1);
    assert!((run_spline.duration_ms() - 1_429).abs() <= 1);
    assert!(
        run_spline.duration_ms() < walk_spline.duration_ms(),
        "C++ RandomMovementGenerator SetWalk(false) uses run speed"
    );
}

#[test]
fn world_creature_default_random_initializes_generator_without_spline_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70005);
    let mut creature = test_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 12.0;
    creature.seed_runtime_rng_like_cpp(0x7005);

    assert!(creature.initialize_default_random_movement_like_cpp());

    assert_eq!(creature.move_target(), None);
    assert!(creature.active_move_spline_like_cpp().is_none());
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    assert_eq!(creature.state(), wow_entities::CreatureAiState::Idle);
    assert!(
        (2..=10).contains(&creature.creature.ai_ownership().wander_steps_remaining),
        "C++ RandomMovementGenerator::DoInitialize seeds 2..10 steps but SetRandomLocation consumes the first step later"
    );
    assert_eq!(
        creature.creature.ai_ownership().wander_delay_ms,
        0,
        "C++ RandomMovementGenerator::DoInitialize resets its timer to 0 so the next update can choose a path"
    );
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Random
    );
}

#[test]
fn world_creature_random_wander_steps_pause_only_after_step_batch_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70006);
    let mut creature = test_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_steps_remaining = 2;

    assert!(creature.record_random_movement_launch_like_cpp());
    assert_eq!(creature.creature.ai_ownership().wander_steps_remaining, 1);
    assert!(creature.schedule_after_random_movement_like_cpp());
    assert_eq!(creature.creature.ai_ownership().wander_delay_ms, 0);

    assert!(creature.record_random_movement_launch_like_cpp());
    assert_eq!(creature.creature.ai_ownership().wander_steps_remaining, 0);
    assert!(creature.schedule_after_random_movement_like_cpp());
    assert!(
        (4_000..=10_000).contains(&creature.creature.ai_ownership().wander_delay_ms),
        "C++ RandomMovementGenerator pauses 4..10 seconds only after its wander step batch"
    );
    assert!(
        (2..=10).contains(&creature.creature.ai_ownership().wander_steps_remaining),
        "C++ RandomMovementGenerator reseeds 2..10 wander steps after a pause"
    );
}

#[test]
fn world_creature_interaction_pause_stops_and_updates_home_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70005);
    let mut creature = test_creature(guid);
    let current = Position::new(14.0, 15.0, 16.0, 1.5);
    creature.creature.unit_mut().world_mut().relocate(current);
    creature
        .creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .start_spline(42, 1_000);

    assert!(creature.pause_interaction_movement_like_cpp());

    let motion = &creature.creature.unit().subsystems().motion;
    assert!(motion.paused);
    assert!(motion.stopped);
    assert!(!motion.spline.enabled);
    assert_eq!(creature.home_position(), current);

    creature
        .creature
        .set_interaction_pause_timer_ms_runtime_like_cpp(0);
    assert!(!creature.pause_interaction_movement_like_cpp());
}

#[test]
fn world_creature_wander_rng_matches_cpp_random_movement_bounds() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70002);
    let mut creature = test_creature(guid);
    creature.creature.ai_ownership_mut().wander_radius = 12.0;
    creature.seed_runtime_rng_like_cpp(0x5757);

    for _ in 0..24 {
        let dst = creature
            .pick_wander_destination()
            .expect("authoritative wander destination");
        let dist = creature.home_position().distance(&dst);
        assert!(
            dist <= creature.creature.ai_ownership().wander_radius + f32::EPSILON,
            "wander destination {dst:?} was {dist} yd from home"
        );
    }

    for _ in 0..24 {
        assert!(creature.reset_wander_timer());
        assert!(
            (4_000..=10_000).contains(&creature.creature.ai_ownership().wander_delay_ms),
            "C++ RandomMovementGenerator pauses with urand(4, 10) seconds"
        );
    }
}

fn tilelist_like_cpp(grid_indices: impl IntoIterator<Item = usize>) -> Vec<u8> {
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

#[test]
fn terrain_grid_coords_match_cpp_compute_grid_coord_reversal() {
    assert_eq!(
        terrain_grid_coords_for_wow_position_like_cpp(0.0, 0.0),
        (31, 31)
    );
    assert_eq!(
        terrain_grid_coords_for_wow_position_like_cpp(SIZE_OF_GRIDS_LIKE_CPP, 0.0),
        (30, 31)
    );
    assert_eq!(
        terrain_grid_coords_for_wow_position_like_cpp(-SIZE_OF_GRIDS_LIKE_CPP, 0.0),
        (32, 31)
    );
}

#[test]
fn terrain_map_id_without_visible_maps_returns_source_map_like_cpp() {
    let phase_shift = PhaseShift::default();
    let mut called = false;

    let map_id = terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0, |_, _, _| {
        called = true;
        true
    });

    assert_eq!(map_id, 571);
    assert!(!called);
}

#[test]
fn terrain_map_id_single_visible_map_returns_it_like_cpp() {
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    let mut called = false;

    let map_id = terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0, |_, _, _| {
        called = true;
        false
    });

    assert_eq!(map_id, 609);
    assert!(!called);
}

#[test]
fn terrain_map_id_multiple_visible_maps_uses_child_grid_lookup_like_cpp() {
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(700, 1);
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    let mut checked = Vec::new();

    let map_id = terrain_map_id_for_phase_shift_like_cpp(
        &phase_shift,
        571,
        0.0,
        0.0,
        |visible_map_id, gx, gy| {
            checked.push((visible_map_id, gx, gy));
            visible_map_id == 609
        },
    );

    assert_eq!(map_id, 609);
    assert_eq!(checked, vec![(609, 31, 31)]);
}

#[test]
fn terrain_map_id_multiple_visible_maps_falls_back_to_source_map_like_cpp() {
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    phase_shift.add_visible_map_id_like_cpp(700, 1);

    let map_id =
        terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0, |_, _, _| false);

    assert_eq!(map_id, 571);
}

#[test]
fn terrain_grid_files_read_cpp_tilelist_bitset_string_order() {
    let data_dir = unique_temp_data_dir("terrain-grid-tilelist");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write tilelist");

    let terrain = TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 609, &HashMap::new())
        .expect("load terrain grid files");

    assert!(terrain.has_grid_file_like_cpp(31, 31));
    assert!(!terrain.has_grid_file_like_cpp(31, 30));
    fs::remove_dir_all(data_dir).expect("remove test dir");
}

#[test]
fn terrain_grid_files_fallback_validates_map_header_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-grid-map-header");
    fs::write(
        data_dir.join("maps").join("0609_31_31.map"),
        map_file_header_like_cpp(),
    )
    .expect("write map file");
    fs::write(
        data_dir.join("maps").join("0609_31_30.map"),
        b"not a valid map header",
    )
    .expect("write invalid map file");

    let terrain = TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 609, &HashMap::new())
        .expect("load terrain grid files");

    assert!(terrain.has_grid_file_like_cpp(31, 31));
    assert!(!terrain.has_grid_file_like_cpp(31, 30));
    fs::remove_dir_all(data_dir).expect("remove test dir");
}

#[test]
fn terrain_grid_files_has_child_terrain_grid_file_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-grid-child");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0571.tilelist"),
        tilelist_like_cpp([]),
    )
    .expect("write parent tilelist");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write child tilelist");
    let parent_child_map_data = HashMap::from([(571, vec![609]), (609, Vec::new())]);

    let terrain =
        TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 571, &parent_child_map_data)
            .expect("load terrain grid files");

    assert!(terrain.has_child_terrain_grid_file_like_cpp(609, 31, 31));
    assert!(!terrain.has_child_terrain_grid_file_like_cpp(609, 31, 30));
    assert!(!terrain.has_child_terrain_grid_file_like_cpp(700, 31, 31));
    fs::remove_dir_all(data_dir).expect("remove test dir");
}

#[test]
fn terrain_grid_files_resolve_phase_shift_visible_map_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-grid-resolver");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0571.tilelist"),
        tilelist_like_cpp([]),
    )
    .expect("write parent tilelist");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write child tilelist");
    let parent_child_map_data = HashMap::from([(571, vec![609]), (609, Vec::new())]);
    let terrain =
        TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 571, &parent_child_map_data)
            .expect("load terrain grid files");
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(700, 1);
    phase_shift.add_visible_map_id_like_cpp(609, 1);

    assert_eq!(
        terrain.terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0),
        609
    );
    fs::remove_dir_all(data_dir).expect("remove test dir");
}

#[test]
fn terrain_grid_file_index_resolves_root_and_visible_child_map_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-grid-index");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0571.tilelist"),
        tilelist_like_cpp([]),
    )
    .expect("write parent tilelist");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write child tilelist");
    let mut index =
        TerrainGridFileIndexLikeCpp::new(&data_dir, [(571, vec![609]), (609, Vec::new())]);
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(609, 1);

    assert_eq!(index.root_map_id_like_cpp(609), 571);
    assert_eq!(
        index.terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0),
        609
    );
    fs::remove_dir_all(data_dir).expect("remove test dir");
}

#[test]
fn world_mmap_pathfinder_resolves_mesh_map_from_phase_shift_like_cpp() {
    let data_dir = unique_temp_data_dir("mmap-phase-shift-mesh-map");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0571.tilelist"),
        tilelist_like_cpp([]),
    )
    .expect("write parent tilelist");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write child tilelist");
    let mut pathfinder = WorldMMapPathfinderLikeCpp::new_with_parent_map_data_like_cpp(
        &data_dir,
        [(571, vec![609]), (609, Vec::new())],
    );
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    let request = WorldMMapPathRequestLikeCpp {
        start: Position::new(0.0, 0.0, 0.0, 0.0),
        destination: Position::new(20.0, 0.0, 0.0, 0.0),
        mesh_map_id: 571,
        instance_map_id: 571,
        instance_id: 42,
        filter_context: PathQueryFilterContext::creature(true, false, false, false),
        owner: DetourOwnerCapabilitiesLikeCpp::default(),
        previous_poly_refs: Vec::new(),
        force_destination: false,
        point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
        phase_shift,
    };

    assert_eq!(
        pathfinder.resolve_mesh_map_id_for_path_request_like_cpp(&request),
        609
    );
    fs::remove_dir_all(data_dir).expect("remove test dir");
}

#[test]
fn test_world_to_grid_positive() {
    assert_eq!(world_to_grid_x(0.0), 0);
    assert_eq!(world_to_grid_x(63.9), 0);
    assert_eq!(world_to_grid_x(64.0), 1);
    assert_eq!(world_to_grid_x(127.9), 1);
    assert_eq!(world_to_grid_x(128.0), 2);
}

#[test]
fn test_world_to_grid_negative() {
    assert_eq!(world_to_grid_x(-0.1), -1);
    assert_eq!(world_to_grid_x(-64.0), -1);
    assert_eq!(world_to_grid_x(-64.1), -2);
    assert_eq!(world_to_grid_x(-127.9), -2);
    assert_eq!(world_to_grid_x(-128.0), -2);
}

#[test]
fn test_world_to_grid_coords() {
    let (x, y) = world_to_grid_coords(100.0, -50.0);
    assert_eq!(x, 1); // 100 / 64 = 1.56 -> floor = 1
    assert_eq!(y, -1); // -50 / 64 = -0.78 -> floor = -1
}

#[test]
fn test_grid_round_trip() {
    let world_x = 150.5;
    let grid_x = world_to_grid_x(world_x);
    let world_center = grid_to_world(grid_x);
    // Center should be within half grid size
    assert!((world_x - world_center).abs() <= GRID_SIZE / 2.0);
}

#[test]
fn test_creature_add_remove() {
    let mut grid = Grid::new(0, 0);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let creature = WorldCreature::new(
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
    );

    assert!(grid.add_creature(creature.clone()));
    assert_eq!(grid.creature_count(), 1);
    assert!(grid.get_creature(guid).is_some());

    assert!(grid.remove_creature(guid));
    assert_eq!(grid.creature_count(), 0);
    assert!(grid.get_creature(guid).is_none());
}

#[test]
fn test_duplicate_creature_rejected() {
    let mut grid = Grid::new(0, 0);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let creature = WorldCreature::new(
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
    );

    assert!(grid.add_creature(creature.clone()));
    assert!(!grid.add_creature(creature)); // Duplicate should fail
}

#[test]
fn test_player_enter_leave() {
    let mut grid = Grid::new(0, 0);
    let player = ObjectGuid::create_player(1, 1);

    grid.player_enter(player);
    assert!(grid.player_guids.contains(&player));

    grid.player_leave(player);
    assert!(!grid.player_guids.contains(&player));
}

#[test]
fn test_should_unload() {
    let mut grid = Grid::new(0, 0);
    grid.last_player_time = Instant::now() - Duration::from_secs(400);
    assert!(grid.should_unload(Duration::from_secs(300)));
}

#[test]
fn test_should_not_unload_with_player() {
    let mut grid = Grid::new(0, 0);
    let player = ObjectGuid::create_player(1, 1);
    grid.player_enter(player);
    grid.last_player_time = Instant::now() - Duration::from_secs(400);
    assert!(!grid.should_unload(Duration::from_secs(300)));
}

#[test]
fn map_instance_load_personal_phase_grid_tracks_cpp_grid_id_once() {
    let owner = ObjectGuid::create_player(1, 1);
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_phase_like_cpp(10, PhaseFlags::PERSONAL, 1);
    phase_shift.set_personal_guid_like_cpp(owner);
    let mut map = MapInstance::new(571, 0);
    let mut loaded = Vec::new();

    assert!(map.load_personal_phase_grid_like_cpp(
        &phase_shift,
        3,
        5,
        |phase_id| phase_id == 10,
        |owner, phase_id| loaded.push((owner, phase_id)),
    ));
    assert!(map.is_grid_loaded(3, 5));
    assert_eq!(loaded, vec![(owner, 10)]);

    assert!(!map.load_personal_phase_grid_like_cpp(
        &phase_shift,
        3,
        5,
        |phase_id| phase_id == 10,
        |owner, phase_id| loaded.push((owner, phase_id)),
    ));
    assert_eq!(loaded, vec![(owner, 10)]);

    let tracker = map.personal_phases.owner_tracker_like_cpp(owner).unwrap();
    assert!(tracker.is_grid_loaded_for_phase_like_cpp(3 * 64 + 5, 10));
}

#[test]
fn map_instance_unload_grid_purges_personal_phase_grid_tracking_like_cpp() {
    let owner = ObjectGuid::create_player(1, 1);
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_phase_like_cpp(10, PhaseFlags::PERSONAL, 1);
    phase_shift.set_personal_guid_like_cpp(owner);
    let mut map = MapInstance::new(571, 0);

    map.load_personal_phase_grid_like_cpp(&phase_shift, 3, 5, |_| true, |_, _| {});
    assert!(map.remove_grid(3, 5));
    assert!(map.personal_phases.owner_tracker_like_cpp(owner).is_none());
}

#[test]
fn map_instance_update_personal_phases_queues_and_removes_expired_objects_like_cpp() {
    let owner = ObjectGuid::create_player(1, 1);
    let object = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 100);
    let mut map = MapInstance::new(571, 0);
    map.add_creature(0, 0, test_creature(object));
    map.register_personal_phase_object_like_cpp(10, owner, object);
    map.mark_personal_phases_for_deletion_like_cpp(owner);

    map.update_personal_phases_like_cpp(Duration::from_secs(60));
    assert_eq!(map.queued_personal_phase_remove_count_like_cpp(), 1);
    assert!(map.get_creature(0, 0, object).is_some());

    assert_eq!(map.remove_personal_phase_objects_like_cpp(), 1);
    assert!(map.get_creature(0, 0, object).is_none());
}

#[test]
fn test_map_manager_create_map() {
    let mut manager = MapManager::new();
    let map = manager.get_or_create_map(0, 0);
    assert_eq!(map.map_id, 0);
    assert_eq!(map.instance_id, 0);
}

#[test]
fn instance_id_allocator_generates_lowest_free_id_like_cpp() {
    let mut manager = MapManager::new();

    assert_eq!(manager.generate_instance_id(), Some(1));
    assert_eq!(manager.generate_instance_id(), Some(2));
    assert_eq!(manager.generate_instance_id(), Some(3));

    manager.free_instance_id(2);
    assert_eq!(manager.generate_instance_id(), Some(2));
    assert_eq!(manager.generate_instance_id(), Some(4));
}

#[test]
fn instance_id_allocator_registers_loaded_ids_in_order_like_cpp() {
    let mut manager = MapManager::new();
    manager.init_instance_ids_from_max(5);

    manager.register_instance_id(1);
    manager.register_instance_id(2);
    manager.register_instance_id(4);

    assert_eq!(manager.generate_instance_id(), Some(3));
    assert_eq!(manager.generate_instance_id(), Some(5));
    assert_eq!(manager.generate_instance_id(), Some(6));
}

#[test]
fn instance_id_allocator_keeps_zero_reserved_like_cpp() {
    let mut manager = MapManager::new();

    manager.free_instance_id(0);

    assert_eq!(manager.generate_instance_id(), Some(1));
}

#[test]
fn test_add_creature_to_map() {
    let mut manager = MapManager::new();
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let creature = WorldCreature::new(
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
    );

    assert!(manager.add_creature(0, 0, 0, 0, creature));
    assert!(manager.get_creature(0, 0, 0, 0, guid).is_some());
}

#[test]
fn map_manager_uses_canonical_creature_guid_position_and_runtime() {
    let mut manager = MapManager::new();
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );

    assert!(manager.add_creature(0, 0, 0, 0, creature));
    let stored = manager
        .find_creature(0, 0, guid)
        .expect("canonical creature stored");
    assert_eq!(stored.guid(), guid);
    assert_eq!(stored.position(), Position::new(10.0, 10.0, 0.0, 0.0));
    assert_eq!(stored.current_hp(), 50);

    manager
        .find_creature_mut(0, 0, guid)
        .expect("canonical creature mutable")
        .take_damage(25);
    let stored = manager
        .find_creature(0, 0, guid)
        .expect("canonical creature stored");
    assert_eq!(stored.current_hp(), 25);
    assert_eq!(stored.creature.unit().data().health, 25);
}

#[test]
fn world_creature_move_spline_bridge_advances_and_finalizes_like_cpp_unit_tick() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54321);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .expect("bind test creature to map");
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let dst = Position::new(15.0, 10.0, 0.0, 0.0);

    let (from, spline) = creature
        .begin_move_spline_like_cpp(dst)
        .expect("valid two-point spline");

    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert!(creature.active_move_spline.is_some());
    assert_eq!(creature.spline_id(), 2);
    assert!(
        creature
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD),
        "C++ MoveSplineInit::Launch writes MOVEMENTFLAG_FORWARD to Unit::m_movementInfo"
    );
    assert!(
        MovementFlag::from_bits_retain(creature.create_data.movement_flags)
            .contains(MovementFlag::FORWARD),
        "the create bridge must mirror Unit::m_movementInfo after Launch"
    );
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion_spline = &creature.creature.unit().subsystems().motion.spline;
    assert!(motion_spline.enabled);
    assert!(!motion_spline.finalized);
    assert_eq!(motion_spline.spline_id, spline.id());
    assert_eq!(motion_spline.duration_ms, spline.duration_ms() as u32);
    assert_eq!(motion_spline.final_destination, Some((15, 10, 0)));

    let duration_ms = spline.duration_ms() as u32;
    let now_ms = creature.now_ms();
    creature.creature.ai_ownership_mut().move_start_ms =
        now_ms.saturating_sub(u64::from(duration_ms / 2));
    assert!(!creature.update_move_spline_like_cpp());
    let mid = creature.position();
    assert!(mid.x > 10.0 && mid.x < 15.0, "mid position was {mid:?}");
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .motion
            .spline
            .progress_ms,
        duration_ms / 2
    );

    let now_ms = creature.now_ms();
    creature.creature.ai_ownership_mut().move_start_ms =
        now_ms.saturating_sub(u64::from(duration_ms));
    assert!(creature.update_move_spline_like_cpp());
    assert!(creature.active_move_spline.is_none());
    assert_eq!(creature.position(), dst);
    let motion_spline = &creature.creature.unit().subsystems().motion.spline;
    assert!(!motion_spline.enabled);
    assert!(motion_spline.finalized);
    assert_eq!(motion_spline.progress_ms, motion_spline.duration_ms);
    assert!(
        !creature
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD),
        "C++ Unit::DisableSpline removes MOVEMENTFLAG_FORWARD on arrival"
    );
    assert!(
        !MovementFlag::from_bits_retain(creature.create_data.movement_flags)
            .contains(MovementFlag::FORWARD),
        "the create bridge must mirror Unit::m_movementInfo after DisableSpline"
    );
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
}

#[test]
fn world_creature_move_spline_by_path_uses_cpp_moveby_path_bridge() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54322);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let path = [
        Position::new(10.0, 10.0, 0.0, 0.0),
        Position::new(12.0, 11.0, 0.0, 0.0),
        Position::new(15.0, 12.0, 0.0, 0.0),
    ];

    let (from, spline) = creature
        .begin_move_spline_by_path_like_cpp(path)
        .expect("valid multi-point path spline");

    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert!(creature.active_move_spline.is_some());
    assert_eq!(creature.spline_id(), 2);
    assert_eq!(creature.move_target(), Some(path[2]));
    assert_eq!(spline.final_destination(), Some(path[2]));
    assert_eq!(spline.monster_move_path_data().points, vec![path[2]]);
    assert_eq!(spline.monster_move_path_data().packed_deltas.len(), 1);
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion_spline = &creature.creature.unit().subsystems().motion.spline;
    assert!(motion_spline.enabled);
    assert_eq!(motion_spline.spline_id, spline.id());
    assert_eq!(motion_spline.final_destination, Some((15, 12, 0)));
}

#[test]
fn world_creature_waypoint_default_initialize_stores_generator_and_stops_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54329);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let path = WaypointPath::new(
        77,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        ],
    );

    let action = creature.initialize_default_waypoint_movement_like_cpp(Some(path));

    assert_eq!(action, WaypointMovementAction::StopMoving);
    assert!(creature.creature.unit().subsystems().motion.stopped);
    let generator = creature
        .active_waypoint_generator_like_cpp()
        .expect("waypoint generator stored");
    assert_eq!(
        generator.next_move_time_ms(),
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP
    );
    assert_eq!(generator.stop_moving_calls, 1);
}

#[test]
fn world_creature_waypoint_default_initialize_missing_path_does_not_stop_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54330);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );

    let action = creature.initialize_default_waypoint_movement_like_cpp(None);

    assert_eq!(action, WaypointMovementAction::MissingPath);
    assert!(!creature.creature.unit().subsystems().motion.stopped);
    assert!(creature.active_waypoint_generator_like_cpp().is_some());
}

#[test]
fn world_creature_waypoint_default_initialize_resolves_owner_path_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54338);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.creature.load_path_like_cpp(90_001);
    let path = WaypointPath::new(
        90_001,
        vec![wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0)],
    );

    let action =
        creature.initialize_default_waypoint_movement_with_path_resolver_like_cpp(|path_id| {
            (path_id == path.id).then_some(path.clone())
        });

    assert_eq!(action, WaypointMovementAction::StopMoving);
    assert!(
        creature.creature.unit().subsystems().motion.stopped,
        "C++ DoInitialize calls owner->StopMoving() after sWaypointMgr resolves the path"
    );
    assert_eq!(
        creature
            .active_waypoint_generator_like_cpp()
            .map(WaypointMovementGenerator::next_move_time_ms),
        Some(wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP)
    );
}

#[test]
fn world_creature_waypoint_update_launches_initial_node_spline_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54331);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let path = WaypointPath::new(
        77,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        ],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );

    let action = creature.update_default_waypoint_movement_like_cpp(
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
    );

    match action {
        WaypointMovementAction::Launch(launch) => {
            assert_eq!(launch.node_id, 10);
            assert_eq!(launch.path_id, 77);
            assert_eq!(launch.destination, Position::new(11.0, 10.0, 0.0, 0.0));
        }
        other => panic!("expected initial waypoint launch, got {other:?}"),
    }
    assert!(creature.active_move_spline.is_some());
    assert_eq!(
        creature.move_target(),
        Some(Position::new(11.0, 10.0, 0.0, 0.0))
    );
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    assert_eq!(
        creature
            .active_waypoint_generator_like_cpp()
            .expect("waypoint generator")
            .waypoint_started
            .len(),
        1
    );
}

#[test]
fn world_creature_waypoint_generate_path_uses_detour_point_path_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54348);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let destination = Position::new(30.0, 10.0, 0.0, 0.0);
    let path = WaypointPath::new(
        77,
        vec![wow_movement::WaypointNode::new(10, 30.0, 10.0, 0.0)],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    let mut resolver_calls = 0;

    let (action, launched) = creature.update_default_waypoint_movement_with_path_resolver_like_cpp(
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
        true,
        |query| {
            resolver_calls += 1;
            assert_eq!(query.start, Position::new(10.0, 10.0, 0.0, 0.0));
            assert_eq!(query.destination, destination);
            assert_eq!(query.point_path_limit, MAX_POINT_PATH_LENGTH_LIKE_CPP);
            Some(DetourPolyPath {
                poly_refs: vec![11, 22, 33],
                point_path: wow_recastdetour::DetourPointPath {
                    points: vec![[10.0, 10.0, 0.0], [20.0, 15.0, 2.0], [30.0, 10.0, 0.0]],
                    actual_end: [30.0, 10.0, 0.0],
                    path_type: DetourPathType::NORMAL,
                },
                start_far_from_poly: false,
                end_far_from_poly: false,
            })
        },
    );

    assert!(matches!(action, WaypointMovementAction::Launch(_)));
    assert_eq!(resolver_calls, 1);
    let (_from, spline) = launched.expect("waypoint detour path launches");
    assert_eq!(spline.final_destination(), Some(destination));
    assert!(
        spline
            .create_object_path_points_like_cpp()
            .contains(&Position::new(20.0, 15.0, 2.0, 0.0)),
        "C++ MoveSplineInit::MoveTo(generatePath=true) switches to MovebyPath(PathGenerator::GetPath())"
    );
    assert!(
        !spline.monster_move_path_data().packed_deltas.is_empty(),
        "a generated multi-point waypoint path must not serialize as a single direct segment"
    );
}

#[test]
fn world_creature_waypoint_generate_path_nopath_falls_back_direct_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54349);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let destination = Position::new(30.0, 10.0, 0.0, 0.0);
    let path = WaypointPath::new(
        77,
        vec![wow_movement::WaypointNode::new(10, 30.0, 10.0, 0.0)],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    let mut resolver_calls = 0;

    let (_action, launched) = creature
        .update_default_waypoint_movement_with_path_resolver_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
            true,
            |_query| {
                resolver_calls += 1;
                Some(DetourPolyPath {
                    poly_refs: Vec::new(),
                    point_path: wow_recastdetour::DetourPointPath {
                        points: vec![[10.0, 10.0, 0.0], [20.0, 15.0, 2.0], [30.0, 10.0, 0.0]],
                        actual_end: [30.0, 10.0, 0.0],
                        path_type: DetourPathType::NOPATH,
                    },
                    start_far_from_poly: false,
                    end_far_from_poly: false,
                })
            },
        );

    assert_eq!(resolver_calls, 1);
    let (_from, spline) = launched.expect("waypoint direct fallback launches");
    assert_eq!(spline.final_destination(), Some(destination));
    assert!(
        spline.monster_move_path_data().packed_deltas.is_empty(),
        "C++ MoveSplineInit::MoveTo(generatePath=true) falls back to a direct path when PathGenerator reports NOPATH"
    );
}

#[test]
fn world_creature_waypoint_launch_applies_land_takeoff_anim_tier_like_cpp() {
    for (move_type, expected_anim_tier) in [
        (wow_movement::WaypointMoveType::Land, 0),
        (wow_movement::WaypointMoveType::TakeOff, 2),
    ] {
        let guid = ObjectGuid::create_world_object(
            HighGuid::Creature,
            0,
            1,
            0,
            0,
            1,
            54335 + i64::from(expected_anim_tier),
        );
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let mut path = WaypointPath::new(
            90 + expected_anim_tier as u32,
            vec![wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0)],
        );
        path.move_type = move_type;
        assert_eq!(
            creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
            WaypointMovementAction::StopMoving
        );

        assert!(matches!(
            creature.update_default_waypoint_movement_like_cpp(
                wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
            ),
            WaypointMovementAction::Launch(_)
        ));

        assert_eq!(
            creature
                .active_move_spline
                .as_ref()
                .and_then(MoveSpline::anim_tier)
                .map(|anim| anim.anim_tier),
            Some(expected_anim_tier)
        );
    }
}

#[test]
fn world_creature_waypoint_arrival_records_inform_and_launches_next_node_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54332);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let path = WaypointPath::new(
        77,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0).with_delay(500),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        ],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        WaypointMovementAction::Launch(_)
    ));
    creature
        .active_move_spline
        .as_mut()
        .expect("initial waypoint spline")
        .finalize();
    assert!(creature.update_move_spline_like_cpp());

    let arrived = creature.update_default_waypoint_movement_like_cpp(0);

    match arrived {
        WaypointMovementAction::Arrived(arrived) => {
            assert_eq!(arrived.inform.node_id, 10);
            assert_eq!(arrived.inform.path_id, 77);
            assert_eq!(arrived.timer_ms, Some(500));
        }
        other => panic!("expected waypoint arrival, got {other:?}"),
    }
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Waypoint.trinity_id(),
            movement_id: 10,
        })
    );

    let next = creature.update_default_waypoint_movement_like_cpp(500);

    match next {
        WaypointMovementAction::Launch(launch) => {
            assert_eq!(launch.node_id, 20);
            assert_eq!(launch.path_id, 77);
            assert_eq!(launch.destination, Position::new(12.0, 10.0, 0.0, 0.0));
        }
        other => panic!("expected next waypoint launch, got {other:?}"),
    }
    assert_eq!(
        creature.move_target(),
        Some(Position::new(12.0, 10.0, 0.0, 0.0))
    );
}

#[test]
fn world_creature_waypoint_arrival_without_delay_launches_next_node_same_tick_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54333);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let path = WaypointPath::new(
        88,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        ],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        WaypointMovementAction::Launch(_)
    ));
    creature
        .active_move_spline
        .as_mut()
        .expect("single waypoint spline")
        .finalize();
    assert!(creature.update_move_spline_like_cpp());

    let action = creature.update_default_waypoint_movement_like_cpp(0);

    match action {
        WaypointMovementAction::Launch(launch) => {
            assert_eq!(launch.node_id, 20);
            assert_eq!(launch.path_id, 88);
            assert_eq!(launch.destination, Position::new(12.0, 10.0, 0.0, 0.0));
        }
        other => panic!("expected same-tick next waypoint launch, got {other:?}"),
    }
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Waypoint.trinity_id(),
            movement_id: 10,
        })
    );
    assert_eq!(
        creature.move_target(),
        Some(Position::new(12.0, 10.0, 0.0, 0.0))
    );
}

#[test]
fn world_creature_waypoint_tick_advances_spline_before_motionmaster_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54338);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let path = WaypointPath::new(
        92,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        ],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        WaypointMovementAction::Launch(_)
    ));
    creature
        .active_move_spline
        .as_mut()
        .expect("initial waypoint spline")
        .finalize();
    assert!(
        !creature
            .creature
            .unit()
            .subsystems()
            .motion
            .spline
            .finalized,
        "the represented MotionSubsystem is stale until Unit::UpdateSplineMovement runs"
    );

    let action = creature.update_default_waypoint_movement_like_cpp(0);

    match action {
        WaypointMovementAction::Launch(launch) => {
            assert_eq!(launch.node_id, 20);
            assert_eq!(launch.path_id, 92);
            assert_eq!(launch.destination, Position::new(12.0, 10.0, 0.0, 0.0));
        }
        other => panic!("expected next waypoint launch after spline advance, got {other:?}"),
    }
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Waypoint.trinity_id(),
            movement_id: 10,
        })
    );
}

#[test]
fn world_creature_waypoint_single_node_path_ends_same_tick_after_arrival_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54334);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let path = WaypointPath::new(
        89,
        vec![wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0)],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        WaypointMovementAction::Launch(_)
    ));
    creature
        .active_move_spline
        .as_mut()
        .expect("single waypoint spline")
        .finalize();
    assert!(creature.update_move_spline_like_cpp());

    let ended = creature.update_default_waypoint_movement_like_cpp(0);

    match ended {
        WaypointMovementAction::PathEnded(ended) => {
            assert_eq!(ended.node_id, 10);
            assert_eq!(ended.path_id, 89);
        }
        other => panic!("expected waypoint path end, got {other:?}"),
    }
    assert_eq!(
        creature.home_position(),
        Position::new(11.0, 10.0, 0.0, 0.0)
    );
}

#[test]
fn world_creature_waypoint_path_end_random_handoff_launches_active_random_spline_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54337);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.seed_runtime_rng_like_cpp(0x91_5EED);
    let mut path = WaypointPath::new(
        91,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(30, 13.0, 10.0, 0.0),
        ],
    );
    path.follow_path_backwards_from_end_to_start = true;
    creature.active_waypoint_generator = Some(WaypointMovementGenerator::from_path(
        path,
        true,
        Some(10_000),
        None,
        wow_movement::MovementWalkRunSpeedSelectionMode::Default,
        Some((1_000, 2_000)),
        Some(5.0),
        true,
        true,
    ));

    for expected_node in [10, 20, 30] {
        match creature.update_default_waypoint_movement_like_cpp(0) {
            WaypointMovementAction::Launch(launch) => assert_eq!(launch.node_id, expected_node),
            other => panic!("expected waypoint launch for node {expected_node}, got {other:?}"),
        }
        creature
            .active_move_spline
            .as_mut()
            .expect("active waypoint spline")
            .finalize();
        assert!(creature.update_move_spline_like_cpp());
    }

    let action = creature.update_default_waypoint_movement_with_wait_roll_like_cpp(0, Some(1_500));

    match action {
        WaypointMovementAction::Arrived(arrived) => {
            assert_eq!(arrived.inform.node_id, 30);
            assert_eq!(
                arrived.move_random_at_path_end,
                Some(WaypointRandomAtPathEnd {
                    wander_distance: 5.0,
                    duration_ms: 1_500,
                })
            );
            assert_eq!(arrived.duration_after_wait_ms, Some(8_500));
        }
        other => panic!("expected endpoint random handoff arrival, got {other:?}"),
    }
    let random_target = creature
        .move_target()
        .expect("C++ MoveRandom handoff should launch an active random spline");
    assert!(creature.active_move_spline.is_some());
    assert!(
        random_target.distance_2d(&Position::new(13.0, 10.0, 0.0, 0.0)) <= 5.001,
        "C++ RandomMovementGenerator chooses a destination within _wanderDistance of its reference"
    );
    assert_eq!(
        creature.active_waypoint_random_at_path_end_like_cpp(),
        Some(WaypointRandomAtPathEnd {
            wander_distance: 5.0,
            duration_ms: 1_500,
        })
    );
    assert_eq!(
        creature
            .active_waypoint_generator_like_cpp()
            .and_then(WaypointMovementGenerator::duration_ms),
        Some(8_500)
    );

    assert_eq!(
        creature.update_default_waypoint_movement_like_cpp(100),
        WaypointMovementAction::Continue
    );
    assert!(creature.active_move_spline.is_some());
    assert_eq!(
        creature.active_waypoint_random_at_path_end_like_cpp(),
        Some(WaypointRandomAtPathEnd {
            wander_distance: 5.0,
            duration_ms: 1_400,
        })
    );
}

#[test]
fn world_creature_detour_path_bridge_uses_moveby_path_or_direct_fallback_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54324);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let normal_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 0.0], [12.0, 11.0, 0.0], [15.0, 12.0, 0.0]],
            actual_end: [15.0, 12.0, 0.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };
    let dst = Position::new(15.0, 12.0, 0.0, 0.0);

    let (from, spline, path) = creature
        .begin_move_spline_with_detour_path_like_cpp(dst, Some(&normal_path), false)
        .expect("detour path launches");

    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert_eq!(spline.final_destination(), Some(dst));
    assert_eq!(spline.monster_move_path_data().points, vec![dst]);
    let path = path.expect("path generator");
    assert_eq!(path.path_type(), PathType::NORMAL);
    assert_eq!(path.poly_length(), 2);
    assert_eq!(
        path.path_points(),
        &[
            Position::new(10.0, 10.0, 0.0, 0.0),
            Position::new(12.0, 11.0, 0.0, 0.0),
            dst
        ]
    );

    let nopath = DetourPolyPath {
        poly_refs: Vec::new(),
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[15.0, 12.0, 0.0], [20.0, 10.0, 0.0]],
            actual_end: [20.0, 10.0, 0.0],
            path_type: DetourPathType::NOPATH,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };
    let fallback_dst = Position::new(20.0, 10.0, 0.0, 0.0);

    let (_from, fallback_spline, fallback_path) = creature
        .begin_move_spline_with_detour_path_like_cpp(fallback_dst, Some(&nopath), false)
        .expect("direct fallback launches");

    assert_eq!(fallback_spline.final_destination(), Some(fallback_dst));
    assert!(
        fallback_path
            .expect("fallback path metadata")
            .path_type()
            .contains(PathType::NOPATH)
    );
}

#[test]
fn world_creature_detour_path_bridge_normalizes_points_to_terrain_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54350);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 1.75, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(1, 0)
        .expect("bind test creature to terrain map");
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let data_dir = temp_dir_with_constant_tile(1, 31, 31, 2.0);
    let terrain = LiveTerrainHeights::new(&data_dir);
    assert!(
        (terrain.static_height_like_cpp(1, 10.0, 10.0, 51.75) - 2.0).abs() < 1e-3,
        "synthetic terrain tile must cover the test path"
    );
    let dst = Position::new(15.0, 12.0, 1.75, 0.0);
    let normal_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 1.75], [12.0, 11.0, 1.75], [15.0, 12.0, 1.75]],
            actual_end: [15.0, 12.0, 1.75],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&normal_path),
            false,
            Some(&terrain),
        )
        .expect("terrain-normalized detour path launches");

    let path = path.expect("path generator");
    assert_eq!(
        path.path_points(),
        &[
            Position::new(10.0, 10.0, 2.0, 0.0),
            Position::new(12.0, 11.0, 2.0, 0.0),
            Position::new(15.0, 12.0, 2.0, 0.0),
        ],
        "C++ PathGenerator::NormalizePath calls UpdateAllowedPositionZ for every path point"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 2.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn world_creature_detour_path_bridge_raises_low_mmap_points_to_grid_ground_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54351);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 43.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(1, 0)
        .expect("bind test creature to terrain map");
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let data_dir = temp_dir_with_constant_tile(1, 31, 31, 50.0);
    let terrain = LiveTerrainHeights::new(&data_dir);
    assert!(
        terrain.static_height_like_cpp(1, 10.0, 10.0, 43.0 + Z_OFFSET_FIND_HEIGHT)
            <= INVALID_HEIGHT,
        "the C++ probe gate rejects this low Rust MMap point"
    );
    assert!(
        (terrain.grid_height_like_cpp(1, 10.0, 10.0) - 50.0).abs() < 1e-3,
        "synthetic terrain still has a usable raw ground height"
    );
    let dst = Position::new(15.0, 12.0, 43.0, 0.0);
    let low_mmap_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 43.0], [12.0, 11.0, 43.0], [15.0, 12.0, 43.0]],
            actual_end: [15.0, 12.0, 43.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&low_mmap_path),
            false,
            Some(&terrain),
        )
        .expect("terrain-normalized low detour path launches");

    let path = path.expect("path generator");
    assert_eq!(
        path.path_points(),
        &[
            Position::new(10.0, 10.0, 50.0, 0.0),
            Position::new(12.0, 11.0, 50.0, 0.0),
            Position::new(15.0, 12.0, 50.0, 0.0),
        ],
        "NormalizePath must not serialize underground Rust MMap points to the client"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 50.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn world_creature_detour_path_bridge_preserves_elevated_mmap_points_without_vmap() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54352);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 30.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        wow_constants::UnitFlags::CAN_SWIM.bits(),
    );
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(1, 0)
        .expect("bind test creature to terrain map");
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let data_dir = temp_dir_with_constant_tile(1, 31, 31, 2.0);
    let terrain = LiveTerrainHeights::new(&data_dir);
    let dst = Position::new(15.0, 12.0, 30.0, 0.0);
    let elevated_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 30.0], [12.0, 11.0, 30.0], [15.0, 12.0, 30.0]],
            actual_end: [15.0, 12.0, 30.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&elevated_path),
            false,
            Some(&terrain),
        )
        .expect("elevated detour path launches");

    assert_eq!(
        path.expect("path generator").path_points(),
        &[
            Position::new(10.0, 10.0, 30.0, 0.0),
            Position::new(12.0, 11.0, 30.0, 0.0),
            Position::new(15.0, 12.0, 30.0, 0.0),
        ]
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 30.0, 0.0))
    );
    assert!(creature.creature.can_swim_like_cpp());
    assert!(
        spline.flags().contains(MoveSplineFlag::CAN_SWIM),
        "MoveSplineInit must snapshot Unit::CanSwim like C++"
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn world_creature_detour_path_bridge_does_not_join_unproven_flat_elevated_surfaces() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54353);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 30.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(1, 0)
        .expect("bind test creature to terrain map");
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let data_dir = temp_dir_with_constant_tile(1, 31, 31, 2.0);
    let terrain = LiveTerrainHeights::new(&data_dir);
    let dst = Position::new(15.0, 12.0, 30.0, 0.0);
    let projected_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 2.0], [12.0, 11.0, 2.0], [15.0, 12.0, 2.0]],
            actual_end: [15.0, 12.0, 2.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&projected_path),
            false,
            Some(&terrain),
        )
        .expect("projected elevated detour path launches");

    assert_eq!(
        path.expect("path generator").path_points(),
        &[
            Position::new(10.0, 10.0, 2.0, 0.0),
            Position::new(12.0, 11.0, 2.0, 0.0),
            Position::new(15.0, 12.0, 2.0, 0.0),
        ],
        "equal endpoint heights do not prove one continuous elevated surface"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 2.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn world_creature_detour_path_bridge_does_not_invent_a_sloped_vmap_surface() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54354);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 30.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(1, 0)
        .expect("bind test creature to terrain map");
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let data_dir = temp_dir_with_constant_tile(1, 31, 31, 2.0);
    let terrain = LiveTerrainHeights::new(&data_dir);
    let dst = Position::new(15.0, 12.0, 34.0, 0.0);
    let projected_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 2.0], [12.0, 11.0, 2.0], [15.0, 12.0, 2.0]],
            actual_end: [15.0, 12.0, 2.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&projected_path),
            false,
            Some(&terrain),
        )
        .expect("unproven sloped path still launches");

    assert_eq!(
        path.expect("path generator").path_points(),
        &[
            Position::new(10.0, 10.0, 2.0, 0.0),
            Position::new(12.0, 11.0, 2.0, 0.0),
            Position::new(15.0, 12.0, 2.0, 0.0),
        ],
        "without VMap height Rust must not guess a changing elevated surface"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 2.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn world_creature_detour_path_bridge_keeps_far_below_points_without_ground_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54352);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, -5.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(1, 0)
        .expect("bind test creature to terrain map");
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let data_dir = temp_dir_with_constant_tile(1, 31, 31, 50.0);
    let terrain = LiveTerrainHeights::new(&data_dir);
    let dst = Position::new(15.0, 12.0, -5.0, 0.0);
    let far_below_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, -5.0], [12.0, 11.0, -5.0], [15.0, 12.0, -5.0]],
            actual_end: [15.0, 12.0, -5.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&far_below_path),
            false,
            Some(&terrain),
        )
        .expect("far-below detour path launches without terrain lift");

    let path = path.expect("path generator");
    assert_eq!(
        path.path_points(),
        &[
            Position::new(10.0, 10.0, -5.0, 0.0),
            Position::new(12.0, 11.0, -5.0, 0.0),
            Position::new(15.0, 12.0, -5.0, 0.0),
        ],
        "points farther than DEFAULT_HEIGHT_SEARCH below raw ground keep C++ no-ground behavior"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, -5.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn world_creature_random_detour_rejects_nopath_and_shortcut_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54327);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let dst = Position::new(20.0, 10.0, 0.0, 0.0);

    for path_type in [DetourPathType::NOPATH, DetourPathType::SHORTCUT] {
        let detour_path = DetourPolyPath {
            poly_refs: Vec::new(),
            point_path: wow_recastdetour::DetourPointPath {
                points: vec![[10.0, 10.0, 0.0], [20.0, 10.0, 0.0]],
                actual_end: [20.0, 10.0, 0.0],
                path_type,
            },
            start_far_from_poly: false,
            end_far_from_poly: false,
        };

        assert!(
            creature
                .begin_random_move_spline_with_detour_path_like_cpp(dst, Some(&detour_path), false)
                .is_none(),
            "C++ RandomMovementGenerator retries later instead of launching {:?} paths",
            path_type
        );
        assert!(creature.active_move_spline_like_cpp().is_none());
    }
}

#[test]
fn world_creature_random_missing_path_retries_instead_of_direct_fallback_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54340);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 8.0;
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    creature.seed_runtime_rng_like_cpp(0x24_5A0);

    let mut resolver_called = false;
    let movement =
        creature.update_default_random_movement_with_path_resolver_like_cpp(10, true, |_query| {
            resolver_called = true;
            None
        });

    assert!(resolver_called);
    assert!(
        movement.is_none(),
        "C++ RandomMovementGenerator retries when PathGenerator cannot build a usable path"
    );
    assert!(creature.active_move_spline_like_cpp().is_none());
    assert_eq!(
        creature
            .active_random_generator
            .as_ref()
            .expect("random generator")
            .timer_ms(),
        wow_movement::RANDOM_PATH_RETRY_MS_LIKE_CPP
    );
}

/// C++ `PathGenerator::UpdateFilter` adds `NAV_GROUND_STEEP` while the owner
/// `IsInCombat()` (`PathGenerator.cpp:694-696`). `Creature::enter_ai_combat`
/// does not set the client-visible `UNIT_FLAG_IN_COMBAT`, so the filter has
/// to read the runtime combat state the tick actually maintains.
#[test]
fn world_creature_chase_filter_includes_ground_steep_while_engaged_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54350);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 77);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::Run as u8,
    );
    creature.creature.set_swim_allowed_runtime_like_cpp(false);

    let idle = create_path_query_filter_like_cpp(creature.path_query_filter_context_like_cpp())
        .expect("filter");
    assert_eq!(
        idle.include_flags(),
        wow_recastdetour::NavTerrainFlag::GROUND.bits(),
        "an idle ground creature must not get NAV_GROUND_STEEP"
    );

    creature.enter_combat(victim);
    assert!(
        creature.creature.is_in_combat(),
        "enter_combat must leave a runtime combat signal the filter can read"
    );
    let engaged = create_path_query_filter_like_cpp(creature.path_query_filter_context_like_cpp())
        .expect("filter");
    assert_eq!(
        engaged.include_flags(),
        (wow_recastdetour::NavTerrainFlag::GROUND | wow_recastdetour::NavTerrainFlag::GROUND_STEEP)
            .bits(),
        "C++ UpdateFilter grants steep ground to a creature in combat"
    );
}

/// C++ `Unit::isInAccessiblePlaceFor` branches on the victim's real
/// `IsInWater()`. Creature victims carry no liquid state here, and guessing
/// "not in water" would make an aquatic, non-walking chaser report
/// `CannotReachTarget` on a victim C++ would let it reach.
#[test]
fn world_creature_chase_unknown_water_does_not_block_an_aquatic_chaser_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54351);
    let victim_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54352);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    // A swim-only creature: no ground movement, no flight.
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::None as u8,
    );
    creature.creature.set_swim_allowed_runtime_like_cpp(true);
    creature.creature.set_flight_movement_type_runtime_like_cpp(
        wow_constants::CreatureFlightMovementType::None as u8,
    );
    assert!(!creature.creature.can_walk_like_cpp());
    assert!(creature.creature.can_enter_water_like_cpp());

    let target = |in_water| ChaseTargetSnapshotLikeCpp {
        guid: victim_guid,
        position: Position::new(20.0, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water,
    };

    assert!(
        creature
            .chase_unit_snapshot_like_cpp(target(None))
            .target_accessible,
        "an unknown water state must not block a chaser that can enter water"
    );
    assert!(
        creature
            .chase_unit_snapshot_like_cpp(target(Some(true)))
            .target_accessible,
        "C++ takes the water branch and asks CanEnterWater()"
    );
    assert!(
        !creature
            .chase_unit_snapshot_like_cpp(target(Some(false)))
            .target_accessible,
        "C++ takes the else branch and asks CanWalk() || CanFly()"
    );
}

/// C++ installs a new `ChaseMovementGenerator` per `MoveChase`, and its
/// `AbstractFollower` is bound to that victim for the generator's life
/// (`ChaseMovementGenerator.cpp:68-76`). A victim switch must not inherit the
/// previous follower, timers or arrival inform counter.
#[test]
fn world_creature_chase_rebuilds_the_generator_when_the_victim_changes_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54353);
    let first = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 91);
    let second = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 92);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );

    let target = |victim, x| ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(x, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    };

    creature.enter_combat(first);
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        target(first, 40.0),
        false,
        None,
        |_| None,
    );
    assert_eq!(
        creature
            .active_chase_generator_like_cpp()
            .and_then(wow_movement::ChaseMovementGenerator::target),
        Some(first)
    );

    creature.enter_combat(second);
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        target(second, 45.0),
        false,
        None,
        |_| None,
    );
    assert_eq!(
        creature
            .active_chase_generator_like_cpp()
            .and_then(wow_movement::ChaseMovementGenerator::target),
        Some(second),
        "the generator must follow the new victim, not the previous one"
    );
}

/// C++ replaces its owned `PathGenerator` before `CalculatePath` when
/// `moveToward != _movingTowards`, while keeping it for same-direction
/// recalculations (`ChaseMovementGenerator.cpp:170-175`). The retained
/// Detour corridor has to follow that exact lifecycle.
#[test]
fn world_creature_chase_direction_flip_resets_corridor_and_same_direction_reuses_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54361);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 98);
    let mut creature = test_creature(guid);
    creature.enter_combat(victim);

    let launched = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 60.0),
        true,
        None,
        |query| {
            assert!(query.previous_poly_refs.is_empty());
            Some(test_chase_corridor(vec![101, 102], 59.0))
        },
    );
    assert!(matches!(launched, ChaseTickOutcomeLikeCpp::Launched(..)));
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[101, 102]);
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .moving_towards()
    );

    let launched = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 65.0),
        true,
        None,
        |query| {
            assert_eq!(query.previous_poly_refs, vec![101, 102]);
            Some(test_chase_corridor(vec![201, 202], 64.0))
        },
    );
    assert!(matches!(launched, ChaseTickOutcomeLikeCpp::Launched(..)));
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[201, 202]);

    let launched = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 10.5),
        true,
        None,
        |query| {
            assert!(
                query.previous_poly_refs.is_empty(),
                "a toward-to-away flip must discard the old path object"
            );
            Some(test_chase_corridor(vec![301, 302], 7.0))
        },
    );
    assert!(matches!(launched, ChaseTickOutcomeLikeCpp::Launched(..)));
    assert!(
        !creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .moving_towards()
    );
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[301, 302]);

    let launched = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 70.0),
        true,
        None,
        |query| {
            assert!(
                query.previous_poly_refs.is_empty(),
                "an away-to-toward flip must discard the old path object"
            );
            Some(test_chase_corridor(vec![401, 402], 69.0))
        },
    );
    assert!(matches!(launched, ChaseTickOutcomeLikeCpp::Launched(..)));
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .moving_towards()
    );
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[401, 402]);
}

#[test]
fn world_creature_failed_direction_flip_does_not_publish_unlaunched_direction() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54362);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 99);
    let mut creature = test_creature(guid);
    creature.enter_combat(victim);

    assert!(matches!(
        creature.update_runtime_chase_movement_like_cpp(
            1,
            test_chase_target(victim, 60.0),
            true,
            None,
            |_| Some(test_chase_corridor(vec![501, 502], 59.0)),
        ),
        ChaseTickOutcomeLikeCpp::Launched(..)
    ));
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[501, 502]);

    let failed = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 10.5),
        true,
        None,
        |query| {
            assert!(
                query.previous_poly_refs.is_empty(),
                "the old toward corridor must be gone before the away query"
            );
            None
        },
    );
    assert!(matches!(
        failed,
        ChaseTickOutcomeLikeCpp::Stopped(_) | ChaseTickOutcomeLikeCpp::Idle
    ));
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .moving_towards(),
        "a failed query must not publish a move-away direction that never launched"
    );
    assert!(creature.active_chase_path_poly_refs_like_cpp().is_empty());
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .has_flag(RuntimeMovementGeneratorFlags::INFORM_ENABLED),
        "failed path handling keeps the existing C++ inform lifecycle"
    );
}

#[test]
fn world_creature_nopath_after_arrival_does_not_reenable_consumed_inform_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54363);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 100);
    let mut creature = test_creature(guid);
    creature.enter_combat(victim);

    assert!(matches!(
        creature.update_runtime_chase_movement_like_cpp(
            1,
            test_chase_target(victim, 60.0),
            true,
            None,
            |_| Some(test_chase_corridor(vec![601, 602], 59.0)),
        ),
        ChaseTickOutcomeLikeCpp::Launched(..)
    ));

    let duration_ms = creature
        .active_move_spline_like_cpp()
        .expect("chase spline")
        .duration_ms();
    creature
        .backdate_runtime_clock_for_test(Duration::from_millis(u64::from(duration_ms as u32) + 50));
    assert!(creature.update_move_spline_like_cpp());
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        test_chase_target(victim, 60.0),
        true,
        None,
        |_| panic!("arrival must not calculate another path"),
    );
    assert!(
        !creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .has_flag(RuntimeMovementGeneratorFlags::INFORM_ENABLED)
    );
    assert!(
        creature.creature.take_ai_movement_inform().is_some(),
        "the successful spline arrival must consume and publish its inform"
    );

    let failed = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 100.0),
        true,
        None,
        |_| None,
    );
    assert!(matches!(
        failed,
        ChaseTickOutcomeLikeCpp::Stopped(_) | ChaseTickOutcomeLikeCpp::Idle
    ));
    let generator = creature
        .active_chase_generator_like_cpp()
        .expect("chase generator");
    assert!(generator.cannot_reach_target);
    assert!(
        !generator.has_flag(RuntimeMovementGeneratorFlags::INFORM_ENABLED),
        "NOPATH after arrival must preserve the consumed inform state"
    );

    let owner_position = creature.position();
    let in_range_target = ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(
            owner_position.x + 3.0,
            owner_position.y,
            owner_position.z,
            0.0,
        ),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    };
    let snapshot = creature.chase_unit_snapshot_like_cpp(in_range_target);
    let bounds = creature
        .active_chase_generator_like_cpp()
        .expect("chase generator")
        .bounds_like_cpp(snapshot);
    assert!(
        !snapshot.owner_has_chase_move
            && wow_movement::generators::chase::position_okay_like_cpp(
                snapshot,
                Some(bounds.min_range),
                Some(bounds.max_range),
                None,
            ),
        "the final target must exercise the in-range, no-active-spline branch: \
         snapshot={snapshot:?}, bounds={bounds:?}"
    );
    assert_eq!(
        creature.update_runtime_chase_movement_like_cpp(
            100,
            in_range_target,
            true,
            None,
            |_| panic!("an in-range target must not calculate another path"),
        ),
        ChaseTickOutcomeLikeCpp::Idle
    );
    assert!(
        creature.creature.take_ai_movement_inform().is_none(),
        "a route that never launched must not publish a duplicate arrival inform"
    );
}

/// C++ chase calls `CalculatePath(x, y, z, owner->CanFly())`
/// (`ChaseMovementGenerator.cpp:196`), and `_forceDestination` is consumed
/// *inside* `BuildPointPath`, so the flag has to reach the query itself.
#[test]
fn world_creature_chase_passes_can_fly_as_force_destination_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54354);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 93);

    for can_fly in [false, true] {
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature
            .creature
            .set_flight_movement_type_runtime_like_cpp(if can_fly {
                wow_constants::CreatureFlightMovementType::CanFly as u8
            } else {
                wow_constants::CreatureFlightMovementType::None as u8
            });
        assert_eq!(creature.creature.can_fly_like_cpp(), can_fly);
        creature.enter_combat(victim);

        let mut observed = None;
        let _ = creature.update_runtime_chase_movement_like_cpp(
            100,
            ChaseTargetSnapshotLikeCpp {
                guid: victim,
                position: Position::new(60.0, 10.0, 0.0, 0.0),
                combat_reach: 1.0,
                in_world: true,
                in_water: Some(false),
            },
            true,
            None,
            |query| {
                observed = Some(query.force_destination);
                None
            },
        );
        assert_eq!(
            observed,
            Some(can_fly),
            "the chase query must carry forceDest = CanFly()"
        );
    }
}

/// `resolve_creature_detour_path_like_cpp` already answers a missing
/// navmesh with the C++ `BuildShortcut()` path, so a `None` from it means the
/// query was attempted and failed. C++ has no such case — its own failures
/// went through `BuildShortcut()` + `PATHFIND_NOPATH` — so it must stop and
/// retry rather than launch a straight line through blocked geometry.
#[test]
fn world_creature_chase_query_failure_stops_instead_of_straight_lining_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54355);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 94);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.enter_combat(victim);
    let target = ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(60.0, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    };

    // Pathfinding attempted and failed: no spline, and cannot-reach is set.
    let outcome =
        creature.update_runtime_chase_movement_like_cpp(100, target, true, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(creature.active_move_spline_like_cpp().is_none());
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .cannot_reach_target,
        "a failed query must record cannot-reach like the C++ NOPATH branch"
    );

    // Pathfinding disabled for this map/owner: C++ CalculatePath answers with
    // BuildShortcut() + NORMAL|NOT_USING_PATH, which chase launches.
    let mut disabled = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    disabled.enter_combat(victim);
    let outcome =
        disabled.update_runtime_chase_movement_like_cpp(100, target, false, None, |_| None);
    assert!(
        matches!(outcome, ChaseTickOutcomeLikeCpp::Launched(..)),
        "with no navmesh C++ still launches the shortcut, got {outcome:?}"
    );
    assert!(
        !disabled
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .cannot_reach_target,
        "a disabled navmesh is not a path failure"
    );
}

#[test]
fn world_creature_taunt_priority_expires_at_aura_duration_like_cpp() {
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54355);
    let taunter = ObjectGuid::create_player(1, 1);
    let newer_taunter = ObjectGuid::create_player(1, 3);
    let tank = ObjectGuid::create_player(1, 2);
    let mut creature = WorldCreature::new(
        creature_guid,
        1,
        Position::default(),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    {
        let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
        combat.add_threat(taunter, 10.0);
        combat.add_threat(newer_taunter, 20.0);
        combat.add_threat(tank, 100.0);
        assert_eq!(combat.reselect_victim(&HashSet::new()), Some(tank));
    }

    assert_eq!(
        creature.apply_taunt_aura_like_cpp(taunter, 100, 1, 2_000),
        Some(0)
    );
    assert_eq!(
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .reselect_victim(&HashSet::new()),
        Some(taunter),
        "an active taunt bypasses ordinary threat-switch thresholds"
    );
    assert_eq!(
        creature.apply_taunt_aura_like_cpp(newer_taunter, 100, 1, 1_000),
        Some(1)
    );
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .sorted_threat_guids()
            .first(),
        Some(&newer_taunter)
    );
    {
        let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
        assert_eq!(combat.reselect_victim(&HashSet::new()), Some(newer_taunter));
        assert!(
            combat
                .set_threat_online_state(newer_taunter, wow_entities::ThreatOnlineState::Offline,)
        );
        assert_eq!(
            combat.reselect_victim(&HashSet::new()),
            Some(taunter),
            "C++ falls back to the older active taunt while the newest taunter is unavailable"
        );
        assert!(
            combat.set_threat_online_state(newer_taunter, wow_entities::ThreatOnlineState::Online,)
        );
    }

    creature.backdate_runtime_clock_for_test(Duration::from_millis(1_200));
    assert_eq!(creature.expire_taunt_auras_if_due_like_cpp(), vec![1]);
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .sorted_threat_guids()
            .first(),
        Some(&taunter),
        "when the newest taunt expires C++ restores the still-active older taunt"
    );

    creature.backdate_runtime_clock_for_test(Duration::from_millis(2_200));
    assert_eq!(creature.expire_taunt_auras_if_due_like_cpp(), vec![0]);
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .sorted_threat_guids()
            .first(),
        Some(&tank)
    );

    let reset_slot = creature
        .apply_taunt_aura_like_cpp(taunter, 102, 1, 2_000)
        .expect("taunt aura slot");
    assert_eq!(creature.reset_combat(), vec![reset_slot]);
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .auras
            .visible_auras
            .values()
            .all(|aura| aura.spell_id != 102),
        "C++ RemoveAurasOnEvade removes the visible taunt aura during combat reset"
    );
    assert!(
        creature.expire_taunt_auras_if_due_like_cpp().is_empty(),
        "an evade-removed taunt must not expire a second time"
    );
}

#[test]
fn world_creature_permanent_taunt_never_expires_like_cpp() {
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54356);
    let taunter = ObjectGuid::create_player(1, 4);
    let tank = ObjectGuid::create_player(1, 5);
    let mut creature = WorldCreature::new(
        creature_guid,
        1,
        Position::default(),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
    combat.add_threat(taunter, 10.0);
    combat.add_threat(tank, 100.0);
    assert_eq!(
        creature.apply_taunt_aura_like_cpp(taunter, 101, 1, -1),
        Some(0)
    );
    creature.backdate_runtime_clock_for_test(Duration::from_secs(86_400));
    assert!(creature.expire_taunt_auras_if_due_like_cpp().is_empty());
    assert_eq!(
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .reselect_victim(&HashSet::new()),
        Some(taunter)
    );
}

#[test]
fn world_creature_taunt_aura_persists_without_threat_reference_like_cpp() {
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54356);
    let caster = ObjectGuid::create_player(1, 4);
    let mut creature = WorldCreature::new(
        creature_guid,
        1,
        Position::default(),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );

    let slot = creature
        .apply_taunt_aura_like_cpp(caster, 355, 1, 2_000)
        .expect("C++ still applies MOD_TAUNT when EffectTaunt sees an empty threat list");

    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .is_threat_list_empty(true)
    );
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .auras
            .visible_auras
            .contains_key(&slot)
    );
}

/// C++ adds `UNIT_STATE_EVADE` immediately before `MoveTargetedHome()`
/// (`CreatureAI.cpp:237`) and only `HomeMovementGenerator::DoFinalize` clears
/// it (`HomeMovementGenerator.cpp:143`). It is what makes the creature immune
/// to attacks and un-aggroable for the whole walk back, and
/// `SetTargetLocation` deliberately preserves it by clearing
/// `UNIT_STATE_ALL_ERASABLE & ~UNIT_STATE_EVADE` (`:60`).
#[test]
fn world_creature_home_return_holds_evade_state_until_finalize_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54356);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(40.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature
        .creature
        .set_ai_home_position(Position::new(10.0, 10.0, 0.0, 0.0));
    creature.creature.unit_mut().set_health(17);
    assert!(!creature.creature.is_in_evade_mode_like_cpp());

    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert!(
        matches!(outcome, ChaseTickOutcomeLikeCpp::Launched(..)),
        "the home return must launch, got {outcome:?}"
    );
    assert!(
        creature.creature.is_in_evade_mode_like_cpp(),
        "the creature must be evading for the whole walk home, or a player \
         can damage and re-aggro a fully reset creature"
    );
    assert!(
        creature.creature.is_evading_attacks_like_cpp(),
        "C++ IsEvadingAttacks() gates damage while returning"
    );

    // Advancing while the spline is still running must not drop the state,
    // and C++ only fires the reached-home payload once `DoUpdate` has seen
    // the spline finalized (it is what sets `INFORM_ENABLED`).
    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(creature.creature.is_in_evade_mode_like_cpp());
    assert!(
        !creature.creature.take_ai_just_reached_home(),
        "JustReachedHome must not fire while the creature is still walking"
    );

    // Let real time pass beyond the spline duration and advance it, exactly
    // as `Unit::Update` does before `MotionMaster::Update`.
    let duration_ms = creature
        .active_move_spline_like_cpp()
        .expect("home spline")
        .duration_ms();
    assert!(duration_ms > 0, "the home spline must have a duration");
    creature
        .backdate_runtime_clock_for_test(Duration::from_millis(u64::from(duration_ms as u32) + 50));
    assert!(creature.update_move_spline_like_cpp());

    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(
        !creature.creature.is_in_evade_mode_like_cpp(),
        "DoFinalize clears UNIT_STATE_EVADE on arrival"
    );
    assert_eq!(
        creature.state(),
        CreatureAiState::Idle,
        "the creature is home and idle again"
    );
    assert_eq!(
        creature.creature.unit().data().health,
        creature.creature.unit().data().max_health,
        "C++ SetSpawnHealth restores health on home finalization"
    );
    assert!(
        creature.take_home_health_restored_pending_like_cpp(),
        "the global tick must publish the health restored by home finalization"
    );
    assert!(
        !creature.take_home_health_restored_pending_like_cpp(),
        "the values update publication marker is consumed once"
    );
    assert!(
        creature.creature.take_ai_just_reached_home(),
        "C++ AI()->JustReachedHome() must fire on a natural home arrival"
    );
    assert!(
        !creature.creature.take_ai_just_reached_home(),
        "the callback is consumed once"
    );
}

/// C++ sets `UNIT_STATE_EVADE` before `MoveTargetedHome()` constructs the
/// path (`CreatureAI.cpp:237`), so `UpdateFilter` includes `NAV_GROUND_STEEP`
/// for the return route (`PathGenerator.cpp:694-696`). The bridge therefore
/// has to build the query *after* it enters evade, not let the caller sample
/// the filter one step early.
#[test]
fn world_creature_home_query_filter_is_sampled_after_entering_evade_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54357);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(40.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::Run as u8,
    );
    creature.creature.set_swim_allowed_runtime_like_cpp(false);
    creature
        .creature
        .set_ai_home_position(Position::new(10.0, 10.0, 0.0, 0.0));

    // The creature is neither in combat nor evading when the return starts,
    // which is exactly the state `reset_combat` leaves behind.
    assert!(!creature.creature.is_in_combat());
    assert!(!creature.creature.is_in_evade_mode_like_cpp());

    let mut observed = None;
    let _ = creature.update_runtime_home_movement_like_cpp(true, None, |query| {
        observed = Some(query.filter_context);
        None
    });

    let filter = create_path_query_filter_like_cpp(observed.expect("home query")).expect("filter");
    assert_eq!(
        filter.include_flags(),
        (wow_recastdetour::NavTerrainFlag::GROUND | wow_recastdetour::NavTerrainFlag::GROUND_STEEP)
            .bits(),
        "the home route must be queried with the evade state already applied, \
         or it excludes steep polygons C++ allows"
    );
}

/// A chase that just switched victim must query with a fresh corridor. C++
/// installs a new `ChaseMovementGenerator` owning a new `PathGenerator`, so
/// there is nothing to reuse; feeding the previous victim's `_pathPolyRefs`
/// would let `BuildPolyPath`'s ~80% prefix branch steer the first spline back
/// toward the old target (`PathGenerator.cpp:339-413`).
#[test]
fn world_creature_chase_first_query_after_victim_switch_has_no_corridor_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54358);
    let first = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 95);
    let second = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 96);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );

    let target = |victim, x| ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(x, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    };
    let corridor = |points: Vec<[f32; 3]>, refs: Vec<u64>| DetourPolyPath {
        poly_refs: refs,
        point_path: wow_recastdetour::DetourPointPath {
            actual_end: *points.last().expect("points"),
            points,
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    // First victim: the query produces a corridor the generator retains.
    creature.enter_combat(first);
    let mut first_query = None;
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        target(first, 60.0),
        true,
        None,
        |query| {
            first_query = Some(query.previous_poly_refs.clone());
            Some(corridor(
                vec![[10.0, 10.0, 0.0], [35.0, 10.0, 0.0], [59.0, 10.0, 0.0]],
                vec![101, 102, 103],
            ))
        },
    );
    assert_eq!(
        first_query.as_deref(),
        Some(&[][..]),
        "a brand new generator starts with no corridor"
    );
    assert_eq!(
        creature.active_chase_path_poly_refs_like_cpp(),
        &[101, 102, 103],
        "the corridor must be retained for the same victim"
    );

    // Switching victim must reset it before the next query is built.
    creature.enter_combat(second);
    let mut second_query = None;
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        target(second, 65.0),
        true,
        None,
        |query| {
            second_query = Some(query.previous_poly_refs.clone());
            None
        },
    );
    assert_eq!(
        second_query.as_deref(),
        Some(&[][..]),
        "the first query for a new victim must not carry the previous corridor"
    );
}

/// When the victim leaves the world, C++ chase `Update` returns false and
/// `MotionMaster` finalizes the generator (`ChaseMovementGenerator.cpp:101-103`,
/// `:251-260`). The runtime must drop the chase generator and clear
/// `UNIT_STATE_CHASE_MOVE`, not merely clear the corridor, or the creature is
/// re-selected as chasing and keeps driving toward the corpse every tick.
#[test]
fn world_creature_chase_finalizes_when_the_victim_leaves_the_world_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54359);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 97);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.enter_combat(victim);

    // A live victim far enough to trigger a launch installs the generator and
    // marks the creature chase-moving.
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        ChaseTargetSnapshotLikeCpp {
            guid: victim,
            position: Position::new(60.0, 10.0, 0.0, 0.0),
            combat_reach: 1.0,
            in_world: true,
            in_water: Some(false),
        },
        false,
        None,
        |_| None,
    );
    assert!(creature.active_chase_generator_like_cpp().is_some());
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(wow_constants::UnitState::CHASE_MOVE.bits())
    );

    // The victim leaves the world: the snapshot reports `in_world: false`.
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        ChaseTargetSnapshotLikeCpp {
            guid: victim,
            position: Position::new(60.0, 10.0, 0.0, 0.0),
            combat_reach: 1.0,
            in_world: false,
            in_water: Some(false),
        },
        false,
        None,
        |_| None,
    );
    assert!(
        creature.active_chase_generator_like_cpp().is_none(),
        "the chase generator must be retired when the victim leaves the world"
    );
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(wow_constants::UnitState::CHASE_MOVE.bits()),
        "UNIT_STATE_CHASE_MOVE must be cleared on finalize"
    );
    assert!(creature.active_chase_path_poly_refs_like_cpp().is_empty());
}

/// C++ `HomeMovementGenerator::SetTargetLocation` sets
/// `MOVEMENTGENERATOR_FLAG_INTERRUPTED` and returns without launching while
/// ROOT/STUNNED/DISTRACTED (`HomeMovementGenerator.cpp:53-58`); only the next
/// `DoUpdate` sets `INFORM_ENABLED` and finalizes (`:117-122`). Finalizing in
/// the initialize frame would skip `INFORM_ENABLED`, suppressing
/// `JustReachedHome` and clearing evade a frame early.
#[test]
fn world_creature_home_interrupted_survives_one_frame_before_finalize_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54360);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(40.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature
        .creature
        .set_ai_home_position(Position::new(10.0, 10.0, 0.0, 0.0));
    // The creature is returning (what `reset_combat` leaves behind), but
    // rooted, so C++ `SetTargetLocation` interrupts instead of launching.
    creature.creature.set_ai_state(CreatureAiState::Returning);
    creature
        .creature
        .unit_mut()
        .add_unit_state(wow_constants::UnitState::ROOT.bits());

    // Initialize frame: interrupted, but the generator must stay installed
    // and evade must be held, and the reached-home callback must NOT fire.
    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(
        creature.creature.is_in_evade_mode_like_cpp(),
        "evade must still be held while interrupted"
    );
    assert!(
        !creature.creature.take_ai_just_reached_home(),
        "JustReachedHome must not fire in the interrupt frame"
    );
    assert_eq!(
        creature.state(),
        CreatureAiState::Returning,
        "the interrupted home generator stays selected for one more tick"
    );

    // Next frame: the update path sees INTERRUPTED, sets INFORM_ENABLED and
    // finalizes, clearing evade and firing JustReachedHome.
    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(
        !creature.creature.is_in_evade_mode_like_cpp(),
        "DoFinalize clears UNIT_STATE_EVADE on the update frame"
    );
    assert!(
        creature.creature.take_ai_just_reached_home(),
        "JustReachedHome fires on the finalize frame, even for an interrupted return"
    );
    assert_eq!(creature.state(), CreatureAiState::Idle);
}

#[test]
fn detour_path_without_navmesh_matches_cpp_calculate_path_early_return() {
    let start = Position::new(10.0, 10.0, 3.0, 0.0);
    let destination = Position::new(25.0, 18.0, 4.0, 1.0);

    let path = detour_path_without_navmesh_like_cpp(start, destination);

    // C++ `BuildShortcut()` is exactly "start -> actual end", and
    // `CalculatePath` types it `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`
    // (`PathGenerator.cpp:83-85`).
    assert_eq!(
        path.point_path.path_type,
        DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH
    );
    assert_eq!(
        path.point_path.points,
        vec![[10.0, 10.0, 3.0], [25.0, 18.0, 4.0]]
    );
    assert_eq!(path.point_path.actual_end, [25.0, 18.0, 4.0]);
    assert!(path.poly_refs.is_empty());
    assert!(!path.start_far_from_poly);
    assert!(!path.end_far_from_poly);

    // The type must be usable, otherwise the random generator would refuse
    // to launch it the way it refuses NOPATH/SHORTCUT.
    let path_type = path_type_from_detour_like_cpp(path.point_path.path_type);
    assert_eq!(
        random_path_result_from_path_type_like_cpp(path_type),
        RandomPathResult::Success
    );
    assert!(!path_type.intersects(PathType::NOPATH | PathType::SHORTCUT));
}

#[test]
fn world_creature_random_launches_cpp_shortcut_when_navmesh_is_absent() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54341);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.seed_runtime_rng_like_cpp(0x5434_1);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 8.0;
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);

    // C++ `CalculatePath` answers a missing navmesh/tile with
    // `BuildShortcut()` + `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`
    // (`PathGenerator.cpp:79-86`). Neither bit is checked by
    // `RandomMovementGenerator::SetRandomLocation`
    // (`RandomMovementGenerator.cpp:146-153`), so the creature launches the
    // two-point path instead of retrying forever.
    let mut resolver_called = false;
    let movement =
        creature.update_default_random_movement_with_path_resolver_like_cpp(10, true, |query| {
            resolver_called = true;
            Some(detour_path_without_navmesh_like_cpp(
                query.start,
                query.destination,
            ))
        });

    assert!(resolver_called);
    let (from, spline) =
        movement.expect("C++ launches the no-navmesh shortcut instead of standing still");
    assert!(creature.active_move_spline_like_cpp().is_some());
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::WalkingRandom
    );

    // The launched spline is the C++ `BuildShortcut()` segment: straight
    // from the creature's position to the rolled wander destination, with
    // no navmesh waypoints in between.
    let destination = spline
        .final_destination()
        .expect("the launched shortcut has a destination");
    let shortcut = detour_path_without_navmesh_like_cpp(from, destination);
    let path = path_generator_from_detour_like_cpp(from, destination, &shortcut, false);
    assert_eq!(
        path.path_points(),
        &[from, destination],
        "the C++ no-navmesh case is a two-point shortcut"
    );
    assert_eq!(
        path.path_type(),
        PathType::NORMAL | PathType::NOT_USING_PATH
    );
}

#[test]
fn world_creature_path_query_filter_context_follows_cpp_create_filter() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54342);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );

    // C++ `CreatureMovementData` defaults to `Ground = Run` and
    // `Swim = true` (`Creature.cpp:58`), so `CanWalk()` and
    // `CanEnterWater()` both hold and `CreateFilter` includes
    // NAV_GROUND | NAV_WATER | NAV_MAGMA_SLIME.
    let context = creature.path_query_filter_context_like_cpp();
    assert_eq!(
        context.owner,
        wow_recastdetour::PathQueryFilterOwner::Creature {
            can_walk: true,
            can_enter_water: true,
            in_combat: false,
            in_evade_mode: false,
        }
    );
    let filter = create_path_query_filter_like_cpp(context).expect("filter");
    assert_eq!(
        filter.include_flags(),
        (wow_recastdetour::NavTerrainFlag::GROUND
            | wow_recastdetour::NavTerrainFlag::WATER
            | wow_recastdetour::NavTerrainFlag::MAGMA_SLIME)
            .bits()
    );

    // A ground-only, non-swimming template must lose the water bits, which
    // the previously hardcoded context could never express.
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::None as u8,
    );
    creature.creature.set_swim_allowed_runtime_like_cpp(false);
    let context = creature.path_query_filter_context_like_cpp();
    assert_eq!(
        context.owner,
        wow_recastdetour::PathQueryFilterOwner::Creature {
            can_walk: false,
            can_enter_water: false,
            in_combat: false,
            in_evade_mode: false,
        }
    );

    // C++ `UpdateFilter` adds NAV_GROUND_STEEP while the creature
    // `IsInCombat()` or `IsInEvadeMode()` (`PathGenerator.cpp:694-696`).
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::Run as u8,
    );
    creature.creature.set_in_evade_mode_like_cpp(true);
    let context = creature.path_query_filter_context_like_cpp();
    assert!(matches!(
        context.owner,
        wow_recastdetour::PathQueryFilterOwner::Creature {
            in_evade_mode: true,
            ..
        }
    ));
    let filter = create_path_query_filter_like_cpp(context).expect("filter");
    assert_eq!(
        filter.include_flags(),
        (wow_recastdetour::NavTerrainFlag::GROUND | wow_recastdetour::NavTerrainFlag::GROUND_STEEP)
            .bits()
    );
}

#[test]
fn calculate_creature_detour_path_returns_none_until_runtime_mmap_exists_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54325);
    let creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let dst = Position::new(20.0, 10.0, 0.0, 0.0);
    let filter_context = PathQueryFilterContext::creature(true, false, false, false);

    assert_eq!(
        calculate_creature_detour_path_like_cpp(&creature, dst, None, 0, 0, filter_context, false),
        Ok(None)
    );

    let mmap_data = MMapData::new(wow_recastdetour::DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 533.3333,
        tile_height: 533.3333,
        max_tiles: 16,
        max_polys: 16,
    })
    .expect("navmesh allocation");
    assert_eq!(
        calculate_creature_detour_path_like_cpp(
            &creature,
            dst,
            Some(&mmap_data),
            0,
            0,
            filter_context,
            false,
        ),
        Ok(None)
    );
}

/// C++ `PathGenerator::CalculatePath` needs `HaveTile(start)` **and**
/// `HaveTile(dest)` (`PathGenerator.cpp:79-86`); it gets both because
/// `TerrainInfo::LoadMMap` pushes each grid's `.mmtile` in as the grid
/// loads. RustyCore loads tiles on demand from the path request, so the
/// pathfinder has to demand-load the destination's tile too — otherwise a
/// destination one tile over reports "no navmesh" and the caller degrades to
/// a straight line even though the mesh is on disk.
#[test]
fn world_mmap_pathfinder_demand_loads_the_destination_tile_like_cpp() {
    use wow_recastdetour::test_fixtures::{
        OBSTACLE_TILE_CELL_SIZE, write_obstacle_ring_mmaps_like_cpp,
    };

    const MAP_ID: u32 = 1;
    let half = OBSTACLE_TILE_CELL_SIZE / 2.0;
    // WoW y drives the Detour x axis, so stepping y across
    // `SIZE_OF_GRIDS_LIKE_CPP` moves both the navmesh tile and the grid file
    // the tile is named after.
    let start = Position::new(half, half, 0.0, 0.0);
    let destination = Position::new(half, SIZE_OF_GRIDS_LIKE_CPP + half, 0.0, 0.0);
    assert_ne!(
        wow_recastdetour::mmap_tile_coords_for_wow_position_like_cpp(start.x, start.y),
        wow_recastdetour::mmap_tile_coords_for_wow_position_like_cpp(destination.x, destination.y),
        "the fixture must straddle a grid-file seam to be meaningful"
    );

    let root = unique_test_dir("world-mmap-pathfinder-destination-tile");
    let _ = std::fs::remove_dir_all(&root);
    write_obstacle_ring_mmaps_like_cpp(
        &root,
        MAP_ID,
        &[(start.x, start.y), (destination.x, destination.y)],
    );

    let mut pathfinder = WorldMMapPathfinderLikeCpp::new(&root);
    let result = pathfinder.calculate_path_from_positions_like_cpp(
        start,
        destination,
        MAP_ID,
        MAP_ID,
        0,
        PathQueryFilterContext::creature(true, false, false, false),
        DetourOwnerCapabilitiesLikeCpp::default(),
        &[],
        false,
        MAX_POINT_PATH_LENGTH_LIKE_CPP,
    );

    // The query must actually run: reporting `Ok(None)` here would mean the
    // destination tile was never loaded and the caller silently
    // straight-lines.
    let path = result
        .expect("query must not error")
        .expect("both endpoint tiles are on disk, so the query must run");

    // The two fixture tiles are disconnected islands, so `findPath` returns
    // a partial corridor that never reaches `endPoly`. That is the C++
    // `PATHFIND_INCOMPLETE` tail in `BuildPolyPath`
    // (`PathGenerator.cpp:519-522`) — a real navmesh answer, not a shortcut.
    assert!(
        path.point_path
            .path_type
            .contains(DetourPathType::INCOMPLETE),
        "expected a partial navmesh corridor, got {:?}",
        path.point_path.path_type
    );
    assert!(
        !path
            .point_path
            .path_type
            .intersects(DetourPathType::NOT_USING_PATH),
        "the mesh was queried, so this must not be the no-navmesh shortcut: {:?}",
        path.point_path.path_type
    );
    assert_eq!(
        pathfinder.mmap_manager().get_loaded_tiles_count(),
        2,
        "the start tile and the destination tile must both be resident"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn world_mmap_pathfinder_falls_back_when_runtime_tile_missing_like_cpp() {
    let root = unique_test_dir("world-mmap-pathfinder-missing-tile");
    std::fs::create_dir_all(root.join("mmaps")).unwrap();
    let params = wow_recastdetour::DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 533.3333,
        tile_height: 533.3333,
        max_tiles: 4096,
        max_polys: 16_384,
    };
    std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54326);
    let creature = WorldCreature::new(
        guid,
        1,
        Position::new(0.0, 0.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let mut pathfinder = WorldMMapPathfinderLikeCpp::new(&root);
    let filter_context = PathQueryFilterContext::creature(true, false, false, false);

    assert_eq!(
        pathfinder.calculate_creature_path_like_cpp(
            &creature,
            Position::new(20.0, 0.0, 0.0, 0.0),
            1,
            1,
            42,
            filter_context,
            false,
        ),
        Ok(None)
    );
    assert!(
        pathfinder
            .mmap_manager()
            .get_nav_mesh_query(1, 1, 42)
            .is_some()
    );
    assert_eq!(pathfinder.mmap_manager().get_loaded_tiles_count(), 0);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn world_mmap_pathfinder_worker_keeps_detour_off_session_thread_like_cpp() {
    let root = unique_test_dir("world-mmap-pathfinder-worker-missing-tile");
    std::fs::create_dir_all(root.join("mmaps")).unwrap();
    let params = wow_recastdetour::DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 533.3333,
        tile_height: 533.3333,
        max_tiles: 4096,
        max_polys: 16_384,
    };
    std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn(&root);
    let result = worker.calculate_path_like_cpp(WorldMMapPathRequestLikeCpp {
        start: Position::new(0.0, 0.0, 0.0, 0.0),
        destination: Position::new(20.0, 0.0, 0.0, 0.0),
        mesh_map_id: 1,
        instance_map_id: 1,
        instance_id: 42,
        filter_context: PathQueryFilterContext::creature(true, false, false, false),
        owner: DetourOwnerCapabilitiesLikeCpp::default(),
        previous_poly_refs: Vec::new(),
        force_destination: false,
        point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
        phase_shift: PhaseShift::default(),
    });

    assert_eq!(result, Ok(None));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn world_mmap_pathfinder_initializes_thread_unsafe_parent_map_data_like_cpp() {
    let root = unique_test_dir("world-mmap-pathfinder-parent-map-data");
    let pathfinder = WorldMMapPathfinderLikeCpp::new_with_parent_map_data_like_cpp(
        &root,
        [(571, vec![609]), (609, Vec::new())],
    );

    assert!(!pathfinder.mmap_manager().is_thread_safe_environment());
    assert_eq!(pathfinder.mmap_manager().get_loaded_maps_count(), 2);
    assert_eq!(pathfinder.mmap_manager().parent_map_id(609), Some(571));
}

#[test]
fn world_creature_begin_point_movement_uses_point_lifecycle_and_real_spline() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54323);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let dst = Position::new(14.0, 10.0, 0.0, 0.0);

    let (from, spline) = creature
        .begin_point_movement_like_cpp(42, dst, true)
        .expect("point movement starts direct spline");

    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert!(creature.active_move_spline.is_some());
    assert_eq!(creature.move_target(), Some(dst));
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion = &creature.creature.unit().subsystems().motion;
    let generator = motion.current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Point);
    assert_eq!(generator.movement_id, 42);
    assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZED));
    assert!(!generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert!(motion.spline.enabled);
    assert_eq!(motion.spline.spline_id, spline.id());
    assert_eq!(motion.spline.final_destination, Some((14, 10, 0)));

    {
        let motion = &mut creature.creature.unit_mut().subsystems_mut().motion;
        let generator = motion
            .active_generators
            .iter_mut()
            .find(|generator| generator.kind == MovementGeneratorKind::Point)
            .expect("point generator");
        assert_eq!(
            generator.update_point_like_cpp(true, true),
            PointMovementAction::Finished
        );
    }
    assert_eq!(
        creature.finalize_point_movement_like_cpp(true, true),
        Some(PointMovementInform {
            kind: MovementGeneratorKind::Point,
            movement_id: 42,
        })
    );
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Point.trinity_id(),
            movement_id: 42,
        })
    );
}

#[test]
fn world_creature_begin_point_movement_handles_blocked_and_prepath_branches() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54324);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let dst = Position::new(14.0, 10.0, 0.0, 0.0);

    assert!(
        creature
            .begin_point_movement_like_cpp(43, dst, false)
            .is_none()
    );
    assert!(creature.active_move_spline.is_none());
    let generator = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INTERRUPTED));
    assert!(creature.creature.unit().subsystems().motion.stopped);

    assert!(
        creature
            .begin_point_movement_like_cpp(EVENT_CHARGE_PREPATH, dst, true)
            .is_none()
    );
    assert!(creature.active_move_spline.is_none());
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let generator = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Point);
    assert_eq!(generator.movement_id, EVENT_CHARGE_PREPATH);
    assert_eq!(generator.base_unit_state, UnitState::CHARGING.bits());
}

#[test]
fn world_creature_finalize_generic_movement_records_ai_inform_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54326);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let target = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54327);
    {
        let motion = &mut creature.creature.unit_mut().subsystems_mut().motion;
        motion.launch_generic_movement(
            MovementGeneratorKind::Effect,
            77,
            1_000,
            Some((1234, target)),
        );
        let generator = motion
            .active_generators
            .iter_mut()
            .find(|generator| generator.kind == MovementGeneratorKind::Effect)
            .expect("generic effect generator");
        generator.initialize_generic_like_cpp();
        assert!(!generator.update_generic_like_cpp(1_000, false, false));
    }

    assert_eq!(
        creature.finalize_generic_movement_like_cpp(MovementGeneratorKind::Effect, 77, true),
        Some(GenericMovementInform {
            kind: MovementGeneratorKind::Effect,
            movement_id: 77,
            arrival_spell_id: Some(1234),
            arrival_spell_target_guid: Some(target),
        })
    );
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Effect.trinity_id(),
            movement_id: 77,
        })
    );
}

#[test]
fn world_creature_begin_distract_and_rotate_launch_facing_splines_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54325);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    creature
        .creature
        .unit_mut()
        .set_stand_state_like_cpp(UnitStandStateType::Sit);

    let (action, from, spline) = creature
        .begin_distract_movement_like_cpp(500, 1.25)
        .expect("distract launches facing spline");

    assert_eq!(
        action,
        DistractMovementAction {
            stand_up: true,
            launch_facing_spline: true,
        }
    );
    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert_eq!(
        creature.creature.unit().stand_state_like_cpp(),
        UnitStandStateType::Stand
    );
    assert_eq!(
        spline.facing().kind,
        wow_movement::MonsterMoveType::FacingAngle
    );
    assert!((spline.facing().angle - 1.25).abs() < 0.0001);
    assert!(spline.spline_is_facing_only);
    assert_eq!(creature.spline_id(), spline.id());
    let generator = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Distract);
    assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZED));
    creature
        .creature
        .set_ai_home_position(Position::new(10.0, 10.0, 0.0, 2.5));
    {
        let motion = &mut creature.creature.unit_mut().subsystems_mut().motion;
        let generator = motion
            .active_generators
            .iter_mut()
            .find(|generator| generator.kind == MovementGeneratorKind::Distract)
            .expect("distract generator");
        assert!(!generator.update_distract_like_cpp(true, 501));
    }
    assert!(creature.finalize_distract_movement_like_cpp(true));
    assert!((creature.position().orientation - 2.5).abs() < 0.0001);

    creature
        .creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .clear_active();
    assert!(
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .move_rotate_like_cpp(8, 1_000, wow_entities::RotateDirection::Left)
    );
    let (update, spline) = creature
        .tick_rotate_movement_like_cpp(250)
        .expect("rotate tick launches facing spline");
    assert!(update.keep_running);
    let expected_rotate_angle = 2.5 + std::f32::consts::FRAC_PI_2;
    assert!(
        update
            .facing_angle
            .is_some_and(|angle| (angle - expected_rotate_angle).abs() < 0.0001)
    );
    assert_eq!(
        spline.facing().kind,
        wow_movement::MonsterMoveType::FacingAngle
    );
    assert!(
        (spline.facing().angle - expected_rotate_angle).abs() < 0.0001,
        "facing angle was {}",
        spline.facing().angle
    );
    assert!(spline.spline_is_facing_only);
    let generator = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(generator.kind, MovementGeneratorKind::Rotate);
    assert_eq!(generator.duration_ms, Some(750));
    assert_eq!(
        creature.finalize_rotate_movement_like_cpp(true),
        Some(PointMovementInform {
            kind: MovementGeneratorKind::Rotate,
            movement_id: 8,
        })
    );
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Rotate.trinity_id(),
            movement_id: 8,
        })
    );
}

#[test]
fn world_creature_stop_move_spline_emits_cpp_stop_state_before_arrival() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54322);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.clock_started_at = Instant::now() - Duration::from_secs(10);
    let dst = Position::new(20.0, 10.0, 0.0, 0.0);
    let (_, spline) = creature
        .begin_move_spline_like_cpp(dst)
        .expect("valid two-point spline");
    assert!(
        creature
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD)
    );
    let duration_ms = spline.duration_ms() as u32;
    let now_ms = creature.now_ms();
    creature.creature.ai_ownership_mut().move_start_ms =
        now_ms.saturating_sub(u64::from(duration_ms / 2));

    let stop = creature
        .stop_move_spline_like_cpp()
        .expect("active spline stops");

    assert_eq!(stop.spline_id, 3);
    assert_eq!(stop.stop_distance_tolerance, 2);
    assert!(stop.position.x > 10.0 && stop.position.x < 20.0);
    assert_eq!(creature.position(), stop.position);
    assert!(creature.active_move_spline.is_none());
    assert_eq!(creature.move_target(), None);
    assert!(
        !creature
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD),
        "C++ MoveSplineInit::Stop removes MOVEMENTFLAG_FORWARD"
    );
    assert!(
        !MovementFlag::from_bits_retain(creature.create_data.movement_flags)
            .contains(MovementFlag::FORWARD)
    );
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion_spline = &creature.creature.unit().subsystems().motion.spline;
    assert!(!motion_spline.enabled);
    assert!(motion_spline.finalized);
    assert_eq!(motion_spline.spline_id, stop.spline_id);
    assert!(creature.stop_move_spline_like_cpp().is_none());
}

#[test]
fn test_visible_creatures() {
    let mut manager = MapManager::new();
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let creature = WorldCreature::new(
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
    );

    manager.add_creature(0, 0, 0, 0, creature);

    // Should find creature at (10, 10)
    let visible = manager.get_visible_creatures(0, 0, 10.0, 10.0, 0.0);
    assert!(!visible.is_empty());
    assert_eq!(visible[0].guid(), guid);

    // Should not find creature far away
    let visible = manager.get_visible_creatures(0, 0, 1000.0, 1000.0, 0.0);
    assert!(visible.is_empty());
}

#[test]
fn visible_creatures_in_phase_filters_like_cpp_grid_searchers() {
    let mut manager = MapManager::new();
    let visible_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 100);
    let hidden_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 101);

    let mut seer_phase = PhaseShift::default();
    seer_phase.add_phase_like_cpp(20, wow_constants::PhaseFlags::empty(), 1);

    let mut visible_creature = WorldCreature::new(
        visible_guid,
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
    );
    visible_creature
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(20, wow_constants::PhaseFlags::empty(), 1);

    let mut hidden_creature = WorldCreature::new(
        hidden_guid,
        1,
        Position::new(11.0, 10.0, 0.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    );
    hidden_creature
        .creature
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .add_phase_like_cpp(30, wow_constants::PhaseFlags::empty(), 1);

    manager.add_creature(0, 0, 0, 0, visible_creature);
    manager.add_creature(0, 0, 0, 0, hidden_creature);

    let visible = manager.get_visible_creatures_in_phase(
        0,
        0,
        10.0,
        10.0,
        0.0,
        VISIBILITY_RADIUS,
        Some(&seer_phase),
    );
    let visible_guids: HashSet<ObjectGuid> = visible.iter().map(WorldCreature::guid).collect();
    assert!(visible_guids.contains(&visible_guid));
    assert!(!visible_guids.contains(&hidden_guid));

    let unfiltered = manager.get_visible_creatures(0, 0, 10.0, 10.0, 0.0);
    let unfiltered_guids: HashSet<ObjectGuid> =
        unfiltered.iter().map(WorldCreature::guid).collect();
    assert!(unfiltered_guids.contains(&visible_guid));
    assert!(unfiltered_guids.contains(&hidden_guid));
}

#[test]
fn get_visible_creatures_uses_cpp_2d_sight_range() {
    let mut manager = MapManager::new();
    manager.get_or_create_map(1, 0);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 72);
    let creature = WorldCreature::new(
        guid,
        1,
        Position::new(80.0, 0.0, 80.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    );
    manager.add_creature(1, 0, 0, 0, creature);

    let visible = manager.get_visible_creatures_in_phase(1, 0, 0.0, 0.0, 0.0, 100.0, None);

    assert_eq!(
        visible.iter().map(WorldCreature::guid).collect::<Vec<_>>(),
        vec![guid],
        "C++ visibility uses horizontal distance; a vertically separated creature inside sight range must still be sent"
    );
}

#[test]
fn world_creature_create_bridge_preserves_npc_flags2_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 102);
    let mut creature = WorldCreature::new(
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
        0x40,
        0,
    );
    creature
        .creature
        .set_npc_flags2_runtime_like_cpp(0x0000_0001);

    let bridged = WorldCreature::from_canonical(creature.creature, creature.create_data);

    assert_eq!(bridged.npc_flags(), 0x40);
    assert_eq!(bridged.npc_flags2(), 0x1);
    assert_eq!(bridged.npc_flags_mask_like_cpp(), 0x1_0000_0040);
    assert_eq!(bridged.create_data.npc_flags, 0x1_0000_0040);
}

#[test]
fn loaded_grid_canonical_bridge_preserves_level_and_stats_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 29_715, 97_932);
    let position = Position::new(5875.25, 609.063, 650.368, 1.676);
    let template = wow_entities::CreatureTemplateLifecycleRecord {
        entry: 29_715,
        original_entry: 29_715,
        difficulty_id: 0,
        name: "Quartermaster".to_string(),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: 2,
        unit_class: 1,
        trainer_class: 0,
        faction: 35,
        npc_flags: 0x280,
        display_id: 26_441,
        model_dimensions: Some(wow_entities::CreatureModelDimensions {
            bounding_radius: 0.389,
            combat_reach: 1.5,
        }),
        scale: 1.0,
        speed_walk: 1.0,
        speed_run: 1.14286,
        spells: [0; wow_entities::MAX_CREATURE_SPELLS],
        classification: 0,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        unit_flags: 0,
        unit_flags2: wow_constants::UnitFlags2::REGENERATE_POWER.bits(),
        unit_flags3: 0,
        flags_extra: 0,
        static_flags: [0; 8],
        creature_type: 7,
        type_flags: 0,
        loot_id: 21_779,
        skin_loot_id: 21_780,
        gold_min: 13,
        gold_max: 31,
        movement_type: wow_entities::MovementGeneratorType::Idle,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: wow_constants::CreatureFlightMovementType::None as u8,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        min_level: 75,
        max_level: 75,
        equipment_id: 0,
        original_equipment_id: 0,
    };
    let spawn = wow_entities::CreatureSpawnLifecycleRecord {
        spawn_id: 97_932,
        map_id: 571,
        instance_id: 0,
        position,
        home_position: position,
        phase_id: None,
        phase_group: None,
        terrain_swap_map: None,
        spawn_group_id: None,
        spawn_group_name: None,
        pool_id: None,
        equipment_id: Some(0),
        original_equipment_id: Some(0),
        wander_distance: 0.0,
        respawn_delay: 120,
        respawn_time: 0,
        movement_type: wow_entities::MovementGeneratorType::Idle,
        string_id: None,
        is_active: true,
        inactive_by_spawn_group: false,
        duplicate_spawn_found: false,
        add_to_map: true,
        respawn_compatibility_mode: false,
    };
    let canonical = wow_entities::Creature::load_from_db_lifecycle(
        wow_entities::CreatureLoadFromDbLifecycleRecord {
            create: wow_entities::CreatureCreateLifecycleRecord {
                guid,
                entry: 29_715,
                map_id: 571,
                instance_id: 0,
                position,
                dynamic: false,
                vehicle_id: None,
                vehicle_kit_create_input: None,
                add_to_world_vehicle_reset_context: None,
                template,
                spawn: Some(spawn.clone()),
                selected_level: 75,
                stats: wow_entities::CreatureLifecycleStats::new(4_652, 4_652, 0, 0),
                selected_display_id: 26_441,
                selected_model_dimensions: Some(wow_entities::CreatureModelDimensions {
                    bounding_radius: 0.389,
                    combat_reach: 1.5,
                }),
                selected_equipment_id: 0,
                selected_original_equipment_id: 0,
                selected_virtual_items: [(0, 0, 0); 3],
                corpse_delay: 60,
                ignore_corpse_decay_ratio: false,
                addon: None,
            },
            spawn,
        },
    );

    let bridged = WorldCreature::from_loaded_grid_canonical_like_cpp(canonical, |_| None);

    assert_eq!(bridged.level(), 75);
    assert_eq!(bridged.current_hp(), 4_652);
    assert_eq!(bridged.max_hp(), 4_652);
    assert_eq!(bridged.create_data.level, 75);
    assert_eq!(bridged.create_data.health, 4_652);
    assert_eq!(bridged.create_data.max_health, 4_652);
    assert_eq!(bridged.create_data.display_id, 26_441);
    assert_eq!(bridged.create_data.npc_flags, 0x280);
    assert_eq!(
        bridged.create_data.unit_flags2,
        wow_constants::UnitFlags2::REGENERATE_POWER.bits()
    );
    assert_eq!(bridged.create_data.speed_walk_rate, 1.0);
    assert_eq!(bridged.create_data.speed_run_rate, 1.14286);
    assert_eq!(bridged.creature.ai_ownership().loot_id, 21_779);
    assert_eq!(bridged.creature.ai_ownership().skin_loot_id, 21_780);
    assert_eq!(bridged.creature.ai_ownership().gold_min, 13);
    assert_eq!(bridged.creature.ai_ownership().gold_max, 31);
}

#[test]
fn loaded_grid_canonical_bridge_only_sets_vehicle_create_flag_for_real_vehicle_kit_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Vehicle, 0, 1, 571, 0, 29_715, 97_933);
    let position = Position::new(5875.25, 609.063, 650.368, 1.676);
    let template = wow_entities::CreatureTemplateLifecycleRecord {
        entry: 29_715,
        original_entry: 29_715,
        difficulty_id: 0,
        name: "Vehicle-shaped creature".to_string(),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: 2,
        unit_class: 1,
        trainer_class: 0,
        faction: 35,
        npc_flags: 0,
        display_id: 26_441,
        model_dimensions: Some(wow_entities::CreatureModelDimensions {
            bounding_radius: 0.389,
            combat_reach: 1.5,
        }),
        scale: 1.0,
        speed_walk: 1.0,
        speed_run: 1.14286,
        spells: [0; wow_entities::MAX_CREATURE_SPELLS],
        classification: 0,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        flags_extra: 0,
        static_flags: [0; 8],
        creature_type: 7,
        type_flags: 0,
        loot_id: 0,
        skin_loot_id: 0,
        gold_min: 0,
        gold_max: 0,
        movement_type: wow_entities::MovementGeneratorType::Idle,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: wow_constants::CreatureFlightMovementType::None as u8,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        min_level: 75,
        max_level: 75,
        equipment_id: 0,
        original_equipment_id: 0,
    };
    let spawn = wow_entities::CreatureSpawnLifecycleRecord {
        spawn_id: 97_933,
        map_id: 571,
        instance_id: 0,
        position,
        home_position: position,
        phase_id: None,
        phase_group: None,
        terrain_swap_map: None,
        spawn_group_id: None,
        spawn_group_name: None,
        pool_id: None,
        equipment_id: Some(0),
        original_equipment_id: Some(0),
        wander_distance: 0.0,
        respawn_delay: 120,
        respawn_time: 0,
        movement_type: wow_entities::MovementGeneratorType::Idle,
        string_id: None,
        is_active: true,
        inactive_by_spawn_group: false,
        duplicate_spawn_found: false,
        add_to_map: true,
        respawn_compatibility_mode: false,
    };
    let canonical = wow_entities::Creature::load_from_db_lifecycle(
        wow_entities::CreatureLoadFromDbLifecycleRecord {
            create: wow_entities::CreatureCreateLifecycleRecord {
                guid,
                entry: 29_715,
                map_id: 571,
                instance_id: 0,
                position,
                dynamic: false,
                vehicle_id: Some(909),
                vehicle_kit_create_input: None,
                add_to_world_vehicle_reset_context: None,
                template,
                spawn: Some(spawn.clone()),
                selected_level: 75,
                stats: wow_entities::CreatureLifecycleStats::new(4_652, 4_652, 0, 0),
                selected_display_id: 26_441,
                selected_model_dimensions: Some(wow_entities::CreatureModelDimensions {
                    bounding_radius: 0.389,
                    combat_reach: 1.5,
                }),
                selected_equipment_id: 0,
                selected_original_equipment_id: 0,
                selected_virtual_items: [(0, 0, 0); 3],
                corpse_delay: 60,
                ignore_corpse_decay_ratio: false,
                addon: None,
            },
            spawn,
        },
    );

    assert_eq!(canonical.lifecycle_metadata().vehicle_id, Some(909));
    assert!(canonical.unit().subsystems().vehicle.kit.is_none());

    let bridged = WorldCreature::from_loaded_grid_canonical_like_cpp(canonical, |_| None);

    assert_eq!(bridged.create_data.vehicle_id, 0);
}

#[test]
fn set_creature_anim_kit_id_like_cpp_mutates_state_create_data_and_returns_fanout() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 103);
    let mut manager = MapManager::new();
    manager.add_creature(
        571,
        0,
        0,
        0,
        WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 20.0, 30.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        ),
    );

    let event = manager
        .set_creature_anim_kit_id_like_cpp(571, 0, guid, CreatureAnimKitSlotLikeCpp::Ai, 77, |id| {
            id == 77
        })
        .expect("valid changed anim kit emits fanout event");

    let creature = manager.find_creature(571, 0, guid).expect("creature");
    assert_eq!(creature.creature.unit().ai_anim_kit_id_like_cpp(), 77);
    assert_eq!(
        creature.create_data.ai_anim_kit_id, 77,
        "late CREATE viewers must see the mutated anim kit state"
    );
    assert_eq!(event.source_guid, guid);
    match event.recipients {
        RecipientRule::NearbyVisible {
            source_guid,
            map_id,
            instance_id,
            source_position,
            range,
            required_3d,
        } => {
            assert_eq!(source_guid, guid);
            assert_eq!(map_id, 571);
            assert_eq!(instance_id, 0);
            assert_eq!(source_position, Position::new(10.0, 20.0, 30.0, 0.0));
            assert_eq!(range, VISIBILITY_RADIUS);
            assert!(!required_3d);
        }
        other => panic!("expected NearbyVisible, got {other:?}"),
    }
    let opcode = u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]]);
    assert_eq!(
        opcode,
        wow_constants::ServerOpcodes::SetAiAnimKit as u16,
        "C++ Unit::SetAIAnimKitId sends SMSG_SET_AI_ANIM_KIT after mutation"
    );
}

#[test]
fn set_creature_anim_kit_id_like_cpp_rejects_same_and_invalid_nonzero_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 104);
    let mut manager = MapManager::new();
    manager.add_creature(
        571,
        0,
        0,
        0,
        WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 20.0, 30.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        ),
    );

    assert!(
        manager
            .set_creature_anim_kit_id_like_cpp(
                571,
                0,
                guid,
                CreatureAnimKitSlotLikeCpp::Movement,
                88,
                |_| false,
            )
            .is_none(),
        "C++ Unit::SetMovementAnimKitId rejects nonzero IDs missing from sAnimKitStore"
    );
    assert_eq!(
        manager
            .find_creature(571, 0, guid)
            .unwrap()
            .creature
            .unit()
            .movement_anim_kit_id_like_cpp(),
        0
    );

    assert!(
        manager
            .set_creature_anim_kit_id_like_cpp(
                571,
                0,
                guid,
                CreatureAnimKitSlotLikeCpp::Melee,
                0,
                |_| false,
            )
            .is_none(),
        "same ID must not emit the C++ live packet"
    );
}

fn unique_test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rustycore-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ))
}

// ── Slice 4A.1a tests ────────────────────────────────────────────────────

/// `into_owning_session_plan` must produce one `SelfOnly` `RuntimeEvent`
/// per packet, in the same order, with `source_guid` set on every event.
#[test]
fn into_owning_session_plan_preserves_packets_as_self_only() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 42);

    let pkt_a = vec![0x01, 0x02, 0x03];
    let pkt_b = vec![0xAA, 0xBB];
    let pkt_c = vec![0xFF];

    let mut output = RuntimeOutput::new();
    output.packets.push(pkt_a.clone());
    output.packets.push(pkt_b.clone());
    output.packets.push(pkt_c.clone());

    let plan = output.into_owning_session_plan(guid);

    assert_eq!(plan.events.len(), 3, "must produce one event per packet");

    for (i, event) in plan.events.iter().enumerate() {
        assert_eq!(
            event.source_guid, guid,
            "event[{i}] must carry the source guid"
        );
        assert_eq!(
            event.recipients,
            RecipientRule::SelfOnly,
            "event[{i}] must be SelfOnly"
        );
    }

    // Packet bytes preserved in order.
    assert_eq!(plan.events[0].packet_bytes, pkt_a);
    assert_eq!(plan.events[1].packet_bytes, pkt_b);
    assert_eq!(plan.events[2].packet_bytes, pkt_c);
}

/// Empty `RuntimeOutput` produces an empty `RuntimePlan`.
#[test]
fn into_owning_session_plan_empty_output_gives_empty_plan() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 1);
    let plan = RuntimeOutput::new().into_owning_session_plan(guid);
    assert!(plan.events.is_empty());
}

/// Smoke: `RecipientRule::NearbyVisible` stores all its fields correctly.
#[test]
fn recipient_rule_nearby_visible_stores_fields() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 7);
    let pos = Position::new(1.0, 2.0, 3.0, 0.5);

    let rule = RecipientRule::NearbyVisible {
        source_guid: guid,
        map_id: 571,
        instance_id: 0,
        source_position: pos,
        range: 100.0,
        required_3d: true,
    };

    if let RecipientRule::NearbyVisible {
        source_guid,
        map_id,
        instance_id,
        source_position,
        range,
        required_3d,
    } = rule
    {
        assert_eq!(source_guid, guid);
        assert_eq!(map_id, 571);
        assert_eq!(instance_id, 0);
        assert_eq!(source_position.x, 1.0);
        assert_eq!(source_position.y, 2.0);
        assert_eq!(source_position.z, 3.0);
        assert!((range - 100.0).abs() < f32::EPSILON);
        assert!(required_3d);
    } else {
        panic!("expected NearbyVisible");
    }
}

/// Smoke: `RecipientRule::MapBroadcastVisible` stores map_id and instance_id.
#[test]
fn recipient_rule_map_broadcast_visible_stores_fields() {
    let rule = RecipientRule::MapBroadcastVisible {
        map_id: 0,
        instance_id: 5,
    };

    if let RecipientRule::MapBroadcastVisible {
        map_id,
        instance_id,
    } = rule
    {
        assert_eq!(map_id, 0);
        assert_eq!(instance_id, 5);
    } else {
        panic!("expected MapBroadcastVisible");
    }
}

/// `active_map_keys` returns the exact `(map_id, instance_id)` pairs of
/// the maps that have been created in the manager.
#[test]
fn active_map_keys_returns_inserted_map_keys() {
    let mut manager = MapManager::new();

    // No maps yet.
    assert!(manager.active_map_keys().is_empty());

    // Insert two distinct maps.
    manager.get_or_create_map(0, 0);
    manager.get_or_create_map(571, 1);

    let mut keys = manager.active_map_keys();
    keys.sort_unstable(); // deterministic order for assertions

    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], (0, 0));
    assert_eq!(keys[1], (571, 1));
}

// ── Slice 4A.2a: respawn queue tests ──────────────────────────────────────

fn make_pending_respawn(respawn_at: Instant) -> PendingRespawn {
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

/// A newly created `MapInstance` starts with an empty respawn queue.
#[test]
fn respawn_queue_starts_empty_like_cpp() {
    let map = MapInstance::new(0, 0);
    assert_eq!(map.respawn_queue_len(), 0);
}

/// Pushing one entry increments the length to 1.
#[test]
fn push_respawn_increments_len_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let now = Instant::now();
    map.push_respawn(make_pending_respawn(now));
    assert_eq!(map.respawn_queue_len(), 1);
}

#[test]
fn pending_respawn_save_load_roundtrip_uses_cpp_respawn_table_statement() {
    let mut manager = MapManager::new();
    let now = Instant::now();
    let now_secs = 1_700_000_000;
    let mut pending = make_pending_respawn(now + Duration::from_secs(45));
    pending.spawn_id = u64::from(u32::MAX) + 17;
    pending.map_id = 571;

    let stmt = manager
        .save_pending_respawn_time_like_cpp(571, 0, &pending, now, now_secs)
        .expect("future creature respawn should queue CHAR_REP_RESPAWN");

    assert_eq!(stmt.sql(), CharStatements::REP_RESPAWN.sql());
    assert!(matches!(stmt.params()[0], wow_database::SqlParam::U16(0)));
    assert!(matches!(
        stmt.params()[1],
        wow_database::SqlParam::U64(value) if value == pending.spawn_id
    ));
    assert!(matches!(
        stmt.params()[2],
        wow_database::SqlParam::I64(value) if value == now_secs + 45
    ));
    assert!(matches!(stmt.params()[3], wow_database::SqlParam::U16(571)));
    assert!(matches!(stmt.params()[4], wow_database::SqlParam::U32(0)));

    assert_eq!(
        manager.persisted_respawn_time_like_cpp(
            571,
            0,
            SpawnObjectType::Creature,
            pending.spawn_id
        ),
        Some(now_secs + 45)
    );

    let rows = manager.persisted_respawn_rows_like_cpp(571, 0);
    let mut restarted = MapManager::new();
    let report =
        restarted.load_persisted_respawns_into_queue_like_cpp(rows, now, now_secs, |_row, at| {
            Some(make_pending_respawn(at))
        });

    assert_eq!(report.rows, 1);
    assert_eq!(report.timers_loaded, 1);
    assert_eq!(report.creature_queued, 1);
    assert_eq!(restarted.respawn_queue_len(571, 0), 1);
    assert!(
        restarted
            .drain_ready_respawns(571, 0, now + Duration::from_secs(44))
            .is_empty()
    );
    let ready = restarted.drain_ready_respawns(571, 0, now + Duration::from_secs(45));
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].spawn_id, pending.spawn_id);
}

#[test]
fn push_respawn_replaces_later_duplicate_spawn_id_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let now = Instant::now();
    let later = now + Duration::from_secs(60);
    let earlier = now + Duration::from_secs(10);

    let mut first = make_pending_respawn(later);
    first.spawn_id = 42;
    let mut replacement = make_pending_respawn(earlier);
    replacement.spawn_id = 42;

    map.push_respawn(first);
    map.push_respawn(replacement);

    assert_eq!(map.respawn_queue_len(), 1);
    let ready = map.drain_ready_respawns(now + Duration::from_secs(11));
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].spawn_id, 42);
}

#[test]
fn push_respawn_ignores_later_duplicate_spawn_id_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let now = Instant::now();
    let earlier = now + Duration::from_secs(10);
    let later = now + Duration::from_secs(60);

    let mut first = make_pending_respawn(earlier);
    first.spawn_id = 77;
    let mut duplicate = make_pending_respawn(later);
    duplicate.spawn_id = 77;

    map.push_respawn(first);
    map.push_respawn(duplicate);

    assert_eq!(map.respawn_queue_len(), 1);
    let ready = map.drain_ready_respawns(now + Duration::from_secs(11));
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].spawn_id, 77);
}

#[test]
fn push_respawn_keeps_persistent_and_synthetic_id_namespaces_separate_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let now = Instant::now();

    let mut persistent = make_pending_respawn(now);
    persistent.spawn_id = 91;
    persistent.persistent_spawn = true;
    let mut synthetic = make_pending_respawn(now);
    synthetic.spawn_id = 91;
    synthetic.persistent_spawn = false;

    map.push_respawn(persistent);
    map.push_respawn(synthetic);

    assert_eq!(map.respawn_queue_len(), 2);
    let ready = map.drain_ready_respawns(now);
    assert_eq!(ready.len(), 2);
    assert!(ready.iter().any(|respawn| respawn.persistent_spawn));
    assert!(ready.iter().any(|respawn| !respawn.persistent_spawn));
}

#[test]
fn pending_respawn_rebuild_preserves_zero_wander_distance_like_cpp() {
    let mut pending = make_pending_respawn(Instant::now());
    pending.respawn_delay_secs = 45;
    pending.selected_equipment_id = 6;
    pending.original_equipment_id = -1;
    pending.string_id = Some("respawn-string".to_string());

    let creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);

    assert_eq!(
        creature.creature.ai_ownership().wander_radius,
        0.0,
        "C++ respawn uses CreatureData::wander_distance; idle spawns must not regain an invented wander radius"
    );
    assert_eq!(
        creature.creature.ai_ownership().respawn_time_secs,
        45,
        "C++ Creature::LoadFromDB copies CreatureData::spawntimesecs into m_respawnDelay"
    );
    assert_eq!(
        creature.creature.equipment_id(),
        6,
        "C++ LoadEquipment mutates m_equipmentId to the selected equipment template"
    );
    assert_eq!(
        creature.creature.original_equipment_id(),
        -1,
        "C++ InitEntry keeps CreatureData::equipmentId in m_originalEquipmentId before random equipment selection mutates the selected id"
    );
    assert_eq!(
        creature.creature.lifecycle_metadata().string_id.as_deref(),
        Some("respawn-string"),
        "C++ respawn reloads CreatureData::StringId through Creature::LoadFromDB"
    );
    assert!(!creature.should_wander());
}

#[test]
fn pending_respawn_rebuild_preserves_random_movement_type_like_cpp() {
    let mut pending = make_pending_respawn(Instant::now());
    pending.random_movement_type = wow_constants::CreatureRandomMovementType::AlwaysRun as u8;

    let creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);

    assert_eq!(
        creature.creature.random_movement_type_like_cpp(),
        wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
        "C++ respawn keeps using Creature::GetMovementTemplate(); Rust respawn must preserve the captured Random movement metadata"
    );
}

#[test]
fn pending_respawn_rebuild_preserves_default_movement_and_path_like_cpp() {
    let mut pending = make_pending_respawn(Instant::now());
    pending.default_movement_type = MovementGeneratorType::Waypoint;
    pending.waypoint_path_id = 9_002;

    let creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);

    assert_eq!(
        creature.creature.default_movement_type(),
        MovementGeneratorType::Waypoint,
        "C++ respawn reload path uses Creature::LoadFromDB/LoadCreaturesAddon and keeps the selected default motion"
    );
    assert_eq!(
        creature.creature.waypoint_path_id_like_cpp(),
        9_002,
        "C++ Creature::LoadCreaturesAddon preserves nonzero PathId for waypoint movement after respawn"
    );
}

/// `drain_ready_respawns` returns only entries whose `respawn_at <= now`.
#[test]
fn drain_returns_only_ready_entries_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let now = Instant::now();
    let past = now - Duration::from_secs(5);
    let future = now + Duration::from_secs(60);

    map.push_respawn(make_pending_respawn(past));
    map.push_respawn(make_pending_respawn(future));

    let ready = map.drain_ready_respawns(now);
    assert_eq!(ready.len(), 1);
    assert_eq!(map.respawn_queue_len(), 1);
}

/// Entries that are not yet ready remain in the queue after drain.
#[test]
fn future_entries_remain_after_drain_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let future = Instant::now() + Duration::from_secs(60);

    map.push_respawn(make_pending_respawn(future));

    let ready = map.drain_ready_respawns(Instant::now());
    assert_eq!(ready.len(), 0);
    assert_eq!(map.respawn_queue_len(), 1);
}

#[test]
fn persisted_respawn_restart_load_expired_ready_future_queued_and_gameobject_timer_loaded() {
    let mut manager = MapManager::new();
    let now = Instant::now();
    let now_secs = 2_000_000;
    let rows = vec![
        PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 10,
            respawn_time: now_secs - 5,
            map_id: 571,
            instance_id: 0,
        },
        PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 11,
            respawn_time: now_secs + 60,
            map_id: 571,
            instance_id: 0,
        },
        PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id: 12,
            respawn_time: now_secs + 90,
            map_id: 571,
            instance_id: 0,
        },
    ];

    let report =
        manager.load_persisted_respawns_into_queue_like_cpp(rows, now, now_secs, |_row, at| {
            Some(make_pending_respawn(at))
        });

    assert_eq!(report.rows, 3);
    assert_eq!(report.timers_loaded, 3);
    assert_eq!(report.creature_queued, 2);
    assert_eq!(report.gameobject_loaded, 1);
    assert_eq!(
        manager.persisted_respawn_time_like_cpp(571, 0, SpawnObjectType::GameObject, 12),
        Some(now_secs + 90)
    );

    let ready = manager.drain_ready_respawns(571, 0, now);
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].spawn_id, 10);
    assert_eq!(manager.respawn_queue_len(571, 0), 1);

    let delete = manager
        .remove_persisted_respawn_time_like_cpp(571, 0, SpawnObjectType::Creature, 10)
        .expect("processed C++ respawn should queue CHAR_DEL_RESPAWN");
    assert_eq!(delete.sql(), CharStatements::DEL_RESPAWN.sql());
    assert!(matches!(delete.params()[0], wow_database::SqlParam::U16(0)));
    assert!(matches!(
        delete.params()[1],
        wow_database::SqlParam::U64(10)
    ));
    assert!(matches!(
        delete.params()[2],
        wow_database::SqlParam::U16(571)
    ));
    assert!(matches!(delete.params()[3], wow_database::SqlParam::U32(0)));
    assert_eq!(
        manager.persisted_respawn_time_like_cpp(571, 0, SpawnObjectType::Creature, 10),
        None
    );

    let future = manager.drain_ready_respawns(571, 0, now + Duration::from_secs(59));
    assert!(future.is_empty());
    assert_eq!(manager.respawn_queue_len(571, 0), 1);
}

/// Ready entries are returned in insertion order.
#[test]
fn drain_preserves_insertion_order_like_cpp() {
    let mut map = MapInstance::new(0, 0);
    let t0 = Instant::now() - Duration::from_secs(10);
    let t1 = Instant::now() - Duration::from_secs(5);
    let t2 = Instant::now() - Duration::from_secs(1);

    // Insert in REVERSE temporal order (t2, t1, t0) — all in the past, all ready.
    // drain must return them in INSERTION order, not sorted by respawn_at, mirroring
    // the original Vec partition in run_creatures_tick (session.rs:20189-20201).
    map.push_respawn(make_pending_respawn(t2));
    map.push_respawn(make_pending_respawn(t1));
    map.push_respawn(make_pending_respawn(t0));

    let now = Instant::now();
    let ready = map.drain_ready_respawns(now);

    assert_eq!(ready.len(), 3);
    // Insertion order (t2, t1, t0), distinct from temporal order (t0, t1, t2).
    assert_eq!(ready[0].respawn_at, t2);
    assert_eq!(ready[1].respawn_at, t1);
    assert_eq!(ready[2].respawn_at, t0);
}

/// Queues are independent per (map_id, instance_id).
/// Pushing to (0, 0) must not affect (571, 1).
#[test]
fn respawn_queues_are_isolated_by_map_and_instance_like_cpp() {
    let mut manager = MapManager::new();
    let now = Instant::now();
    let past = now - Duration::from_secs(1);

    manager.push_respawn(0, 0, make_pending_respawn(past));

    assert_eq!(manager.respawn_queue_len(0, 0), 1);
    assert_eq!(manager.respawn_queue_len(571, 1), 0);

    let ready_571 = manager.drain_ready_respawns(571, 1, now);
    assert_eq!(ready_571.len(), 0);

    let ready_0 = manager.drain_ready_respawns(0, 0, now);
    assert_eq!(ready_0.len(), 1);
}

/// Unique temp `maps/` dir holding one synthetic constant-height tile.
fn temp_dir_with_constant_tile(map_id: u32, gx: i32, gy: i32, height: f32) -> PathBuf {
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

#[test]
fn respawn_ground_snap_uses_real_terrain_like_cpp() {
    // World (0,0) → raw tile (32,32). Ground at 77.0; spawn hovering above it.
    let dir = temp_dir_with_constant_tile(0, 32, 32, 77.0);
    let terrain = LiveTerrainHeights::new(&dir);

    let mut pending = make_pending_respawn(Instant::now());
    pending.home_pos.z = 80.0; // above ground; probe accepts the surface
    let mut creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    assert!((creature.creature.unit().world().position().z - 80.0).abs() < 1e-3);

    snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

    // Grounded, non-hovering: snapped exactly onto the surface (+0 hover).
    assert!(
        (creature.creature.unit().world().position().z - 77.0).abs() < 1e-2,
        "respawn must sit on the .map ground like Creature::Respawn/UpdateAllowedPositionZ"
    );
    // C++ SetHomePosition takes the snapped Z too.
    assert!((creature.home_position().z - 77.0).abs() < 1e-2);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn respawn_ground_snap_noop_without_terrain_tile_like_cpp() {
    // Empty maps dir → no tile → GetGridHeight invalid → Z untouched.
    let dir = temp_dir_with_constant_tile(0, 10, 10, 5.0); // tile for a different grid
    let terrain = LiveTerrainHeights::new(&dir);

    let mut pending = make_pending_respawn(Instant::now());
    pending.home_pos.z = 80.0;
    let mut creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

    assert!(
        (creature.creature.unit().world().position().z - 80.0).abs() < 1e-3,
        "no terrain under the spawn → C++ leaves Z unchanged"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn respawn_ground_snap_skips_creature_far_below_surface_like_cpp() {
    // Spawn well below ground: probe z < gridHeight - tolerance → GetStaticHeight
    // returns invalid, so C++ does NOT rescue a buried creature.
    let dir = temp_dir_with_constant_tile(0, 32, 32, 77.0);
    let terrain = LiveTerrainHeights::new(&dir);

    let mut pending = make_pending_respawn(Instant::now());
    pending.home_pos.z = 10.0; // far under the 77.0 surface
    let mut creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

    assert!((creature.creature.unit().world().position().z - 10.0).abs() < 1e-3);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pending_respawn_preserves_combat_log_state_across_legacy_and_canonical_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 45);
    let seed = test_creature(guid);
    let mut canonical = seed.creature;
    canonical.set_spawn_id(45);
    canonical.unit_mut().set_class(Class::Hunter as u8);
    canonical.set_power_type(PowerType::Focus);
    canonical.unit_mut().set_max_power(PowerType::Focus, 100);
    canonical.unit_mut().set_power(PowerType::Focus, 37);
    let combat_log_stats = CreatureCombatLogStatsLikeCpp {
        attack_power: 111,
        ranged_attack_power: 222,
        spell_power: 333,
        armor: 444,
    };
    canonical.set_combat_log_stats_like_cpp(combat_log_stats);
    let mut loaded_grid = WorldCreature::from_loaded_grid_canonical_like_cpp(canonical, |_| None);

    assert!(
        loaded_grid
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    );
    loaded_grid
        .creature
        .set_death_state_runtime(DeathState::JustDied, 0);
    assert!(
        !loaded_grid
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp(),
        "death cleanup must revoke the live marker before respawn"
    );

    let pending = pending_respawn_from_world_creature_like_cpp(&loaded_grid, Instant::now(), 0);
    assert_eq!(pending.create_data.unit_class, Class::Hunter as u8);
    assert_eq!(pending.create_data.display_power, PowerType::Focus as u8);
    assert_eq!(pending.create_data.power[0], 37);
    assert_eq!(pending.combat_log_stats, combat_log_stats);
    assert!(pending.spell_hit_aura_source_authority_like_cpp);
    assert!(pending.spell_cast_log_aura_source_authority_like_cpp);

    let legacy = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    let canonical_mirror = legacy.creature.clone();
    for creature in [&legacy.creature, &canonical_mirror] {
        assert_eq!(creature.unit().data().class_id, Class::Hunter as u8);
        assert_eq!(creature.power_type(), PowerType::Focus);
        assert_eq!(creature.unit().get_power(PowerType::Focus), 37);
        assert_eq!(creature.combat_log_stats_like_cpp(), combat_log_stats);
        assert_eq!(creature.combat_log_attack_power_like_cpp(), 222);
        assert!(
            creature
                .unit()
                .subsystems()
                .auras
                .has_complete_spell_hit_inert_aura_authority_like_cpp()
        );
        assert!(
            creature
                .unit()
                .subsystems()
                .auras
                .has_complete_spell_cast_log_aura_authority_like_cpp()
        );
    }
}

#[test]
fn pending_respawn_preserves_flags_extra_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 42);
    let mut creature = test_creature(guid);
    creature.creature.set_spawn_id(42);
    creature
        .creature
        .set_flags_extra_runtime_like_cpp(CreatureFlagsExtra::CIVILIAN.bits());
    let mut static_flags = [0; 8];
    static_flags[0] = wow_constants::creature::CreatureStaticFlags::NO_MELEE_FLEE.bits();
    creature
        .creature
        .set_static_flags_runtime_like_cpp(static_flags);
    creature
        .creature
        .set_ai_identity_names_runtime_like_cpp("SmartAI", "npc_respawn_identity");
    creature
        .creature
        .set_spawn_string_id_runtime_like_cpp(Some("respawn-string".to_string()));
    creature.creature.set_flight_movement_type_runtime_like_cpp(
        wow_constants::CreatureFlightMovementType::CanFly as u8,
    );
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::None as u8,
    );
    creature.creature.set_swim_allowed_runtime_like_cpp(false);

    let mut pending = pending_respawn_from_world_creature_like_cpp(&creature, Instant::now(), 0);
    pending.create_data.hover_height = 1.5;
    pending.ground_movement_type = wow_constants::CreatureGroundMovementType::Hover as u8;
    pending.addon = Some(CreatureAddonLifecycleRecordLikeCpp {
        path_id: 88_001,
        visibility_distance_type: wow_entities::VisibilityDistanceTypeLikeCpp::Large,
        auras: vec![70_020],
        ..CreatureAddonLifecycleRecordLikeCpp::default()
    });
    assert_eq!(
        pending.spawn_id, 42,
        "creature respawn must preserve C++ RespawnInfo::spawnId from Creature::GetSpawnId"
    );
    assert_eq!(pending.flags_extra, CreatureFlagsExtra::CIVILIAN.bits());
    assert_eq!(pending.static_flags[0], static_flags[0]);
    assert_eq!(pending.ai_name, "SmartAI");
    assert_eq!(pending.script_name, "npc_respawn_identity");
    assert_eq!(pending.string_id.as_deref(), Some("respawn-string"));
    assert_eq!(
        pending.ground_movement_type,
        wow_constants::CreatureGroundMovementType::Hover as u8
    );
    assert!(!pending.swim_allowed);
    assert_eq!(
        pending.flight_movement_type,
        wow_constants::CreatureFlightMovementType::CanFly as u8
    );

    let respawned = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    assert_eq!(
        respawned.creature.spawn_id(),
        42,
        "C++ Creature::LoadFromDB restores m_spawnId before registering the respawned creature"
    );
    assert!(
        respawned.creature.is_civilian_like_cpp(),
        "map-owned respawn must keep C++ flags_extra gates"
    );
    assert_eq!(respawned.creature.lifecycle_metadata().ai_name, "SmartAI");
    assert_eq!(
        respawned.creature.lifecycle_metadata().script_name,
        "npc_respawn_identity"
    );
    assert_eq!(
        respawned.creature.lifecycle_metadata().string_id.as_deref(),
        Some("respawn-string")
    );
    assert!(respawned.creature.can_walk_like_cpp());
    assert!(!respawned.creature.can_enter_water_like_cpp());
    assert!(respawned.creature.can_fly_like_cpp());
    assert_eq!(
        respawned.position().z,
        1.5,
        "C++ Creature::Create adds GetHoverOffset() to Z when respawn reloads a hovering creature"
    );
    assert_eq!(respawned.creature.unit().data().hover_height, 1.5);
    assert_eq!(
        respawned.creature.waypoint_path_id_like_cpp(),
        88_001,
        "C++ respawn goes back through Creature::LoadFromDB and reapplies LoadCreaturesAddon PathId"
    );
    assert!(
        respawned
            .creature
            .unit()
            .unit_flags2_like_cpp()
            .contains(wow_constants::unit::UnitFlags2::LARGE_AOI),
        "C++ LoadCreaturesAddon reapplies addon visibility/AOI flags on respawn"
    );
    assert!(
        respawned
            .creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_020),
        "C++ LoadCreaturesAddon reapplies addon auras on respawn"
    );
}

#[test]
fn pending_respawn_keeps_guid_counter_queue_only_for_legacy_zero_spawn_id() {
    let first_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 43);
    let second_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 44);
    let first = test_creature(first_guid);
    let second = test_creature(second_guid);

    assert_eq!(first.creature.spawn_id(), 0);
    assert_eq!(second.creature.spawn_id(), 0);

    let first_pending = pending_respawn_from_world_creature_like_cpp(&first, Instant::now(), 0);
    let second_pending = pending_respawn_from_world_creature_like_cpp(&second, Instant::now(), 0);

    assert_eq!(first_pending.spawn_id, first_guid.low_value() as u64);
    assert_eq!(second_pending.spawn_id, second_guid.low_value() as u64);
    assert_ne!(first_pending.spawn_id, second_pending.spawn_id);
    assert!(!first_pending.persistent_spawn);
    let respawned = world_creature_from_pending_respawn_like_cpp(&first_pending, 0);
    assert_eq!(respawned.creature.spawn_id(), 0);
    assert!(
        !respawned
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp(),
        "generic queue-only respawns must remain fail-closed"
    );
}
