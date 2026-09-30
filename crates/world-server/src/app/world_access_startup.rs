//! Ordered world access startup composition.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;
use crate::load_disable_mgr_like_cpp;

pub(super) struct WorldAccessCatalogs {
    pub(super) access_requirement_report: wow_data::AccessRequirementLoadReportLikeCpp,
    pub(super) mmap_disabled_map_ids: std::collections::HashSet<u32>,
    pub(super) disable_mgr: Arc<wow_data::DisableMgrLikeCpp>,
    pub(super) access_requirement_store: Arc<wow_data::AccessRequirementStoreLikeCpp>,
}

pub(super) async fn load(
    world_auxiliary_catalog_persistence: &dyn wow_persistence::WorldAuxiliaryCatalogPersistencePortLikeCpp,
    condition_disable_catalog_persistence: &dyn wow_persistence::ConditionDisableCatalogPersistencePortLikeCpp,
    geography: &super::geography_startup::GeographyBase,
    map_difficulty_store: &Arc<wow_data::MapDifficultyStore>,
    inventory: &super::inventory_catalogs::InventoryBaseCatalogs,
    quest_admission: &super::quest_admission_startup::QuestAdmissionCatalogs,
    condition_references: &super::condition_reference_startup::ConditionReferences,
    spell_store: &wow_data::SpellStore,
) -> anyhow::Result<WorldAccessCatalogs> {
    let access_requirement_outcome =
        crate::world::auxiliary_catalog::load_access_requirements_like_cpp(
            world_auxiliary_catalog_persistence,
            &geography.map_store,
            map_difficulty_store,
            &inventory.item_store,
            quest_admission.quest_store.as_ref(),
            condition_references.achievement_store.as_ref(),
        )
        .await
        .context("Failed to load C++ access_requirement rows")?;
    let access_requirement_store = Arc::new(access_requirement_outcome.store);
    info!(
        "Loaded {} C++ access requirement rows ({} rows seen; {} map/difficulty skips; {} reference clears)",
        access_requirement_outcome.report.loaded_rows,
        access_requirement_outcome.report.rows_seen,
        access_requirement_outcome.report.skipped_missing_map.len()
            + access_requirement_outcome
                .report
                .skipped_missing_difficulty
                .len(),
        access_requirement_outcome.report.cleared_missing_item.len()
            + access_requirement_outcome
                .report
                .cleared_missing_item2
                .len()
            + access_requirement_outcome
                .report
                .cleared_missing_quest_a
                .len()
            + access_requirement_outcome
                .report
                .cleared_missing_quest_h
                .len()
            + access_requirement_outcome
                .report
                .cleared_missing_achievement
                .len()
    );
    let disable_mgr = Arc::new(
        load_disable_mgr_like_cpp(
            condition_disable_catalog_persistence,
            &geography.map_store,
            map_difficulty_store,
            spell_store,
            quest_admission.quest_store.as_ref(),
            condition_references.criteria_store.as_ref(),
            condition_references.battlemaster_list_store.as_ref(),
        )
        .await?,
    );
    let mmap_disabled_map_ids = disable_mgr.disabled_mmap_map_ids_like_cpp();
    info!(
        "Loaded {} C++ mmap disable rows",
        mmap_disabled_map_ids.len()
    );
    Ok(WorldAccessCatalogs {
        access_requirement_store,
        disable_mgr,
        mmap_disabled_map_ids,
        access_requirement_report: access_requirement_outcome.report,
    })
}
