//! World-state startup catalog decoding and composition.

use std::collections::BTreeSet;

use anyhow::{Result, bail};

use super::*;
pub use wow_entities::{
    WorldStateMgrLikeCpp, WorldStateSetValueOutcomeLikeCpp, WorldStateTemplateLikeCpp,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldStateMgrLoadReportLikeCpp {
    pub template_rows: u32,
    pub templates_loaded: u32,
    pub skipped_invalid_map_list: u32,
    pub skipped_invalid_area_list: u32,
    pub realm_area_requirements_ignored: u32,
    pub saved_rows: u32,
    pub saved_applied: u32,
    pub saved_skipped_unknown: u32,
}

pub fn from_db_rows_like_cpp(
    template_rows: impl IntoIterator<Item = WorldStateDbTemplateRowLikeCpp>,
    saved_values: impl IntoIterator<Item = (i32, i32)>,
    map_exists: impl Fn(i32) -> bool,
    area_continent_id: impl Fn(u32) -> Option<u16>,
) -> (WorldStateMgrLikeCpp, WorldStateMgrLoadReportLikeCpp) {
    let mut validated_templates = Vec::new();
    let mut report = WorldStateMgrLoadReportLikeCpp::default();

    for row in template_rows {
        report.template_rows += 1;
        let map_ids = parse_world_state_map_ids_like_cpp(row.id, &row.map_ids_csv, &map_exists);
        if !row.map_ids_csv.is_empty() && map_ids.is_empty() {
            report.skipped_invalid_map_list += 1;
            continue;
        }

        let mut area_ids = BTreeSet::new();
        if !map_ids.is_empty() {
            area_ids = parse_world_state_area_ids_like_cpp(
                row.id,
                &row.area_ids_csv,
                &map_ids,
                &area_continent_id,
            );
            if !row.area_ids_csv.is_empty() && area_ids.is_empty() {
                report.skipped_invalid_area_list += 1;
                continue;
            }
        } else if !row.area_ids_csv.is_empty() {
            report.realm_area_requirements_ignored += 1;
        }

        validated_templates.push(WorldStateTemplateLikeCpp {
            id: row.id,
            default_value: row.default_value,
            map_ids,
            area_ids,
            script_name: row.script_name,
        });
        report.templates_loaded += 1;
    }

    let saved_values = saved_values.into_iter().collect::<Vec<_>>();
    let world_state_mgr = WorldStateMgrLikeCpp::from_templates_and_saved_values(
        validated_templates,
        saved_values.iter().copied(),
    );
    for &(world_state_id, _) in &saved_values {
        report.saved_rows += 1;
        if world_state_mgr.template_like_cpp(world_state_id).is_some() {
            report.saved_applied += 1;
        } else {
            report.saved_skipped_unknown += 1;
        }
    }

    (world_state_mgr, report)
}

fn parse_world_state_map_ids_like_cpp(
    _world_state_id: i32,
    map_ids_csv: &str,
    map_exists: &impl Fn(i32) -> bool,
) -> BTreeSet<i32> {
    let mut map_ids = BTreeSet::new();
    for token in map_ids_csv.split(',').filter(|token| !token.is_empty()) {
        let Ok(map_id) = token.trim().parse::<i32>() else {
            continue;
        };
        if map_id != -1 && !map_exists(map_id) {
            continue;
        }
        map_ids.insert(map_id);
    }
    map_ids
}

fn parse_world_state_area_ids_like_cpp(
    _world_state_id: i32,
    area_ids_csv: &str,
    map_ids: &BTreeSet<i32>,
    area_continent_id: &impl Fn(u32) -> Option<u16>,
) -> BTreeSet<u32> {
    let mut area_ids = BTreeSet::new();
    for token in area_ids_csv.split(',').filter(|token| !token.is_empty()) {
        let Ok(area_id) = token.trim().parse::<u32>() else {
            continue;
        };
        let Some(continent_id) = area_continent_id(area_id) else {
            continue;
        };
        if !map_ids.contains(&i32::from(continent_id)) {
            continue;
        }
        area_ids.insert(area_id);
    }
    area_ids
}

pub async fn load_world_state_mgr_like_cpp(
    persistence: &dyn WorldStateStartupPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    area_table_store: &wow_data::AreaTableStore,
) -> Result<(WorldStateMgrLikeCpp, WorldStateMgrLoadReportLikeCpp)> {
    let catalog = match persistence.load_world_then_character_like_cpp().await {
        WorldStateStartupLoadOutcomeLikeCpp::Loaded(catalog) => catalog,
        WorldStateStartupLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("WorldState startup catalog failed: {reason}")
        }
    };
    let saved_values = catalog
        .saved_values
        .into_iter()
        .map(|row| (row.id, row.value));

    Ok(from_db_rows_like_cpp(
        catalog.templates,
        saved_values,
        |map_id| {
            u32::try_from(map_id)
                .ok()
                .is_some_and(|map_id| map_store.get(map_id).is_some())
        },
        |area_id| area_table_store.get(area_id).map(|area| area.continent_id),
    ))
}
