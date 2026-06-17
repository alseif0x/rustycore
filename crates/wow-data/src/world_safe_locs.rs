// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadWorldSafeLocs`.

use std::collections::HashMap;

use anyhow::Result;
use wow_core::Position;
use wow_database::{WorldDatabase, WorldStatements};

use crate::MapStore;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldSafeLoc {
    pub id: u32,
    pub map_id: u32,
    pub position: Position,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorldSafeLocLoadReport {
    pub loaded: usize,
    pub missing_maps: Vec<WorldSafeLocRow>,
    pub invalid_positions: Vec<WorldSafeLocRow>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldSafeLocRow {
    pub id: u32,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub facing_degrees: f32,
}

#[derive(Debug, Clone, Default)]
pub struct WorldSafeLocStore {
    by_id: HashMap<u32, WorldSafeLoc>,
}

impl WorldSafeLocStore {
    pub fn get(&self, id: u32) -> Option<&WorldSafeLoc> {
        self.by_id.get(&id)
    }

    pub fn contains(&self, id: u32) -> bool {
        self.by_id.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn load_from_rows_like_cpp(
        rows: impl IntoIterator<Item = WorldSafeLocRow>,
        map_store: &MapStore,
    ) -> WorldSafeLocLoadReport {
        let mut store = Self::default();
        let mut report = WorldSafeLocLoadReport::default();
        store.load_rows_like_cpp(rows, map_store, &mut report);
        report
    }

    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = WorldSafeLocRow>,
        map_store: &MapStore,
    ) -> (Self, WorldSafeLocLoadReport) {
        let mut store = Self::default();
        let mut report = WorldSafeLocLoadReport::default();
        store.load_rows_like_cpp(rows, map_store, &mut report);
        (store, report)
    }

    #[cfg(test)]
    pub fn from_locs_for_test(locs: impl IntoIterator<Item = WorldSafeLoc>) -> Self {
        Self {
            by_id: locs.into_iter().map(|loc| (loc.id, loc)).collect(),
        }
    }

    fn load_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = WorldSafeLocRow>,
        map_store: &MapStore,
        report: &mut WorldSafeLocLoadReport,
    ) {
        self.by_id.clear();

        for row in rows {
            if map_store.get(row.map_id).is_none() {
                report.missing_maps.push(row);
                continue;
            }

            let position = Position::new(row.x, row.y, row.z, row.facing_degrees.to_radians());
            if !position.is_valid_map_coord_like_cpp() {
                report.invalid_positions.push(row);
                continue;
            }

            self.by_id.insert(
                row.id,
                WorldSafeLoc {
                    id: row.id,
                    map_id: row.map_id,
                    position,
                },
            );
        }

        report.loaded = self.by_id.len();
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        map_store: &MapStore,
    ) -> Result<(Self, WorldSafeLocLoadReport)> {
        let stmt = db.prepare(WorldStatements::SEL_WORLD_SAFE_LOCS);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok((Self::default(), WorldSafeLocLoadReport::default()));
        }

        let mut rows = Vec::new();
        loop {
            rows.push(WorldSafeLocRow {
                id: result.read(0),
                map_id: result.read(1),
                x: result.read(2),
                y: result.read(3),
                z: result.read(4),
                facing_degrees: result.read(5),
            });

            if !result.next_row() {
                break;
            }
        }

        Ok(Self::from_rows_like_cpp(rows, map_store))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MapEntry;

    fn row(id: u32, map_id: u32, x: f32, y: f32, z: f32, facing_degrees: f32) -> WorldSafeLocRow {
        WorldSafeLocRow {
            id,
            map_id,
            x,
            y,
            z,
            facing_degrees,
        }
    }

    #[test]
    fn world_safe_locs_load_validates_map_and_position_like_cpp() {
        let map_store = MapStore::from_entries([MapEntry {
            id: 571,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            instance_type: 0,
            expansion_id: 0,
            flags1: 0,
            flags2: 0,
        }]);

        let (store, report) = WorldSafeLocStore::from_rows_like_cpp(
            [
                row(1, 571, 10.0, 20.0, 30.0, 90.0),
                row(2, 999, 10.0, 20.0, 30.0, 90.0),
                row(3, 571, f32::INFINITY, 20.0, 30.0, 90.0),
            ],
            &map_store,
        );

        assert_eq!(report.loaded, 1);
        assert_eq!(
            report.missing_maps,
            vec![row(2, 999, 10.0, 20.0, 30.0, 90.0)]
        );
        assert_eq!(
            report.invalid_positions,
            vec![row(3, 571, f32::INFINITY, 20.0, 30.0, 90.0)]
        );
        let loc = store.get(1).unwrap();
        assert_eq!(loc.map_id, 571);
        assert_eq!(loc.position.orientation, 90.0_f32.to_radians());
        assert!(!store.contains(2));
    }
}
