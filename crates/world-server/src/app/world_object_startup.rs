//! Creature addons, canonical world spawns and persisted respawn startup.

use anyhow::Context;
use std::sync::{Arc, Mutex};
use tracing::info;
use wow_persistence::RespawnPersistencePortLikeCpp;

use crate::{
    load_persisted_respawn_times_like_cpp, spawn_store_loader,
    PersistedRespawnLoadReportLikeCpp, PersistedRespawnTimesLikeCpp,
    SharedCanonicalSpawnMetadataLikeCpp,
};

pub(super) async fn load_creature_addons(
    world_object_catalog_persistence: &dyn wow_persistence::WorldObjectCatalogPersistencePortLikeCpp,
    creature_template_lifecycle_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
    creature_spawn_store: &wow_data::WorldSpawnIdStore,
    creature_display_info_store: &wow_data::CreatureDisplayInfoStore,
    emotes_store: &wow_data::EmotesStore,
    anim_kit_store: &wow_data::AnimKitStore,
    spell_store: &wow_data::SpellStore,
    spell_misc_store: &wow_data::SpellMiscStore,
    spell_duration_store: &wow_data::SpellDurationStore,
) -> anyhow::Result<Arc<wow_data::CreatureAddonStoreLikeCpp>> {
    let creature_addon_store = Arc::new(
        crate::world::object_catalog::load_creature_addons_like_cpp(
            world_object_catalog_persistence,
            creature_template_lifecycle_store,
            creature_spawn_store,
            creature_display_info_store,
            emotes_store,
            anim_kit_store,
            spell_store,
            spell_misc_store,
            spell_duration_store,
        )
        .await
        .context("Failed to load represented creature_addon / creature_template_addon rows for C++ Creature::LoadCreaturesAddon")?,
    );
    info!(
        "Loaded {} represented creature addon rows",
        creature_addon_store.len()
    );
    Ok(creature_addon_store)
}

pub(super) struct WorldSpawnStartup {
    pub(super) persisted_respawn_report: PersistedRespawnLoadReportLikeCpp,
    pub(super) canonical_spawn_report: spawn_store_loader::CanonicalSpawnStoreLoadReport,
    pub(super) respawn_persistence: Arc<dyn RespawnPersistencePortLikeCpp>,
    pub(super) canonical_spawn_catalog: Arc<dyn wow_persistence::CanonicalSpawnCatalogPersistencePortLikeCpp>,
    pub(super) game_event_world_catalog: Arc<dyn wow_persistence::GameEventWorldCatalogPersistencePortLikeCpp>,
    pub(super) game_event_persistence: Arc<dyn wow_persistence::GameEventPersistencePortLikeCpp>,
    pub(super) script_name_interner: Arc<wow_data::ScriptNameInternerLikeCpp>,
    pub(super) persisted_respawn_times: Arc<PersistedRespawnTimesLikeCpp>,
    pub(super) canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    pub(super) creature_equipment_store: Arc<wow_data::CreatureEquipmentStoreLikeCpp>,
    pub(super) item_stats_store: Arc<wow_data::ItemStatsStore>,
    pub(super) item_modified_appearance_store: Arc<wow_data::ItemModifiedAppearanceStore>,
    pub(super) item_appearance_store: Arc<wow_data::ItemAppearanceStore>,
}

pub(super) async fn load_world_spawns(
    data_dir: &str,
    locale: &str,
    char_db: &Arc<wow_database::CharacterDatabase>,
    world_db: &Arc<wow_database::WorldDatabase>,
    world_object_catalog_persistence: &dyn wow_persistence::WorldObjectCatalogPersistencePortLikeCpp,
    creature_template_lifecycle_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    spawn_group_store: &wow_data::SpawnGroupTemplateStore,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    spell_store: &wow_data::SpellStore,
    mut script_name_interner: wow_data::ScriptNameInternerLikeCpp,
) -> anyhow::Result<WorldSpawnStartup> {
    // Load item appearance/equipment dependencies before canonical SpawnStore metadata.
    // C++ `ObjectMgr::LoadCreatureData` validates `creature.equipment_id` through
    // `ObjectMgr::GetEquipmentInfo`, including `-1` random selection, while loading
    // CreatureData.
    let item_appearance_store = Arc::new(
        wow_data::ItemAppearanceStore::load(data_dir, locale)
            .context("Failed to load ItemAppearance.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item appearances from ItemAppearance.db2",
        item_appearance_store.len()
    );
    let item_modified_appearance_store = Arc::new(
        wow_data::ItemModifiedAppearanceStore::load(data_dir, locale).context(
            "Failed to load ItemModifiedAppearance.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item modified appearances from ItemModifiedAppearance.db2",
        item_modified_appearance_store.len()
    );
    let item_stats_store = Arc::new(
        wow_data::ItemStatsStore::load(data_dir, locale)
            .context("Failed to load ItemSparse.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} items with stat modifiers from ItemSparse.db2",
        item_stats_store.len()
    );
    let creature_equipment_store = Arc::new(
        crate::world::object_catalog::load_creature_equipment_like_cpp(
            world_object_catalog_persistence,
            |entry| creature_template_lifecycle_store.get(entry).is_some(),
            |item_id| {
                item_stats_store
                    .sparse_template(item_id)
                    .map(|template| template.inventory_type as u8)
            },
            |item_id, appearance_mod_id| {
                item_modified_appearance_store
                    .get_for_item(item_id, appearance_mod_id)
                    .is_some()
            },
            |item_id| {
                item_modified_appearance_store
                    .get_default_for_item(item_id)
                    .and_then(|entry| u16::try_from(entry.item_appearance_modifier_id).ok())
            },
        )
        .await
        .context("Failed to load C++ creature equipment templates")?,
    );
    info!(
        "Loaded {} C++ creature equipment templates",
        creature_equipment_store.len()
    );

    let game_event_persistence: Arc<dyn wow_persistence::GameEventPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbGameEventPersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
        );
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
            creature_equipment_store.as_ref(),
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
    Ok(WorldSpawnStartup {
        item_appearance_store,
        item_modified_appearance_store,
        item_stats_store,
        creature_equipment_store,
        canonical_spawn_metadata,
        persisted_respawn_times,
        script_name_interner,
        game_event_persistence,
        game_event_world_catalog,
        canonical_spawn_catalog,
        respawn_persistence,
        canonical_spawn_report,
        persisted_respawn_report,
    })
}
