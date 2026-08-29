//! Behaviour tests for [`super`].
//!
//! Extracted from `spawn_store_loader.rs`, which was 11,154 lines of which
//! 5,199 — 47% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::*;

struct FakeGameEventConditionSavePersistenceLikeCpp {
    outcome: wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp,
}

impl wow_persistence::GameEventPersistencePortLikeCpp
    for FakeGameEventConditionSavePersistenceLikeCpp
{
    fn load_condition_saves_like_cpp<'a>(
        &'a self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp,
    > {
        Box::pin(async { self.outcome.clone() })
    }

    fn execute_mutation_like_cpp<'a>(
        &'a self,
        _mutation: wow_persistence::GameEventPersistenceMutationLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::GameEventPersistenceMutationOutcomeLikeCpp,
    > {
        Box::pin(async { wow_persistence::GameEventPersistenceMutationOutcomeLikeCpp::Applied })
    }
}

fn map_store(ids: &[u32]) -> wow_data::MapStore {
    wow_data::MapStore::from_entries(ids.iter().copied().map(|id| wow_data::MapEntry {
        id,
        instance_type: 0,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }))
}

fn instanceable_map_store(ids: &[u32]) -> wow_data::MapStore {
    wow_data::MapStore::from_entries(ids.iter().copied().map(|id| wow_data::MapEntry {
        id,
        instance_type: wow_data::map::MAP_INSTANCE,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }))
}

fn world_state_row(
    id: i32,
    default_value: i32,
    map_ids_csv: &str,
    area_ids_csv: &str,
) -> WorldStateDbTemplateRowLikeCpp {
    WorldStateDbTemplateRowLikeCpp {
        id,
        default_value,
        map_ids_csv: map_ids_csv.to_string(),
        area_ids_csv: area_ids_csv.to_string(),
        script_name: String::new(),
    }
}

fn area_store(entries: &[(u32, u16)]) -> wow_data::AreaTableStore {
    wow_data::AreaTableStore::from_entries(entries.iter().copied().map(|(id, continent_id)| {
        wow_data::AreaTableEntry {
            id,
            continent_id,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        }
    }))
}

#[test]
fn waypoint_path_store_loads_paths_nodes_and_normalizes_coords_like_cpp() {
    let (store, report) = WaypointPathStoreLikeCpp::from_rows_like_cpp(
        [
            WaypointPathRowLikeCpp {
                path_id: 10,
                move_type: 1,
                flags: 0x01,
            },
            WaypointPathRowLikeCpp {
                path_id: 11,
                move_type: 4,
                flags: 0,
            },
        ],
        [
            WaypointPathNodeRowLikeCpp {
                path_id: 10,
                node_id: 1,
                x: wow_core::Position::MAP_HALFSIZE_LIKE_CPP + 100.0,
                y: -(wow_core::Position::MAP_HALFSIZE_LIKE_CPP + 100.0),
                z: 25.0,
                orientation: Some(1.25),
                delay: 500,
            },
            WaypointPathNodeRowLikeCpp {
                path_id: 12,
                node_id: 1,
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: None,
                delay: 0,
            },
        ],
    );

    assert_eq!(store.len(), 1);
    assert_eq!(report.path_rows, 2);
    assert_eq!(report.paths_loaded, 1);
    assert_eq!(report.skipped_invalid_move_type, 1);
    assert_eq!(report.node_rows, 2);
    assert_eq!(report.nodes_loaded, 1);
    assert_eq!(report.skipped_missing_path, 1);
    assert_eq!(report.backwards_too_short, 1);

    let path = store.get(10).expect("valid path retained");
    assert_eq!(path.move_type, wow_movement::WaypointMoveType::Run);
    assert!(path.follow_path_backwards_from_end_to_start);
    assert_eq!(path.nodes.len(), 1);
    let node = path.nodes[0];
    let limit = wow_core::Position::MAP_HALFSIZE_LIKE_CPP - 0.5;
    assert_eq!(node.id, 1);
    assert_eq!(node.position.x, limit);
    assert_eq!(node.position.y, -limit);
    assert_eq!(node.position.z, 25.0);
    assert_eq!(node.orientation, Some(1.25));
    assert_eq!(node.delay_ms, 500);
}

#[test]
fn waypoint_path_store_reports_empty_paths_and_clamped_delay_like_cpp() {
    let (store, report) = WaypointPathStoreLikeCpp::from_rows_like_cpp(
        [
            WaypointPathRowLikeCpp {
                path_id: 20,
                move_type: 0,
                flags: 0,
            },
            WaypointPathRowLikeCpp {
                path_id: 21,
                move_type: 3,
                flags: 0,
            },
        ],
        [WaypointPathNodeRowLikeCpp {
            path_id: 21,
            node_id: 7,
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: None,
            delay: u32::MAX,
        }],
    );

    assert_eq!(store.len(), 2);
    assert_eq!(report.empty_paths, 1);
    assert_eq!(report.clamped_delay, 1);
    assert_eq!(
        store.get(21).unwrap().move_type,
        wow_movement::WaypointMoveType::TakeOff
    );
    assert_eq!(store.get(21).unwrap().nodes[0].delay_ms, i32::MAX);
}

#[test]
fn waypoint_path_store_initializes_world_creature_default_waypoint_like_cpp() {
    let (store, _report) = WaypointPathStoreLikeCpp::from_rows_like_cpp(
        [WaypointPathRowLikeCpp {
            path_id: 30,
            move_type: 1,
            flags: 0,
        }],
        [WaypointPathNodeRowLikeCpp {
            path_id: 30,
            node_id: 7,
            x: 11.0,
            y: 12.0,
            z: 13.0,
            orientation: Some(1.5),
            delay: 250,
        }],
    );
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54_340);
    let mut creature = wow_world::map_manager::WorldCreature::new(
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
    creature.creature.load_path_like_cpp(30);

    let action =
        initialize_world_creature_default_waypoint_from_store_like_cpp(&mut creature, &store);

    assert_eq!(action, wow_movement::WaypointMovementAction::StopMoving);
    assert!(creature.creature.unit().subsystems().motion.stopped);
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        wow_movement::WaypointMovementAction::Launch(launch)
            if launch.path_id == 30
                && launch.node_id == 7
                && launch.destination == Position::new(11.0, 12.0, 13.0, 0.0)
    ));
}

#[test]
fn creature_spawntimesecs_uses_unsigned_db_domain_like_cpp() {
    assert_eq!(creature_spawntimesecs_to_i32_like_cpp(0).unwrap(), 0);
    assert_eq!(creature_spawntimesecs_to_i32_like_cpp(300).unwrap(), 300);
    assert_eq!(
        creature_spawntimesecs_to_i32_like_cpp(i32::MAX as u32).unwrap(),
        i32::MAX
    );
    assert!(creature_spawntimesecs_to_i32_like_cpp(i32::MAX as u32 + 1).is_err());
}

#[test]
fn signed_phase_ids_are_normalized_to_unsigned_domain_like_cpp_getuint32() {
    assert_eq!(
        normalize_signed_db_u32_like_cpp(0, "creature.phaseid").unwrap(),
        0
    );
    assert_eq!(
        normalize_signed_db_u32_like_cpp(123, "creature.phaseid").unwrap(),
        123
    );
    assert_eq!(
        normalize_signed_db_u32_like_cpp(i64::from(u32::MAX), "creature.phaseid").unwrap(),
        u32::MAX
    );
    assert!(normalize_signed_db_u32_like_cpp(-1, "creature.phaseid").is_err());
    assert!(normalize_signed_db_u32_like_cpp(i64::from(u32::MAX) + 1, "creature.phaseid").is_err());
}

#[test]
fn signed_linked_respawn_guids_are_normalized_like_cpp_getuint64() {
    assert_eq!(
        normalize_signed_db_u64_like_cpp(0, "linked_respawn.linkedGuid").unwrap(),
        0
    );
    assert_eq!(
        normalize_signed_db_u64_like_cpp(123_456, "linked_respawn.linkedGuid").unwrap(),
        123_456
    );
    assert!(normalize_signed_db_u64_like_cpp(-1, "linked_respawn.linkedGuid").is_err());
}

#[test]
fn signed_game_event_times_are_normalized_like_cpp_getuint64() {
    assert_eq!(
        normalize_signed_db_u64_like_cpp(0, "game_event.start_time").unwrap(),
        0
    );
    assert_eq!(
        normalize_signed_db_u64_like_cpp(1_893_456_000, "game_event.end_time").unwrap(),
        1_893_456_000
    );
    assert!(normalize_signed_db_u64_like_cpp(-1, "game_event.start_time").is_err());
}

#[test]
fn signed_game_event_pool_ids_use_getint8_domain_like_cpp() {
    assert_eq!(
        normalize_signed_db_i8_like_cpp(-1, "game_event_pool.eventEntry").unwrap(),
        -1
    );
    assert_eq!(
        normalize_signed_db_i8_like_cpp(127, "game_event_pool.eventEntry").unwrap(),
        127
    );
    assert!(normalize_signed_db_i8_like_cpp(-129, "game_event_pool.eventEntry").is_err());
    assert!(normalize_signed_db_i8_like_cpp(128, "game_event_pool.eventEntry").is_err());
}

#[test]
fn game_event_world_state_load_inserts_realm_default_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(100, 7, "", "")],
        [],
        |_| false,
        |_| None,
    );

    assert_eq!(report.template_rows, 1);
    assert_eq!(report.templates_loaded, 1);
    assert_eq!(mgr.realm_value_like_cpp(100), 7);
    assert_eq!(
        mgr.template_like_cpp(100)
            .map(|template| template.area_ids.len()),
        Some(0)
    );
}

#[test]
fn game_event_world_state_saved_value_overlays_realm_default_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(101, 7, "", "")],
        [(101, 9)],
        |_| false,
        |_| None,
    );

    assert_eq!(report.saved_rows, 1);
    assert_eq!(report.saved_applied, 1);
    assert_eq!(mgr.realm_value_like_cpp(101), 9);
}

#[test]
fn game_event_world_state_map_defaults_and_saved_overlay_all_maps_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(102, 3, "1,2", "")],
        [(102, 11)],
        |map_id| matches!(map_id, 1 | 2),
        |_| None,
    );

    assert_eq!(report.templates_loaded, 1);
    assert_eq!(report.saved_applied, 1);
    assert_eq!(mgr.map_value_like_cpp(1, 102), 11);
    assert_eq!(mgr.map_value_like_cpp(2, 102), 11);
    assert_eq!(mgr.realm_value_like_cpp(102), 0);
}

#[test]
fn game_event_world_state_invalid_map_and_area_lists_skip_rows_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [
            world_state_row(103, 1, "bogus,99", ""),
            world_state_row(104, 2, "1", "bogus,999"),
            world_state_row(105, 3, "1,not-int", "10,bad"),
        ],
        [],
        |map_id| map_id == 1,
        |area_id| (area_id == 10).then_some(1),
    );

    assert_eq!(report.template_rows, 3);
    assert_eq!(report.skipped_invalid_map_list, 1);
    assert_eq!(report.skipped_invalid_area_list, 1);
    assert_eq!(report.templates_loaded, 1);
    assert!(mgr.template_like_cpp(103).is_none());
    assert!(mgr.template_like_cpp(104).is_none());
    assert_eq!(mgr.map_value_like_cpp(1, 105), 3);
    assert_eq!(
        mgr.template_like_cpp(105)
            .map(|template| template.area_ids.contains(&10)),
        Some(true)
    );
}

#[test]
fn game_event_world_state_area_continent_must_match_required_maps_like_cpp() {
    let areas = area_store(&[(20, 2), (21, 1)]);
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(106, 4, "1", "20,21")],
        [],
        |map_id| map_id == 1,
        |area_id| areas.get(area_id).map(|area| area.continent_id),
    );

    assert_eq!(report.templates_loaded, 1);
    assert_eq!(
        mgr.template_like_cpp(106)
            .map(|template| template.area_ids.contains(&20)),
        Some(false)
    );
    assert_eq!(
        mgr.template_like_cpp(106)
            .map(|template| template.area_ids.contains(&21)),
        Some(true)
    );
}

#[test]
fn game_event_world_state_realm_row_with_area_ids_still_loads_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(107, 5, "", "20")],
        [],
        |_| false,
        |_| Some(1),
    );

    assert_eq!(report.realm_area_requirements_ignored, 1);
    assert_eq!(report.templates_loaded, 1);
    assert_eq!(mgr.realm_value_like_cpp(107), 5);
}

#[test]
fn game_event_world_state_unknown_saved_value_is_skipped_like_cpp() {
    let (mgr, report) = WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        [world_state_row(108, 5, "", "")],
        [(999, 12)],
        |_| false,
        |_| None,
    );

    assert_eq!(report.saved_rows, 1);
    assert_eq!(report.saved_skipped_unknown, 1);
    assert_eq!(report.saved_applied, 0);
    assert_eq!(mgr.realm_value_like_cpp(108), 5);
}

fn map_difficulty_store(entries: &[(u32, Difficulty)]) -> wow_data::MapDifficultyStore {
    wow_data::MapDifficultyStore::from_entries(entries.iter().enumerate().map(
        |(idx, (map_id, difficulty_id))| wow_data::MapDifficultyEntry {
            id: u32::try_from(idx + 1).unwrap_or(u32::MAX),
            message: String::new(),
            map_id: *map_id,
            difficulty_id: *difficulty_id,
            lock_id: 0,
            reset_interval: 0,
            max_players: 0,
            flags: 0,
        },
    ))
}

fn creature_row(spawn_id: SpawnId, event_entry: i16, difficulties: &str) -> CreatureSpawnRow {
    CreatureSpawnRow {
        spawn_id,
        entry: 123,
        map_id: 1,
        x: 10.0,
        y: 20.0,
        z: 30.0,
        orientation: 1.0,
        spawn_time_secs: 300,
        model_id: 0,
        equipment_id: 0,
        wander_distance: 0.0,
        curhealth: 0,
        curmana: 0,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        spawn_difficulties: difficulties.to_string(),
        event_entry,
        pool_id: 0,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        script_name: String::new(),
        string_id: String::new(),
    }
}

#[test]
fn creature_spawn_equipment_random_is_normalized_before_runtime_row_like_cpp() {
    let mut row = creature_row(1, 0, "0");
    row.entry = 123;
    row.equipment_id = -1;
    let equipment_store = wow_data::CreatureEquipmentStoreLikeCpp::from_entries([(
        123,
        1,
        wow_data::CreatureEquipmentInfoLikeCpp::default(),
    )]);

    normalize_creature_spawn_equipment_id_like_cpp(&mut row, &equipment_store);
    let runtime = creature_row_to_runtime_row_like_cpp(&row);

    assert_eq!(row.equipment_id, 1);
    assert_eq!(runtime.equipment_id, 1);
}

#[test]
fn creature_spawn_missing_equipment_is_normalized_to_zero_like_cpp() {
    let mut row = creature_row(1, 0, "0");
    row.entry = 123;
    row.equipment_id = 7;
    let equipment_store = wow_data::CreatureEquipmentStoreLikeCpp::from_entries([(
        123,
        1,
        wow_data::CreatureEquipmentInfoLikeCpp::default(),
    )]);

    normalize_creature_spawn_equipment_id_like_cpp(&mut row, &equipment_store);

    assert_eq!(row.equipment_id, 0);
}

fn gameobject_row(spawn_id: SpawnId, event_entry: i16, difficulties: &str) -> GameObjectSpawnRow {
    GameObjectSpawnRow {
        spawn_id,
        entry: 456,
        map_id: 1,
        x: 11.0,
        y: 21.0,
        z: 31.0,
        orientation: 1.0,
        rotation: [0.0, 0.0, 0.0, 1.0],
        spawn_time_secs: 300,
        anim_progress: 100,
        state: 1,
        spawn_difficulties: difficulties.to_string(),
        event_entry,
        pool_id: 0,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        script_name: String::new(),
        string_id: String::new(),
    }
}

fn area_trigger_row(spawn_id: SpawnId, difficulties: &str) -> AreaTriggerSpawnRow {
    AreaTriggerSpawnRow {
        spawn_id,
        create_properties_id: 789,
        is_custom: false,
        map_id: 1,
        spawn_difficulties: difficulties.to_string(),
        x: 12.0,
        y: 22.0,
        z: 32.0,
        orientation: 1.0,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        spell_for_visuals: None,
        script_name: String::new(),
    }
}

fn world_safe_locs() -> wow_data::WorldSafeLocStore {
    let maps = map_store(&[1]);
    wow_data::WorldSafeLocStore::from_rows_like_cpp([], &maps).0
}

fn area_trigger_create_properties_row(
    create_properties_id: u32,
) -> wow_data::AreaTriggerCreatePropertiesRowLikeCpp {
    wow_data::AreaTriggerCreatePropertiesRowLikeCpp {
        id: create_properties_id,
        is_custom: false,
        area_trigger_id: 0,
        is_areatrigger_custom: false,
        flags: wow_data::area_trigger_template::AREATRIGGER_CREATE_PROPERTIES_FLAG_NONE_LIKE_CPP,
        move_curve_id: 0,
        scale_curve_id: 0,
        morph_curve_id: 0,
        facing_curve_id: 0,
        anim_id: 0,
        anim_kit_id: 0,
        decal_properties_id: 0,
        time_to_target: 0,
        time_to_target_scale: 0,
        shape: wow_data::area_trigger_template::AREATRIGGER_SHAPE_SPHERE_LIKE_CPP,
        shape_data: [0.0; wow_data::area_trigger_template::MAX_AREATRIGGER_ENTITY_DATA_LIKE_CPP],
        script_name: String::new(),
    }
}

fn area_trigger_template_store_with(
    create_properties_row: wow_data::AreaTriggerCreatePropertiesRowLikeCpp,
    spline_points: impl IntoIterator<Item = wow_data::AreaTriggerSplinePointRowLikeCpp>,
    orbit_rows: impl IntoIterator<Item = wow_data::AreaTriggerCreatePropertiesOrbitRowLikeCpp>,
) -> wow_data::AreaTriggerTemplateStore {
    wow_data::AreaTriggerTemplateStore::from_rows_like_cpp(
        [],
        [],
        [],
        spline_points,
        [create_properties_row],
        orbit_rows,
        &world_safe_locs(),
        |_| true,
        |_| wow_data::ScriptIdLikeCpp(0),
    )
    .store
}

fn valid_area_trigger_template_store() -> wow_data::AreaTriggerTemplateStore {
    area_trigger_template_store_with(area_trigger_create_properties_row(789), [], [])
}

fn area_trigger_spline_point(
    create_properties_id: u32,
    x: f32,
) -> wow_data::AreaTriggerSplinePointRowLikeCpp {
    wow_data::AreaTriggerSplinePointRowLikeCpp {
        create_properties_id,
        is_custom: false,
        x,
        y: 0.0,
        z: 0.0,
    }
}

fn area_trigger_orbit(
    create_properties_id: u32,
) -> wow_data::AreaTriggerCreatePropertiesOrbitRowLikeCpp {
    wow_data::AreaTriggerCreatePropertiesOrbitRowLikeCpp {
        create_properties_id,
        is_custom: false,
        start_delay: 0,
        circle_radius: 1.0,
        blend_from_radius: 0.0,
        initial_angle: 0.0,
        z_offset: 0.0,
        counter_clockwise: false,
        can_loop: false,
    }
}

fn event(
    event_id: u16,
    state: GameEventStateLikeCpp,
    start: u64,
    end: u64,
    occurence: u32,
    length: u32,
) -> GameEventDataLikeCpp {
    GameEventDataLikeCpp {
        event_id,
        start,
        end,
        next_start: 0,
        occurence,
        length,
        holiday_id: 0,
        holiday_stage: 0,
        state_raw: state as u8,
        prerequisite_events: BTreeSet::new(),
        conditions: BTreeMap::new(),
        description: String::new(),
        announce: 0,
    }
}

fn event_with_raw_state(
    event_id: u16,
    state_raw: u8,
    start: u64,
    end: u64,
    occurence: u32,
    length: u32,
) -> GameEventDataLikeCpp {
    let mut game_event = event(
        event_id,
        GameEventStateLikeCpp::Normal,
        start,
        end,
        occurence,
        length,
    );
    game_event.state_raw = state_raw;
    game_event
}

fn event_with_next_start(
    mut game_event: GameEventDataLikeCpp,
    next_start: u64,
) -> GameEventDataLikeCpp {
    game_event.next_start = next_start;
    game_event
}

fn event_with_prerequisites(
    mut game_event: GameEventDataLikeCpp,
    prerequisites: impl IntoIterator<Item = u16>,
) -> GameEventDataLikeCpp {
    game_event.prerequisite_events = prerequisites.into_iter().collect();
    game_event
}

fn event_with_holiday(
    mut game_event: GameEventDataLikeCpp,
    holiday_id: u32,
) -> GameEventDataLikeCpp {
    game_event.holiday_id = holiday_id;
    game_event
}

fn event_with_condition(
    mut game_event: GameEventDataLikeCpp,
    condition_id: u32,
    condition: GameEventConditionLikeCpp,
) -> GameEventDataLikeCpp {
    game_event.conditions.insert(condition_id, condition);
    game_event
}

fn condition(req_num: f32, done: f32) -> GameEventConditionLikeCpp {
    GameEventConditionLikeCpp {
        req_num,
        done,
        max_world_state: 77,
        done_world_state: 88,
    }
}

fn game_event_store(
    events: impl IntoIterator<Item = GameEventDataLikeCpp>,
) -> GameEventDataStoreLikeCpp {
    game_event_store_with_max(8, events)
}

fn game_event_store_with_max(
    max_event_entry: u32,
    events: impl IntoIterator<Item = GameEventDataLikeCpp>,
) -> GameEventDataStoreLikeCpp {
    events.into_iter().fold(
        GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(max_event_entry)),
        GameEventDataStoreLikeCpp::with_event_like_cpp,
    )
}

#[test]
fn game_event_active_set_insert_dedupe_order_remove_and_clear_like_cpp() {
    let mut active = GameEventActiveSetLikeCpp::new();

    assert!(active.add_active_event_like_cpp(7));
    assert!(active.add_active_event_like_cpp(2));
    assert!(!active.add_active_event_like_cpp(7));
    assert!(active.add_active_event_like_cpp(5));
    assert_eq!(
        active.active_event_ids_like_cpp().collect::<Vec<_>>(),
        vec![2, 5, 7]
    );

    assert!(active.remove_active_event_like_cpp(5));
    assert!(!active.remove_active_event_like_cpp(5));
    assert_eq!(
        active.active_event_ids_like_cpp().collect::<Vec<_>>(),
        vec![2, 7]
    );

    active.clear_active_events_like_cpp();
    assert_eq!(active.active_event_ids_like_cpp().count(), 0);
}

#[test]
fn game_event_is_active_event_checks_membership_like_cpp() {
    let mut active = GameEventActiveSetLikeCpp::new();
    active.add_active_event_like_cpp(3);

    assert!(active.is_active_event_like_cpp(3));
    assert!(!active.is_active_event_like_cpp(4));
}

#[test]
fn game_event_is_holiday_active_matches_cpp_and_reports_missing_active_event_like_cpp() {
    let store = game_event_store([
        event_with_holiday(event(1, GameEventStateLikeCpp::Normal, 0, 0, 0, 0), 141),
        event_with_holiday(event(2, GameEventStateLikeCpp::Normal, 0, 0, 0, 0), 142),
    ]);
    let mut active = GameEventActiveSetLikeCpp::new();

    assert_eq!(
        active.is_holiday_active_like_cpp(&store, 0),
        GameEventHolidayActiveOutcomeLikeCpp::Active(false)
    );

    active.add_active_event_like_cpp(2);
    assert_eq!(
        active.is_holiday_active_like_cpp(&store, 142),
        GameEventHolidayActiveOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        active.is_holiday_active_like_cpp(&store, 141),
        GameEventHolidayActiveOutcomeLikeCpp::Active(false)
    );

    active.add_active_event_like_cpp(99);
    assert_eq!(
        active.is_holiday_active_like_cpp(&store, 141),
        GameEventHolidayActiveOutcomeLikeCpp::MissingActiveEvent { event_id: 99 }
    );
}

#[test]
fn game_event_active_set_lives_with_canonical_metadata_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new());

    assert!(
        !metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(4)
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(4);
    assert!(
        metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(4)
    );
}

#[test]
fn game_event_condition_metadata_load_replaces_duplicate_and_skips_out_of_range_like_cpp() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventConditionLoadReportLikeCpp::default();

    apply_game_event_condition_row_like_cpp(
        GameEventConditionRowLikeCpp {
            event_id: 1,
            condition_id: 10,
            req_num: 3.5,
            max_world_state: 100,
            done_world_state: 101,
        },
        &mut events,
        &mut report,
    );
    apply_game_event_condition_row_like_cpp(
        GameEventConditionRowLikeCpp {
            event_id: 1,
            condition_id: 10,
            req_num: 7.0,
            max_world_state: 200,
            done_world_state: 201,
        },
        &mut events,
        &mut report,
    );
    apply_game_event_condition_row_like_cpp(
        GameEventConditionRowLikeCpp {
            event_id: 3,
            condition_id: 11,
            req_num: 1.0,
            max_world_state: 0,
            done_world_state: 0,
        },
        &mut events,
        &mut report,
    );

    let loaded = events
        .event_like_cpp(1)
        .unwrap()
        .conditions
        .get(&10)
        .unwrap();
    assert_eq!(loaded.req_num, 7.0);
    assert_eq!(loaded.done, 0.0);
    assert_eq!(loaded.max_world_state, 200);
    assert_eq!(loaded.done_world_state, 201);
    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 2);
    assert_eq!(report.skipped_out_of_range, 1);
}

#[test]
fn game_event_condition_save_applies_only_existing_event_condition_like_cpp() {
    let mut events = game_event_store([event_with_condition(
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
        10,
        condition(7.0, 0.0),
    )]);
    let mut report = GameEventConditionSaveLoadReportLikeCpp::default();

    apply_game_event_condition_save_row_like_cpp(
        GameEventConditionSaveRowLikeCpp {
            event_id: 1,
            condition_id: 10,
            done: 4.0,
        },
        &mut events,
        &mut report,
    );
    apply_game_event_condition_save_row_like_cpp(
        GameEventConditionSaveRowLikeCpp {
            event_id: 1,
            condition_id: 99,
            done: 6.0,
        },
        &mut events,
        &mut report,
    );
    apply_game_event_condition_save_row_like_cpp(
        GameEventConditionSaveRowLikeCpp {
            event_id: 99,
            condition_id: 10,
            done: 6.0,
        },
        &mut events,
        &mut report,
    );

    assert_eq!(
        events
            .event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        4.0
    );
    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_missing_condition, 1);
    assert_eq!(report.skipped_out_of_range_event, 1);
}

#[tokio::test]
async fn game_event_condition_save_loader_uses_typed_rows_and_preserves_validation_like_cpp() {
    let mut events = game_event_store([event_with_condition(
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
        10,
        condition(7.0, 0.0),
    )]);
    let persistence = FakeGameEventConditionSavePersistenceLikeCpp {
        outcome: wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Loaded(vec![
            wow_persistence::GameEventConditionSavePersistenceRowLikeCpp {
                event_id: 1,
                condition_id: 10,
                done: 4.0,
            },
            wow_persistence::GameEventConditionSavePersistenceRowLikeCpp {
                event_id: 1,
                condition_id: 99,
                done: 6.0,
            },
        ]),
    };
    let mut report = CanonicalSpawnStoreLoadReport::default();

    load_game_event_condition_saves_like_cpp(&persistence, &mut events, &mut report)
        .await
        .unwrap();

    assert_eq!(
        events
            .event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        4.0
    );
    assert_eq!(report.game_event_condition_saves.rows, 2);
    assert_eq!(report.game_event_condition_saves.loaded, 1);
    assert_eq!(
        report.game_event_condition_saves.skipped_missing_condition,
        1
    );
}

#[tokio::test]
async fn game_event_condition_save_loader_propagates_typed_failure_without_mutation_like_cpp() {
    let mut events = game_event_store([event_with_condition(
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
        10,
        condition(7.0, 2.0),
    )]);
    let persistence = FakeGameEventConditionSavePersistenceLikeCpp {
        outcome: wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Failed {
            reason: "fixture load failure".to_string(),
        },
    };
    let mut report = CanonicalSpawnStoreLoadReport::default();

    let error = load_game_event_condition_saves_like_cpp(&persistence, &mut events, &mut report)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("fixture load failure"));
    assert_eq!(
        events
            .event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        2.0
    );
    assert_eq!(report.game_event_condition_saves.rows, 0);
}

#[test]
fn game_event_world_state_update_evidence_orders_conditions_done_then_max_like_cpp() {
    let event = event_with_condition(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            20,
            GameEventConditionLikeCpp {
                req_num: 9.8,
                done: 4.2,
                max_world_state: 220,
                done_world_state: 221,
            },
        ),
        10,
        GameEventConditionLikeCpp {
            req_num: 7.0,
            done: 3.0,
            max_world_state: 120,
            done_world_state: 121,
        },
    );
    let events = game_event_store([event]);

    assert_eq!(
        events.send_world_state_update_evidence_like_cpp(1),
        GameEventWorldStateUpdateOutcomeLikeCpp::Updates {
            event_id: 1,
            updates: vec![
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 121,
                    value: 3,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 120,
                    value: 7,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 221,
                    value: 4,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 220,
                    value: 9,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                },
            ],
            skipped: Vec::new(),
        }
    );
}

#[test]
fn game_event_world_state_update_skips_zero_worldstate_ids_like_cpp() {
    let events = game_event_store([event_with_condition(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            GameEventConditionLikeCpp {
                req_num: 2.0,
                done: 1.0,
                max_world_state: 0,
                done_world_state: 88,
            },
        ),
        20,
        GameEventConditionLikeCpp {
            req_num: 4.0,
            done: 3.0,
            max_world_state: 77,
            done_world_state: 0,
        },
    )]);

    assert_eq!(
        events.send_world_state_update_evidence_like_cpp(1),
        GameEventWorldStateUpdateOutcomeLikeCpp::Updates {
            event_id: 1,
            updates: vec![
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 88,
                    value: 1,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 77,
                    value: 4,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                },
            ],
            skipped: Vec::new(),
        }
    );
}

#[test]
fn game_event_world_state_update_missing_event_is_explicit_like_cpp() {
    let events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(1));

    assert_eq!(
        events.send_world_state_update_evidence_like_cpp(2),
        GameEventWorldStateUpdateOutcomeLikeCpp::MissingEvent { event_id: 2 }
    );
}

#[test]
fn game_event_world_state_update_skips_invalid_numeric_values_like_cpp() {
    let events = game_event_store([event_with_condition(
        event_with_condition(
            event_with_condition(
                event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
                10,
                GameEventConditionLikeCpp {
                    req_num: f32::INFINITY,
                    done: -1.0,
                    max_world_state: 110,
                    done_world_state: 111,
                },
            ),
            20,
            GameEventConditionLikeCpp {
                req_num: 2_147_483_648.0,
                done: 2.0,
                max_world_state: 220,
                done_world_state: 221,
            },
        ),
        30,
        GameEventConditionLikeCpp {
            req_num: 3.0,
            done: f32::NAN,
            max_world_state: 330,
            done_world_state: 331,
        },
    )]);

    assert_eq!(
        events.send_world_state_update_evidence_like_cpp(1),
        GameEventWorldStateUpdateOutcomeLikeCpp::Updates {
            event_id: 1,
            updates: vec![
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 221,
                    value: 2,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                },
                GameEventWorldStateUpdateEvidenceLikeCpp {
                    event_id: 1,
                    condition_id: 30,
                    variable_id: 330,
                    value: 3,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                },
            ],
            skipped: vec![
                GameEventWorldStateUpdateSkipLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 111,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                    reason: GameEventWorldStateValueSkipReasonLikeCpp::Negative,
                },
                GameEventWorldStateUpdateSkipLikeCpp {
                    event_id: 1,
                    condition_id: 10,
                    variable_id: 110,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                    reason: GameEventWorldStateValueSkipReasonLikeCpp::NonFinite,
                },
                GameEventWorldStateUpdateSkipLikeCpp {
                    event_id: 1,
                    condition_id: 20,
                    variable_id: 220,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Max,
                    reason: GameEventWorldStateValueSkipReasonLikeCpp::OutOfI32Range,
                },
                GameEventWorldStateUpdateSkipLikeCpp {
                    event_id: 1,
                    condition_id: 30,
                    variable_id: 331,
                    source: GameEventWorldStateUpdateSourceLikeCpp::Done,
                    reason: GameEventWorldStateValueSkipReasonLikeCpp::NonFinite,
                },
            ],
        }
    );
}

#[test]
fn game_event_check_one_conditions_empty_loop_completes_and_preserves_next_start_like_cpp() {
    let mut events = game_event_store([event_with_next_start(
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
        999,
    )]);

    assert_eq!(
        events.check_one_game_event_conditions_like_cpp(1, 100),
        GameEventConditionCheckOutcomeLikeCpp::Completed(GameEventConditionCheckSummaryLikeCpp {
            event_id: 1,
            condition_count: 0,
            state_before_raw: GameEventStateLikeCpp::WorldConditions as u8,
            state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            next_start_before: 999,
            next_start_after: 999,
        })
    );
}

#[test]
fn game_event_check_one_conditions_blocks_until_all_done_like_cpp() {
    let mut events = game_event_store([event_with_condition(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            20,
            condition(2.0, 2.0),
        ),
        10,
        condition(3.0, 1.0),
    )]);

    assert_eq!(
        events.check_one_game_event_conditions_like_cpp(1, 100),
        GameEventConditionCheckOutcomeLikeCpp::NotCompleted {
            event_id: 1,
            blocking_condition_id: 10,
        }
    );
    assert_eq!(
        events.event_like_cpp(1).unwrap().state_raw,
        GameEventStateLikeCpp::WorldConditions as u8
    );
}

#[test]
fn game_event_condition_progress_saturates_saves_then_completes_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    let outcome =
        metadata.represented_update_game_event_condition_progress_like_cpp(1, 10, 5.0, 100);

    assert_eq!(
        outcome,
        GameEventConditionProgressOutcomeLikeCpp::Progressed(
            GameEventConditionProgressSummaryLikeCpp {
                event_id: 1,
                condition_id: 10,
                done_before: 1.0,
                done_after: 3.0,
                req_num: 3.0,
                persistence_event_id: 1,
                completed_event: true,
                check_outcome: GameEventConditionCheckOutcomeLikeCpp::Completed(
                    GameEventConditionCheckSummaryLikeCpp {
                        event_id: 1,
                        condition_count: 1,
                        state_before_raw: GameEventStateLikeCpp::WorldConditions as u8,
                        state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
                        next_start_before: 0,
                        next_start_after: 400,
                    }
                ),
                save_world_event_state_requested: true,
                force_game_event_update_requested: true,
            }
        )
    );
}

#[test]
fn game_event_condition_progress_early_returns_do_not_mutate_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        )]));

    assert_eq!(
        metadata.represented_update_game_event_condition_progress_like_cpp(1, 10, 1.0, 100),
        GameEventConditionProgressOutcomeLikeCpp::InactiveEvent { event_id: 1 }
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);
    assert_eq!(
        metadata.represented_update_game_event_condition_progress_like_cpp(1, 99, 1.0, 100),
        GameEventConditionProgressOutcomeLikeCpp::MissingCondition {
            event_id: 1,
            condition_id: 99,
        }
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}

#[test]
fn game_event_quest_condition_metadata_load_skips_out_of_range_and_last_row_wins_like_cpp() {
    let events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut quest_conditions = BTreeMap::new();
    let mut report = GameEventQuestConditionLoadReportLikeCpp::default();

    apply_game_event_quest_condition_row_like_cpp(
        GameEventQuestConditionRowLikeCpp {
            quest_id: 7000,
            event_id: 1,
            condition_id: 10,
            num: 1.25,
        },
        &events,
        &mut quest_conditions,
        &mut report,
    );
    apply_game_event_quest_condition_row_like_cpp(
        GameEventQuestConditionRowLikeCpp {
            quest_id: 7000,
            event_id: 2,
            condition_id: 20,
            num: 2.5,
        },
        &events,
        &mut quest_conditions,
        &mut report,
    );
    apply_game_event_quest_condition_row_like_cpp(
        GameEventQuestConditionRowLikeCpp {
            quest_id: 8000,
            event_id: 3,
            condition_id: 30,
            num: 4.0,
        },
        &events,
        &mut quest_conditions,
        &mut report,
    );

    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 2);
    assert_eq!(report.overwrites, 1);
    assert_eq!(report.skipped_out_of_range_event, 1);
    assert_eq!(
        quest_conditions.get(&7000),
        Some(&GameEventQuestConditionRecordLikeCpp {
            quest_id: 7000,
            event_id: 2,
            condition_id: 20,
            num: 2.5,
        })
    );
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_event_quest_conditions_like_cpp(quest_conditions.clone());
    assert_eq!(
        metadata.game_event_quest_condition_like_cpp(7000),
        quest_conditions.get(&7000)
    );
    assert!(!quest_conditions.contains_key(&8000));
}

fn metadata_with_quest_condition_like_cpp(
    event: GameEventDataLikeCpp,
    quest_id: u32,
    event_id: u16,
    condition_id: u32,
    num: f32,
) -> CanonicalSpawnMetadataLikeCpp {
    let mut quest_conditions = BTreeMap::new();
    quest_conditions.insert(
        quest_id,
        GameEventQuestConditionRecordLikeCpp {
            quest_id,
            event_id,
            condition_id,
            num,
        },
    );
    let max_event_entry = u32::from(event.event_id).max(8);
    CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store_with_max(max_event_entry, [event]))
        .with_game_event_quest_conditions_like_cpp(quest_conditions)
}

#[test]
fn game_event_quest_complete_missing_mapping_does_not_mutate_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100),
        GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping { quest_id: 7000 }
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}

#[test]
fn game_event_quest_complete_inactive_event_does_not_mutate_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        ),
        7000,
        1,
        10,
        1.0,
    );

    assert_eq!(
        metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100),
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::InactiveEvent { event_id: 1 }
        )
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}

#[test]
fn game_event_quest_complete_non_world_conditions_does_not_mutate_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event(1, GameEventStateLikeCpp::Normal, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        ),
        7000,
        1,
        10,
        1.0,
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100),
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::NotWorldConditions {
                event_id: 1,
                state_raw: GameEventStateLikeCpp::Normal as u8,
            }
        )
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}

#[test]
fn game_event_quest_complete_missing_condition_does_not_mutate_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
            10,
            condition(3.0, 1.0),
        ),
        7000,
        1,
        99,
        1.0,
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100),
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::MissingCondition {
                event_id: 1,
                condition_id: 99,
            }
        )
    );
    assert_eq!(
        metadata
            .game_event_like_cpp(1)
            .unwrap()
            .conditions
            .get(&10)
            .unwrap()
            .done,
        1.0
    );
}

#[test]
fn game_event_quest_complete_increments_clamps_and_emits_condition_save_evidence_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event_with_condition(
                event(257, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
                10,
                condition(3.0, 1.0),
            ),
            20,
            condition(4.0, 1.0),
        ),
        7000,
        257,
        10,
        5.0,
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(257);

    let outcome = metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100);

    assert_eq!(
        outcome,
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::Progressed(
                GameEventConditionProgressSummaryLikeCpp {
                    event_id: 257,
                    condition_id: 10,
                    done_before: 1.0,
                    done_after: 3.0,
                    req_num: 3.0,
                    persistence_event_id: 1,
                    completed_event: false,
                    check_outcome: GameEventConditionCheckOutcomeLikeCpp::NotCompleted {
                        event_id: 257,
                        blocking_condition_id: 20,
                    },
                    save_world_event_state_requested: false,
                    force_game_event_update_requested: false,
                }
            )
        )
    );
}

#[test]
fn game_event_quest_complete_all_conditions_done_requests_save_and_force_like_cpp() {
    let mut metadata = metadata_with_quest_condition_like_cpp(
        event_with_condition(
            event_with_condition(
                event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 5),
                10,
                condition(3.0, 1.0),
            ),
            20,
            condition(4.0, 4.0),
        ),
        7000,
        1,
        10,
        2.0,
    );
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    let outcome = metadata.represented_handle_game_event_quest_complete_like_cpp(7000, 100);

    assert!(matches!(
        outcome,
        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            GameEventConditionProgressOutcomeLikeCpp::Progressed(
                GameEventConditionProgressSummaryLikeCpp {
                    completed_event: true,
                    save_world_event_state_requested: true,
                    force_game_event_update_requested: true,
                    check_outcome: GameEventConditionCheckOutcomeLikeCpp::Completed(
                        GameEventConditionCheckSummaryLikeCpp {
                            state_after_raw,
                            next_start_after: 400,
                            ..
                        }
                    ),
                    ..
                }
            )
        ) if state_after_raw == GameEventStateLikeCpp::WorldNextPhase as u8
    ));
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldNextPhase as u8);
    assert_eq!(event.next_start, 400);
}

#[test]
fn game_event_start_normal_internal_adds_active_apply_only_like_cpp() {
    for state in [
        GameEventStateLikeCpp::Normal,
        GameEventStateLikeCpp::Internal,
    ] {
        let mut metadata =
            CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
                .with_game_events_like_cpp(game_event_store([event(1, state, 100, 1_000, 10, 2)]));

        assert_eq!(
            metadata.start_game_event_like_cpp(1, false, 500, true),
            GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
                event_id: 1,
                state_before_raw: state as u8,
                state_after_raw: state as u8,
                active_added: true,
                active_was_present: false,
                apply_new_event_requested: true,
                save_world_event_state_requested: false,
                force_game_event_update_requested: false,
                completed: false,
            })
        );
        assert!(
            metadata
                .game_event_active_set_like_cpp()
                .is_active_event_like_cpp(1)
        );
        assert_eq!(metadata.game_event_like_cpp(1).unwrap().start, 100);
    }
}

#[test]
fn game_event_start_normal_overwrite_repairs_end_without_minutes_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event(
            1,
            GameEventStateLikeCpp::Normal,
            100,
            400,
            10,
            7,
        )]));

    let outcome = metadata.start_game_event_like_cpp(1, true, 500, false);

    assert!(matches!(
        outcome,
        GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
            completed: false,
            save_world_event_state_requested: false,
            ..
        })
    ));
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.start, 500);
    assert_eq!(event.end, 507);
}

#[test]
fn game_event_start_world_inactive_conditions_false_saves_without_nextphase_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event(
            1,
            GameEventStateLikeCpp::WorldInactive,
            0,
            0,
            0,
            7,
        )]));

    assert_eq!(
        metadata.start_game_event_like_cpp(1, true, 500, false),
        GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
            event_id: 1,
            state_before_raw: GameEventStateLikeCpp::WorldInactive as u8,
            state_after_raw: GameEventStateLikeCpp::WorldConditions as u8,
            active_added: true,
            active_was_present: false,
            apply_new_event_requested: true,
            save_world_event_state_requested: true,
            force_game_event_update_requested: false,
            completed: false,
        })
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(
        event.state_raw,
        GameEventStateLikeCpp::WorldConditions as u8
    );
    assert_eq!(event.next_start, 0);
    assert!(
        metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}

#[test]
fn game_event_start_serverwide_conditions_true_nextphase_and_force_flag_like_cpp() {
    for overwrite in [false, true] {
        let mut metadata =
            CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
                .with_game_events_like_cpp(game_event_store([event(
                    1,
                    GameEventStateLikeCpp::WorldConditions,
                    0,
                    0,
                    0,
                    7,
                )]));

        assert_eq!(
            metadata.start_game_event_like_cpp(1, overwrite, 500, true),
            GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
                event_id: 1,
                state_before_raw: GameEventStateLikeCpp::WorldConditions as u8,
                state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
                active_added: true,
                active_was_present: false,
                apply_new_event_requested: true,
                save_world_event_state_requested: true,
                force_game_event_update_requested: overwrite,
                completed: true,
            })
        );
        let event = metadata.game_event_like_cpp(1).unwrap();
        assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldNextPhase as u8);
        assert_eq!(event.next_start, 920);
    }

    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_next_start(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 7),
            777,
        )]));
    metadata.start_game_event_like_cpp(1, true, 500, true);
    assert_eq!(metadata.game_event_like_cpp(1).unwrap().next_start, 777);
}

#[test]
fn game_event_start_unknown_raw_state_is_serverwide_no_panic_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_raw_state(1, 99, 0, 0, 0, 3)]));

    assert_eq!(
        metadata.start_game_event_like_cpp(1, false, 100, true),
        GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
            event_id: 1,
            state_before_raw: 99,
            state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            active_added: true,
            active_was_present: false,
            apply_new_event_requested: true,
            save_world_event_state_requested: true,
            force_game_event_update_requested: false,
            completed: true,
        })
    );
    assert_eq!(metadata.game_event_like_cpp(1).unwrap().next_start, 280);
}

#[test]
fn game_event_stop_normal_overwrite_removes_active_and_repairs_without_minutes_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event(
            1,
            GameEventStateLikeCpp::Normal,
            0,
            70,
            10,
            7,
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.stop_game_event_like_cpp(1, true, 500),
        GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: GameEventStateLikeCpp::Normal as u8,
            state_after_raw: GameEventStateLikeCpp::Normal as u8,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: false,
            condition_reset_requested: false,
            delete_world_event_state_requested: false,
            delete_condition_saves_requested: false,
        })
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.start, 80);
    assert_eq!(event.end, 87);
    assert!(
        !metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}

#[test]
fn game_event_stop_serverwide_non_finished_resets_and_reports_deletes_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_next_start(
            event(1, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 7),
            777,
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.stop_game_event_like_cpp(1, false, 500),
        GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            state_after_raw: GameEventStateLikeCpp::WorldInactive as u8,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: true,
            condition_reset_requested: true,
            delete_world_event_state_requested: true,
            delete_condition_saves_requested: true,
        })
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldInactive as u8);
    assert_eq!(event.next_start, 0);
}

#[test]
fn game_event_stop_world_finished_without_overwrite_keeps_state_but_unapplies_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_next_start(
            event(1, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 7),
            777,
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    assert_eq!(
        metadata.stop_game_event_like_cpp(1, false, 500),
        GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
            event_id: 1,
            state_before_raw: GameEventStateLikeCpp::WorldFinished as u8,
            state_after_raw: GameEventStateLikeCpp::WorldFinished as u8,
            active_removed: true,
            active_was_present: true,
            unapply_event_requested: true,
            serverwide: true,
            condition_reset_requested: false,
            delete_world_event_state_requested: false,
            delete_condition_saves_requested: false,
        })
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldFinished as u8);
    assert_eq!(event.next_start, 777);
    assert!(
        !metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}

#[test]
fn game_event_start_stop_missing_event_do_not_mutate_active_set_or_events_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event(
            1,
            GameEventStateLikeCpp::Normal,
            100,
            200,
            10,
            7,
        )]));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);
    let before = metadata.game_event_like_cpp(1).unwrap().clone();
    let active_before = metadata
        .game_event_active_set_like_cpp()
        .active_event_ids_like_cpp()
        .collect::<Vec<_>>();

    assert_eq!(
        metadata.start_game_event_like_cpp(99, true, 500, true),
        GameEventStartOutcomeLikeCpp::MissingEvent { event_id: 99 }
    );
    assert_eq!(
        metadata.stop_game_event_like_cpp(99, true, 500),
        GameEventStopOutcomeLikeCpp::MissingEvent { event_id: 99 }
    );
    assert_eq!(metadata.game_event_like_cpp(1).unwrap(), &before);
    assert_eq!(
        metadata
            .game_event_active_set_like_cpp()
            .active_event_ids_like_cpp()
            .collect::<Vec<_>>(),
        active_before
    );
}

#[test]
fn game_event_update_queues_starts_before_stops_sorted_and_updates_active_set_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store_with_max(
            3,
            [
                event(1, GameEventStateLikeCpp::Normal, 200, 1_000, 10, 2),
                event(2, GameEventStateLikeCpp::Normal, 0, 1_000, 10, 2),
                event(3, GameEventStateLikeCpp::Normal, 0, 1_000, 10, 2),
            ],
        ));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(3);
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(2);

    let outcome = metadata.update_game_events_like_cpp(250, true, |_| false);

    assert_eq!(outcome.scanned_event_ids, vec![1, 2, 3]);
    assert_eq!(outcome.queued_activation_event_ids, vec![1]);
    assert_eq!(outcome.queued_deactivation_event_ids, vec![2, 3]);
    assert!(matches!(
        outcome.start_outcomes.as_slice(),
        [GameEventStartOutcomeLikeCpp::Started(
            GameEventStartSummaryLikeCpp {
                event_id: 1,
                active_added: true,
                ..
            }
        )]
    ));
    assert!(matches!(
        outcome.stop_outcomes.as_slice(),
        [
            GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
                event_id: 2,
                active_removed: true,
                ..
            }),
            GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
                event_id: 3,
                active_removed: true,
                ..
            })
        ]
    ));
    assert_eq!(
        metadata
            .game_event_active_set_like_cpp()
            .active_event_ids_like_cpp()
            .collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn game_event_update_world_nextphase_finish_saves_stops_and_skips_nextcheck_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store_with_max(
            2,
            [
                event_with_next_start(
                    event(1, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 5),
                    500,
                ),
                event(2, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2),
            ],
        ));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    let outcome = metadata.update_game_events_like_cpp(500, true, |_| false);

    assert_eq!(
        outcome.world_nextphase_finished,
        vec![GameEventWorldNextPhaseFinishedLikeCpp {
            event_id: 1,
            was_active_before_queue: true,
            state_before_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            state_after_raw: GameEventStateLikeCpp::WorldFinished as u8,
            next_start_before: 500,
            next_start_after: 0,
            save_state_requested: true,
        }]
    );
    assert_eq!(outcome.queued_deactivation_event_ids, vec![1]);
    assert!(
        !outcome
            .next_check_outcomes
            .iter()
            .any(|(event_id, _)| *event_id == 1)
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldFinished as u8);
    assert_eq!(event.next_start, 0);
    assert!(
        !metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}

#[test]
fn game_event_update_inactive_not_active_records_negative_spawn_only_after_init_like_cpp() {
    for (is_system_init, expected_negative_spawns) in [(false, vec![-1]), (true, vec![])] {
        let mut metadata =
            CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
                .with_game_events_like_cpp(game_event_store_with_max(
                    1,
                    [event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2)],
                ));

        let outcome = metadata.update_game_events_like_cpp(650, is_system_init, |_| false);

        assert_eq!(outcome.negative_spawn_event_ids, expected_negative_spawns);
        assert!(outcome.queued_activation_event_ids.is_empty());
        assert!(outcome.queued_deactivation_event_ids.is_empty());
        assert!(
            metadata
                .game_event_active_set_like_cpp()
                .active_event_ids_like_cpp()
                .collect::<Vec<_>>()
                .is_empty()
        );
    }
}

#[test]
fn game_event_update_world_conditions_true_saves_starts_completed_and_forces_delay_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store_with_max(
            1,
            [event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 7)],
        ));

    let outcome = metadata.update_game_events_like_cpp(500, true, |event_id| event_id == 1);

    assert_eq!(
        outcome.world_conditions_save_requested,
        vec![GameEventWorldStateSaveEvidenceLikeCpp {
            event_id: 1,
            state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            next_start_after: 920,
        }]
    );
    assert_eq!(outcome.queued_activation_event_ids, vec![1]);
    assert!(matches!(
        outcome.start_outcomes.as_slice(),
        [GameEventStartOutcomeLikeCpp::Started(
            GameEventStartSummaryLikeCpp {
                event_id: 1,
                completed: true,
                save_world_event_state_requested: true,
                ..
            }
        )]
    ));
    assert_eq!(outcome.next_event_delay_secs_before_padding, 0);
    assert_eq!(outcome.next_update_delay_millis, 1_000);
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldNextPhase as u8);
    assert_eq!(event.next_start, 920);
    assert!(
        metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}

#[test]
fn game_event_update_invalid_zero_occurrence_surfaces_without_fake_start_or_stop_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store_with_max(
            1,
            [event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 0, 2)],
        ));

    let outcome = metadata.update_game_events_like_cpp(200, false, |_| false);

    assert_eq!(
        outcome.invalid_check_outcomes,
        vec![GameEventCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { event_id: 1 }]
    );
    assert!(outcome.start_outcomes.is_empty());
    assert!(outcome.stop_outcomes.is_empty());
    assert!(outcome.queued_activation_event_ids.is_empty());
    assert!(outcome.queued_deactivation_event_ids.is_empty());
    assert!(outcome.negative_spawn_event_ids.is_empty());
    assert_eq!(
        outcome.next_event_delay_secs_before_padding,
        MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP
    );
    assert_eq!(
        metadata
            .game_event_active_set_like_cpp()
            .active_event_ids_like_cpp()
            .collect::<Vec<_>>(),
        Vec::<u16>::new()
    );
}

#[test]
fn game_event_seasonal_last_start_time_normal_event_like_cpp() {
    let store = game_event_store([event(1, GameEventStateLikeCpp::Normal, 100, 2_000, 10, 2)]);

    assert_eq!(store.last_start_time_like_cpp(1, 1_350), 1_300);
}

#[test]
fn game_event_seasonal_last_start_time_non_normal_out_of_range_and_zero_occurrence_like_cpp() {
    let store = game_event_store_with_max(
        2,
        [
            event(1, GameEventStateLikeCpp::WorldInactive, 100, 2_000, 10, 2),
            event(2, GameEventStateLikeCpp::Normal, 100, 2_000, 0, 2),
        ],
    );

    assert_eq!(store.last_start_time_like_cpp(1, 1_350), 0);
    assert_eq!(store.last_start_time_like_cpp(3, 1_350), 0);
    assert_eq!(store.last_start_time_like_cpp(2, 1_350), 0);
}

#[test]
fn game_event_check_normal_window_and_strict_start_end_like_cpp() {
    let store = game_event_store([event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2)]);

    assert_eq!(
        store.check_one_game_event_like_cpp(1, 100),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 101),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 221),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 1_000),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
}

#[test]
fn game_event_check_unknown_raw_state_uses_normal_default_like_cpp() {
    let store = game_event_store([event_with_raw_state(1, 99, 100, 1_000, 10, 2)]);

    assert_eq!(
        store.check_one_game_event_like_cpp(1, 101),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 221),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
}

#[test]
fn game_event_check_world_state_branches_like_cpp() {
    let store = game_event_store([
        event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 0),
        event(2, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
        event(3, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 0),
        event(4, GameEventStateLikeCpp::Internal, 0, 0, 0, 0),
    ]);

    assert_eq!(
        store.check_one_game_event_like_cpp(1, 500),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(2, 500),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(3, 500),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(4, 500),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );
}

#[test]
fn game_event_check_inactive_prerequisites_like_cpp() {
    let base_events = [
        event(1, GameEventStateLikeCpp::WorldInactive, 0, 0, 0, 0),
        event_with_next_start(
            event(2, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
            400,
        ),
        event_with_next_start(
            event(3, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 0),
            500,
        ),
        event_with_next_start(
            event(4, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
            700,
        ),
        event(5, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2),
    ];
    let store = game_event_store(base_events.clone());

    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );

    let store = game_event_store([
        event_with_prerequisites(base_events[0].clone(), [2, 3]),
        base_events[1].clone(),
        base_events[2].clone(),
        base_events[3].clone(),
        base_events[4].clone(),
    ]);
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::Active(true)
    );

    let store = game_event_store([
        event_with_prerequisites(base_events[0].clone(), [5]),
        base_events[1].clone(),
        base_events[2].clone(),
        base_events[3].clone(),
        base_events[4].clone(),
    ]);
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );

    let store = game_event_store([
        event_with_prerequisites(base_events[0].clone(), [4]),
        base_events[1].clone(),
        base_events[2].clone(),
        base_events[3].clone(),
        base_events[4].clone(),
    ]);
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::Active(false)
    );

    let store = game_event_store([
        event_with_prerequisites(base_events[0].clone(), [9]),
        base_events[1].clone(),
        base_events[2].clone(),
        base_events[3].clone(),
        base_events[4].clone(),
    ]);
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 600),
        GameEventCheckOutcomeLikeCpp::MissingPrerequisite { event_id: 9 }
    );
}

#[test]
fn game_event_check_missing_and_zero_occurrence_are_explicit_like_cpp() {
    let store = game_event_store([event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 0, 2)]);

    assert_eq!(
        store.check_one_game_event_like_cpp(9, 500),
        GameEventCheckOutcomeLikeCpp::MissingEvent { event_id: 9 }
    );
    assert_eq!(
        store.check_one_game_event_like_cpp(1, 500),
        GameEventCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { event_id: 1 }
    );
}

#[test]
fn game_event_prerequisite_loader_accepts_world_events_dedupes_and_sorts_like_cpp() {
    let mut store = game_event_store([
        event(1, GameEventStateLikeCpp::WorldInactive, 0, 0, 0, 0),
        event(2, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
        event(3, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 0),
        event(4, GameEventStateLikeCpp::Normal, 0, 0, 0, 0),
        event(5, GameEventStateLikeCpp::Internal, 0, 0, 0, 0),
    ]);
    let mut report = GameEventPrerequisiteLoadReportLikeCpp::default();

    for row in [
        GameEventPrerequisiteRowLikeCpp {
            event_id: 1,
            prerequisite_event: 3,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 1,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 1,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 4,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 5,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 99,
            prerequisite_event: 2,
        },
        GameEventPrerequisiteRowLikeCpp {
            event_id: 1,
            prerequisite_event: 99,
        },
    ] {
        apply_game_event_prerequisite_row_like_cpp(row, &mut store, &mut report);
    }

    assert_eq!(report.rows, 7);
    assert_eq!(report.loaded, 2);
    assert_eq!(report.duplicate_ignored, 1);
    assert_eq!(report.skipped_non_world_event, 2);
    assert_eq!(report.skipped_out_of_range_event, 1);
    assert_eq!(report.skipped_out_of_range_prerequisite, 1);
    assert_eq!(
        store
            .prerequisite_events_like_cpp(1)
            .expect("test event exists")
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
}

#[test]
fn game_event_next_check_world_phase_and_conditions_like_cpp() {
    let store = game_event_store([
        event_with_next_start(
            event(1, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
            700,
        ),
        event_with_next_start(
            event(2, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 0),
            650,
        ),
        event(3, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 7),
        event(4, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 0),
    ]);

    assert_eq!(
        store.next_check_like_cpp(1, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(100)
    );
    assert_eq!(
        store.next_check_like_cpp(2, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(50)
    );
    assert_eq!(
        store.next_check_like_cpp(3, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(420)
    );
    assert_eq!(
        store.next_check_like_cpp(4, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP)
    );
}

#[test]
fn game_event_next_check_periodic_delays_and_end_clamp_like_cpp() {
    let store = game_event_store([
        event(1, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2),
        event(2, GameEventStateLikeCpp::Normal, 900, 1_000, 10, 2),
        event(3, GameEventStateLikeCpp::Normal, 100, 350, 10, 2),
        event(4, GameEventStateLikeCpp::Normal, 100, 500, 0, 2),
    ]);

    assert_eq!(
        store.next_check_like_cpp(1, 1_001),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP)
    );
    assert_eq!(
        store.next_check_like_cpp(2, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(300)
    );
    assert_eq!(
        store.next_check_like_cpp(1, 150),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(70)
    );
    assert_eq!(
        store.next_check_like_cpp(1, 221),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(479)
    );
    assert_eq!(
        store.next_check_like_cpp(3, 221),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(129)
    );
    assert_eq!(
        store.next_check_like_cpp(4, 150),
        GameEventNextCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { event_id: 4 }
    );
    assert_eq!(
        store.next_check_like_cpp(9, 150),
        GameEventNextCheckOutcomeLikeCpp::MissingEvent { event_id: 9 }
    );
}

#[test]
fn pool_mgr_loader_skip_order_missing_spawn_before_template_and_chance_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut spawn_report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let spawn = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut spawn_report,
    )
    .unwrap();
    store.add_object_spawn(&spawn, is_personal_phase_like_cpp_represented);
    let mut mgr = PoolMgrLikeCpp::new();
    let mut report = PoolMgrLoadReportLikeCpp::default();

    apply_pool_spawn_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 999,
            pool_spawn_id: 88,
            chance: 200.0,
        },
        &store,
        PoolMemberKindLikeCpp::Creature,
        &mut mgr,
        &mut report,
    );
    apply_pool_spawn_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 100,
            pool_spawn_id: 88,
            chance: 200.0,
        },
        &store,
        PoolMemberKindLikeCpp::Creature,
        &mut mgr,
        &mut report,
    );
    mgr.insert_template_like_cpp(88, PoolTemplateDataLikeCpp::new(1, -1));
    apply_pool_spawn_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 100,
            pool_spawn_id: 88,
            chance: 200.0,
        },
        &store,
        PoolMemberKindLikeCpp::Creature,
        &mut mgr,
        &mut report,
    );

    assert_eq!(report.creature_members.rows, 3);
    assert_eq!(report.creature_members.skipped_missing_spawn, 1);
    assert_eq!(report.creature_members.skipped_missing_template, 1);
    assert_eq!(report.creature_members.skipped_invalid_chance, 1);
    assert_eq!(report.creature_members.loaded, 0);
}

#[test]
fn pool_mgr_loader_map_propagation_mismatch_and_cycle_removal_like_cpp() {
    let mut propagated = PoolMgrLikeCpp::new();
    let mut report = PoolMgrLoadReportLikeCpp::default();
    propagated.insert_template_like_cpp(1, PoolTemplateDataLikeCpp::new(1, 571));
    propagated.insert_template_like_cpp(2, PoolTemplateDataLikeCpp::new(1, -1));
    apply_pool_pool_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 1,
            pool_spawn_id: 2,
            chance: 0.0,
        },
        &mut propagated,
        &mut report,
    );
    apply_pool_map_propagation_like_cpp(&mut propagated, &mut report);
    assert_eq!(propagated.templates.get(&2).unwrap().map_id, 571);
    assert_eq!(report.relation_removals, 0);

    let mut mismatch = PoolMgrLikeCpp::new();
    let mut mismatch_report = PoolMgrLoadReportLikeCpp::default();
    mismatch.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 1));
    mismatch.insert_template_like_cpp(20, PoolTemplateDataLikeCpp::new(1, 2));
    apply_pool_pool_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 10,
            pool_spawn_id: 20,
            chance: 0.0,
        },
        &mut mismatch,
        &mut mismatch_report,
    );
    apply_pool_map_propagation_like_cpp(&mut mismatch, &mut mismatch_report);
    assert!(!mismatch.child_pool_to_parent.contains_key(&10));
    assert_eq!(mismatch_report.map_mismatches, 1);
    assert_eq!(mismatch_report.relation_removals, 1);

    let mut cyclic = PoolMgrLikeCpp::new();
    let mut cycle_report = PoolMgrLoadReportLikeCpp::default();
    cyclic.insert_template_like_cpp(30, PoolTemplateDataLikeCpp::new(1, -1));
    cyclic.insert_template_like_cpp(31, PoolTemplateDataLikeCpp::new(1, -1));
    apply_pool_pool_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 31,
            pool_spawn_id: 30,
            chance: 0.0,
        },
        &mut cyclic,
        &mut cycle_report,
    );
    apply_pool_pool_member_row_like_cpp(
        PoolMemberRowLikeCpp {
            spawn_id: 30,
            pool_spawn_id: 31,
            chance: 0.0,
        },
        &mut cyclic,
        &mut cycle_report,
    );
    apply_pool_map_propagation_like_cpp(&mut cyclic, &mut cycle_report);
    assert_eq!(cycle_report.circular_relations, 1);
    assert_eq!(cycle_report.relation_removals, 1);
    assert_eq!(cyclic.child_pool_to_parent.len(), 1);
}

#[test]
fn pool_mgr_loader_autospawn_skips_empty_broken_and_child_like_cpp() {
    let mut mgr = PoolMgrLikeCpp::new();
    let mut report = PoolMgrLoadReportLikeCpp::default();
    mgr.insert_template_like_cpp(1, PoolTemplateDataLikeCpp::new(1, 0));
    mgr.insert_template_like_cpp(2, PoolTemplateDataLikeCpp::new(1, 0));
    mgr.insert_template_like_cpp(3, PoolTemplateDataLikeCpp::new(1, 0));
    mgr.insert_template_like_cpp(4, PoolTemplateDataLikeCpp::new(1, 0));
    let mut valid = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 1);
    valid.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 0.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 1, valid)
        .unwrap();
    let mut broken = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 3);
    broken.add_entry_like_cpp(PoolObjectLikeCpp::new(301, 50.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 3, broken)
        .unwrap();
    let mut child = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 4);
    child.add_entry_like_cpp(PoolObjectLikeCpp::new(401, 0.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 4, child)
        .unwrap();

    apply_pool_autospawn_candidate_row_like_cpp(
        PoolAutospawnCandidateRowLikeCpp {
            pool_entry: 1,
            child_pool_id: 0,
            mother_pool_id: 0,
        },
        &mut mgr,
        &mut report,
    );
    apply_pool_autospawn_candidate_row_like_cpp(
        PoolAutospawnCandidateRowLikeCpp {
            pool_entry: 2,
            child_pool_id: 0,
            mother_pool_id: 0,
        },
        &mut mgr,
        &mut report,
    );
    apply_pool_autospawn_candidate_row_like_cpp(
        PoolAutospawnCandidateRowLikeCpp {
            pool_entry: 3,
            child_pool_id: 0,
            mother_pool_id: 0,
        },
        &mut mgr,
        &mut report,
    );
    apply_pool_autospawn_candidate_row_like_cpp(
        PoolAutospawnCandidateRowLikeCpp {
            pool_entry: 4,
            child_pool_id: 4,
            mother_pool_id: 99,
        },
        &mut mgr,
        &mut report,
    );

    assert_eq!(report.autospawn_rows, 4);
    assert_eq!(report.autospawn_loaded, 1);
    assert_eq!(report.autospawn_skipped_empty, 1);
    assert_eq!(report.autospawn_skipped_broken, 1);
    assert_eq!(report.autospawn_skipped_child, 1);
    assert_eq!(mgr.auto_spawn_pools_for_map_like_cpp(0), &[1]);
}

fn game_event_data_row(
    event_id: u16,
    length: u32,
    state_raw: u8,
    holiday_id: u32,
) -> GameEventDataRowLikeCpp {
    GameEventDataRowLikeCpp {
        event_id,
        start: 100,
        end: 200,
        occurence: 30,
        length,
        holiday_id,
        holiday_stage: 2,
        description: format!("event-{event_id}"),
        state_raw,
        announce: 1,
    }
}

#[test]
fn game_event_data_store_uses_cpp_master_sizing_and_indexing() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(1, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );
    apply_game_event_data_row_like_cpp(
        game_event_data_row(3, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );

    assert_eq!(events.len_like_cpp(), 4);
    assert!(events.event_like_cpp(0).is_some());
    assert_eq!(
        events.event_like_cpp(1).map(|event| event.event_id),
        Some(1)
    );
    assert_eq!(
        events.event_like_cpp(3).map(|event| event.event_id),
        Some(3)
    );
    assert!(events.event_like_cpp(4).is_none());
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 2);
}

#[test]
fn game_event_data_reserved_zero_is_reported_and_not_loaded() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(0, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );

    let slot_zero = events.event_like_cpp(0).unwrap();
    assert_eq!(slot_zero.start, 1);
    assert_eq!(slot_zero.description, "");
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_reserved_zero, 1);
}

#[test]
fn game_event_data_preserves_cpp_field_order_and_next_start_zero() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        GameEventDataRowLikeCpp {
            event_id: 2,
            start: 1_700_000_001,
            end: 1_700_000_999,
            occurence: 120,
            length: 45,
            holiday_id: 341,
            holiday_stage: 3,
            description: "Darkmoon metadata".to_string(),
            state_raw: GameEventStateLikeCpp::WorldConditions as u8,
            announce: 2,
        },
        &mut events,
        &mut report,
    );

    let event = events.event_like_cpp(2).unwrap();
    assert_eq!(event.start, 1_700_000_001);
    assert_eq!(event.end, 1_700_000_999);
    assert_eq!(event.occurence, 120);
    assert_eq!(event.length, 45);
    assert_eq!(event.holiday_id, 341);
    assert_eq!(event.holiday_stage, 3);
    assert_eq!(event.description, "Darkmoon metadata");
    assert_eq!(
        event.state_raw,
        GameEventStateLikeCpp::WorldConditions as u8
    );
    assert_eq!(
        event.state_like_cpp(),
        Some(GameEventStateLikeCpp::WorldConditions)
    );
    assert_eq!(event.announce, 2);
    assert_eq!(event.next_start, 0);
    assert_eq!(report.loaded, 1);
}

#[test]
fn game_event_data_validity_matches_cpp_normal_zero_length_rule() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(1, 0, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );
    apply_game_event_data_row_like_cpp(
        game_event_data_row(2, 0, GameEventStateLikeCpp::WorldInactive as u8, 0),
        &mut events,
        &mut report,
    );
    apply_game_event_data_row_like_cpp(
        game_event_data_row(3, 0, GameEventStateLikeCpp::Internal as u8, 0),
        &mut events,
        &mut report,
    );

    assert!(!events.event_like_cpp(1).unwrap().is_valid_like_cpp());
    assert!(events.event_like_cpp(2).unwrap().is_valid_like_cpp());
    assert!(events.event_like_cpp(3).unwrap().is_valid_like_cpp());
    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 3);
    assert_eq!(report.invalid_normal_zero_length, 1);
}

#[test]
fn game_event_data_preserves_holiday_values_and_defers_db2_validation() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(1, 10, GameEventStateLikeCpp::Normal as u8, 777),
        &mut events,
        &mut report,
    );

    let event = events.event_like_cpp(1).unwrap();
    assert_eq!(event.holiday_id, 777);
    assert_eq!(event.holiday_stage, 2);
    assert_eq!(event.start, 100);
    assert_eq!(event.end, 200);
    assert_eq!(report.holiday_validation_deferred, 1);
    assert_eq!(report.loaded, 1);
}

#[test]
fn game_event_data_skip_out_of_range_without_truncation() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();

    apply_game_event_data_row_like_cpp(
        game_event_data_row(4, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );

    assert_eq!(events.len_like_cpp(), 4);
    assert!(events.event_like_cpp(4).is_none());
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_out_of_range, 1);
}

#[test]
fn canonical_metadata_exposes_game_event_master_metadata_like_cpp() {
    let mut events = GameEventDataStoreLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventDataLoadReportLikeCpp::default();
    apply_game_event_data_row_like_cpp(
        game_event_data_row(1, 10, GameEventStateLikeCpp::Normal as u8, 0),
        &mut events,
        &mut report,
    );
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_events_like_cpp(events);

    assert_eq!(metadata.game_events_like_cpp().len_like_cpp(), 4);
    assert_eq!(metadata.game_events_like_cpp().iter_like_cpp().count(), 4);
    assert_eq!(
        metadata.game_event_like_cpp(1).map(|event| event.length),
        Some(10)
    );
    assert!(metadata.game_event_like_cpp(4).is_none());
}

fn game_event_pool_mgr_with_test_pools() -> PoolMgrLikeCpp {
    let mut mgr = PoolMgrLikeCpp::new();
    for pool_id in [10, 11, 12, 13, 14] {
        mgr.insert_template_like_cpp(pool_id, PoolTemplateDataLikeCpp::new(1, 571));
        let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, pool_id);
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(u64::from(pool_id) * 100, 0.0), 1);
        mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, pool_id, group)
            .unwrap();
    }
    mgr.insert_template_like_cpp(99, PoolTemplateDataLikeCpp::new(1, 571));
    let mut broken = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 99);
    broken.add_entry_like_cpp(PoolObjectLikeCpp::new(9900, 50.0), 1);
    mgr.insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 99, broken)
        .unwrap();
    mgr
}

#[test]
fn game_event_pool_ids_preserve_order_and_signed_internal_index_like_cpp() {
    let mgr = game_event_pool_mgr_with_test_pools();
    let mut pools = GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventPoolLoadReportLikeCpp::default();

    for row in [
        GameEventPoolRowLikeCpp {
            pool_entry: 10,
            event_id: 1,
        },
        GameEventPoolRowLikeCpp {
            pool_entry: 11,
            event_id: -1,
        },
        GameEventPoolRowLikeCpp {
            pool_entry: 12,
            event_id: 1,
        },
        GameEventPoolRowLikeCpp {
            pool_entry: 13,
            event_id: -1,
        },
    ] {
        apply_game_event_pool_row_like_cpp(row, &mgr, &mut pools, &mut report);
    }

    assert_eq!(pools.game_event_size_like_cpp(), 4);
    assert_eq!(pools.internal_event_id_like_cpp(1), Some(4));
    assert_eq!(pools.internal_event_id_like_cpp(-1), Some(2));
    assert_eq!(pools.pool_ids_like_cpp(1), Some([10, 12].as_slice()));
    assert_eq!(pools.pool_ids_like_cpp(-1), Some([11, 13].as_slice()));
    assert_eq!(report.rows, 4);
    assert_eq!(report.loaded, 4);
}

#[test]
fn game_event_pool_ids_skip_out_of_range_without_panic_like_cpp() {
    let mgr = game_event_pool_mgr_with_test_pools();
    let mut pools = GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventPoolLoadReportLikeCpp::default();

    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 10,
            event_id: -5,
        },
        &mgr,
        &mut pools,
        &mut report,
    );
    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 11,
            event_id: 4,
        },
        &mgr,
        &mut pools,
        &mut report,
    );

    assert_eq!(pools.pool_ids_like_cpp(-5), None);
    assert_eq!(pools.pool_ids_like_cpp(4), None);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_out_of_range, 2);
}

#[test]
fn game_event_pool_ids_skip_broken_pool_but_keep_pool_mgr_metadata_like_cpp() {
    let mgr = game_event_pool_mgr_with_test_pools();
    let mut pools = GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventPoolLoadReportLikeCpp::default();

    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 99,
            event_id: 1,
        },
        &mgr,
        &mut pools,
        &mut report,
    );
    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 404,
            event_id: 1,
        },
        &mgr,
        &mut pools,
        &mut report,
    );
    apply_game_event_pool_row_like_cpp(
        GameEventPoolRowLikeCpp {
            pool_entry: 10,
            event_id: 1,
        },
        &mgr,
        &mut pools,
        &mut report,
    );

    assert!(mgr.templates.contains_key(&99));
    assert!(!mgr.check_pool_like_cpp(99));
    assert_eq!(pools.pool_ids_like_cpp(1), Some([10].as_slice()));
    assert_eq!(report.rows, 3);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_broken_pool, 2);
}

fn game_event_guid_test_spawn(
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
    pool_id: u32,
) -> SpawnData {
    SpawnData {
        object_type,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::legacy_group(),
        id: u32::try_from(spawn_id).unwrap_or(u32::MAX),
        spawn_point: SpawnPosition::new(1.0, 2.0, 3.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id,
        spawn_time_secs: 120,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    }
}

fn game_event_guid_test_store() -> SpawnStore {
    let mut store = SpawnStore::new();
    for spawn in [
        game_event_guid_test_spawn(SpawnObjectType::Creature, 100, 0),
        game_event_guid_test_spawn(SpawnObjectType::Creature, 101, 88),
        game_event_guid_test_spawn(SpawnObjectType::Creature, 102, 0),
        game_event_guid_test_spawn(SpawnObjectType::GameObject, 200, 0),
        game_event_guid_test_spawn(SpawnObjectType::GameObject, 201, 89),
        game_event_guid_test_spawn(SpawnObjectType::GameObject, 202, 0),
    ] {
        store.insert_spawn_metadata_like_cpp(&spawn);
    }
    store
}

#[test]
fn game_event_spawn_guids_signed_internal_mapping_and_empty_valid_slice_like_cpp() {
    let guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));

    assert_eq!(guids.game_event_size_like_cpp(), 4);
    assert_eq!(guids.internal_event_id_like_cpp(1), Some(4));
    assert_eq!(guids.internal_event_id_like_cpp(-1), Some(2));
    assert_eq!(guids.internal_event_id_like_cpp(-5), None);
    assert_eq!(guids.internal_event_id_like_cpp(4), None);
    assert_eq!(guids.creature_guids_like_cpp(2), Some([].as_slice()));
    assert_eq!(guids.gameobject_guids_like_cpp(-2), Some([].as_slice()));
    assert_eq!(guids.creature_guids_like_cpp(4), None);
}

#[test]
fn game_event_spawn_guids_preserve_creature_and_gameobject_order_like_cpp() {
    let store = game_event_guid_test_store();
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut creature_report = GameEventObjectGuidLoadReportLikeCpp::default();
    let mut gameobject_report = GameEventObjectGuidLoadReportLikeCpp::default();

    for row in [
        GameEventObjectGuidRowLikeCpp {
            guid: 100,
            event_id: 1,
        },
        GameEventObjectGuidRowLikeCpp {
            guid: 102,
            event_id: 1,
        },
    ] {
        apply_game_event_object_guid_row_like_cpp(
            row,
            SpawnObjectType::Creature,
            &store,
            &mut guids,
            &mut creature_report,
        );
    }
    for row in [
        GameEventObjectGuidRowLikeCpp {
            guid: 200,
            event_id: -1,
        },
        GameEventObjectGuidRowLikeCpp {
            guid: 202,
            event_id: -1,
        },
    ] {
        apply_game_event_object_guid_row_like_cpp(
            row,
            SpawnObjectType::GameObject,
            &store,
            &mut guids,
            &mut gameobject_report,
        );
    }

    assert_eq!(
        guids.creature_guids_like_cpp(1),
        Some([100, 102].as_slice())
    );
    assert_eq!(
        guids.gameobject_guids_like_cpp(-1),
        Some([200, 202].as_slice())
    );
    assert_eq!(creature_report.rows, 2);
    assert_eq!(creature_report.loaded, 2);
    assert_eq!(gameobject_report.rows, 2);
    assert_eq!(gameobject_report.loaded, 2);
}

#[test]
fn game_event_spawn_guids_skip_missing_spawn_metadata_like_cpp() {
    let store = game_event_guid_test_store();
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventObjectGuidLoadReportLikeCpp::default();

    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 404,
            event_id: 1,
        },
        SpawnObjectType::Creature,
        &store,
        &mut guids,
        &mut report,
    );

    assert_eq!(guids.creature_guids_like_cpp(1), Some([].as_slice()));
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_missing_spawn_metadata, 1);
}

#[test]
fn game_event_spawn_guids_count_pooled_but_still_load_like_cpp() {
    let store = game_event_guid_test_store();
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut creature_report = GameEventObjectGuidLoadReportLikeCpp::default();
    let mut gameobject_report = GameEventObjectGuidLoadReportLikeCpp::default();

    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 101,
            event_id: 1,
        },
        SpawnObjectType::Creature,
        &store,
        &mut guids,
        &mut creature_report,
    );
    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 201,
            event_id: -1,
        },
        SpawnObjectType::GameObject,
        &store,
        &mut guids,
        &mut gameobject_report,
    );

    assert_eq!(guids.creature_guids_like_cpp(1), Some([101].as_slice()));
    assert_eq!(guids.gameobject_guids_like_cpp(-1), Some([201].as_slice()));
    assert_eq!(creature_report.loaded, 1);
    assert_eq!(creature_report.pooled_still_loaded, 1);
    assert_eq!(gameobject_report.loaded, 1);
    assert_eq!(gameobject_report.pooled_still_loaded, 1);
}

#[test]
fn game_event_spawn_guids_skip_out_of_range_like_cpp() {
    let store = game_event_guid_test_store();
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = GameEventObjectGuidLoadReportLikeCpp::default();

    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 100,
            event_id: -5,
        },
        SpawnObjectType::Creature,
        &store,
        &mut guids,
        &mut report,
    );
    apply_game_event_object_guid_row_like_cpp(
        GameEventObjectGuidRowLikeCpp {
            guid: 102,
            event_id: 4,
        },
        SpawnObjectType::Creature,
        &store,
        &mut guids,
        &mut report,
    );

    assert_eq!(guids.creature_guids_like_cpp(-5), None);
    assert_eq!(guids.creature_guids_like_cpp(4), None);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_out_of_range, 2);
}

#[test]
fn game_event_model_equip_accepts_zero_equipment_and_preserves_order_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let equipment_templates = BTreeSet::new();
    let mut report = GameEventModelEquipLoadReportLikeCpp::default();

    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 100,
            entry: 10,
            event_id: 1,
            model_id: 111,
            equipment_id: 0,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );
    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 101,
            entry: 11,
            event_id: 1,
            model_id: 112,
            equipment_id: 0,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );

    let records = model_equip.records_like_cpp(1).expect("event 1 exists");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].spawn_id, 100);
    assert_eq!(records[0].model_id, 111);
    assert_eq!(records[0].model_id_prev, 0);
    assert_eq!(records[0].equipment_id, 0);
    assert_eq!(records[0].equipment_id_prev, 0);
    assert_eq!(records[1].spawn_id, 101);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 2);
    assert_eq!(report.missing_equipment_template, 0);
}

#[test]
fn game_event_model_equip_skips_out_of_range_event_id_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let equipment_templates = BTreeSet::new();
    let mut report = GameEventModelEquipLoadReportLikeCpp::default();

    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 100,
            entry: 10,
            event_id: 4,
            model_id: 111,
            equipment_id: 0,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );

    assert_eq!(model_equip.records_like_cpp(4), None);
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.invalid_event_id, 1);
}

#[test]
fn game_event_model_equip_skips_missing_positive_equipment_template_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let equipment_templates = BTreeSet::from([(10_u32, 2_u8)]);
    let mut report = GameEventModelEquipLoadReportLikeCpp::default();

    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 100,
            entry: 10,
            event_id: 1,
            model_id: 111,
            equipment_id: 1,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );

    assert_eq!(model_equip.records_like_cpp(1), Some([].as_slice()));
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.missing_equipment_template, 1);
}

#[test]
fn game_event_model_equip_accepts_existing_positive_equipment_template_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let equipment_templates = BTreeSet::from([(10_u32, 1_u8)]);
    let mut report = GameEventModelEquipLoadReportLikeCpp::default();

    apply_game_event_model_equip_row_like_cpp(
        GameEventModelEquipRowLikeCpp {
            spawn_id: 100,
            entry: 10,
            event_id: 1,
            model_id: 111,
            equipment_id: 1,
        },
        &equipment_templates,
        &mut model_equip,
        &mut report,
    );

    let records = model_equip.records_like_cpp(1).expect("event 1 exists");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].spawn_id, 100);
    assert_eq!(records[0].equipment_id, 1);
    assert_eq!(records[0].equipment_id_prev, 0);
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.missing_equipment_template, 0);
}

#[test]
fn canonical_metadata_exposes_game_event_model_equip_slices_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 111,
            model_id_prev: 0,
            equipment_id: 0,
            equipment_id_prev: 0,
        },
    ));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip);

    let records = metadata
        .game_event_model_equip_like_cpp(1)
        .expect("event 1 exists");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].spawn_id, 100);
    assert_eq!(metadata.game_event_model_equip_like_cpp(4), None);
}

#[test]
fn game_event_npc_flag_loader_preserves_order_skips_range_and_u64_like_cpp() {
    let mut npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventNpcFlagLoadReportLikeCpp::default();

    for row in [
        GameEventNpcFlagRowLikeCpp {
            spawn_id: 100,
            event_id: 1,
            npcflag: 0x1_0000_0002,
        },
        GameEventNpcFlagRowLikeCpp {
            spawn_id: 101,
            event_id: 1,
            npcflag: 0x4,
        },
        GameEventNpcFlagRowLikeCpp {
            spawn_id: 200,
            event_id: 3,
            npcflag: 0x8,
        },
        GameEventNpcFlagRowLikeCpp {
            spawn_id: 102,
            event_id: 2,
            npcflag: 0x10,
        },
    ] {
        apply_game_event_npc_flag_row_like_cpp(row, &mut npc_flags, &mut report);
    }
    report.events_touched = npc_flags
        .records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();

    let event_one = npc_flags
        .records_like_cpp(1)
        .expect("event 1 bucket exists");
    assert_eq!(event_one.len(), 2);
    assert_eq!(event_one[0].spawn_id, 100);
    assert_eq!(event_one[0].npcflag, 0x1_0000_0002);
    assert_eq!(event_one[1].spawn_id, 101);
    assert_eq!(event_one[1].npcflag, 0x4);
    assert_eq!(npc_flags.records_like_cpp(2).unwrap()[0].spawn_id, 102);
    assert_eq!(npc_flags.records_like_cpp(3), None);
    assert_eq!(report.rows, 4);
    assert_eq!(report.loaded, 3);
    assert_eq!(report.skipped_out_of_range, 1);
    assert_eq!(report.events_touched, 2);
}

#[test]
fn game_event_npc_flag_get_npc_flag_or_over_active_events_like_cpp() {
    let mut npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(npc_flags.push_record_like_cpp(
        1,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0x1,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        2,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0x1_0000_0002,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        2,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 101,
            npcflag: 0x80,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        3,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0x4,
        },
    ));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_npc_flags_like_cpp(npc_flags);

    assert_eq!(
        metadata.game_event_npc_flag_mask_like_cpp(100, &[1, 2, 99]),
        0x1_0000_0003
    );
    assert_eq!(
        metadata.game_event_npc_flag_mask_like_cpp(101, &[1, 2]),
        0x80
    );
    assert_eq!(metadata.game_event_npc_flag_mask_like_cpp(100, &[3]), 0x4);
    assert_eq!(metadata.game_event_npc_flag_mask_like_cpp(100, &[]), 0);
    assert_eq!(metadata.game_event_npc_flag_mask_like_cpp(999, &[1, 2]), 0);
}

fn game_event_quest_row(
    event_id: u8,
    giver_id: u32,
    quest_id: u32,
) -> GameEventQuestRelationRowLikeCpp {
    GameEventQuestRelationRowLikeCpp {
        event_id,
        giver_id,
        quest_id,
    }
}

fn game_event_quest_row_from_raw_event_entry_get_uint8_like_cpp(
    raw_event_entry: u16,
    giver_id: u32,
    quest_id: u32,
) -> GameEventQuestRelationRowLikeCpp {
    game_event_quest_row(raw_event_entry as u8, giver_id, quest_id)
}

#[test]
fn game_event_quest_sizing_accessors_and_out_of_range_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventQuestRelationFamilyLoadReportLikeCpp::default();

    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(2, 100, 7000),
        &mut quests,
        &mut report,
    );
    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(3, 101, 7001),
        &mut quests,
        &mut report,
    );

    assert_eq!(quests.creature_records_like_cpp(0).unwrap(), &[]);
    assert_eq!(quests.creature_records_like_cpp(1).unwrap(), &[]);
    assert_eq!(
        quests.creature_records_like_cpp(2).unwrap()[0].quest_id,
        7000
    );
    assert_eq!(quests.creature_records_like_cpp(3), None);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_out_of_range, 1);
}

#[test]
fn game_event_quest_creature_preserves_order_duplicates_and_get_uint8_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventQuestRelationFamilyLoadReportLikeCpp::default();

    for row in [
        game_event_quest_row(2, 100, 7000),
        game_event_quest_row(2, 100, 7000),
        game_event_quest_row(2, 101, 7001),
        game_event_quest_row_from_raw_event_entry_get_uint8_like_cpp(258, 102, 7002),
    ] {
        apply_game_event_creature_quest_relation_row_like_cpp(row, &mut quests, &mut report);
    }

    let records = quests.creature_records_like_cpp(2).unwrap();
    assert_eq!(
        records,
        &[
            GameEventQuestRelationRecordLikeCpp {
                giver_id: 100,
                quest_id: 7000,
            },
            GameEventQuestRelationRecordLikeCpp {
                giver_id: 100,
                quest_id: 7000,
            },
            GameEventQuestRelationRecordLikeCpp {
                giver_id: 101,
                quest_id: 7001,
            },
            GameEventQuestRelationRecordLikeCpp {
                giver_id: 102,
                quest_id: 7002,
            },
        ]
    );
    assert_eq!(report.loaded, 4);
    assert_eq!(report.skipped_out_of_range, 0);
}

#[test]
fn game_event_quest_gameobject_preserves_order_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventQuestRelationFamilyLoadReportLikeCpp::default();

    apply_game_event_gameobject_quest_relation_row_like_cpp(
        game_event_quest_row(1, 200, 8000),
        &mut quests,
        &mut report,
    );
    apply_game_event_gameobject_quest_relation_row_like_cpp(
        game_event_quest_row(1, 201, 8001),
        &mut quests,
        &mut report,
    );

    let records = quests.gameobject_records_like_cpp(1).unwrap();
    assert_eq!(records[0].giver_id, 200);
    assert_eq!(records[0].quest_id, 8000);
    assert_eq!(records[1].giver_id, 201);
    assert_eq!(records[1].quest_id, 8001);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 2);
}

#[test]
fn game_event_quest_valid_event_accepts_high_quest_id_no_template_validation_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventQuestRelationFamilyLoadReportLikeCpp::default();

    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(1, 100, u32::MAX),
        &mut quests,
        &mut report,
    );

    let records = quests.creature_records_like_cpp(1).unwrap();
    assert_eq!(records[0].quest_id, u32::MAX);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_out_of_range, 0);
}

#[test]
fn game_event_quest_relation_events_touched_counts_non_empty_buckets_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    let mut report = CanonicalSpawnStoreLoadReport::default();

    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(1, 100, 7000),
        &mut quests,
        &mut report.game_event_quest_relations.creature,
    );
    apply_game_event_creature_quest_relation_row_like_cpp(
        game_event_quest_row(3, 101, 7001),
        &mut quests,
        &mut report.game_event_quest_relations.creature,
    );
    apply_game_event_gameobject_quest_relation_row_like_cpp(
        game_event_quest_row(2, 200, 8000),
        &mut quests,
        &mut report.game_event_quest_relations.gameobject,
    );

    report.game_event_quest_relations.creature.events_touched = quests
        .creature_records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();
    report.game_event_quest_relations.gameobject.events_touched = quests
        .gameobject_records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();

    assert_eq!(report.game_event_quest_relations.creature.events_touched, 2);
    assert_eq!(
        report.game_event_quest_relations.gameobject.events_touched,
        1
    );
}

#[test]
fn game_event_quest_canonical_metadata_accessors_expose_both_families_like_cpp() {
    let mut quests = GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    assert!(quests.push_creature_record_like_cpp(
        1,
        GameEventQuestRelationRecordLikeCpp {
            giver_id: 100,
            quest_id: 7000,
        },
    ));
    assert!(quests.push_gameobject_record_like_cpp(
        1,
        GameEventQuestRelationRecordLikeCpp {
            giver_id: 200,
            quest_id: 8000,
        },
    ));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_quest_relations_like_cpp(quests);

    assert_eq!(
        metadata.game_event_creature_quests_like_cpp(1).unwrap()[0].giver_id,
        100
    );
    assert_eq!(
        metadata.game_event_gameobject_quests_like_cpp(1).unwrap()[0].giver_id,
        200
    );
    assert_eq!(metadata.game_event_creature_quests_like_cpp(2), None);
    assert_eq!(metadata.game_event_gameobject_quests_like_cpp(2), None);
}

fn game_event_quest_relation_record(
    giver_id: u32,
    quest_id: u32,
) -> GameEventQuestRelationRecordLikeCpp {
    GameEventQuestRelationRecordLikeCpp { giver_id, quest_id }
}

fn game_event_quest_cache_metadata_like_cpp(
    max_event_entry: u32,
    creature_records: &[(u16, u32, u32)],
    gameobject_records: &[(u16, u32, u32)],
) -> CanonicalSpawnMetadataLikeCpp {
    let mut quests =
        GameEventQuestRelationsLikeCpp::from_game_event_max_entry_like_cpp(Some(max_event_entry));
    for (event_id, giver_id, quest_id) in creature_records {
        assert!(quests.push_creature_record_like_cpp(
            *event_id,
            game_event_quest_relation_record(*giver_id, *quest_id),
        ));
    }
    for (event_id, giver_id, quest_id) in gameobject_records {
        assert!(quests.push_gameobject_record_like_cpp(
            *event_id,
            game_event_quest_relation_record(*giver_id, *quest_id),
        ));
    }
    CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_quest_relations_like_cpp(quests)
}

#[test]
fn game_event_quest_activation_inserts_active_relations_and_duplicates_like_cpp() {
    let mut metadata = game_event_quest_cache_metadata_like_cpp(
        1,
        &[(1, 100, 7000), (1, 100, 7000)],
        &[(1, 200, 8000), (1, 200, 8001)],
    );

    let summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, true);

    assert_eq!(summary.creature_records_seen, 2);
    assert_eq!(summary.gameobject_records_seen, 2);
    assert_eq!(summary.creature_inserted, 2);
    assert_eq!(summary.gameobject_inserted, 2);
    assert_eq!(
        metadata.game_event_active_creature_quest_relations_like_cpp(100),
        &[
            game_event_quest_relation_record(100, 7000),
            game_event_quest_relation_record(100, 7000),
        ]
    );
    assert_eq!(
        metadata
            .game_event_active_gameobject_quest_relations_like_cpp(200)
            .iter()
            .map(|record| record.quest_id)
            .collect::<Vec<_>>(),
        vec![8000, 8001]
    );
}

#[test]
fn game_event_quest_deactivation_removes_first_matching_relation_like_cpp() {
    let mut metadata =
        game_event_quest_cache_metadata_like_cpp(1, &[(1, 100, 7000)], &[(1, 200, 8000)]);
    metadata.update_game_event_quest_relation_cache_like_cpp(1, true);
    metadata.update_game_event_quest_relation_cache_like_cpp(1, true);

    let summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, false);

    assert_eq!(summary.creature_removed, 1);
    assert_eq!(summary.gameobject_removed, 1);
    assert_eq!(
        metadata.game_event_active_creature_quest_relations_like_cpp(100),
        &[game_event_quest_relation_record(100, 7000)]
    );
    assert_eq!(
        metadata.game_event_active_gameobject_quest_relations_like_cpp(200),
        &[game_event_quest_relation_record(200, 8000)]
    );
}

#[test]
fn game_event_quest_deactivation_skips_when_other_active_event_has_same_quest_like_cpp() {
    let mut metadata = game_event_quest_cache_metadata_like_cpp(
        2,
        &[(1, 100, 7000), (2, 101, 7000)],
        &[(1, 200, 8000), (2, 201, 8000)],
    );
    metadata.update_game_event_quest_relation_cache_like_cpp(1, true);
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(2);

    let summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, false);

    assert_eq!(summary.creature_skipped_active_other_event, 1);
    assert_eq!(summary.gameobject_skipped_active_other_event, 1);
    assert_eq!(summary.creature_removed, 0);
    assert_eq!(summary.gameobject_removed, 0);
    assert_eq!(
        metadata.game_event_active_creature_quest_relations_like_cpp(100),
        &[game_event_quest_relation_record(100, 7000)]
    );
    assert_eq!(
        metadata.game_event_active_gameobject_quest_relations_like_cpp(200),
        &[game_event_quest_relation_record(200, 8000)]
    );
}

#[test]
fn game_event_quest_deactivation_remove_miss_and_missing_bucket_are_no_panic_like_cpp() {
    let mut metadata =
        game_event_quest_cache_metadata_like_cpp(1, &[(1, 100, 7000)], &[(1, 200, 8000)]);

    let miss_summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, false);
    assert_eq!(miss_summary.creature_remove_misses, 1);
    assert_eq!(miss_summary.gameobject_remove_misses, 1);

    metadata.update_game_event_quest_relation_cache_like_cpp(1, true);
    metadata.update_game_event_quest_relation_cache_like_cpp(1, false);
    let no_match_summary = metadata.update_game_event_quest_relation_cache_like_cpp(1, false);
    assert_eq!(no_match_summary.creature_remove_misses, 1);
    assert_eq!(no_match_summary.gameobject_remove_misses, 1);

    let mut no_match_metadata = game_event_quest_cache_metadata_like_cpp(
        2,
        &[(1, 100, 7000), (2, 100, 9000)],
        &[(1, 200, 8000), (2, 200, 9000)],
    );
    no_match_metadata.update_game_event_quest_relation_cache_like_cpp(2, true);
    let no_match_summary =
        no_match_metadata.update_game_event_quest_relation_cache_like_cpp(1, false);
    assert_eq!(no_match_summary.creature_no_match, 1);
    assert_eq!(no_match_summary.gameobject_no_match, 1);

    let missing_summary = metadata.update_game_event_quest_relation_cache_like_cpp(2, false);
    assert!(missing_summary.creature_missing_event_bucket);
    assert!(missing_summary.gameobject_missing_event_bucket);
}

fn game_event_npc_vendor_store(spawns: &[(SpawnId, u32)]) -> SpawnStore {
    let maps = map_store(&[1]);
    let map_difficulties = map_difficulty_store(&[(1, DIFFICULTY_NONE_LIKE_CPP)]);
    let mut store = SpawnStore::new();
    for (spawn_id, entry) in spawns {
        let mut row = creature_row(*spawn_id, 0, "0");
        row.entry = *entry;
        let mut report = SpawnKindLoadReport::default();
        let spawn =
            creature_row_to_spawn_data_like_cpp(&row, &maps, &map_difficulties, &mut report)
                .expect("valid test creature spawn");
        store.add_object_spawn(&spawn, |_| false);
    }
    store
}

fn game_event_npc_vendor_row(
    event_id: u8,
    spawn_id: SpawnId,
    item: u32,
) -> GameEventNpcVendorRowLikeCpp {
    GameEventNpcVendorRowLikeCpp {
        event_id,
        spawn_id,
        item,
        maxcount: 7,
        incrtime: 30,
        extended_cost: 11,
        vendor_type: 2,
        bonus_list_ids: String::new(),
        player_condition_id: 13,
        ignore_filtering: true,
    }
}

fn game_event_npc_vendor_row_from_raw_event_entry_get_uint8_like_cpp(
    raw_event_entry: u16,
    spawn_id: SpawnId,
    item: u32,
) -> GameEventNpcVendorRowLikeCpp {
    game_event_npc_vendor_row(raw_event_entry as u8, spawn_id, item)
}

#[test]
fn game_event_npc_vendor_sizing_records_and_out_of_range_like_cpp() {
    let store = game_event_npc_vendor_store(&[(100, 9001)]);
    let npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(2, 100, 6000),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );
    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(3, 100, 6001),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );

    assert_eq!(vendors.records_like_cpp(0).unwrap(), &[]);
    assert_eq!(vendors.records_like_cpp(1).unwrap(), &[]);
    assert_eq!(vendors.records_like_cpp(2).unwrap()[0].item, 6000);
    assert_eq!(vendors.records_like_cpp(3), None);
    assert_eq!(report.rows, 2);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_out_of_range, 1);
    assert_eq!(report.validation_deferred, 1);
}

#[test]
fn game_event_npc_vendor_event_entry_uses_get_uint8_truncation_like_cpp() {
    let store = game_event_npc_vendor_store(&[(100, 9001)]);
    let npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row_from_raw_event_entry_get_uint8_like_cpp(258, 100, 6000),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );

    assert_eq!(vendors.records_like_cpp(2).unwrap()[0].item, 6000);
    assert_eq!(vendors.records_like_cpp(258), None);
    assert_eq!(report.rows, 1);
    assert_eq!(report.loaded, 1);
    assert_eq!(report.skipped_out_of_range, 0);
}

#[test]
fn game_event_npc_vendor_preserves_order_and_lookup_by_entry_like_cpp() {
    let store = game_event_npc_vendor_store(&[(100, 9001), (101, 9001), (102, 9002)]);
    let npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    for row in [
        game_event_npc_vendor_row(1, 100, 6000),
        game_event_npc_vendor_row(1, 101, 6001),
        game_event_npc_vendor_row(1, 102, 6002),
    ] {
        apply_game_event_npc_vendor_row_like_cpp(
            row,
            &store,
            &npc_flags,
            &mut vendors,
            &mut report,
        );
    }

    let records = vendors.records_like_cpp(1).unwrap();
    assert_eq!(
        records.iter().map(|record| record.item).collect::<Vec<_>>(),
        vec![6000, 6001, 6002]
    );
    let entry_records = vendors.records_for_entry_like_cpp(1, 9001).unwrap();
    assert_eq!(entry_records.len(), 2);
    assert_eq!(entry_records[0].spawn_id, 100);
    assert_eq!(entry_records[1].spawn_id, 101);
}

#[test]
fn game_event_npc_vendor_missing_creature_metadata_skips_no_dummy_like_cpp() {
    let store = game_event_npc_vendor_store(&[]);
    let npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(1, 404, 6000),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );

    assert_eq!(vendors.records_like_cpp(1).unwrap(), &[]);
    assert_eq!(report.loaded, 0);
    assert_eq!(report.skipped_missing_creature_spawn_metadata, 1);
    assert_eq!(report.validation_deferred, 0);
}

#[test]
fn game_event_npc_vendor_event_npc_flag_first_match_low32_or_zero_like_cpp() {
    let store = game_event_npc_vendor_store(&[(100, 9001), (101, 9002)]);
    let mut npc_flags = GameEventNpcFlagsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    assert!(npc_flags.push_record_like_cpp(
        1,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0x1_0000_00AA,
        },
    ));
    assert!(npc_flags.push_record_like_cpp(
        1,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: 100,
            npcflag: 0xBB,
        },
    ));
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    let mut report = GameEventNpcVendorLoadReportLikeCpp::default();

    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(1, 100, 6000),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );
    apply_game_event_npc_vendor_row_like_cpp(
        game_event_npc_vendor_row(1, 101, 6001),
        &store,
        &npc_flags,
        &mut vendors,
        &mut report,
    );

    let records = vendors.records_like_cpp(1).unwrap();
    assert_eq!(records[0].event_npc_flag_low32, 0xAA);
    assert_eq!(records[1].event_npc_flag_low32, 0);
}

#[test]
fn game_event_npc_vendor_bonus_list_ids_parse_like_cpp() {
    assert_eq!(
        parse_game_event_npc_vendor_bonus_list_ids_like_cpp("7 bad -9 7 0x10 12"),
        vec![7, -9, 7, 12]
    );
}

#[test]
fn game_event_npc_vendor_metadata_accessor_like_cpp() {
    let mut vendors = GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(1));
    assert!(
        vendors.push_record_like_cpp(1, game_event_npc_vendor_record_like_cpp(100, 9001, 6000, 2),)
    );
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_npc_vendors_like_cpp(vendors);

    assert_eq!(
        metadata.game_event_npc_vendors_like_cpp(1).unwrap().len(),
        1
    );
    assert_eq!(
        metadata
            .game_event_npc_vendor_records_for_entry_like_cpp(1, 9001)
            .unwrap()[0]
            .item,
        6000
    );
    assert_eq!(metadata.game_event_npc_vendors_like_cpp(2), None);
}

fn game_event_npc_vendor_record_like_cpp(
    spawn_id: SpawnId,
    entry: u32,
    item: u32,
    vendor_type: u8,
) -> GameEventNpcVendorRecordLikeCpp {
    GameEventNpcVendorRecordLikeCpp {
        spawn_id,
        guid: spawn_id,
        entry,
        item,
        maxcount: 7,
        incrtime: 30,
        extended_cost: 11,
        vendor_type,
        item_type: vendor_type,
        bonus_list_ids: vec![1, -2],
        player_condition_id: 13,
        ignore_filtering: true,
        event_npc_flag_low32: 0xAA,
    }
}

fn game_event_npc_vendor_metadata_with_records_like_cpp(
    max_event_entry: u32,
    records: &[(u16, SpawnId, u32, u32, u8)],
) -> CanonicalSpawnMetadataLikeCpp {
    let mut vendors =
        GameEventNpcVendorsLikeCpp::from_game_event_max_entry_like_cpp(Some(max_event_entry));
    for (event_id, spawn_id, entry, item, vendor_type) in records {
        assert!(vendors.push_record_like_cpp(
            *event_id,
            game_event_npc_vendor_record_like_cpp(*spawn_id, *entry, *item, *vendor_type),
        ));
    }
    CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_npc_vendors_like_cpp(vendors)
}

#[test]
fn game_event_npc_vendor_cache_activate_appends_without_dedupe_like_cpp() {
    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        1,
        &[
            (1, 100, 9001, 6000, 2),
            (1, 101, 9001, 6000, 2),
            (1, 102, 9001, 6001, 2),
        ],
    );

    let first = metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);
    let second = metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);

    assert_eq!(first.records_seen, 3);
    assert_eq!(first.items_added, 3);
    assert_eq!(second.records_seen, 3);
    assert_eq!(second.items_added, 3);
    assert_eq!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .iter()
            .map(|record| record.item)
            .collect::<Vec<_>>(),
        vec![6000, 6000, 6001, 6000, 6000, 6001]
    );
}

#[test]
fn game_event_npc_vendor_cache_deactivate_removes_all_item_type_matches_like_cpp() {
    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        2,
        &[
            (1, 100, 9001, 6000, 2),
            (1, 101, 9001, 6000, 2),
            (1, 102, 9001, 6000, 3),
            (2, 200, 9001, 6000, 2),
        ],
    );
    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);
    metadata.update_game_event_npc_vendor_cache_like_cpp(2, true);

    let summary = metadata.update_game_event_npc_vendor_cache_like_cpp(2, false);

    assert_eq!(summary.records_seen, 1);
    assert_eq!(summary.items_removed, 3);
    assert_eq!(summary.no_match, 0);
    assert_eq!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .iter()
            .map(|record| (record.item, record.vendor_type))
            .collect::<Vec<_>>(),
        vec![(6000, 3)]
    );
}

#[test]
fn game_event_npc_vendor_cache_deactivate_miss_and_no_match_no_panic_like_cpp() {
    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        2,
        &[(1, 100, 9001, 6000, 2), (2, 200, 9002, 6001, 2)],
    );
    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);

    let summary = metadata.update_game_event_npc_vendor_cache_like_cpp(2, false);

    assert_eq!(summary.records_seen, 1);
    assert_eq!(summary.remove_misses, 1);
    assert_eq!(summary.items_removed, 0);
    assert_eq!(
        metadata.game_event_active_npc_vendor_items_like_cpp(9001)[0].item,
        6000
    );

    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        2,
        &[(1, 100, 9001, 6000, 2), (2, 200, 9001, 6001, 2)],
    );
    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);
    let no_match = metadata.update_game_event_npc_vendor_cache_like_cpp(2, false);
    assert_eq!(no_match.no_match, 1);
    assert_eq!(no_match.items_removed, 0);
}

#[test]
fn game_event_npc_vendor_cache_missing_bucket_is_explicit_noop_like_cpp() {
    let mut metadata =
        game_event_npc_vendor_metadata_with_records_like_cpp(1, &[(1, 100, 9001, 6000, 2)]);
    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);

    let summary = metadata.update_game_event_npc_vendor_cache_like_cpp(2, true);

    assert!(summary.missing_event_bucket);
    assert_eq!(summary.records_seen, 0);
    assert_eq!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .len(),
        1
    );
}

#[test]
fn game_event_npc_vendor_cache_preserves_order_per_entry_like_cpp() {
    let mut metadata = game_event_npc_vendor_metadata_with_records_like_cpp(
        2,
        &[
            (1, 100, 9001, 6000, 2),
            (1, 101, 9002, 7000, 2),
            (1, 102, 9001, 6001, 2),
            (2, 200, 9001, 6002, 2),
        ],
    );

    metadata.update_game_event_npc_vendor_cache_like_cpp(1, true);
    metadata.update_game_event_npc_vendor_cache_like_cpp(2, true);

    assert_eq!(
        metadata
            .game_event_active_npc_vendor_items_like_cpp(9001)
            .iter()
            .map(|record| record.item)
            .collect::<Vec<_>>(),
        vec![6000, 6001, 6002]
    );
    assert_eq!(
        metadata.game_event_active_npc_vendor_items_like_cpp(9002)[0].item,
        7000
    );
}

fn game_event_model_equip_runtime_row_like_cpp(
    spawn_id: SpawnId,
    model_id: u32,
    equipment_id: i8,
) -> CreatureSpawnRuntimeRowLikeCpp {
    CreatureSpawnRuntimeRowLikeCpp {
        spawn_id,
        model_id,
        equipment_id,
        wander_distance: 0.0,
        curhealth: 1,
        curmana: 0,
        movement_type: 0,
        npc_flags: None,
        unit_flags: None,
        unit_flags2: None,
        unit_flags3: None,
        ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms:
            wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        string_id: String::new(),
        spawn_time_secs: 120,
    }
}

#[test]
fn game_event_change_equip_or_model_baseline_activate_saves_prev_and_applies_new_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 0,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 111, 3),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, true);

    assert_eq!(summary.records_seen, 1);
    assert_eq!(summary.records_applied, 1);
    let record = &metadata.game_event_model_equip_like_cpp(1).unwrap()[0];
    assert_eq!(record.model_id_prev, 111);
    assert_eq!(record.equipment_id_prev, 3);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 222);
    assert_eq!(row.equipment_id, 7);
}

#[test]
fn game_event_change_equip_or_model_baseline_activate_zero_model_resets_display_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 0,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 0,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 111, 3),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, true);

    assert_eq!(summary.records_applied, 1);
    let record = &metadata.game_event_model_equip_like_cpp(1).unwrap()[0];
    assert_eq!(record.model_id_prev, 111);
    assert_eq!(record.equipment_id_prev, 3);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 0);
    assert_eq!(row.equipment_id, 7);
}

#[test]
fn game_event_change_equip_or_model_baseline_deactivate_restores_prev_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 111,
            equipment_id: 7,
            equipment_id_prev: 3,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 222, 7),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, false);

    assert_eq!(summary.records_applied, 1);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 111);
    assert_eq!(row.equipment_id, 3);
}

#[test]
fn game_event_change_equip_or_model_baseline_deactivate_zero_prev_model_resets_display_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 3,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 222, 7),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, false);

    assert_eq!(summary.records_applied, 1);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 0);
    assert_eq!(row.equipment_id, 3);
}

#[test]
fn game_event_change_equip_or_model_baseline_missing_row_and_bucket_do_not_panic_like_cpp() {
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 0,
        },
    ));
    let mut store = SpawnStore::new();
    store.insert_spawn_metadata_like_cpp(&game_event_guid_test_spawn(
        SpawnObjectType::Creature,
        100,
        0,
    ));
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip);

    let missing_row = metadata.change_game_event_model_equip_baseline_like_cpp(1, true);
    let missing_bucket = metadata.change_game_event_model_equip_baseline_like_cpp(4, true);

    assert_eq!(missing_row.records_seen, 1);
    assert_eq!(missing_row.records_applied, 0);
    assert_eq!(missing_row.missing_creature_runtime_rows, 1);
    assert!(missing_bucket.missing_event_bucket);
}

#[test]
fn game_event_change_equip_or_model_baseline_missing_spawn_metadata_does_not_create_dummy_like_cpp()
{
    let mut model_equip = GameEventModelEquipLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(model_equip.push_record_like_cpp(
        1,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: 100,
            model_id: 222,
            model_id_prev: 0,
            equipment_id: 7,
            equipment_id_prev: 0,
        },
    ));
    let mut rows = BTreeMap::new();
    rows.insert(
        100,
        game_event_model_equip_runtime_row_like_cpp(100, 111, 3),
    );
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_model_equip_like_cpp(model_equip)
        .with_creature_runtime_rows_like_cpp(rows);

    let summary = metadata.change_game_event_model_equip_baseline_like_cpp(1, true);

    assert_eq!(summary.records_seen, 1);
    assert_eq!(summary.records_applied, 0);
    assert_eq!(summary.missing_spawn_metadata, 1);
    let row = metadata.creature_runtime_row_like_cpp(100).unwrap();
    assert_eq!(row.model_id, 111);
    assert_eq!(row.equipment_id, 3);
}

#[test]
fn canonical_metadata_exposes_game_event_spawn_guid_slices_like_cpp() {
    let mut guids = GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    assert!(guids.push_guid_like_cpp(SpawnObjectType::Creature, 1, 100));
    assert!(guids.push_guid_like_cpp(SpawnObjectType::GameObject, -1, 200));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::new(), BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids);

    assert_eq!(
        metadata.game_event_creature_guids_like_cpp(1),
        Some([100].as_slice())
    );
    assert_eq!(
        metadata.game_event_gameobject_guids_like_cpp(-1),
        Some([200].as_slice())
    );
    assert_eq!(
        metadata.game_event_creature_guids_like_cpp(2),
        Some([].as_slice())
    );
    assert_eq!(metadata.game_event_gameobject_guids_like_cpp(4), None);
}

#[test]
fn linked_respawn_loader_validation_invalid_type_and_missing_master_like_cpp() {
    let maps = instanceable_map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut kind_report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let creature = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    store.add_object_spawn(&creature, is_personal_phase_like_cpp_represented);
    let mut linked_store = LinkedRespawnStoreLikeCpp::new();
    let mut report = LinkedRespawnLoadReportLikeCpp::default();

    apply_linked_respawn_row_like_cpp(
        LinkedRespawnRowLikeCpp {
            guid: 100,
            linked_guid: 200,
            link_type: 99,
        },
        &store,
        &maps,
        &mut linked_store,
        &mut report,
    );
    apply_linked_respawn_row_like_cpp(
        LinkedRespawnRowLikeCpp {
            guid: 100,
            linked_guid: 200,
            link_type: LinkedRespawnTypeLikeCpp::CreatureToCreature as u8,
        },
        &store,
        &maps,
        &mut linked_store,
        &mut report,
    );

    assert_eq!(report.rows, 2);
    assert_eq!(report.invalid_type, 1);
    assert_eq!(report.missing_master, 1);
    assert!(linked_store.is_empty());
}

#[test]
fn linked_respawn_loader_validation_difficulty_mismatch_like_cpp() {
    let maps = instanceable_map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0), (1, 1)]);
    let mut kind_report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let slave = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    let master = creature_row_to_spawn_data_like_cpp(
        &creature_row(200, 0, "1"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    store.add_object_spawn(&slave, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&master, is_personal_phase_like_cpp_represented);
    let mut linked_store = LinkedRespawnStoreLikeCpp::new();
    let mut report = LinkedRespawnLoadReportLikeCpp::default();

    apply_linked_respawn_row_like_cpp(
        LinkedRespawnRowLikeCpp {
            guid: 100,
            linked_guid: 200,
            link_type: LinkedRespawnTypeLikeCpp::CreatureToCreature as u8,
        },
        &store,
        &maps,
        &mut linked_store,
        &mut report,
    );

    assert_eq!(report.difficulty_mismatch, 1);
    assert!(linked_store.is_empty());
}

#[test]
fn linked_respawn_loader_validation_valid_creature_to_gameobject_inserts_like_cpp() {
    let maps = instanceable_map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut kind_report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let slave = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    let master = gameobject_row_to_spawn_data_like_cpp(
        &gameobject_row(200, 0, "0"),
        &maps,
        &difficulties,
        &mut kind_report,
    )
    .unwrap();
    store.add_object_spawn(&slave, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&master, is_personal_phase_like_cpp_represented);
    let mut linked_store = LinkedRespawnStoreLikeCpp::new();
    let mut report = LinkedRespawnLoadReportLikeCpp::default();

    apply_linked_respawn_row_like_cpp(
        LinkedRespawnRowLikeCpp {
            guid: 100,
            linked_guid: 200,
            link_type: LinkedRespawnTypeLikeCpp::CreatureToGameObject as u8,
        },
        &store,
        &maps,
        &mut linked_store,
        &mut report,
    );

    assert_eq!(report.inserted, 1);
    assert_eq!(linked_store.len(), 1);
    let slave_guid = spawn_data_guid_like_cpp(&slave);
    let master_guid = spawn_data_guid_like_cpp(&master);
    assert_eq!(
        linked_store.get_linked_respawn_guid_like_cpp(slave_guid),
        master_guid
    );
}

#[test]
fn spawn_difficulty_parser_matches_cpp_token_rules() {
    let difficulties = map_difficulty_store(&[(1, 0), (1, 1)]);
    let parsed = parse_spawn_difficulties_like_cpp("0,1", 1, false, &difficulties);
    assert_eq!(parsed.difficulties, vec![0, 1]);
    assert_eq!(parsed.report.invalid_tokens_as_none, 0);
    assert!(parsed.report.unsupported.is_empty());

    let parsed = parse_spawn_difficulties_like_cpp("bad,1", 1, false, &difficulties);
    assert_eq!(parsed.difficulties, vec![0, 1]);
    assert_eq!(parsed.report.invalid_tokens_as_none, 1);

    let parsed = parse_spawn_difficulties_like_cpp("0,2,1", 1, false, &difficulties);
    assert_eq!(parsed.difficulties, vec![0, 1]);
    assert_eq!(parsed.report.unsupported, vec![2]);

    let parsed = parse_spawn_difficulties_like_cpp("2", 1, true, &difficulties);
    assert_eq!(parsed.difficulties, vec![2]);

    let parsed = parse_spawn_difficulties_like_cpp("", 1, false, &difficulties);
    assert!(parsed.difficulties.is_empty());
}

#[test]
fn creature_row_indexes_only_non_event_rows_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();

    let indexed = creature_row_to_spawn_data_like_cpp(
        &creature_row(100, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .expect("non-event creature spawn should convert");
    store.add_object_spawn(&indexed, is_personal_phase_like_cpp_represented);

    let event_managed = creature_row_to_spawn_data_like_cpp(
        &creature_row(101, 7, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .expect("event-managed creature spawn metadata should convert");
    store.insert_spawn_metadata_like_cpp(&event_managed);

    assert!(
        store
            .cell_object_guids(1, 0, indexed.cell_id())
            .is_some_and(|cell| cell.creatures.contains(&100))
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 101)
            .map(|spawn| spawn.spawn_id),
        Some(101)
    );
    assert!(
        store
            .cell_object_guids(1, 0, event_managed.cell_id())
            .is_none_or(|cell| !cell.creatures.contains(&101))
    );
}

#[test]
fn row_conversion_skips_missing_map_and_empty_difficulties() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut report = SpawnKindLoadReport::default();

    let mut missing_map = creature_row(200, 0, "0");
    missing_map.map_id = 999;
    assert!(
        creature_row_to_spawn_data_like_cpp(&missing_map, &maps, &difficulties, &mut report)
            .is_none()
    );
    assert_eq!(report.skipped_missing_map, 1);

    assert!(
        creature_row_to_spawn_data_like_cpp(
            &creature_row(201, 0, ""),
            &maps,
            &difficulties,
            &mut report,
        )
        .is_none()
    );
    assert_eq!(report.skipped_empty_difficulties, 1);
}

fn formation_test_store(spawn_ids: &[SpawnId]) -> SpawnStore {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    for spawn_id in spawn_ids {
        let spawn = creature_row_to_spawn_data_like_cpp(
            &creature_row(*spawn_id, 0, "0"),
            &maps,
            &difficulties,
            &mut report,
        )
        .expect("test creature spawn row should be valid");
        store.insert_spawn_metadata_like_cpp(&spawn);
    }
    store
}

fn formation_row(
    leader_spawn_id: SpawnId,
    member_spawn_id: SpawnId,
    dist: f32,
    angle_degrees: f32,
) -> CreatureFormationRowLikeCpp {
    CreatureFormationRowLikeCpp {
        leader_spawn_id,
        member_spawn_id,
        dist,
        angle_degrees,
        group_ai: 17,
        point_1: 101,
        point_2: 102,
    }
}

#[test]
fn creature_formation_loader_converts_member_degrees_to_radians_like_cpp() {
    let store = formation_test_store(&[10, 11]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [
            formation_row(10, 10, 99.0, 180.0),
            formation_row(10, 11, 7.5, 90.0),
        ],
        &store,
        &mut report,
    );

    let member = formations.get(&11).expect("member formation should load");
    assert_eq!(member.leader_spawn_id, 10);
    assert_eq!(member.follow_dist, 7.5);
    assert!((member.follow_angle_radians - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    assert_eq!(member.group_ai, 17);
    assert_eq!(member.leader_waypoint_ids, [101, 102]);
    assert_eq!(report.loaded, 2);
}

#[test]
fn creature_formation_loader_forces_leader_self_dist_angle_zero_like_cpp() {
    let store = formation_test_store(&[20]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [formation_row(20, 20, 33.0, 270.0)],
        &store,
        &mut report,
    );

    let leader = formations.get(&20).expect("leader self row should load");
    assert_eq!(leader.follow_dist, 0.0);
    assert_eq!(leader.follow_angle_radians, 0.0);
    assert_eq!(report.loaded, 1);
}

#[test]
fn creature_formation_loader_skips_missing_leader_and_member_like_cpp() {
    let store = formation_test_store(&[30, 31]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [
            formation_row(99, 31, 1.0, 1.0),
            formation_row(30, 98, 1.0, 1.0),
            formation_row(30, 30, 0.0, 0.0),
        ],
        &store,
        &mut report,
    );

    assert!(formations.contains_key(&30));
    assert_eq!(formations.len(), 1);
    assert_eq!(report.rows, 3);
    assert_eq!(report.skipped_missing_leader, 1);
    assert_eq!(report.skipped_missing_member, 1);
}

#[test]
fn creature_formation_loader_prunes_group_without_leader_self_row_like_cpp() {
    let store = formation_test_store(&[40, 41]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [formation_row(40, 41, 4.0, 45.0)],
        &store,
        &mut report,
    );

    assert!(formations.is_empty());
    assert_eq!(report.removed_missing_leader_self, 1);
    assert_eq!(report.loaded, 0);
}

#[test]
fn creature_formation_loader_duplicate_member_keeps_first_like_cpp_emplace() {
    let store = formation_test_store(&[50, 51]);
    let mut report = CreatureFormationLoadReportLikeCpp::default();
    let formations = apply_creature_formation_rows_like_cpp(
        [
            formation_row(50, 50, 0.0, 0.0),
            formation_row(50, 51, 3.0, 30.0),
            formation_row(50, 51, 9.0, 90.0),
        ],
        &store,
        &mut report,
    );

    let member = formations.get(&51).expect("first member row should remain");
    assert_eq!(member.follow_dist, 3.0);
    assert!(
        (member.follow_angle_radians - (30.0_f32 * std::f32::consts::PI / 180.0)).abs() < 0.0001
    );
    assert_eq!(report.duplicate_member_ignored, 1);
    assert_eq!(report.loaded, 2);
}

#[test]
fn templates_and_spawn_group_apply_cover_creature_go_at_and_event_gap() {
    let (template_store, _) = wow_data::SpawnGroupTemplateStore::from_rows_like_cpp([
        wow_data::SpawnGroupTemplateRow {
            group_id: 10,
            name: "custom".to_string(),
            flags: 0,
        },
        wow_data::SpawnGroupTemplateRow {
            group_id: 11,
            name: "manual".to_string(),
            flags: wow_data::spawn_group::SPAWN_GROUP_FLAG_MANUAL_SPAWN_LIKE_CPP,
        },
    ]);
    let mut templates = spawn_group_templates_for_spawn_store(&template_store);
    assert_eq!(templates.get(&0).unwrap().map_id, 0);
    assert_eq!(templates.get(&1).unwrap().map_id, 0);
    assert_eq!(templates.get(&10).unwrap().map_id, SPAWNGROUP_MAP_UNSET);

    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let mut report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();
    let area_trigger_templates = valid_area_trigger_template_store();
    let mut area_trigger_runtime_rows = BTreeMap::new();

    let creature = creature_row_to_spawn_data_like_cpp(
        &creature_row(300, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();
    let go = gameobject_row_to_spawn_data_like_cpp(
        &gameobject_row(301, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();
    let at = area_trigger_row_to_spawn_data_like_cpp(
        &area_trigger_row(302, "0"),
        &maps,
        &difficulties,
        &area_trigger_templates,
        &mut |_| true,
        &mut |_| wow_data::ScriptIdLikeCpp(0),
        &mut area_trigger_runtime_rows,
        &mut report,
    )
    .unwrap();
    let event_managed = gameobject_row_to_spawn_data_like_cpp(
        &gameobject_row(303, 5, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();

    store.add_object_spawn(&creature, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&go, is_personal_phase_like_cpp_represented);
    store.add_area_trigger_spawn(&at);
    store.insert_spawn_metadata_like_cpp(&event_managed);

    let apply = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: SpawnObjectType::Creature as u8,
                spawn_id: 300,
            },
            SpawnGroupMemberRow {
                group_id: 11,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: 301,
            },
            SpawnGroupMemberRow {
                group_id: 1,
                spawn_type: SpawnObjectType::AreaTrigger as u8,
                spawn_id: 302,
            },
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: event_managed.spawn_id,
            },
            SpawnGroupMemberRow {
                group_id: 10,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: 999,
            },
        ],
    );

    assert_eq!(apply.assigned, 3);
    assert_eq!(apply.missing_spawn, 1);
    assert_eq!(apply.duplicate_spawn_group, 1);
    assert_eq!(templates.get(&0).unwrap().map_id, 0);
    assert_eq!(templates.get(&1).unwrap().map_id, 0);
    assert_eq!(templates.get(&10).unwrap().map_id, 1);
    assert_eq!(templates.get(&11).unwrap().map_id, 1);
    assert!(templates.contains_key(&0));
    assert!(templates.contains_key(&1));
    let metadata = CanonicalSpawnMetadataLikeCpp::new(store.clone(), templates.clone());
    assert_eq!(metadata.spawn_group_templates().get(&10).unwrap().map_id, 1);
    assert!(metadata.spawn_group_templates().contains_key(&0));
    assert!(metadata.spawn_group_templates().contains_key(&1));
    assert_eq!(
        metadata
            .spawn_store()
            .spawn_group_ids_by_map(1)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::Creature, 300)
            .unwrap()
            .spawn_group_id(),
        10
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::GameObject, 301)
            .unwrap()
            .spawn_group_id(),
        11
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::AreaTrigger, 302)
            .unwrap()
            .spawn_group_id(),
        1
    );
    assert_eq!(
        store
            .spawn_data(SpawnObjectType::GameObject, 303)
            .unwrap()
            .spawn_group_id(),
        10
    );
    assert!(
        store
            .cell_object_guids(1, 0, event_managed.cell_id())
            .is_none_or(|cell| !cell.gameobjects.contains(&303))
    );
}

#[test]
fn area_trigger_spawn_loads_cpp_validated_metadata_and_script_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let area_trigger_templates = valid_area_trigger_template_store();
    let mut row = area_trigger_row(302, "0");
    row.script_name = "at_spawn_script".to_string();
    row.spell_for_visuals = Some(1234);
    let mut report = SpawnKindLoadReport::default();
    let mut runtime_rows = BTreeMap::new();

    let spawn = area_trigger_row_to_spawn_data_like_cpp(
        &row,
        &maps,
        &difficulties,
        &area_trigger_templates,
        &mut |spell_id| spell_id == 1234,
        &mut |name| {
            assert_eq!(name, "at_spawn_script");
            wow_data::ScriptIdLikeCpp(77)
        },
        &mut runtime_rows,
        &mut report,
    )
    .expect("valid static area trigger spawn should load");

    assert_eq!(spawn.object_type, SpawnObjectType::AreaTrigger);
    assert_eq!(spawn.id, 789);
    assert_eq!(spawn.script_id, 77);
    assert_eq!(report.validation_skipped, 0);
    assert_eq!(report.script_id_unresolved, 0);
    assert!(report.corrected_invalid_spell_for_visuals.is_empty());
    assert_eq!(
        runtime_rows.get(&302),
        Some(&AreaTriggerSpawnRuntimeRowLikeCpp {
            spawn_id: 302,
            create_properties_id: wow_data::AreaTriggerIdLikeCpp {
                id: 789,
                is_custom: false,
            },
            spell_for_visuals: Some(1234),
        })
    );
}

#[test]
fn area_trigger_spawn_resets_invalid_spell_for_visuals_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let area_trigger_templates = valid_area_trigger_template_store();
    let mut row = area_trigger_row(302, "0");
    row.spell_for_visuals = Some(-7);
    let mut report = SpawnKindLoadReport::default();
    let mut runtime_rows = BTreeMap::new();

    let spawn = area_trigger_row_to_spawn_data_like_cpp(
        &row,
        &maps,
        &difficulties,
        &area_trigger_templates,
        &mut |_| false,
        &mut |_| wow_data::ScriptIdLikeCpp(0),
        &mut runtime_rows,
        &mut report,
    )
    .expect("invalid SpellForVisuals is reset, not a skipped spawn");

    assert_eq!(spawn.spawn_id, 302);
    assert_eq!(report.corrected_invalid_spell_for_visuals, [(302, -7)]);
    assert_eq!(runtime_rows.get(&302).unwrap().spell_for_visuals, None);
}

#[test]
fn area_trigger_spawn_skips_missing_create_properties_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let area_trigger_templates =
        area_trigger_template_store_with(area_trigger_create_properties_row(111), [], []);
    let mut report = SpawnKindLoadReport::default();
    let mut runtime_rows = BTreeMap::new();

    let spawn = area_trigger_row_to_spawn_data_like_cpp(
        &area_trigger_row(302, "0"),
        &maps,
        &difficulties,
        &area_trigger_templates,
        &mut |_| true,
        &mut |_| wow_data::ScriptIdLikeCpp(0),
        &mut runtime_rows,
        &mut report,
    );

    assert!(spawn.is_none());
    assert_eq!(
        report.skipped_invalid_create_properties,
        [(302, 789, false)]
    );
    assert!(runtime_rows.is_empty());
}

#[test]
fn area_trigger_spawn_skips_non_static_create_properties_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let load = |spawn_id, store: wow_data::AreaTriggerTemplateStore| {
        let mut report = SpawnKindLoadReport::default();
        let mut runtime_rows = BTreeMap::new();
        let spawn = area_trigger_row_to_spawn_data_like_cpp(
            &area_trigger_row(spawn_id, "0"),
            &maps,
            &difficulties,
            &store,
            &mut |_| true,
            &mut |_| wow_data::ScriptIdLikeCpp(0),
            &mut runtime_rows,
            &mut report,
        );
        (spawn, runtime_rows, report)
    };

    let mut flags_row = area_trigger_create_properties_row(789);
    flags_row.flags =
        wow_data::area_trigger_template::AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ATTACHED_LIKE_CPP;
    let (spawn, runtime_rows, report) =
        load(302, area_trigger_template_store_with(flags_row, [], []));
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(
        report.skipped_nonzero_create_properties_flags,
        [(302, 789, false)]
    );

    let mut curve_row = area_trigger_create_properties_row(789);
    curve_row.move_curve_id = 44;
    let (spawn, runtime_rows, report) =
        load(303, area_trigger_template_store_with(curve_row, [], []));
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(report.skipped_create_properties_curves, [(303, 789, false)]);

    let mut time_row = area_trigger_create_properties_row(789);
    time_row.time_to_target = 1;
    let (spawn, runtime_rows, report) =
        load(304, area_trigger_template_store_with(time_row, [], []));
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(
        report.skipped_create_properties_time_to_target,
        [(304, 789, false)]
    );

    let (spawn, runtime_rows, report) = load(
        305,
        area_trigger_template_store_with(
            area_trigger_create_properties_row(789),
            [],
            [area_trigger_orbit(789)],
        ),
    );
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(report.skipped_create_properties_orbit, [(305, 789, false)]);

    let (spawn, runtime_rows, report) = load(
        306,
        area_trigger_template_store_with(
            area_trigger_create_properties_row(789),
            [
                area_trigger_spline_point(789, 1.0),
                area_trigger_spline_point(789, 2.0),
            ],
            [],
        ),
    );
    assert!(spawn.is_none());
    assert!(runtime_rows.is_empty());
    assert_eq!(
        report.skipped_create_properties_splines,
        [(306, 789, false)]
    );
}

#[test]
fn canonical_spawn_metadata_spawn_group_helper_filters_by_map_and_template_like_cpp() {
    let (template_store, _) = wow_data::SpawnGroupTemplateStore::from_rows_like_cpp([
        wow_data::SpawnGroupTemplateRow {
            group_id: 20,
            name: "map-one-a".to_string(),
            flags: 0,
        },
        wow_data::SpawnGroupTemplateRow {
            group_id: 21,
            name: "map-one-b".to_string(),
            flags: 0,
        },
        wow_data::SpawnGroupTemplateRow {
            group_id: 22,
            name: "map-two".to_string(),
            flags: 0,
        },
    ]);
    let mut templates = spawn_group_templates_for_spawn_store(&template_store);
    let maps = map_store(&[1, 2]);
    let difficulties = map_difficulty_store(&[(1, 0), (2, 0)]);
    let mut report = SpawnKindLoadReport::default();
    let mut store = SpawnStore::new();

    let map_one_a = creature_row_to_spawn_data_like_cpp(
        &creature_row(400, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();
    let map_one_b = gameobject_row_to_spawn_data_like_cpp(
        &gameobject_row(401, 0, "0"),
        &maps,
        &difficulties,
        &mut report,
    )
    .unwrap();
    let mut map_two_row = creature_row(402, 0, "0");
    map_two_row.map_id = 2;
    let map_two =
        creature_row_to_spawn_data_like_cpp(&map_two_row, &maps, &difficulties, &mut report)
            .unwrap();

    store.add_object_spawn(&map_one_a, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&map_one_b, is_personal_phase_like_cpp_represented);
    store.add_object_spawn(&map_two, is_personal_phase_like_cpp_represented);
    let apply = store.apply_spawn_groups_like_cpp(
        &mut templates,
        [
            SpawnGroupMemberRow {
                group_id: 21,
                spawn_type: SpawnObjectType::GameObject as u8,
                spawn_id: 401,
            },
            SpawnGroupMemberRow {
                group_id: 20,
                spawn_type: SpawnObjectType::Creature as u8,
                spawn_id: 400,
            },
            SpawnGroupMemberRow {
                group_id: 22,
                spawn_type: SpawnObjectType::Creature as u8,
                spawn_id: 402,
            },
        ],
    );
    assert_eq!(apply.assigned, 3);

    // Simulate a future C++-shaped filter miss without panicking: the group id is indexed
    // for the map, but `GetSpawnGroupData`/map filtering no longer returns a matching template.
    templates.get_mut(&21).unwrap().map_id = 2;
    let metadata = CanonicalSpawnMetadataLikeCpp::new(store, templates);

    let map_one_groups = metadata.spawn_group_templates_for_map_like_cpp(1);
    assert_eq!(
        map_one_groups
            .iter()
            .map(|(group_id, template)| (*group_id, template.name.as_str()))
            .collect::<Vec<_>>(),
        vec![(20, "map-one-a")]
    );
    let map_two_groups = metadata.spawn_group_templates_for_map_like_cpp(2);
    assert_eq!(
        map_two_groups
            .iter()
            .map(|(group_id, template)| (*group_id, template.name.as_str()))
            .collect::<Vec<_>>(),
        vec![(22, "map-two")]
    );
    assert!(
        metadata
            .spawn_group_templates_for_map_like_cpp(999)
            .is_empty()
    );
}
