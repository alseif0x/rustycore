//! Canonical map respawn persistence, periodic work, and update loops.

use super::map_tick::{canonical_map_tick_begin_like_cpp, canonical_map_tick_resume_like_cpp};
use super::*;
mod creature_addon_provenance;
mod creature_record_builder;
pub(crate) use creature_addon_provenance::LoadedGridCreaturePreparationError;
pub(crate) use creature_addon_provenance::creature_addon_spell_x_spell_visual_id_like_cpp;
pub(crate) use creature_record_builder::{
    build_creature_records_with_respawn_time, build_creature_respawn_records,
    build_creature_spawn_records,
};
mod game_event_persistence_bridge;
pub(crate) use game_event_persistence_bridge::{
    GameEventQuestCompleteConditionSaveDbOperationLikeCpp,
    GameEventQuestCompleteDbBridgeSummaryLikeCpp, GameEventWorldEventStateDbBridgeSummaryLikeCpp,
    GameEventWorldEventStateDbOperationKindLikeCpp, GameEventWorldEventStateDbOperationLikeCpp,
    execute_game_event_quest_complete_condition_save_db_bridge_like_cpp,
    execute_game_event_world_event_state_db_bridge_like_cpp,
    game_event_world_event_state_db_delete_operation_like_cpp,
    game_event_world_event_state_db_save_operation_like_cpp,
    materialize_game_event_quest_complete_db_bridge_like_cpp,
    materialize_game_event_world_event_state_db_bridge_like_cpp,
};
mod respawn_projection;
pub(crate) use respawn_projection::{
    queue_respawn_db_delete_like_cpp, queue_respawn_db_save_like_cpp,
};

/// Supply the Group owner's loaded-difficulty port from the DB2 store.
///
/// `wow-social` owns the Group rules but must not depend on a data adapter, so
/// the composition root binds the port to the concrete `wow_data` store. Both
/// the trait and the store are foreign to this crate, so the binding is an
/// explicit borrowing adapter rather than a blanket impl. Every method forwards
/// to the existing C++-anchored validation unchanged.
pub(crate) struct GroupDifficultyStorePortLikeCpp<'a>(pub(crate) &'a wow_data::DifficultyStore);

impl wow_social::group::GroupDifficultyValidatorLikeCpp for GroupDifficultyStorePortLikeCpp<'_> {
    fn check_loaded_dungeon_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        self.0
            .check_loaded_dungeon_difficulty_id_like_cpp(difficulty)
    }

    fn check_loaded_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        self.0.check_loaded_raid_difficulty_id_like_cpp(difficulty)
    }

    fn check_loaded_legacy_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        self.0
            .check_loaded_legacy_raid_difficulty_id_like_cpp(difficulty)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CanonicalRespawnConditionSchedulerLikeCpp {
    pub(crate) timer_ms: u32,
    pub(crate) interval_ms: u32,
}

impl CanonicalRespawnConditionSchedulerLikeCpp {
    pub(crate) fn new(interval_ms: u32) -> Self {
        let interval_ms = interval_ms.max(1);
        Self {
            timer_ms: interval_ms,
            interval_ms,
        }
    }

    pub(crate) fn update(&mut self, diff_ms: u32) -> bool {
        if self.timer_ms <= diff_ms {
            self.timer_ms = self.interval_ms;
            true
        } else {
            self.timer_ms -= diff_ms;
            false
        }
    }

    #[cfg(test)]
    pub(crate) const fn timer_ms(&self) -> u32 {
        self.timer_ms
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RespawnDbDeleteLikeCpp {
    pub(crate) object_type: wow_map::SpawnObjectType,
    pub(crate) spawn_id: wow_map::SpawnId,
    pub(crate) map_id: u16,
    pub(crate) instance_id: u32,
    pub(crate) mutation: RespawnPersistenceMutationLikeCpp,
}

#[derive(Debug, Clone)]
pub(crate) enum RespawnDbDeleteQueueOutcomeLikeCpp {
    Queued(RespawnDbDeleteLikeCpp),
    SkippedNonWorldMap,
    SkippedInstanceableMap,
    SkippedInvalidMapId,
}

#[derive(Debug, Clone)]
pub(crate) struct RespawnDbSaveLikeCpp {
    pub(crate) object_type: wow_map::SpawnObjectType,
    pub(crate) spawn_id: wow_map::SpawnId,
    pub(crate) respawn_time: i64,
    pub(crate) map_id: u16,
    pub(crate) instance_id: u32,
    pub(crate) mutation: RespawnPersistenceMutationLikeCpp,
}

#[derive(Debug, Clone)]
pub(crate) enum RespawnDbSaveQueueOutcomeLikeCpp {
    Queued(RespawnDbSaveLikeCpp),
    SkippedNonWorldMap,
    SkippedInstanceableMap,
    SkippedInvalidMapId,
}

pub(crate) async fn load_groups_from_character_database_like_cpp(
    persistence: &dyn wow_persistence::RepresentedGroupStartupLoadPortLikeCpp,
    group_registry: &GroupRegistry,
    difficulty_store: &wow_data::DifficultyStore,
) -> Result<GroupLoadSummaryLikeCpp> {
    let (characters, groups, members) = match persistence.load_represented_groups_like_cpp().await {
        wow_persistence::RepresentedGroupStartupLoadOutcomeLikeCpp::Loaded {
            characters,
            groups,
            members,
        } => (characters, groups, members),
        wow_persistence::RepresentedGroupStartupLoadOutcomeLikeCpp::Failed { stage, reason } => {
            anyhow::bail!("represented Group startup persistence failed at {stage:?}: {reason}")
        }
    };
    let character_cache = characters
        .into_iter()
        .filter(|character| character.guid != 0)
        .map(|character| {
            (
                character.guid,
                GroupMemberCharacterLikeCpp {
                    name: character.name,
                    race: character.race,
                    class: character.class,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let group_rows = groups.into_iter().map(|row| GroupDbRowLikeCpp {
        leader_guid_low: row.leader_guid_low,
        loot_method: row.loot_method,
        looter_guid_low: row.looter_guid_low,
        loot_threshold: row.loot_threshold,
        target_icons: row.target_icons,
        group_flags: row.group_flags,
        dungeon_difficulty_id: row.dungeon_difficulty_id,
        raid_difficulty_id: row.raid_difficulty_id,
        legacy_raid_difficulty_id: row.legacy_raid_difficulty_id,
        master_looter_guid_low: row.master_looter_guid_low,
        db_store_id: row.db_store_id,
        lfg_dungeon_id: row.lfg_dungeon_id,
        lfg_state: row.lfg_state,
    });
    let member_rows = members.into_iter().map(|row| GroupMemberDbRowLikeCpp {
        db_store_id: row.db_store_id,
        member_guid_low: row.member_guid_low,
        member_flags: row.member_flags,
        subgroup: row.subgroup,
        roles: row.roles,
    });

    Ok(load_groups_from_db_rows_like_cpp(
        group_registry,
        group_rows,
        member_rows,
        &character_cache,
        &GroupDifficultyStorePortLikeCpp(difficulty_store),
    ))
}

#[derive(Clone)]
pub(crate) struct LoadedGridCreatureRespawnCachesLikeCpp {
    pub(crate) realm_id: u16,
    pub(crate) template_store: Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    pub(crate) sparring_store: Arc<wow_data::CreatureTemplateSparringStoreLikeCpp>,
    pub(crate) difficulty_store: Arc<wow_data::CreatureDifficultyStoreLikeCpp>,
    pub(crate) base_stats_store: Arc<wow_data::CreatureBaseStatsStoreLikeCpp>,
    pub(crate) chr_classes_store: Arc<wow_data::character_progression::ChrClassesStore>,
    pub(crate) power_type_store: Arc<wow_data::character_progression::PowerTypeStore>,
    pub(crate) health_rates: wow_data::CreatureClassificationHealthRatesLikeCpp,
    pub(crate) display_store: Arc<wow_data::CreatureDisplayInfoStore>,
    pub(crate) model_store: Arc<wow_data::CreatureModelDataStore>,
    pub(crate) model_info_store: Arc<wow_data::CreatureModelInfoStoreLikeCpp>,
    pub(crate) creature_equipment_store: Arc<wow_data::CreatureEquipmentStoreLikeCpp>,
    pub(crate) creature_addon_store: Arc<wow_data::CreatureAddonStoreLikeCpp>,
    pub(crate) spell_x_spell_visual_store: Arc<wow_data::SpellXSpellVisualStore>,
    pub(crate) vehicle_store: Arc<wow_data::VehicleStore>,
    pub(crate) vehicle_seat_store: Arc<wow_data::VehicleSeatStore>,
    pub(crate) vehicle_accessory_store: Arc<wow_data::VehicleAccessoryStoreLikeCpp>,
    pub(crate) gameobject_template_store: Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    pub(crate) gameobject_override_store: Arc<wow_data::GameObjectOverrideLifecycleStoreLikeCpp>,
}

pub(crate) struct MapCreatureModelSelectionRandomLikeCpp<'a, Terrain, Lifecycle>
where
    Terrain: wow_map::TerrainGridLoader,
    Lifecycle: wow_map::GridLifecycle,
{
    map: &'a mut wow_map::Map<Terrain, Lifecycle>,
}

impl<Terrain, Lifecycle> wow_data::CreatureModelSelectionRandomLikeCpp
    for MapCreatureModelSelectionRandomLikeCpp<'_, Terrain, Lifecycle>
where
    Terrain: wow_map::TerrainGridLoader,
    Lifecycle: wow_map::GridLifecycle,
{
    fn weighted_model_roll_like_cpp(&mut self, total_weight: f32) -> f32 {
        self.map.frand_exclusive_like_cpp(0.0, total_weight)
    }

    fn other_gender_roll_zero_like_cpp(&mut self) -> bool {
        self.map.urand_inclusive_like_cpp(0, 1) == 0
    }
}

impl<Terrain, Lifecycle> creature_loaded_grid::LoadedGridCreatureRandomSourceLikeCpp
    for MapCreatureModelSelectionRandomLikeCpp<'_, Terrain, Lifecycle>
where
    Terrain: wow_map::TerrainGridLoader,
    Lifecycle: wow_map::GridLifecycle,
{
    fn select_creature_level_like_cpp(&mut self, min_level: u8, max_level: u8) -> u8 {
        self.map
            .select_creature_level_like_cpp(min_level, max_level)
    }
}

pub(crate) fn build_loaded_grid_creature_respawn_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    build_creature_respawn_records(map, object_type, spawn_id, canonical_spawn_metadata, caches)
        .ok()
        .flatten()
}

pub(crate) fn build_loaded_grid_creature_spawn_group_spawn_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    build_creature_spawn_records(map, object_type, spawn_id, canonical_spawn_metadata, caches)
        .ok()
        .flatten()
}

pub(crate) fn build_loaded_grid_creature_record_with_respawn_time_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
    respawn_time: i64,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    build_creature_records_with_respawn_time(
        map,
        object_type,
        spawn_id,
        canonical_spawn_metadata,
        caches,
        respawn_time,
    )
    .ok()
    .flatten()
}

pub(crate) fn build_loaded_grid_gameobject_respawn_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    if object_type != wow_map::SpawnObjectType::GameObject {
        return None;
    }

    let Some(spawn) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, spawn_id)
    else {
        debug!(
            respawn_type = object_type as u8,
            spawn_id, "C++ loaded-grid GameObject DoRespawn blocked: missing canonical SpawnData"
        );
        return None;
    };
    let Some(runtime_row) = canonical_spawn_metadata.gameobject_runtime_row_like_cpp(spawn_id)
    else {
        debug!(
            spawn_id,
            entry = spawn.id,
            "C++ loaded-grid GameObject DoRespawn blocked: missing DB-backed gameobject runtime row"
        );
        return None;
    };
    // C++ `Map::ProcessRespawns` erases the due map-owned respawn timer before
    // `DoRespawn -> GameObject::LoadFromDB(addToMap=true)`. Therefore
    // `GetMap()->GetGORespawnTime(m_spawnId)` observes no timer and the newly
    // respawned object's effective `m_respawnTime` is 0.
    let inputs = gameobject_loaded_grid::build_loaded_grid_gameobject_inputs_from_db_like_cpp(
        spawn,
        runtime_row,
        caches.gameobject_template_store.as_ref(),
        caches.gameobject_override_store.as_ref(),
        map.instance_id(),
        0,
        true,
    );
    let (template, resolved_spawn) = match inputs {
        Ok(inputs) => inputs,
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid GameObject DoRespawn blocked: failed to compose DB-backed LoadFromDB inputs"
            );
            return None;
        }
    };

    let map_object_guid = if template.go_type == wow_entities::GAMEOBJECT_TYPE_TRANSPORT {
        let low = match map.generate_low_guid_like_cpp(HighGuid::Transport) {
            Ok(low) => low,
            Err(error) => {
                debug!(
                    ?error,
                    spawn_id,
                    entry = spawn.id,
                    "C++ loaded-grid GameObject DoRespawn blocked: map-owned Transport low-guid generation failed"
                );
                return None;
            }
        };
        ObjectGuid::create_transport(HighGuid::Transport, low)
    } else {
        let Ok(map_id) = u16::try_from(map.map_id()) else {
            warn!(
                map_id = map.map_id(),
                spawn_id,
                entry = spawn.id,
                "C++ loaded-grid GameObject DoRespawn blocked: map id does not fit ObjectGuid world-object map field"
            );
            return None;
        };
        let low = match map.generate_low_guid_like_cpp(HighGuid::GameObject) {
            Ok(low) => low,
            Err(error) => {
                debug!(
                    ?error,
                    spawn_id,
                    entry = spawn.id,
                    "C++ loaded-grid GameObject DoRespawn blocked: map-owned GameObject low-guid generation failed"
                );
                return None;
            }
        };
        ObjectGuid::create_gameobject_like_cpp(map_id, template.entry, low)
    };
    let mut linked_trap_guid = None;
    let mut resolver_templates = vec![template.clone()];
    let linked_entry = wow_entities::GameObjectTemplateData::new(template.go_type, template.data)
        .get_linked_gameobject_entry_like_cpp();
    if linked_entry != 0 && template.go_type != wow_entities::GAMEOBJECT_TYPE_TRANSPORT {
        if let Some(linked_template_record) = caches.gameobject_template_store.get(linked_entry) {
            let linked_template =
                match gameobject_loaded_grid::resolved_template_from_lifecycle_record_like_cpp(
                    linked_template_record,
                    None,
                ) {
                    Ok(linked_template)
                        if linked_template.go_type != wow_entities::GAMEOBJECT_TYPE_TRANSPORT =>
                    {
                        Some(linked_template)
                    }
                    Ok(_) => {
                        debug!(
                            spawn_id,
                            entry = spawn.id,
                            linked_entry,
                            "C++ loaded-grid GameObject linked trap skipped: linked transport template not represented by this seam"
                        );
                        None
                    }
                    Err(error) => {
                        debug!(
                            ?error,
                            spawn_id,
                            entry = spawn.id,
                            linked_entry,
                            "C++ loaded-grid GameObject linked trap skipped: linked template rejected"
                        );
                        None
                    }
                };
            if let Some(linked_template) = linked_template {
                let Ok(map_id) = u16::try_from(map.map_id()) else {
                    warn!(
                        map_id = map.map_id(),
                        spawn_id,
                        entry = spawn.id,
                        linked_entry,
                        "C++ loaded-grid GameObject linked trap skipped: map id does not fit ObjectGuid world-object map field"
                    );
                    let resolver =
                        gameobject_loaded_grid::GameObjectLoadedGridLifecycleResolverLikeCpp::new(
                            resolver_templates,
                            [resolved_spawn],
                        );
                    return match resolver
                        .resolve_loaded_grid_gameobject_like_cpp(spawn_id, map_object_guid)
                    {
                        Ok(resolved) => resolved.map_object_record.map(|primary_record| {
                            wow_map::map::LoadedGridRespawnRecordsLikeCpp {
                                pre_add_records: resolved.pre_add_records,
                                primary_record,
                            }
                        }),
                        Err(error) => {
                            debug!(
                                ?error,
                                spawn_id,
                                entry = spawn.id,
                                guid = ?map_object_guid,
                                "C++ loaded-grid GameObject DoRespawn blocked: resolver rejected loaded GameObject record"
                            );
                            None
                        }
                    };
                };
                let trap_low = match map.generate_low_guid_like_cpp(HighGuid::GameObject) {
                    Ok(low) => Some(low),
                    Err(error) => {
                        debug!(
                            ?error,
                            spawn_id,
                            entry = spawn.id,
                            linked_entry,
                            "C++ loaded-grid GameObject linked trap skipped: map-owned GameObject low-guid generation failed"
                        );
                        None
                    }
                };
                if let Some(trap_low) = trap_low {
                    linked_trap_guid = Some(ObjectGuid::create_gameobject_like_cpp(
                        map_id,
                        linked_entry,
                        trap_low,
                    ));
                    resolver_templates.push(linked_template);
                }
            }
        } else {
            debug!(
                spawn_id,
                entry = spawn.id,
                linked_entry,
                "C++ loaded-grid GameObject linked trap skipped: missing linked trap template"
            );
        }
    }
    let resolver = gameobject_loaded_grid::GameObjectLoadedGridLifecycleResolverLikeCpp::new(
        resolver_templates,
        [resolved_spawn],
    );
    match resolver.resolve_loaded_grid_gameobject_with_linked_trap_like_cpp(
        spawn_id,
        map_object_guid,
        linked_trap_guid,
    ) {
        Ok(resolved) => resolved.map_object_record.map(|primary_record| {
            wow_map::map::LoadedGridRespawnRecordsLikeCpp {
                pre_add_records: resolved.pre_add_records,
                primary_record,
            }
        }),
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                entry = spawn.id,
                guid = ?map_object_guid,
                "C++ loaded-grid GameObject DoRespawn blocked: resolver rejected loaded GameObject record"
            );
            None
        }
    }
}

#[allow(dead_code)]
pub(crate) fn build_loaded_grid_area_trigger_record_like_cpp(
    map: &mut wow_map::Map,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
) -> Option<wow_map::map::LoadedGridRespawnRecordsLikeCpp> {
    if object_type != wow_map::SpawnObjectType::AreaTrigger {
        return None;
    }

    let Some(spawn) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, spawn_id)
    else {
        debug!(
            respawn_type = object_type as u8,
            spawn_id, "C++ loaded-grid AreaTrigger load blocked: missing canonical SpawnData"
        );
        return None;
    };
    let Some(runtime_row) = canonical_spawn_metadata.area_trigger_runtime_row_like_cpp(spawn_id)
    else {
        debug!(
            spawn_id,
            create_properties_id = spawn.id,
            "C++ loaded-grid AreaTrigger load blocked: missing DB-backed area trigger runtime row"
        );
        return None;
    };
    let Some(create_properties) = area_trigger_template_store
        .get_create_properties_like_cpp(runtime_row.create_properties_id)
    else {
        debug!(
            spawn_id,
            create_properties_id = runtime_row.create_properties_id.id,
            "C++ loaded-grid AreaTrigger load blocked: missing create-properties row"
        );
        return None;
    };
    let template = create_properties
        .template_id
        .and_then(|template_id| area_trigger_template_store.get_template_like_cpp(template_id));

    match area::trigger_loaded_grid::build_loaded_grid_area_trigger_record_from_spawn_data_like_cpp(
        map,
        spawn,
        runtime_row,
        create_properties,
        template,
        0,
    ) {
        Ok(records) => Some(records),
        Err(error) => {
            debug!(
                ?error,
                spawn_id,
                create_properties_id = runtime_row.create_properties_id.id,
                "C++ loaded-grid AreaTrigger load blocked: failed to compose DB-backed LoadFromDB record"
            );
            None
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct LoadedGridAreaTriggerLoadSummaryLikeCpp {
    pub(crate) maps_evaluated: usize,
    pub(crate) loaded_grids_evaluated: usize,
    pub(crate) grid_not_loaded: usize,
    pub(crate) metadata_entries: usize,
    pub(crate) skipped_already_loaded: usize,
    pub(crate) skipped_should_not_spawn: usize,
    pub(crate) stale_index_entries: usize,
    pub(crate) skipped_difficulty_mismatch: usize,
    pub(crate) load_record_missing: usize,
    pub(crate) pre_add_records_added: usize,
    pub(crate) loaded_grid_primary_records: usize,
    pub(crate) loaded_area_trigger_guids: Vec<ObjectGuid>,
    pub(crate) add_to_map_errors: usize,
}

impl LoadedGridAreaTriggerLoadSummaryLikeCpp {
    pub(crate) fn accumulate(
        &mut self,
        grid: &wow_map::map::LoadedGridAreaTriggerRecordsSummaryLikeCpp,
    ) {
        self.grid_not_loaded += usize::from(grid.grid_not_loaded);
        self.metadata_entries += grid.metadata_entries;
        self.skipped_already_loaded += grid.skipped_already_loaded;
        self.skipped_should_not_spawn += grid.skipped_should_not_spawn;
        self.stale_index_entries += grid.stale_index_entries;
        self.skipped_difficulty_mismatch += grid.skipped_difficulty_mismatch;
        self.load_record_missing += grid.load_record_missing;
        self.pre_add_records_added += grid.pre_add_records_added;
        self.loaded_grid_primary_records += grid.loaded_grid_primary_records.len();
        self.loaded_area_trigger_guids.extend(
            grid.loaded_grid_primary_records
                .iter()
                .map(|record| record.object().guid()),
        );
        self.add_to_map_errors += grid.add_to_map_errors;
    }
}

#[allow(dead_code)]
pub(crate) fn load_loaded_grid_area_triggers_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
) -> LoadedGridAreaTriggerLoadSummaryLikeCpp {
    let mut summary = LoadedGridAreaTriggerLoadSummaryLikeCpp::default();
    manager.do_for_all_maps_mut(|managed_map| {
        summary.maps_evaluated += 1;
        let loaded_grid_coords = managed_map.map().loaded_grid_coords_like_cpp();
        summary.loaded_grids_evaluated += loaded_grid_coords.len();
        for coord in loaded_grid_coords {
            let grid_summary = managed_map
                .map_mut()
                .load_loaded_grid_area_trigger_records_like_cpp(
                    coord,
                    canonical_spawn_metadata.spawn_store(),
                    |map, object_type, spawn_id| {
                        build_loaded_grid_area_trigger_record_like_cpp(
                            map,
                            object_type,
                            spawn_id,
                            canonical_spawn_metadata,
                            area_trigger_template_store,
                        )
                    },
                );
            summary.accumulate(&grid_summary);
        }
    });
    summary
}

/// C++ `Group::UpdateReadyCheck` tick: decrements every active group's
/// ready-check timer each `tick_interval_ms` and broadcasts
/// `ReadyCheckCompleted` to connected members when the timer expires.
pub(crate) fn spawn_group_ready_check_tick_loop(
    group_registry: Arc<GroupRegistry>,
    player_registry: Arc<PlayerRegistry>,
    tick_interval_ms: u32,
) -> tokio::task::JoinHandle<()> {
    use wow_packet::ServerPacket;
    use wow_packet::packets::party::{ReadyCheckCompleted, ReadyCheckResponse, ReadyCheckStarted};

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_millis(u64::from(tick_interval_ms)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            let expired = tick_all_group_ready_checks_like_cpp(&group_registry, tick_interval_ms);

            for (group_guid, events) in expired {
                // Snapshot member txs outside the group lock.
                let recipients = if let Some(group) = group_registry.get(&group_guid) {
                    player_registry.group_presences_in_order(&group.members)
                } else {
                    continue;
                };

                // Drop the DashMap ref before sending.
                for event in &events {
                    let bytes = match *event {
                        ReadyCheckEventLikeCpp::Started {
                            party_index,
                            party_guid,
                            initiator_guid,
                            duration_ms,
                        } => ReadyCheckStarted {
                            party_index,
                            party_guid,
                            initiator_guid,
                            duration_ms,
                        }
                        .to_bytes(),
                        ReadyCheckEventLikeCpp::Response {
                            party_guid,
                            player,
                            is_ready,
                        } => ReadyCheckResponse {
                            party_guid,
                            player,
                            is_ready,
                        }
                        .to_bytes(),
                        ReadyCheckEventLikeCpp::Completed {
                            party_index,
                            party_guid,
                        } => ReadyCheckCompleted {
                            party_index,
                            party_guid,
                        }
                        .to_bytes(),
                    };

                    for recipient in &recipients {
                        let _ = player_registry
                            .send_current_packet(recipient.registration, bytes.clone());
                    }
                }
            }
        }
    })
}

mod producer_exit;
mod update_loop;
pub(crate) use producer_exit::{CanonicalMapProducerExit, stop_canonical_map_producer};
pub(crate) use update_loop::spawn_canonical_map_update_loop;

#[cfg(test)]
mod represented_group_startup_tests {
    use super::*;
    use wow_persistence::{
        PersistenceFutureLikeCpp, RepresentedGroupStartupLoadOutcomeLikeCpp,
        RepresentedGroupStartupLoadPortLikeCpp, RepresentedGroupStartupLoadStageLikeCpp,
    };

    struct FixedGroupStartupPortLikeCpp(RepresentedGroupStartupLoadOutcomeLikeCpp);

    impl RepresentedGroupStartupLoadPortLikeCpp for FixedGroupStartupPortLikeCpp {
        fn load_represented_groups_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, RepresentedGroupStartupLoadOutcomeLikeCpp> {
            let outcome = self.0.clone();
            Box::pin(async move { outcome })
        }
    }

    #[tokio::test]
    async fn typed_empty_group_startup_rows_materialize_an_empty_registry() {
        let port =
            FixedGroupStartupPortLikeCpp(RepresentedGroupStartupLoadOutcomeLikeCpp::Loaded {
                characters: Vec::new(),
                groups: Vec::new(),
                members: Vec::new(),
            });
        let registry = GroupRegistry::new();
        let summary = load_groups_from_character_database_like_cpp(
            &port,
            &registry,
            &wow_data::DifficultyStore::from_ids([]),
        )
        .await
        .unwrap();
        assert_eq!(summary, GroupLoadSummaryLikeCpp::default());
    }

    #[tokio::test]
    async fn typed_group_startup_failure_never_materializes_partial_state() {
        let port =
            FixedGroupStartupPortLikeCpp(RepresentedGroupStartupLoadOutcomeLikeCpp::Failed {
                stage: RepresentedGroupStartupLoadStageLikeCpp::Groups,
                reason: "query failed".to_owned(),
            });
        let registry = GroupRegistry::new();
        let error = load_groups_from_character_database_like_cpp(
            &port,
            &registry,
            &wow_data::DifficultyStore::from_ids([]),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("Groups: query failed"));
        assert!(registry.snapshots().is_empty());
    }
}
