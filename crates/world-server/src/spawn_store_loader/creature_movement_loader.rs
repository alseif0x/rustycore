//! Waypoint and creature-formation startup metadata.
use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WaypointPathLoadReportLikeCpp {
    pub path_rows: usize,
    pub paths_loaded: usize,
    pub skipped_invalid_move_type: usize,
    pub node_rows: usize,
    pub nodes_loaded: usize,
    pub skipped_missing_path: usize,
    pub empty_paths: usize,
    pub backwards_too_short: usize,
    pub clamped_delay: usize,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WaypointPathStoreLikeCpp {
    paths: BTreeMap<u32, wow_movement::WaypointPath>,
}
impl WaypointPathStoreLikeCpp {
    pub fn from_rows_like_cpp(
        path_rows: impl IntoIterator<Item = WaypointPathRowLikeCpp>,
        node_rows: impl IntoIterator<Item = WaypointPathNodeRowLikeCpp>,
    ) -> (Self, WaypointPathLoadReportLikeCpp) {
        let mut report = WaypointPathLoadReportLikeCpp::default();
        let mut paths = BTreeMap::new();

        for row in path_rows {
            report.path_rows += 1;
            let Some(move_type) = waypoint_move_type_from_db_like_cpp(row.move_type) else {
                // C++ logs and returns after `_pathStore[pathId]` has already inserted an
                // invalid enum value. Rust keeps the store typed, so invalid paths are skipped.
                report.skipped_invalid_move_type += 1;
                continue;
            };
            let mut path = wow_movement::WaypointPath::new(row.path_id, Vec::new());
            path.move_type = move_type;
            path.follow_path_backwards_from_end_to_start = row.flags & 0x01 != 0;
            paths.insert(row.path_id, path);
            report.paths_loaded += 1;
        }

        for row in node_rows {
            report.node_rows += 1;
            let Some(path) = paths.get_mut(&row.path_id) else {
                report.skipped_missing_path += 1;
                continue;
            };
            let mut x = row.x;
            let mut y = row.y;
            wow_map::normalize_map_coord(&mut x);
            wow_map::normalize_map_coord(&mut y);
            let delay_ms = match i32::try_from(row.delay) {
                Ok(delay) => delay,
                Err(_) => {
                    report.clamped_delay += 1;
                    i32::MAX
                }
            };
            let mut node = wow_movement::WaypointNode::new(row.node_id, x, y, row.z);
            node.delay_ms = delay_ms;
            if let Some(orientation) = row.orientation {
                node.orientation = Some(orientation);
            }
            path.nodes.push(node);
            report.nodes_loaded += 1;
        }

        for path in paths.values() {
            if path.nodes.is_empty() {
                report.empty_paths += 1;
            }
            if path.follow_path_backwards_from_end_to_start
                && path.nodes.len()
                    < wow_movement::WAYPOINT_PATH_FLAG_FOLLOW_PATH_BACKWARDS_MINIMUM_NODES_LIKE_CPP
            {
                report.backwards_too_short += 1;
            }
        }

        (Self { paths }, report)
    }

    pub fn get(&self, path_id: u32) -> Option<&wow_movement::WaypointPath> {
        self.paths.get(&path_id)
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}
pub fn initialize_world_creature_default_waypoint_from_store_like_cpp(
    creature: &mut wow_world::map_manager::WorldCreature,
    waypoint_paths: &WaypointPathStoreLikeCpp,
) -> wow_movement::WaypointMovementAction {
    creature.initialize_default_waypoint_movement_with_path_resolver_like_cpp(|path_id| {
        waypoint_paths.get(path_id).cloned()
    })
}
pub(super) fn waypoint_move_type_from_db_like_cpp(
    move_type: u8,
) -> Option<wow_movement::WaypointMoveType> {
    match move_type {
        0 => Some(wow_movement::WaypointMoveType::Walk),
        1 => Some(wow_movement::WaypointMoveType::Run),
        2 => Some(wow_movement::WaypointMoveType::Land),
        3 => Some(wow_movement::WaypointMoveType::TakeOff),
        _ => None,
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureFormationLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_missing_leader: usize,
    pub skipped_missing_member: usize,
    pub duplicate_member_ignored: usize,
    pub removed_missing_leader_self: usize,
}
pub fn apply_creature_formation_rows_like_cpp(
    rows: impl IntoIterator<Item = CreatureFormationRowLikeCpp>,
    store: &SpawnStore,
    report: &mut CreatureFormationLoadReportLikeCpp,
) -> BTreeMap<SpawnId, CreatureFormationInfoLikeCpp> {
    let mut formations = BTreeMap::new();
    let mut leader_spawn_ids = std::collections::BTreeSet::new();

    for row in rows {
        report.rows += 1;
        if store
            .spawn_data(SpawnObjectType::Creature, row.leader_spawn_id)
            .is_none()
        {
            report.skipped_missing_leader += 1;
            continue;
        }
        if store
            .spawn_data(SpawnObjectType::Creature, row.member_spawn_id)
            .is_none()
        {
            report.skipped_missing_member += 1;
            continue;
        }
        leader_spawn_ids.insert(row.leader_spawn_id);
        if formations.contains_key(&row.member_spawn_id) {
            report.duplicate_member_ignored += 1;
            continue;
        }

        let (follow_dist, follow_angle_radians) = if row.leader_spawn_id == row.member_spawn_id {
            (0.0, 0.0)
        } else {
            (row.dist, row.angle_degrees * std::f32::consts::PI / 180.0)
        };
        formations.insert(
            row.member_spawn_id,
            CreatureFormationInfoLikeCpp {
                leader_spawn_id: row.leader_spawn_id,
                follow_dist,
                follow_angle_radians,
                group_ai: row.group_ai,
                leader_waypoint_ids: [row.point_1, row.point_2],
            },
        );
        report.loaded += 1;
    }

    for leader_spawn_id in leader_spawn_ids {
        if !formations.contains_key(&leader_spawn_id) {
            let before = formations.len();
            formations.retain(|_, info| info.leader_spawn_id != leader_spawn_id);
            report.removed_missing_leader_self += before.saturating_sub(formations.len());
        }
    }
    report.loaded = formations.len();

    formations
}
pub(super) async fn load_creature_formations_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    store: &SpawnStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<BTreeMap<SpawnId, CreatureFormationInfoLikeCpp>> {
    let rows = match persistence.load_creature_formations_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical creature formation catalog failed: {reason}")
        }
    };

    Ok(apply_creature_formation_rows_like_cpp(
        rows,
        store,
        &mut report.creature_formations,
    ))
}
pub(super) async fn load_waypoint_paths_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<WaypointPathStoreLikeCpp> {
    let catalog = match persistence.load_waypoint_paths_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(catalog) => catalog,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical waypoint catalog failed: {reason}")
        }
    };

    let (store, load_report) =
        WaypointPathStoreLikeCpp::from_rows_like_cpp(catalog.paths, catalog.nodes);
    report.waypoint_paths = load_report;
    Ok(store)
}
