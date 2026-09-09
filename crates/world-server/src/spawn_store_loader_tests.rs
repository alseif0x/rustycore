//! Behaviour tests for [`super`].
//!
//! Extracted from `spawn_store_loader.rs`, which was 11,154 lines of which
//! 5,199 — 47% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use std::sync::{Arc, Mutex};

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

struct RecordingGameEventWorldCatalogLikeCpp {
    calls: Arc<Mutex<Vec<&'static str>>>,
    fail_prefix: bool,
    fail_suffix: bool,
}

struct RecordingCanonicalSpawnCatalogLikeCpp {
    calls: Arc<Mutex<Vec<&'static str>>>,
    fail_on: Option<&'static str>,
}

impl RecordingCanonicalSpawnCatalogLikeCpp {
    fn rows<T>(&self, name: &'static str, rows: T) -> CanonicalSpawnCatalogLoadOutcomeLikeCpp<T> {
        self.calls.lock().unwrap().push(name);
        if self.fail_on == Some(name) {
            CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed {
                reason: format!("{name} failed"),
            }
        } else {
            CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows)
        }
    }
}

impl CanonicalSpawnCatalogPersistencePortLikeCpp for RecordingCanonicalSpawnCatalogLikeCpp {
    fn load_creature_spawns_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<
            Vec<wow_persistence::CreatureSpawnPersistenceRowLikeCpp>,
        >,
    > {
        Box::pin(async move { self.rows("creature-spawns", Vec::new()) })
    }

    fn load_waypoint_paths_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<wow_persistence::WaypointPathCatalogLikeCpp>,
    > {
        Box::pin(async move {
            self.rows(
                "waypoints",
                wow_persistence::WaypointPathCatalogLikeCpp {
                    paths: Vec::new(),
                    nodes: Vec::new(),
                },
            )
        })
    }

    fn load_creature_formations_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<
            Vec<wow_persistence::CreatureFormationPersistenceRowLikeCpp>,
        >,
    > {
        Box::pin(async move { self.rows("formations", Vec::new()) })
    }

    fn load_gameobject_spawns_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<
            Vec<wow_persistence::GameObjectSpawnPersistenceRowLikeCpp>,
        >,
    > {
        Box::pin(async move { self.rows("gameobject-spawns", Vec::new()) })
    }

    fn load_area_trigger_spawns_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<
            Vec<wow_persistence::AreaTriggerSpawnPersistenceRowLikeCpp>,
        >,
    > {
        Box::pin(async move { self.rows("area-trigger-spawns", Vec::new()) })
    }

    fn load_linked_respawns_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<
            Vec<wow_persistence::LinkedRespawnPersistenceRowLikeCpp>,
        >,
    > {
        Box::pin(async move { self.rows("linked-respawns", Vec::new()) })
    }

    fn load_pool_templates_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<PoolTemplateRowLikeCpp>>,
    > {
        Box::pin(async move {
            self.rows(
                "pool-templates",
                vec![PoolTemplateRowLikeCpp {
                    entry: 42,
                    max_limit: 1,
                }],
            )
        })
    }

    fn load_pool_members_like_cpp(
        &self,
        kind: PoolMemberKindPersistenceLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<PoolMemberRowLikeCpp>>,
    > {
        let name = match kind {
            PoolMemberKindPersistenceLikeCpp::Creature => "pool-creatures",
            PoolMemberKindPersistenceLikeCpp::GameObject => "pool-gameobjects",
            PoolMemberKindPersistenceLikeCpp::Pool => "pool-pools",
        };
        Box::pin(async move { self.rows(name, Vec::new()) })
    }

    fn load_pool_autospawn_candidates_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<
            Vec<wow_persistence::PoolAutospawnCandidatePersistenceRowLikeCpp>,
        >,
    > {
        Box::pin(async move { self.rows("pool-autospawn", Vec::new()) })
    }

    fn load_spawn_group_members_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<
            Vec<wow_persistence::SpawnGroupMemberPersistenceRowLikeCpp>,
        >,
    > {
        Box::pin(async move { self.rows("spawn-groups", Vec::new()) })
    }
}

struct RecordingWorldStateStartupLikeCpp {
    calls: Arc<Mutex<Vec<&'static str>>>,
    fail: bool,
}

impl WorldStateStartupPersistencePortLikeCpp for RecordingWorldStateStartupLikeCpp {
    fn load_world_then_character_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        wow_persistence::WorldStateStartupLoadOutcomeLikeCpp,
    > {
        Box::pin(async move {
            self.calls.lock().unwrap().push("world-then-characters");
            if self.fail {
                wow_persistence::WorldStateStartupLoadOutcomeLikeCpp::Failed {
                    reason: "world-state failed".to_owned(),
                }
            } else {
                wow_persistence::WorldStateStartupLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::WorldStateStartupCatalogLikeCpp {
                        templates: Vec::new(),
                        saved_values: Vec::new(),
                    },
                )
            }
        })
    }
}

impl GameEventWorldCatalogPersistencePortLikeCpp for RecordingGameEventWorldCatalogLikeCpp {
    fn load_prefix_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        GameEventWorldCatalogLoadOutcomeLikeCpp<GameEventWorldCatalogPrefixLikeCpp>,
    > {
        Box::pin(async move {
            self.calls.lock().unwrap().push("world-prefix");
            if self.fail_prefix {
                GameEventWorldCatalogLoadOutcomeLikeCpp::Failed {
                    reason: "prefix failed".to_owned(),
                }
            } else {
                GameEventWorldCatalogLoadOutcomeLikeCpp::Loaded(empty_game_event_prefix_like_cpp())
            }
        })
    }

    fn load_suffix_like_cpp(
        &self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        GameEventWorldCatalogLoadOutcomeLikeCpp<GameEventWorldCatalogSuffixLikeCpp>,
    > {
        Box::pin(async move {
            self.calls.lock().unwrap().push("world-suffix");
            if self.fail_suffix {
                GameEventWorldCatalogLoadOutcomeLikeCpp::Failed {
                    reason: "suffix failed".to_owned(),
                }
            } else {
                GameEventWorldCatalogLoadOutcomeLikeCpp::Loaded(empty_game_event_suffix_like_cpp())
            }
        })
    }
}

struct RecordingGameEventConditionSaveLikeCpp {
    calls: Arc<Mutex<Vec<&'static str>>>,
    fail: bool,
}

impl wow_persistence::GameEventPersistencePortLikeCpp for RecordingGameEventConditionSaveLikeCpp {
    fn load_condition_saves_like_cpp<'a>(
        &'a self,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp,
    > {
        Box::pin(async move {
            self.calls.lock().unwrap().push("character-saves");
            if self.fail {
                wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Failed {
                    reason: "character failed".to_owned(),
                }
            } else {
                wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Loaded(Vec::new())
            }
        })
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

fn empty_game_event_prefix_like_cpp() -> GameEventWorldCatalogPrefixLikeCpp {
    GameEventWorldCatalogPrefixLikeCpp {
        max_event_entry: None,
        events: Vec::new(),
        prerequisites: Vec::new(),
        conditions: Vec::new(),
    }
}

fn empty_game_event_suffix_like_cpp() -> GameEventWorldCatalogSuffixLikeCpp {
    GameEventWorldCatalogSuffixLikeCpp {
        quest_conditions: Vec::new(),
        pools: Vec::new(),
        creature_guids: Vec::new(),
        gameobject_guids: Vec::new(),
        equipment_ids: Vec::new(),
        model_equips: Vec::new(),
        creature_quest_relations: Vec::new(),
        gameobject_quest_relations: Vec::new(),
        npc_flags: Vec::new(),
        npc_vendors: Vec::new(),
    }
}

async fn run_empty_canonical_spawn_pipeline_like_cpp(
    spawn: &RecordingCanonicalSpawnCatalogLikeCpp,
    character: &RecordingGameEventConditionSaveLikeCpp,
    game_event_world: &RecordingGameEventWorldCatalogLikeCpp,
) -> Result<(CanonicalSpawnMetadataLikeCpp, CanonicalSpawnStoreLoadReport)> {
    let maps = map_store(&[]);
    let difficulties = map_difficulty_store(&[]);
    let spawn_groups = wow_data::SpawnGroupTemplateStore::default();
    let equipment = wow_data::CreatureEquipmentStoreLikeCpp::default();
    let area_triggers = wow_data::AreaTriggerTemplateStore::default();

    load_canonical_spawn_store_like_cpp(
        spawn,
        character,
        game_event_world,
        &maps,
        &difficulties,
        &spawn_groups,
        &equipment,
        &area_triggers,
        |_| false,
        |_| wow_data::ScriptIdLikeCpp(0),
    )
    .await
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

#[path = "spawn_store_loader_tests/creature.rs"]
mod creature;
#[path = "spawn_store_loader_tests/gameobject.rs"]
mod gameobject;
#[path = "spawn_store_loader_tests/group.rs"]
mod group;
#[path = "spawn_store_loader_tests/instance.rs"]
mod instance;
#[path = "spawn_store_loader_tests/item.rs"]
mod item;
#[path = "spawn_store_loader_tests/misc_1.rs"]
mod misc_1;
#[path = "spawn_store_loader_tests/misc_2.rs"]
mod misc_2;
#[path = "spawn_store_loader_tests/movement.rs"]
mod movement;
#[path = "spawn_store_loader_tests/persistence.rs"]
mod persistence;
#[path = "spawn_store_loader_tests/quest.rs"]
mod quest;
#[path = "spawn_store_loader_tests/skill.rs"]
mod skill;
#[path = "spawn_store_loader_tests/spawn.rs"]
mod spawn;
#[path = "spawn_store_loader_tests/spell.rs"]
mod spell;
#[path = "spawn_store_loader_tests/visibility.rs"]
mod visibility;
