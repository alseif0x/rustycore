// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Area Trigger system — collision detection and teleportation.
//!
//! Handles all area trigger shapes (Sphere, Box, Cylinder, Polygon, Disk, BoundedPlane)
//! and supports teleportation destinations.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tracing::info;
use wow_core::Position;
use wow_database::{WorldDatabase, WorldStatements};

use crate::{ScriptIdLikeCpp, ScriptNameInternerLikeCpp};

/// Area trigger shape types (from AreaTriggerShapeType).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerShape {
    /// Spherical trigger (uses radius).
    Sphere = 0,
    /// Box trigger (uses extents and yaw).
    Box = 1,
    /// Polygon trigger (uses 2D vertices).
    Polygon = 3,
    /// Cylinder trigger (uses radius and height).
    Cylinder = 4,
    /// Disk trigger (uses radius, height).
    Disk = 5,
    /// Bounded plane trigger.
    BoundedPlane = 6,
}

impl TriggerShape {
    /// Convert from numeric value (WoW shape type).
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(TriggerShape::Sphere),
            1 => Some(TriggerShape::Box),
            3 => Some(TriggerShape::Polygon),
            4 => Some(TriggerShape::Cylinder),
            5 => Some(TriggerShape::Disk),
            6 => Some(TriggerShape::BoundedPlane),
            _ => None,
        }
    }
}

/// Area trigger teleport destination.
#[derive(Debug, Clone)]
pub struct AreaTriggerTeleport {
    pub id: u32,
    pub target_map: u32,
    pub target_position: Position,
}

/// Complete area trigger record with geometry and optional teleport.
#[derive(Debug, Clone)]
pub struct AreaTriggerData {
    /// Trigger spawn ID (unique identifier).
    pub trigger_id: u32,
    /// Map ID where the trigger exists.
    pub map_id: u16,
    /// Trigger center position.
    pub pos: Position,
    /// Shape type.
    pub shape: TriggerShape,
    /// Radius (for Sphere, Cylinder, Disk).
    pub radius: f32,
    /// Extents [length/2, width/2, height/2] for Box.
    pub extents: [f32; 3],
    /// Height for Cylinder, Polygon, Disk.
    pub height: f32,
    /// Yaw for Box orientation.
    pub yaw: f32,
    /// Polygon vertices (2D XY pairs) if shape is Polygon.
    pub vertices: Vec<(f32, f32)>,
    /// Optional teleport destination.
    pub teleport: Option<AreaTriggerTeleport>,
}

impl AreaTriggerData {
    /// Check if a point is inside this trigger.
    pub fn contains(&self, pos: &Position) -> bool {
        // Quick Z-check: skip if too far vertically
        match self.shape {
            TriggerShape::Sphere | TriggerShape::Disk => {
                let dz = pos.z - self.pos.z;
                if dz.abs() > self.height / 2.0 {
                    return false;
                }
            }
            TriggerShape::Cylinder => {
                let dz = pos.z - self.pos.z;
                if dz < 0.0 || dz > self.height {
                    return false;
                }
            }
            _ => {}
        }

        match self.shape {
            TriggerShape::Sphere => {
                // Simple sphere check: distance to center ≤ radius
                self.pos.is_within_dist(pos, self.radius)
            }
            TriggerShape::Box => {
                // Box check: rotate relative position by -yaw, then check bounds
                self.is_in_box(pos)
            }
            TriggerShape::Cylinder => {
                // Cylinder: 2D distance ≤ radius, Z in [0, height]
                self.pos.is_within_dist_2d(pos, self.radius)
            }
            TriggerShape::Disk => {
                // Disk: 2D distance ≤ radius, Z within half-height
                self.pos.is_within_dist_2d(pos, self.radius)
            }
            TriggerShape::Polygon => {
                // Polygon: point-in-polygon (2D), Z check already done above
                self.is_in_polygon(pos)
            }
            TriggerShape::BoundedPlane => {
                // BoundedPlane: similar to Box but on a plane
                self.is_in_box(pos)
            }
        }
    }

    /// Check if point is inside an axis-aligned box (with orientation).
    fn is_in_box(&self, pos: &Position) -> bool {
        // Relative position from center
        let dx = pos.x - self.pos.x;
        let dy = pos.y - self.pos.y;

        // Rotate by -yaw to align with box axes
        let cos_y = self.yaw.cos();
        let sin_y = self.yaw.sin();
        let rel_x = dx * cos_y + dy * sin_y;
        let rel_y = -dx * sin_y + dy * cos_y;

        // Check against extents
        rel_x.abs() <= self.extents[0]
            && rel_y.abs() <= self.extents[1]
            && (pos.z - self.pos.z).abs() <= self.extents[2]
    }

    /// Check if point is inside a 2D polygon.
    fn is_in_polygon(&self, pos: &Position) -> bool {
        if self.vertices.is_empty() {
            return false;
        }

        // Ray casting algorithm: count intersections to the right
        let px = pos.x;
        let py = pos.y;
        let mut inside = false;

        let mut j = self.vertices.len() - 1;
        for i in 0..self.vertices.len() {
            let (xi, yi) = self.vertices[i];
            let (xj, yj) = self.vertices[j];

            if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }

        inside
    }
}

/// In-memory store of all area triggers for a realm.
pub struct AreaTriggerStore {
    /// Triggers by trigger_id for fast lookup.
    triggers_by_id: HashMap<u32, AreaTriggerData>,
    /// Triggers grouped by map_id for spatial queries.
    triggers_by_map: HashMap<u16, Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaTriggerScriptRowLikeCpp {
    pub entry: u32,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaTriggerScriptLoadReportLikeCpp {
    pub loaded: usize,
    pub skipped_missing_area_trigger: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AreaTriggerScriptStoreLikeCpp {
    scripts_by_trigger_id: BTreeMap<u32, ScriptIdLikeCpp>,
}

pub struct AreaTriggerScriptLoadOutcomeLikeCpp {
    pub store: AreaTriggerScriptStoreLikeCpp,
    pub report: AreaTriggerScriptLoadReportLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TavernAreaTriggerLoadReportLikeCpp {
    pub rows_seen: usize,
    pub loaded: usize,
    pub skipped_missing_area_trigger: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TavernAreaTriggerStoreLikeCpp {
    trigger_ids: BTreeSet<u32>,
}

pub struct TavernAreaTriggerLoadOutcomeLikeCpp {
    pub store: TavernAreaTriggerStoreLikeCpp,
    pub report: TavernAreaTriggerLoadReportLikeCpp,
}

impl AreaTriggerStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            triggers_by_id: HashMap::new(),
            triggers_by_map: HashMap::new(),
        }
    }

    /// Add a trigger to the store.
    pub fn insert(&mut self, trigger: AreaTriggerData) {
        let trigger_id = trigger.trigger_id;
        let map_id = trigger.map_id;

        self.triggers_by_id.insert(trigger_id, trigger);
        self.triggers_by_map
            .entry(map_id)
            .or_insert_with(Vec::new)
            .push(trigger_id);
    }

    /// Check if a position is inside a specific trigger.
    pub fn is_point_in_trigger(&self, trigger_id: u32, pos: &Position) -> bool {
        self.triggers_by_id
            .get(&trigger_id)
            .map(|t| t.contains(pos))
            .unwrap_or(false)
    }

    /// Get a trigger by ID.
    pub fn get_trigger(&self, trigger_id: u32) -> Option<&AreaTriggerData> {
        self.triggers_by_id.get(&trigger_id)
    }

    pub fn contains_trigger_like_cpp(&self, trigger_id: u32) -> bool {
        self.triggers_by_id.contains_key(&trigger_id)
    }

    /// Get all triggers for a specific map.
    pub fn get_triggers_for_map(&self, map_id: u16) -> Vec<&AreaTriggerData> {
        self.triggers_by_map
            .get(&map_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.triggers_by_id.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check which triggers (on a specific map) contain a position.
    pub fn get_triggers_at_position(&self, map_id: u16, pos: &Position) -> Vec<&AreaTriggerData> {
        self.get_triggers_for_map(map_id)
            .into_iter()
            .filter(|t| t.contains(pos))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.triggers_by_id.len()
    }
}

impl Default for AreaTriggerStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Load all area triggers from world database.
///
/// Queries:
/// - areatrigger + areatrigger_teleport for basic data
/// - areatrigger_create_properties for geometry (shape, vertices, etc.)
pub async fn load_area_triggers(db: &WorldDatabase) -> Result<AreaTriggerStore> {
    let mut store = AreaTriggerStore::new();

    // First, load teleport destinations
    let mut teleports: HashMap<u32, AreaTriggerTeleport> = HashMap::new();
    let stmt = db.prepare(WorldStatements::SEL_AREA_TRIGGER_TELEPORT);
    let result = db.query(&stmt).await?;

    if !result.is_empty() {
        let mut result = result;
        loop {
            let id: u32 = result.read(0);
            let target_map: u32 = result.read(1);
            let target_x: f32 = result.read(2);
            let target_y: f32 = result.read(3);
            let target_z: f32 = result.read(4);
            let target_o: f32 = result.read(5);

            teleports.insert(
                id,
                AreaTriggerTeleport {
                    id,
                    target_map,
                    target_position: Position::new(target_x, target_y, target_z, target_o),
                },
            );

            if !result.next_row() {
                break;
            }
        }
    }

    info!("Loaded {} area trigger teleports", teleports.len());

    // TODO: Load triggers from areatrigger table with geometry from areatrigger_create_properties
    // For now, populate with teleport data as fallback
    for (id, teleport) in teleports {
        let trigger = AreaTriggerData {
            trigger_id: id,
            map_id: 0, // Would need to query DB
            pos: teleport.target_position,
            shape: TriggerShape::Sphere,
            radius: 5.0, // Default radius for teleport triggers
            extents: [0.0, 0.0, 0.0],
            height: 0.0,
            yaw: 0.0,
            vertices: Vec::new(),
            teleport: Some(teleport),
        };
        store.insert(trigger);
    }

    info!("Loaded {} area triggers total", store.len());
    Ok(store)
}

impl AreaTriggerScriptStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = AreaTriggerScriptRowLikeCpp>,
        mut area_trigger_exists: impl FnMut(u32) -> bool,
        script_names: &mut ScriptNameInternerLikeCpp,
    ) -> AreaTriggerScriptLoadOutcomeLikeCpp {
        let mut store = Self::default();
        let mut skipped_missing_area_trigger = Vec::new();

        for row in rows {
            if !area_trigger_exists(row.entry) {
                skipped_missing_area_trigger.push(row.entry);
                continue;
            }
            let script_id = script_names.get_script_id_like_cpp(row.script_name, true);
            store.scripts_by_trigger_id.insert(row.entry, script_id);
        }

        AreaTriggerScriptLoadOutcomeLikeCpp {
            report: AreaTriggerScriptLoadReportLikeCpp {
                loaded: store.len(),
                skipped_missing_area_trigger,
            },
            store,
        }
    }

    /// Loads C++ `ObjectMgr::LoadAreaTriggerScripts`.
    ///
    /// C++ anchors:
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:6653-6680`
    /// - validates `entry` against the authoritative `sAreaTriggerStore`
    /// - stores `entry -> GetScriptId(ScriptName)`
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        area_trigger_store: &AreaTriggerStore,
        script_names: &mut ScriptNameInternerLikeCpp,
    ) -> Result<AreaTriggerScriptLoadOutcomeLikeCpp> {
        let mut rows = Vec::new();
        let mut result = db
            .direct_query("SELECT entry, ScriptName FROM areatrigger_scripts")
            .await?;
        if !result.is_empty() {
            loop {
                rows.push(AreaTriggerScriptRowLikeCpp {
                    entry: result.try_read::<u32>(0).unwrap_or(0),
                    script_name: result.try_read::<String>(1).unwrap_or_default(),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            |entry| area_trigger_store.contains_trigger_like_cpp(entry),
            script_names,
        ))
    }

    pub fn get_script_id_like_cpp(&self, trigger_id: u32) -> Option<ScriptIdLikeCpp> {
        self.scripts_by_trigger_id.get(&trigger_id).copied()
    }

    pub fn len(&self) -> usize {
        self.scripts_by_trigger_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scripts_by_trigger_id.is_empty()
    }
}

impl TavernAreaTriggerStoreLikeCpp {
    pub fn from_ids_like_cpp(
        rows: impl IntoIterator<Item = u32>,
        mut area_trigger_exists: impl FnMut(u32) -> bool,
    ) -> TavernAreaTriggerLoadOutcomeLikeCpp {
        let mut store = Self::default();
        let mut rows_seen = 0;
        let mut skipped_missing_area_trigger = Vec::new();

        for trigger_id in rows {
            rows_seen += 1;
            if !area_trigger_exists(trigger_id) {
                skipped_missing_area_trigger.push(trigger_id);
                continue;
            }
            store.trigger_ids.insert(trigger_id);
        }

        TavernAreaTriggerLoadOutcomeLikeCpp {
            report: TavernAreaTriggerLoadReportLikeCpp {
                rows_seen,
                loaded: store.len(),
                skipped_missing_area_trigger,
            },
            store,
        }
    }

    /// Loads C++ `ObjectMgr::LoadTavernAreaTriggers`.
    ///
    /// C++ anchors:
    /// - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:6610-6643`
    /// - validates `id` against authoritative `sAreaTriggerStore`
    /// - stores a set consumed by `ObjectMgr::IsTavernAreaTrigger`
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        area_trigger_store: &AreaTriggerStore,
    ) -> Result<TavernAreaTriggerLoadOutcomeLikeCpp> {
        let mut rows = Vec::new();
        let mut result = db.direct_query("SELECT id FROM areatrigger_tavern").await?;
        if !result.is_empty() {
            loop {
                rows.push(result.try_read::<u32>(0).unwrap_or(0));
                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_ids_like_cpp(rows, |trigger_id| {
            area_trigger_store.contains_trigger_like_cpp(trigger_id)
        }))
    }

    pub fn is_tavern_area_trigger_like_cpp(&self, trigger_id: u32) -> bool {
        self.trigger_ids.contains(&trigger_id)
    }

    pub fn len(&self) -> usize {
        self.trigger_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trigger_ids.is_empty()
    }
}

#[cfg(test)]
mod area_trigger_script_tests {
    use super::*;

    fn trigger(trigger_id: u32) -> AreaTriggerData {
        AreaTriggerData {
            trigger_id,
            map_id: 1,
            pos: Position::default(),
            shape: TriggerShape::Sphere,
            radius: 5.0,
            extents: [0.0; 3],
            height: 5.0,
            yaw: 0.0,
            vertices: Vec::new(),
            teleport: None,
        }
    }

    #[test]
    fn area_trigger_script_store_validates_trigger_and_interns_script_like_cpp() {
        let mut area_triggers = AreaTriggerStore::new();
        area_triggers.insert(trigger(10));
        let mut script_names = ScriptNameInternerLikeCpp::new();

        let outcome = AreaTriggerScriptStoreLikeCpp::from_rows_like_cpp(
            [
                AreaTriggerScriptRowLikeCpp {
                    entry: 10,
                    script_name: "at_valid".to_string(),
                },
                AreaTriggerScriptRowLikeCpp {
                    entry: 11,
                    script_name: "at_missing".to_string(),
                },
            ],
            |entry| area_triggers.contains_trigger_like_cpp(entry),
            &mut script_names,
        );

        assert_eq!(outcome.report.loaded, 1);
        assert_eq!(outcome.report.skipped_missing_area_trigger, vec![11]);
        assert_eq!(
            outcome.store.get_script_id_like_cpp(10),
            Some(ScriptIdLikeCpp(1))
        );
        assert_eq!(outcome.store.get_script_id_like_cpp(11), None);
        assert_eq!(
            script_names.get_script_name_like_cpp(ScriptIdLikeCpp(1)),
            "at_valid"
        );
        assert!(script_names.is_script_database_bound_like_cpp(ScriptIdLikeCpp(1)));
        assert!(script_names.find_by_name_like_cpp("at_missing").is_none());
    }

    #[test]
    fn area_trigger_script_store_overwrites_duplicate_entry_like_cpp() {
        let mut script_names = ScriptNameInternerLikeCpp::new();

        let outcome = AreaTriggerScriptStoreLikeCpp::from_rows_like_cpp(
            [
                AreaTriggerScriptRowLikeCpp {
                    entry: 10,
                    script_name: "first".to_string(),
                },
                AreaTriggerScriptRowLikeCpp {
                    entry: 10,
                    script_name: "second".to_string(),
                },
            ],
            |entry| entry == 10,
            &mut script_names,
        );

        assert_eq!(outcome.report.loaded, 1);
        assert_eq!(
            outcome.store.get_script_id_like_cpp(10),
            Some(ScriptIdLikeCpp(2))
        );
        assert_eq!(
            script_names.get_script_name_like_cpp(ScriptIdLikeCpp(1)),
            "first"
        );
        assert_eq!(
            script_names.get_script_name_like_cpp(ScriptIdLikeCpp(2)),
            "second"
        );
    }

    #[test]
    fn tavern_area_trigger_store_validates_ids_like_cpp() {
        let mut area_triggers = AreaTriggerStore::new();
        area_triggers.insert(trigger(10));

        let outcome =
            TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([10, 11, 10], |trigger_id| {
                area_triggers.contains_trigger_like_cpp(trigger_id)
            });

        assert_eq!(outcome.report.rows_seen, 3);
        assert_eq!(outcome.report.loaded, 1);
        assert_eq!(outcome.report.skipped_missing_area_trigger, vec![11]);
        assert!(outcome.store.is_tavern_area_trigger_like_cpp(10));
        assert!(!outcome.store.is_tavern_area_trigger_like_cpp(11));
    }
}
