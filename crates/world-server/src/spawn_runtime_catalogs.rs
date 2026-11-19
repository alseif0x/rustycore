//! Canonical spawn, respawn and world-state runtime startup.

use super::*;

pub(super) struct SpawnRuntimeCatalogs {
    pub(super) script_name_interner: Arc<wow_data::ScriptNameInternerLikeCpp>,
    pub(super) game_event_persistence: Arc<dyn wow_persistence::GameEventPersistencePortLikeCpp>,
    pub(super) canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    pub(super) persisted_respawn_times: Arc<PersistedRespawnTimesLikeCpp>,
    pub(super) respawn_persistence: Arc<dyn RespawnPersistencePortLikeCpp>,
    pub(super) world_state_mgr: SharedWorldStateMgrLikeCpp,
}

pub(super) async fn load_spawn_runtime_catalogs(
    char_db: &Arc<CharacterDatabase>,
    world_db: &Arc<WorldDatabase>,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    spawn_group_store: &wow_data::SpawnGroupTemplateStore,
    creature_equipment_store: &wow_data::CreatureEquipmentStoreLikeCpp,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    spell_store: &wow_data::SpellStore,
    mut script_name_interner: wow_data::ScriptNameInternerLikeCpp,
    area_table_store: &wow_data::AreaTableStore,
) -> Result<SpawnRuntimeCatalogs> {
    let game_event_persistence: Arc<dyn wow_persistence::GameEventPersistencePortLikeCpp> =
        Arc::new(wow_database::MariaDbGameEventPersistenceAdapterLikeCpp::new(Arc::clone(char_db)));
    let game_event_world_catalog: Arc<
        dyn wow_persistence::GameEventWorldCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbGameEventWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        )),
    );
    let canonical_spawn_catalog: Arc<
        dyn wow_persistence::CanonicalSpawnCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbCanonicalSpawnCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        )),
    );
    let (canonical_spawn_metadata, canonical_spawn_report) =
        spawn_store_loader::load_canonical_spawn_store_like_cpp(
            canonical_spawn_catalog.as_ref(),
            game_event_persistence.as_ref(),
            game_event_world_catalog.as_ref(),
            map_store,
            map_difficulty_store,
            spawn_group_store,
            creature_equipment_store,
            area_trigger_template_store,
            |spell_id| spell_store.get(spell_id as i32).is_some(),
            |name| script_name_interner.get_script_id_like_cpp(name, true),
        )
        .await
        .context("Failed to load canonical SpawnStore metadata from world DB")?;
    info!(
        "Loaded canonical SpawnStore metadata: creatures rows={} indexed={} event-managed={} empty-difficulty={} missing-map={}; formations rows={} loaded={} missing-leader={} missing-member={} duplicate-member={} pruned-missing-leader-self={}; gameobjects rows={} indexed={} event-managed={} empty-difficulty={} missing-map={}; areatriggers rows={} indexed={} empty-difficulty={} missing-map={} invalid-create-properties={} flags={} curves={} time={} orbit={} splines={} invalid-spell={}; poolmgr templates rows={} loaded={} creature-members loaded={}/{} gameobject-members loaded={}/{} pool-members loaded={}/{} relation-removals={} map-mismatches={} circular={} empty={} missing-map={} autospawn loaded={}/{} skipped-empty={} skipped-broken={} skipped-child={}; spawn-group rows={} assigned={} missing-spawn={} invalid-type={} missing-group={} map-mismatch={} duplicate={}; represented validations skipped: creature={} gameobject={} areatrigger={}",
        canonical_spawn_report.creature.rows,
        canonical_spawn_report.creature.indexed,
        canonical_spawn_report.creature.skipped_event,
        canonical_spawn_report.creature.skipped_empty_difficulties,
        canonical_spawn_report.creature.skipped_missing_map,
        canonical_spawn_report.creature_formations.rows,
        canonical_spawn_report.creature_formations.loaded,
        canonical_spawn_report
            .creature_formations
            .skipped_missing_leader,
        canonical_spawn_report
            .creature_formations
            .skipped_missing_member,
        canonical_spawn_report
            .creature_formations
            .duplicate_member_ignored,
        canonical_spawn_report
            .creature_formations
            .removed_missing_leader_self,
        canonical_spawn_report.gameobject.rows,
        canonical_spawn_report.gameobject.indexed,
        canonical_spawn_report.gameobject.skipped_event,
        canonical_spawn_report.gameobject.skipped_empty_difficulties,
        canonical_spawn_report.gameobject.skipped_missing_map,
        canonical_spawn_report.area_trigger.rows,
        canonical_spawn_report.area_trigger.indexed,
        canonical_spawn_report
            .area_trigger
            .skipped_empty_difficulties,
        canonical_spawn_report.area_trigger.skipped_missing_map,
        canonical_spawn_report
            .area_trigger
            .skipped_invalid_create_properties
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_nonzero_create_properties_flags
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_create_properties_curves
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_create_properties_time_to_target
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_create_properties_orbit
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_create_properties_splines
            .len(),
        canonical_spawn_report
            .area_trigger
            .corrected_invalid_spell_for_visuals
            .len(),
        canonical_spawn_report.pool_mgr.template_rows,
        canonical_spawn_report.pool_mgr.templates_loaded,
        canonical_spawn_report.pool_mgr.creature_members.loaded,
        canonical_spawn_report.pool_mgr.creature_members.rows,
        canonical_spawn_report.pool_mgr.gameobject_members.loaded,
        canonical_spawn_report.pool_mgr.gameobject_members.rows,
        canonical_spawn_report.pool_mgr.pool_members.loaded,
        canonical_spawn_report.pool_mgr.pool_members.rows,
        canonical_spawn_report.pool_mgr.relation_removals,
        canonical_spawn_report.pool_mgr.map_mismatches,
        canonical_spawn_report.pool_mgr.circular_relations,
        canonical_spawn_report.pool_mgr.empty_pools,
        canonical_spawn_report.pool_mgr.missing_map_after_non_empty,
        canonical_spawn_report.pool_mgr.autospawn_loaded,
        canonical_spawn_report.pool_mgr.autospawn_rows,
        canonical_spawn_report.pool_mgr.autospawn_skipped_empty,
        canonical_spawn_report.pool_mgr.autospawn_skipped_broken,
        canonical_spawn_report.pool_mgr.autospawn_skipped_child,
        canonical_spawn_report.spawn_group_rows,
        canonical_spawn_report.spawn_group_apply.assigned,
        canonical_spawn_report.spawn_group_apply.missing_spawn,
        canonical_spawn_report.spawn_group_apply.invalid_type,
        canonical_spawn_report.spawn_group_apply.missing_group,
        canonical_spawn_report.spawn_group_apply.map_mismatch,
        canonical_spawn_report
            .spawn_group_apply
            .duplicate_spawn_group,
        canonical_spawn_report.creature.validation_skipped,
        canonical_spawn_report.gameobject.validation_skipped,
        canonical_spawn_report.area_trigger.validation_skipped,
    );
    let script_name_interner = Arc::new(script_name_interner);
    info!(
        "Built C++ ScriptNameContainer core from loaded template/scene/area-trigger/spawn stores: {} names ({} DB-bound)",
        script_name_interner.len_like_cpp(),
        script_name_interner.all_db_script_names_like_cpp().len()
    );
    let respawn_persistence: Arc<dyn RespawnPersistencePortLikeCpp> = Arc::new(
        wow_database::MariaDbRespawnPersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
    );
    let (persisted_respawn_times, persisted_respawn_report) =
        load_persisted_respawn_times_like_cpp(
            respawn_persistence.as_ref(),
            &canonical_spawn_metadata,
        )
        .await
        .context("Failed to load persisted respawn times from character database")?;
    let persisted_respawn_times = Arc::new(persisted_respawn_times);
    info!(
        "Loaded persisted C++ respawn timers: rows={} loaded={} maps={} timers={} invalid-type={} unsupported-areatrigger={} missing-spawn-metadata={}",
        persisted_respawn_report.rows,
        persisted_respawn_report.loaded,
        persisted_respawn_times.maps_len(),
        persisted_respawn_times.respawns_len(),
        persisted_respawn_report.invalid_type,
        persisted_respawn_report.unsupported_area_trigger,
        persisted_respawn_report.missing_spawn_metadata,
    );
    let canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp =
        Arc::new(Mutex::new(canonical_spawn_metadata));
    let world_state_startup: Arc<dyn wow_persistence::WorldStateStartupPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbWorldStateStartupPersistenceAdapterLikeCpp::new(
                Arc::clone(world_db),
                Arc::clone(char_db),
            ),
        );
    let (world_state_mgr, world_state_mgr_report) =
        spawn_store_loader::load_world_state_mgr_like_cpp(
            world_state_startup.as_ref(),
            map_store,
            area_table_store,
        )
        .await
        .context("Failed to load C++ WorldStateMgr startup state")?;
    info!(
        "Loaded C++ WorldStateMgr startup state: template rows={} loaded={} skipped-map-list={} skipped-area-list={} realm-area-ignored={} saved rows={} applied={} skipped-unknown={}",
        world_state_mgr_report.template_rows,
        world_state_mgr_report.templates_loaded,
        world_state_mgr_report.skipped_invalid_map_list,
        world_state_mgr_report.skipped_invalid_area_list,
        world_state_mgr_report.realm_area_requirements_ignored,
        world_state_mgr_report.saved_rows,
        world_state_mgr_report.saved_applied,
        world_state_mgr_report.saved_skipped_unknown,
    );
    let world_state_mgr: SharedWorldStateMgrLikeCpp = Arc::new(Mutex::new(world_state_mgr));

    Ok(SpawnRuntimeCatalogs {
        script_name_interner,
        game_event_persistence,
        canonical_spawn_metadata,
        persisted_respawn_times,
        respawn_persistence,
        world_state_mgr,
    })
}
