//! Creature, GameObject and AreaTrigger spawn-row loading.
//! The parent remains the startup composition facade.

use super::*;
pub(super) async fn load_creature_spawns_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    creature_equipment_store: &wow_data::CreatureEquipmentStoreLikeCpp,
    store: &mut SpawnStore,
    creature_runtime_rows: &mut BTreeMap<SpawnId, CreatureSpawnRuntimeRowLikeCpp>,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let rows = match persistence.load_creature_spawns_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical creature spawn catalog failed: {reason}")
        }
    };
    for mut row in rows {
        normalize_creature_spawn_equipment_id_like_cpp(&mut row, creature_equipment_store);
        let runtime_row = creature_row_to_runtime_row_like_cpp(&row);
        report.creature.rows += 1;
        if let Some(spawn) = creature_row_to_spawn_data_like_cpp(
            &row,
            map_store,
            map_difficulty_store,
            &mut report.creature,
        ) {
            if row.event_entry != 0 {
                store.insert_spawn_metadata_like_cpp(&spawn);
                creature_runtime_rows.insert(row.spawn_id, runtime_row.clone());
                report.creature.skipped_event += 1;
            } else {
                store.add_object_spawn(&spawn, is_personal_phase_like_cpp_represented);
                creature_runtime_rows.insert(row.spawn_id, runtime_row.clone());
                report.creature.indexed += 1;
            }
        }
    }

    Ok(())
}
pub(super) async fn load_gameobject_spawns_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    store: &mut SpawnStore,
    gameobject_runtime_rows: &mut BTreeMap<SpawnId, GameObjectSpawnRuntimeRowLikeCpp>,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let rows = match persistence.load_gameobject_spawns_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical gameobject spawn catalog failed: {reason}")
        }
    };
    for row in rows {
        report.gameobject.rows += 1;
        let runtime_row = gameobject_row_to_runtime_row_like_cpp(&row);
        if let Some(spawn) = gameobject_row_to_spawn_data_like_cpp(
            &row,
            map_store,
            map_difficulty_store,
            &mut report.gameobject,
        ) {
            if row.event_entry != 0 {
                store.insert_spawn_metadata_like_cpp(&spawn);
                gameobject_runtime_rows.insert(row.spawn_id, runtime_row.clone());
                report.gameobject.skipped_event += 1;
            } else {
                store.add_object_spawn(&spawn, is_personal_phase_like_cpp_represented);
                gameobject_runtime_rows.insert(row.spawn_id, runtime_row.clone());
                report.gameobject.indexed += 1;
            }
        }
    }

    Ok(())
}
pub(super) async fn load_area_trigger_spawns_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    spell_exists: &mut impl FnMut(u32) -> bool,
    script_id_for_name: &mut impl FnMut(&str) -> wow_data::ScriptIdLikeCpp,
    store: &mut SpawnStore,
    area_trigger_runtime_rows: &mut BTreeMap<SpawnId, AreaTriggerSpawnRuntimeRowLikeCpp>,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let rows = match persistence.load_area_trigger_spawns_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical area-trigger spawn catalog failed: {reason}")
        }
    };
    for row in rows {
        report.area_trigger.rows += 1;
        if let Some(spawn) = area_trigger_row_to_spawn_data_like_cpp(
            &row,
            map_store,
            map_difficulty_store,
            area_trigger_template_store,
            spell_exists,
            script_id_for_name,
            area_trigger_runtime_rows,
            &mut report.area_trigger,
        ) {
            store.add_area_trigger_spawn(&spawn);
            report.area_trigger.indexed += 1;
        }
    }

    Ok(())
}
pub(super) async fn load_linked_respawns_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    store: &SpawnStore,
    map_store: &wow_data::MapStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<LinkedRespawnStoreLikeCpp> {
    let mut linked_store = LinkedRespawnStoreLikeCpp::new();
    let rows = match persistence.load_linked_respawns_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical linked-respawn catalog failed: {reason}")
        }
    };
    for row in rows {
        apply_linked_respawn_row_like_cpp(
            linked_respawn_row_like_cpp(row),
            store,
            map_store,
            &mut linked_store,
            &mut report.linked_respawn,
        );
    }

    Ok(linked_store)
}
pub(super) fn apply_linked_respawn_row_like_cpp(
    row: LinkedRespawnRowLikeCpp,
    store: &SpawnStore,
    map_store: &wow_data::MapStore,
    linked_store: &mut LinkedRespawnStoreLikeCpp,
    report: &mut LinkedRespawnLoadReportLikeCpp,
) {
    report.rows += 1;
    let Some(link_type) = LinkedRespawnTypeLikeCpp::from_raw(row.link_type) else {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::InvalidType,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: None,
            master_type: None,
            slave_map_id: None,
            master_map_id: None,
        });
        return;
    };

    let slave_type = link_type.slave_type();
    let master_type = link_type.master_type();
    let Some(slave) = store.spawn_data(slave_type, row.guid) else {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::MissingSlave,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: Some(slave_type),
            master_type: Some(master_type),
            slave_map_id: None,
            master_map_id: None,
        });
        return;
    };
    let Some(master) = store.spawn_data(master_type, row.linked_guid) else {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::MissingMaster,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: Some(slave_type),
            master_type: Some(master_type),
            slave_map_id: Some(slave.map_id),
            master_map_id: None,
        });
        return;
    };

    if map_store
        .get(master.map_id)
        .is_none_or(|map| !map_entry_instanceable_like_cpp(*map))
        || master.map_id != slave.map_id
    {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::NotInstanceableOrMapMismatch,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: Some(slave_type),
            master_type: Some(master_type),
            slave_map_id: Some(slave.map_id),
            master_map_id: Some(master.map_id),
        });
        return;
    }

    if !spawn_difficulties_intersect_like_cpp(slave, master) {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::DifficultyMismatch,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: Some(slave_type),
            master_type: Some(master_type),
            slave_map_id: Some(slave.map_id),
            master_map_id: Some(master.map_id),
        });
        return;
    }

    linked_store.insert_like_cpp(
        spawn_data_guid_like_cpp(slave),
        spawn_data_guid_like_cpp(master),
    );
    report.inserted += 1;
}
pub(super) fn spawn_difficulties_intersect_like_cpp(left: &SpawnData, right: &SpawnData) -> bool {
    left.spawn_difficulties
        .iter()
        .any(|difficulty| right.spawn_difficulties.contains(difficulty))
}
pub(super) fn spawn_data_guid_like_cpp(spawn: &SpawnData) -> ObjectGuid {
    let high = match spawn.object_type {
        SpawnObjectType::Creature => HighGuid::Creature,
        SpawnObjectType::GameObject => HighGuid::GameObject,
        SpawnObjectType::AreaTrigger => HighGuid::AreaTrigger,
    };
    ObjectGuid::create_world_object(
        high,
        0,
        0,
        spawn.map_id as u16,
        0,
        spawn.id,
        spawn.spawn_id as i64,
    )
}
pub(super) fn map_entry_instanceable_like_cpp(map: wow_data::MapEntry) -> bool {
    matches!(
        map.instance_type,
        wow_data::map::MAP_INSTANCE
            | wow_data::map::MAP_RAID
            | wow_data::map::MAP_BATTLEGROUND
            | wow_data::map::MAP_ARENA
            | wow_data::map::MAP_SCENARIO
    )
}

pub(super) fn creature_row_to_spawn_data_like_cpp(
    row: &CreatureSpawnRow,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    report: &mut SpawnKindLoadReport,
) -> Option<SpawnData> {
    object_row_to_spawn_data_like_cpp(
        SpawnObjectType::Creature,
        row.spawn_id,
        row.entry,
        row.map_id,
        row.x,
        row.y,
        row.z,
        row.orientation,
        row.spawn_time_secs,
        &row.spawn_difficulties,
        row.pool_id,
        row.phase_use_flags,
        row.phase_id,
        row.phase_group,
        row.terrain_swap_map,
        &row.script_name,
        &row.string_id,
        map_store,
        map_difficulty_store,
        report,
    )
}

pub(super) fn creature_row_to_runtime_row_like_cpp(
    row: &CreatureSpawnRow,
) -> CreatureSpawnRuntimeRowLikeCpp {
    CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: row.spawn_id,
        model_id: row.model_id,
        equipment_id: row.equipment_id,
        wander_distance: row.wander_distance,
        curhealth: row.curhealth,
        curmana: row.curmana,
        movement_type: row.movement_type,
        npc_flags: row.npc_flags,
        unit_flags: row.unit_flags,
        unit_flags2: row.unit_flags2,
        unit_flags3: row.unit_flags3,
        ground_movement_type: row.ground_movement_type,
        swim_allowed: row.swim_allowed,
        flight_movement_type: row.flight_movement_type,
        rooted: row.rooted,
        chase_movement_type: row.chase_movement_type,
        random_movement_type: row.random_movement_type,
        interaction_pause_timer_ms: row.interaction_pause_timer_ms,
        string_id: row.string_id.clone(),
        spawn_time_secs: row.spawn_time_secs,
    }
}

pub(super) fn normalize_creature_spawn_equipment_id_like_cpp(
    row: &mut CreatureSpawnRow,
    equipment_store: &wow_data::CreatureEquipmentStoreLikeCpp,
) {
    // C++ `ObjectMgr::LoadCreatureData`: `-1` means random equipment, `0` means
    // no equipment, and any non-zero id missing from `creature_equip_template`
    // is normalized back to no equipment before `Creature::LoadFromDB`.
    if row.equipment_id == 0 {
        return;
    }

    let mut equipment_id = row.equipment_id;
    if equipment_store
        .get_equipment_info_like_cpp(row.entry, &mut equipment_id, wow_core::urand_like_cpp)
        .is_some()
    {
        row.equipment_id = equipment_id;
    } else {
        row.equipment_id = 0;
    }
}

pub(super) fn gameobject_row_to_runtime_row_like_cpp(
    row: &GameObjectSpawnRow,
) -> GameObjectSpawnRuntimeRowLikeCpp {
    GameObjectSpawnRuntimeRowLikeCpp {
        spawn_id: row.spawn_id,
        rotation: row.rotation,
        anim_progress: row.anim_progress,
        state: row.state,
        string_id: row.string_id.clone(),
        spawn_time_secs: row.spawn_time_secs,
    }
}

pub(super) fn gameobject_row_to_spawn_data_like_cpp(
    row: &GameObjectSpawnRow,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    report: &mut SpawnKindLoadReport,
) -> Option<SpawnData> {
    object_row_to_spawn_data_like_cpp(
        SpawnObjectType::GameObject,
        row.spawn_id,
        row.entry,
        row.map_id,
        row.x,
        row.y,
        row.z,
        row.orientation,
        row.spawn_time_secs,
        &row.spawn_difficulties,
        row.pool_id,
        row.phase_use_flags,
        row.phase_id,
        row.phase_group,
        row.terrain_swap_map,
        &row.script_name,
        &row.string_id,
        map_store,
        map_difficulty_store,
        report,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn object_row_to_spawn_data_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
    entry: u32,
    map_id: u32,
    x: f32,
    y: f32,
    z: f32,
    orientation: f32,
    spawn_time_secs: i32,
    spawn_difficulties: &str,
    pool_id: u32,
    phase_use_flags: u8,
    phase_id: u32,
    phase_group: u32,
    terrain_swap_map: i32,
    script_name: &str,
    string_id: &str,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    report: &mut SpawnKindLoadReport,
) -> Option<SpawnData> {
    if map_store.get(map_id).is_none() {
        report.skipped_missing_map += 1;
        return None;
    }
    if !is_valid_map_coord_like_cpp(x, y, z, orientation) {
        report.skipped_invalid_position += 1;
        return None;
    }

    let is_transport = is_transport_map_like_cpp_represented(map_id);
    let parsed = parse_spawn_difficulties_like_cpp(
        spawn_difficulties,
        map_id,
        is_transport,
        map_difficulty_store,
    );
    if parsed.difficulties.is_empty() {
        report.skipped_empty_difficulties += 1;
        return None;
    }

    report.validation_skipped += 1;
    if !script_name.is_empty() {
        report.script_id_unresolved += 1;
    }

    Some(SpawnData {
        object_type,
        spawn_id,
        map_id,
        db_data: true,
        spawn_group: default_spawn_group_like_cpp(is_transport),
        id: entry,
        spawn_point: SpawnPosition::new(x, y, z, orientation),
        phase_use_flags,
        phase_id,
        phase_group,
        terrain_swap_map,
        pool_id,
        spawn_time_secs,
        spawn_difficulties: parsed.difficulties,
        script_id: 0,
        string_id: string_id.to_string(),
    })
}

pub(super) fn area_trigger_row_to_spawn_data_like_cpp(
    row: &AreaTriggerSpawnRow,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    spell_exists: &mut impl FnMut(u32) -> bool,
    script_id_for_name: &mut impl FnMut(&str) -> wow_data::ScriptIdLikeCpp,
    area_trigger_runtime_rows: &mut BTreeMap<SpawnId, AreaTriggerSpawnRuntimeRowLikeCpp>,
    report: &mut SpawnKindLoadReport,
) -> Option<SpawnData> {
    let create_properties_id = wow_data::AreaTriggerIdLikeCpp {
        id: row.create_properties_id,
        is_custom: row.is_custom,
    };
    let Some(create_properties) =
        area_trigger_template_store.get_create_properties_like_cpp(create_properties_id)
    else {
        report.skipped_invalid_create_properties.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    };
    if create_properties.flags
        != wow_data::area_trigger_template::AREATRIGGER_CREATE_PROPERTIES_FLAG_NONE_LIKE_CPP
    {
        report.skipped_nonzero_create_properties_flags.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if create_properties.scale_curve_id != 0
        || create_properties.morph_curve_id != 0
        || create_properties.facing_curve_id != 0
        || create_properties.move_curve_id != 0
    {
        report.skipped_create_properties_curves.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if create_properties.time_to_target != 0
        || create_properties.time_to_target_scale != 0
        || create_properties.facing_curve_id != 0
        || create_properties.move_curve_id != 0
    {
        report.skipped_create_properties_time_to_target.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if create_properties.orbit_info.is_some() {
        report.skipped_create_properties_orbit.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if create_properties.spline_points.len() >= 2 {
        report.skipped_create_properties_splines.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if map_store.get(row.map_id).is_none() {
        report.skipped_missing_map += 1;
        return None;
    }
    if !is_valid_map_coord_like_cpp(row.x, row.y, row.z, row.orientation) {
        report.skipped_invalid_position += 1;
        return None;
    }

    let parsed = parse_spawn_difficulties_like_cpp(
        &row.spawn_difficulties,
        row.map_id,
        is_transport_map_like_cpp_represented(row.map_id),
        map_difficulty_store,
    );
    if parsed.difficulties.is_empty() {
        report.skipped_empty_difficulties += 1;
        return None;
    }

    let spell_for_visuals = match row.spell_for_visuals {
        Some(spell_id) if spell_id >= 0 && spell_exists(spell_id as u32) => Some(spell_id),
        Some(spell_id) => {
            report
                .corrected_invalid_spell_for_visuals
                .push((row.spawn_id, spell_id));
            None
        }
        None => None,
    };
    let script_id = script_id_for_name(&row.script_name).0;

    area_trigger_runtime_rows.insert(
        row.spawn_id,
        AreaTriggerSpawnRuntimeRowLikeCpp {
            spawn_id: row.spawn_id,
            create_properties_id,
            spell_for_visuals,
        },
    );

    Some(SpawnData {
        object_type: SpawnObjectType::AreaTrigger,
        spawn_id: row.spawn_id,
        map_id: row.map_id,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::legacy_group(),
        id: row.create_properties_id,
        spawn_point: SpawnPosition::new(row.x, row.y, row.z, row.orientation),
        phase_use_flags: row.phase_use_flags,
        phase_id: row.phase_id,
        phase_group: row.phase_group,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: parsed.difficulties,
        script_id,
        string_id: String::new(),
    })
}

pub(super) fn parse_spawn_difficulties_like_cpp(
    difficulty_string: &str,
    map_id: u32,
    is_transport_map: bool,
    map_difficulty_store: &wow_data::MapDifficultyStore,
) -> ParsedSpawnDifficulties {
    let mut difficulties = Vec::new();
    let mut report = SpawnDifficultyParseReport {
        invalid_tokens_as_none: 0,
        unsupported: Vec::new(),
    };

    for token in difficulty_string
        .split(',')
        .filter(|token| !token.is_empty())
    {
        let difficulty = match token.parse::<Difficulty>() {
            Ok(difficulty) => difficulty,
            Err(_) => {
                report.invalid_tokens_as_none += 1;
                DIFFICULTY_NONE_LIKE_CPP
            }
        };

        if !is_transport_map && map_difficulty_store.get(map_id, difficulty).is_none() {
            report.unsupported.push(difficulty);
            continue;
        }

        difficulties.push(difficulty);
    }

    difficulties.sort_unstable();
    ParsedSpawnDifficulties {
        difficulties,
        report,
    }
}

pub(super) fn default_spawn_group_like_cpp(is_transport_map: bool) -> SpawnGroupTemplateData {
    if is_transport_map {
        SpawnGroupTemplateData::legacy_group()
    } else {
        SpawnGroupTemplateData::default_group()
    }
}

pub(super) fn is_valid_map_coord_like_cpp(x: f32, y: f32, z: f32, orientation: f32) -> bool {
    Position::new(x, y, z, orientation).is_valid_map_coord_like_cpp()
}

pub(super) fn is_personal_phase_like_cpp_represented(phase_id: u32) -> bool {
    // C++ checks `PhaseEntryFlags::Personal` via `PhasingHandler::IsPersonalPhase`.
    // Phase DB2 flag lookup is not available in this metadata-only loader yet, so
    // this keeps the predicate isolated and intentionally conservative.
    phase_id & PERSONAL_PHASE_FLAG_LIKE_CPP != 0
}

pub(super) fn is_transport_map_like_cpp_represented(map_id: u32) -> bool {
    // C++ `ObjectMgr::_transportMaps` is populated while validating
    // GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT/GARRISON_BUILDING templates. RustyCore
    // has no canonical transport-map store yet; keep the fallback explicit so a
    // later transport-template slice can replace only this predicate.
    TRANSPORT_MAP_IDS_REPRESENTED.contains(&map_id)
}
