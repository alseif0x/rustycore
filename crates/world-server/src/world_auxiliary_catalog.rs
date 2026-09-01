//! Composition boundary for bounded ObjectMgr auxiliary catalogs.

use anyhow::{Result, bail};
use tracing::warn;
use wow_persistence::{
    AccessRequirementPersistenceRowLikeCpp, GraveyardZonePersistenceRowLikeCpp,
    SceneTemplatePersistenceRowLikeCpp, SpawnGroupTemplatePersistenceRowLikeCpp,
    TrinityStringPersistenceRowLikeCpp, WorldAuxiliaryCatalogPersistencePortLikeCpp,
    WorldAuxiliaryRowsLoadOutcomeLikeCpp,
};

fn loaded<T>(outcome: WorldAuxiliaryRowsLoadOutcomeLikeCpp<T>) -> Result<T> {
    match outcome {
        WorldAuxiliaryRowsLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        WorldAuxiliaryRowsLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_access_requirements_like_cpp(
    persistence: &dyn WorldAuxiliaryCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    item_store: &wow_data::ItemStore,
    quest_store: &wow_data::quest::QuestStore,
    achievement_store: &wow_data::Db2IdStore,
) -> Result<wow_data::AccessRequirementLoadOutcomeLikeCpp> {
    let rows = loaded(persistence.load_access_requirement_rows_like_cpp().await)?;
    let outcome = wow_data::AccessRequirementStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter()
            .map(|row: AccessRequirementPersistenceRowLikeCpp| {
                wow_data::AccessRequirementRowLikeCpp {
                    map_id: row.map_id,
                    difficulty: row.difficulty,
                    level_min: row.level_min,
                    level_max: row.level_max,
                    item: row.item,
                    item2: row.item2,
                    quest_done_a: row.quest_done_a,
                    quest_done_h: row.quest_done_h,
                    completed_achievement: row.completed_achievement,
                    quest_failed_text: row.quest_failed_text,
                }
            }),
        |map_id| map_store.get(map_id).is_some(),
        |map_id, difficulty| map_difficulty_store.get(map_id, difficulty).is_some(),
        |item_id| item_store.get(item_id).is_some(),
        |quest_id| quest_store.get(quest_id).is_some(),
        |achievement_id| achievement_store.contains(achievement_id),
    );
    log_access_requirement_report_like_cpp(&outcome.report);
    Ok(outcome)
}

fn log_access_requirement_report_like_cpp(report: &wow_data::AccessRequirementLoadReportLikeCpp) {
    for map_id in &report.skipped_missing_map {
        warn!(target: "sql.sql", "Map {map_id} referenced in `access_requirement` does not exist, skipped.");
    }
    for (map_id, difficulty) in &report.skipped_missing_difficulty {
        warn!(target: "sql.sql", "Map {map_id} referenced in `access_requirement` does not have difficulty {difficulty}, skipped");
    }
    for (map_id, difficulty, item) in &report.cleared_missing_item {
        warn!(target: "sql.sql", "Key item {item} does not exist for map {map_id} difficulty {difficulty}, removing key requirement.");
    }
    for (map_id, difficulty, item) in &report.cleared_missing_item2 {
        warn!(target: "sql.sql", "Second item {item} does not exist for map {map_id} difficulty {difficulty}, removing key requirement.");
    }
    for (map_id, difficulty, quest) in &report.cleared_missing_quest_a {
        warn!(target: "sql.sql", "Required Alliance Quest {quest} not exist for map {map_id} difficulty {difficulty}, remove quest done requirement.");
    }
    for (map_id, difficulty, quest) in &report.cleared_missing_quest_h {
        warn!(target: "sql.sql", "Required Horde Quest {quest} not exist for map {map_id} difficulty {difficulty}, remove quest done requirement.");
    }
    for (map_id, difficulty, achievement) in &report.cleared_missing_achievement {
        warn!(target: "sql.sql", "Required Achievement {achievement} not exist for map {map_id} difficulty {difficulty}, remove quest done requirement.");
    }
}

pub(super) async fn load_graveyard_zones_like_cpp(
    persistence: &dyn WorldAuxiliaryCatalogPersistencePortLikeCpp,
    store: &mut wow_data::GraveyardStore,
    world_safe_loc_exists: impl FnMut(u32) -> bool,
    area_exists: impl FnMut(u32) -> bool,
) -> Result<wow_data::GraveyardLoadReport> {
    let rows = loaded(persistence.load_graveyard_zone_rows_like_cpp().await)?;
    Ok(store.load_graveyard_zones_from_rows_like_cpp(
        rows.into_iter()
            .map(
                |row: GraveyardZonePersistenceRowLikeCpp| wow_data::GraveyardZoneRow {
                    safe_loc_id: row.safe_loc_id,
                    ghost_zone_id: row.ghost_zone_id,
                },
            ),
        world_safe_loc_exists,
        area_exists,
    ))
}

pub(super) async fn load_scene_templates_like_cpp(
    persistence: &dyn WorldAuxiliaryCatalogPersistencePortLikeCpp,
    script_names: &mut wow_data::ScriptNameInternerLikeCpp,
) -> Result<wow_data::SceneTemplateLoadOutcomeLikeCpp> {
    let rows = loaded(persistence.load_scene_template_rows_like_cpp().await)?;
    Ok(wow_data::SceneTemplateStoreLikeCpp::from_rows_like_cpp(
        rows.into_iter()
            .map(
                |row: SceneTemplatePersistenceRowLikeCpp| wow_data::SceneTemplateRowLikeCpp {
                    scene_id: row.scene_id,
                    flags: row.flags,
                    script_package_id: row.script_package_id,
                    encrypted: row.encrypted,
                    script_name: row.script_name,
                },
            ),
        script_names,
    ))
}

pub(super) async fn load_spawn_group_templates_like_cpp(
    persistence: &dyn WorldAuxiliaryCatalogPersistencePortLikeCpp,
) -> Result<(
    wow_data::SpawnGroupTemplateStore,
    wow_data::SpawnGroupTemplateLoadReport,
)> {
    let rows = loaded(persistence.load_spawn_group_template_rows_like_cpp().await)?;
    Ok(wow_data::SpawnGroupTemplateStore::from_rows_like_cpp(
        rows.into_iter()
            .map(
                |row: SpawnGroupTemplatePersistenceRowLikeCpp| wow_data::SpawnGroupTemplateRow {
                    group_id: row.group_id,
                    name: row.name,
                    flags: row.flags,
                },
            ),
    ))
}

pub(super) async fn load_trinity_strings_like_cpp(
    persistence: &dyn WorldAuxiliaryCatalogPersistencePortLikeCpp,
) -> Result<wow_data::TrinityStringStoreLikeCpp> {
    let rows = loaded(persistence.load_trinity_string_rows_like_cpp().await)?;
    Ok(wow_data::TrinityStringStoreLikeCpp::from_entries_like_cpp(
        rows.into_iter()
            .map(
                |row: TrinityStringPersistenceRowLikeCpp| wow_data::TrinityStringEntryLikeCpp {
                    entry: row.entry,
                    content: row.content,
                },
            ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_source_does_not_publish_an_empty_catalog() {
        let error = loaded::<Vec<TrinityStringPersistenceRowLikeCpp>>(
            WorldAuxiliaryRowsLoadOutcomeLikeCpp::Failed {
                reason: "read failed".into(),
            },
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "read failed");
    }
}
