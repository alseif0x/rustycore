//! World-state startup catalog and mutation model.
use super::*;

/// C++-shaped subset of `WorldStateTemplate` for represented `WorldStateMgr` startup state and realm-wide `SetValue`.
///
/// This intentionally does not close `FillInitialWorldStates`, real player-area login packet filtering,
/// map-local `Map::SetWorldStateValue`, persistence, or real script dispatch. `script_hook_represented`
/// and `global_message_represented` in outcomes are evidence flags only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStateTemplateLikeCpp {
    pub id: i32,
    pub default_value: i32,
    pub map_ids: BTreeSet<i32>,
    pub area_ids: BTreeSet<u32>,
    pub script_name: String,
}

impl WorldStateTemplateLikeCpp {
    pub fn realm_wide(id: i32, default_value: i32) -> Self {
        Self {
            id,
            default_value,
            map_ids: BTreeSet::new(),
            area_ids: BTreeSet::new(),
            script_name: String::new(),
        }
    }

    pub fn map_specific(
        id: i32,
        default_value: i32,
        map_ids: impl IntoIterator<Item = i32>,
    ) -> Self {
        Self {
            id,
            default_value,
            map_ids: map_ids.into_iter().collect(),
            area_ids: BTreeSet::new(),
            script_name: String::new(),
        }
    }

    pub fn with_area_ids(mut self, area_ids: impl IntoIterator<Item = u32>) -> Self {
        self.area_ids = area_ids.into_iter().collect();
        self
    }

    pub fn with_script_name(mut self, script_name: impl Into<String>) -> Self {
        self.script_name = script_name.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldStateSetValueOutcomeLikeCpp {
    RealmInsertedOrChanged {
        world_state_id: i32,
        old_value: i32,
        new_value: i32,
        hidden: bool,
        script_hook_represented: bool,
        global_message_represented: bool,
    },
    RealmUnchanged {
        world_state_id: i32,
        value: i32,
    },
    MapSpecificNoMapUnsupported {
        world_state_id: i32,
    },
}

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldStateMgrLikeCpp {
    world_state_templates: BTreeMap<i32, WorldStateTemplateLikeCpp>,
    realm_world_state_values: BTreeMap<i32, i32>,
    world_states_by_map: BTreeMap<i32, BTreeMap<i32, i32>>,
}

impl WorldStateMgrLikeCpp {
    /// Builds represented state in the same high-level order as C++ LoadFromDB:
    /// `world_state` templates/defaults first, then `world_state_value` saved overlay.
    pub fn from_templates_and_saved_values(
        templates: impl IntoIterator<Item = WorldStateTemplateLikeCpp>,
        saved_values: impl IntoIterator<Item = (i32, i32)>,
    ) -> Self {
        let mut mgr = Self::default();
        for template in templates {
            if template.map_ids.is_empty() {
                mgr.realm_world_state_values
                    .insert(template.id, template.default_value);
            } else {
                for &map_id in &template.map_ids {
                    mgr.world_states_by_map
                        .entry(map_id)
                        .or_default()
                        .insert(template.id, template.default_value);
                }
            }
            mgr.world_state_templates.insert(template.id, template);
        }
        for (world_state_id, value) in saved_values {
            if let Some(template) = mgr.world_state_templates.get(&world_state_id) {
                if template.map_ids.is_empty() {
                    mgr.realm_world_state_values.insert(world_state_id, value);
                } else {
                    for &map_id in &template.map_ids {
                        mgr.world_states_by_map
                            .entry(map_id)
                            .or_default()
                            .insert(world_state_id, value);
                    }
                }
            }
        }
        mgr
    }

    pub fn from_db_rows_like_cpp(
        template_rows: impl IntoIterator<Item = WorldStateDbTemplateRowLikeCpp>,
        saved_values: impl IntoIterator<Item = (i32, i32)>,
        map_exists: impl Fn(i32) -> bool,
        area_continent_id: impl Fn(u32) -> Option<u16>,
    ) -> (Self, WorldStateMgrLoadReportLikeCpp) {
        let mut mgr = Self::default();
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

            let template = WorldStateTemplateLikeCpp {
                id: row.id,
                default_value: row.default_value,
                map_ids,
                area_ids,
                script_name: row.script_name,
            };
            if template.map_ids.is_empty() {
                mgr.realm_world_state_values
                    .insert(template.id, template.default_value);
            } else {
                for &map_id in &template.map_ids {
                    mgr.world_states_by_map
                        .entry(map_id)
                        .or_default()
                        .insert(template.id, template.default_value);
                }
            }
            mgr.world_state_templates.insert(template.id, template);
            report.templates_loaded += 1;
        }

        for (world_state_id, value) in saved_values {
            report.saved_rows += 1;
            let Some(template) = mgr.world_state_templates.get(&world_state_id) else {
                report.saved_skipped_unknown += 1;
                continue;
            };
            if template.map_ids.is_empty() {
                mgr.realm_world_state_values.insert(world_state_id, value);
            } else {
                for &map_id in &template.map_ids {
                    mgr.world_states_by_map
                        .entry(map_id)
                        .or_default()
                        .insert(world_state_id, value);
                }
            }
            report.saved_applied += 1;
        }

        (mgr, report)
    }

    pub fn template_like_cpp(&self, world_state_id: i32) -> Option<&WorldStateTemplateLikeCpp> {
        self.world_state_templates.get(&world_state_id)
    }

    pub fn realm_value_like_cpp(&self, world_state_id: i32) -> i32 {
        self.realm_world_state_values
            .get(&world_state_id)
            .copied()
            .unwrap_or(0)
    }

    pub fn map_value_like_cpp(&self, map_id: i32, world_state_id: i32) -> i32 {
        self.world_states_by_map
            .get(&map_id)
            .and_then(|values| values.get(&world_state_id))
            .copied()
            .unwrap_or(0)
    }

    pub fn initial_world_states_for_map_like_cpp(&self, map_id: i32) -> BTreeMap<i32, i32> {
        let mut values = BTreeMap::new();
        if let Some(any_map_values) = self.world_states_by_map.get(&WORLDSTATE_ANY_MAP_LIKE_CPP) {
            values.extend(any_map_values.iter().map(|(&id, &value)| (id, value)));
        }
        if let Some(map_values) = self.world_states_by_map.get(&map_id) {
            values.extend(map_values.iter().map(|(&id, &value)| (id, value)));
        }
        values
    }

    pub fn set_value_realm_or_map_null_like_cpp(
        &mut self,
        world_state_id: i32,
        value: i32,
        hidden: bool,
    ) -> WorldStateSetValueOutcomeLikeCpp {
        let template = self.world_state_templates.get(&world_state_id);
        if template.is_some_and(|template| !template.map_ids.is_empty()) {
            return WorldStateSetValueOutcomeLikeCpp::MapSpecificNoMapUnsupported {
                world_state_id,
            };
        }

        let inserted = !self.realm_world_state_values.contains_key(&world_state_id);
        let old_value = self
            .realm_world_state_values
            .get(&world_state_id)
            .copied()
            .unwrap_or(0);
        if old_value == value && !inserted {
            return WorldStateSetValueOutcomeLikeCpp::RealmUnchanged {
                world_state_id,
                value,
            };
        }

        self.realm_world_state_values.insert(world_state_id, value);
        WorldStateSetValueOutcomeLikeCpp::RealmInsertedOrChanged {
            world_state_id,
            old_value,
            new_value: value,
            hidden,
            script_hook_represented: template.is_some(),
            global_message_represented: true,
        }
    }
}

const WORLDSTATE_ANY_MAP_LIKE_CPP: i32 = -1;

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
        if map_id != WORLDSTATE_ANY_MAP_LIKE_CPP && !map_exists(map_id) {
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
        .map(|row| (row.id, row.value))
        .collect::<Vec<_>>();

    Ok(WorldStateMgrLikeCpp::from_db_rows_like_cpp(
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
