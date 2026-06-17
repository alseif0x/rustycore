// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr` graveyard-zone links.

use std::collections::HashMap;

use anyhow::Result;
use wow_constants::ConditionSourceType;
use wow_constants::ConditionType;
use wow_database::{WorldDatabase, WorldStatements};

use crate::{ConditionEntriesByTypeStore, ConditionId, ConditionsReference, WorldSafeLocStore};

pub const HORDE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP: u32 = 10;
pub const ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP: u32 = 4;
pub const TEAM_HORDE_LIKE_CPP: u32 = 67;
pub const TEAM_ALLIANCE_LIKE_CPP: u32 = 469;

#[derive(Debug, Clone, Default)]
pub struct GraveyardData {
    pub safe_loc_id: u32,
    pub conditions: ConditionsReference,
}

#[derive(Debug, Clone, Default)]
pub struct GraveyardStore {
    by_zone: HashMap<u32, Vec<GraveyardData>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraveyardZoneRow {
    pub safe_loc_id: u32,
    pub ghost_zone_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraveyardLoadReport {
    pub loaded: usize,
    pub missing_safe_locs: Vec<GraveyardZoneRow>,
    pub missing_zones: Vec<GraveyardZoneRow>,
    pub duplicates: Vec<GraveyardZoneRow>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraveyardConditionAttachmentReport {
    pub attached_condition_count: usize,
    pub missing_graveyards: Vec<ConditionId>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraveyardLookupContextLikeCpp {
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub team: u32,
    pub parent_map_id: Option<u32>,
    pub corpse_map_id: Option<u32>,
    pub is_battleground_or_arena: bool,
}

impl GraveyardStore {
    pub fn graveyards_for_zone(&self, zone_id: u32) -> Option<&[GraveyardData]> {
        self.by_zone.get(&zone_id).map(Vec::as_slice)
    }

    pub fn find_graveyard_data_like_cpp(
        &self,
        safe_loc_id: u32,
        zone_id: u32,
    ) -> Option<&GraveyardData> {
        self.by_zone
            .get(&zone_id)?
            .iter()
            .find(|data| data.safe_loc_id == safe_loc_id)
    }

    fn find_graveyard_data_mut_like_cpp(
        &mut self,
        safe_loc_id: u32,
        zone_id: u32,
    ) -> Option<&mut GraveyardData> {
        self.by_zone
            .get_mut(&zone_id)?
            .iter_mut()
            .find(|data| data.safe_loc_id == safe_loc_id)
    }

    /// C++ `ObjectMgr::AddGraveyardLink`, without DB persistence side effects.
    pub fn add_graveyard_link_like_cpp(&mut self, safe_loc_id: u32, zone_id: u32) -> bool {
        if self
            .find_graveyard_data_like_cpp(safe_loc_id, zone_id)
            .is_some()
        {
            return false;
        }

        self.by_zone
            .entry(zone_id)
            .or_default()
            .push(GraveyardData {
                safe_loc_id,
                conditions: ConditionsReference::default(),
            });
        true
    }

    /// C++ `ObjectMgr::LoadGraveyardZones`.
    pub fn load_graveyard_zones_from_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = GraveyardZoneRow>,
        mut world_safe_loc_exists: impl FnMut(u32) -> bool,
        mut area_exists: impl FnMut(u32) -> bool,
    ) -> GraveyardLoadReport {
        self.by_zone.clear();
        let mut report = GraveyardLoadReport::default();

        for row in rows {
            if !world_safe_loc_exists(row.safe_loc_id) {
                report.missing_safe_locs.push(row);
                continue;
            }

            if !area_exists(row.ghost_zone_id) {
                report.missing_zones.push(row);
                continue;
            }

            if self.add_graveyard_link_like_cpp(row.safe_loc_id, row.ghost_zone_id) {
                report.loaded += 1;
            } else {
                report.duplicates.push(row);
            }
        }

        report
    }

    pub async fn load_graveyard_zones_like_cpp(
        &mut self,
        db: &WorldDatabase,
        world_safe_loc_exists: impl FnMut(u32) -> bool,
        area_exists: impl FnMut(u32) -> bool,
    ) -> Result<GraveyardLoadReport> {
        let stmt = db.prepare(WorldStatements::SEL_GRAVEYARD_ZONE);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            self.by_zone.clear();
            return Ok(GraveyardLoadReport::default());
        }

        let mut rows = Vec::new();
        loop {
            rows.push(GraveyardZoneRow {
                safe_loc_id: result.read(0),
                ghost_zone_id: result.read(1),
            });
            if !result.next_row() {
                break;
            }
        }

        Ok(self.load_graveyard_zones_from_rows_like_cpp(rows, world_safe_loc_exists, area_exists))
    }

    /// C++ `ConditionMgr::addToGraveyardData`.
    pub fn attach_graveyard_conditions_like_cpp(
        &mut self,
        conditions: &ConditionEntriesByTypeStore,
    ) -> GraveyardConditionAttachmentReport {
        let mut report = GraveyardConditionAttachmentReport::default();
        let Some(graveyard_conditions) =
            conditions.entries_for_source_type_like_cpp(ConditionSourceType::Graveyard)
        else {
            return report;
        };

        for (id, condition_bucket) in graveyard_conditions {
            let mut found = false;
            if let Ok(safe_loc_id) = u32::try_from(id.source_entry)
                && let Some(graveyard) =
                    self.find_graveyard_data_mut_like_cpp(safe_loc_id, id.source_group)
            {
                graveyard.conditions = ConditionsReference::new(condition_bucket);
                report.attached_condition_count += condition_bucket.len();
                found = true;
            }

            if !found {
                report.missing_graveyards.push(*id);
            }
        }

        report
    }

    pub fn default_graveyard_safe_loc_id_like_cpp(team: u32) -> Option<u32> {
        match team {
            TEAM_HORDE_LIKE_CPP => Some(HORDE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP),
            TEAM_ALLIANCE_LIKE_CPP => Some(ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP),
            _ => None,
        }
    }

    /// C++ `ObjectMgr::GetClosestGraveyardInZone`, without a live `conditionObject`.
    ///
    /// This preserves the C++ team-condition path used when no condition object is available.
    /// `corpse_map_id` is explicit because Rust's current `MapEntry` does not yet expose the
    /// C++ `MapEntry::CorpseMapID` field used for instance-entrance graveyards.
    pub fn closest_graveyard_in_zone_like_cpp(
        &self,
        zone_id: u32,
        context: GraveyardLookupContextLikeCpp,
        world_safe_locs: &WorldSafeLocStore,
    ) -> Option<u32> {
        let Some(graveyards) = self.graveyards_for_zone(zone_id) else {
            return if !context.is_battleground_or_arena {
                Self::default_graveyard_safe_loc_id_like_cpp(context.team)
            } else {
                None
            };
        };

        let mut nearest: Option<(u32, f32)> = None;
        let mut entrance: Option<u32> = None;
        let mut far: Option<u32> = None;

        for data in graveyards {
            if !graveyard_team_conditions_met_like_cpp(data, context.team) {
                continue;
            }

            let Some(entry) = world_safe_locs.get(data.safe_loc_id) else {
                continue;
            };

            let entry_map_id = entry.map_id;
            let same_or_parent_map =
                entry_map_id == context.map_id || Some(entry_map_id) == context.parent_map_id;

            if !same_or_parent_map {
                if context.corpse_map_id != Some(entry_map_id) {
                    far = Some(data.safe_loc_id);
                    continue;
                }

                entrance = Some(data.safe_loc_id);
                continue;
            }

            let dx = entry.position.x - context.x;
            let dy = entry.position.y - context.y;
            let dz = entry.position.z - context.z;
            let dist2 = dx * dx + dy * dy + dz * dz;
            if nearest.is_none_or(|(_, nearest_dist2)| dist2 < nearest_dist2) {
                nearest = Some((data.safe_loc_id, dist2));
            }
        }

        nearest
            .map(|(safe_loc_id, _)| safe_loc_id)
            .or(entrance)
            .or(far)
    }
}

fn graveyard_team_conditions_met_like_cpp(data: &GraveyardData, team: u32) -> bool {
    if team == 0 {
        return true;
    }

    let Some(conditions) = data.conditions.upgrade() else {
        return true;
    };

    for condition in conditions.iter() {
        if condition.condition_type != ConditionType::Team {
            continue;
        }

        if condition.condition_value1 != team {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Condition, MapEntry, MapStore, WorldSafeLocRow};
    use wow_constants::ConditionType;

    fn row(safe_loc_id: u32, ghost_zone_id: u32) -> GraveyardZoneRow {
        GraveyardZoneRow {
            safe_loc_id,
            ghost_zone_id,
        }
    }

    fn map_store() -> MapStore {
        MapStore::from_entries([
            MapEntry {
                id: 1,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                instance_type: 0,
                expansion_id: 0,
                flags1: 0,
                flags2: 0,
            },
            MapEntry {
                id: 571,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                instance_type: 0,
                expansion_id: 0,
                flags1: 0,
                flags2: 0,
            },
            MapEntry {
                id: 600,
                parent_map_id: 571,
                cosmetic_parent_map_id: -1,
                instance_type: 1,
                expansion_id: 0,
                flags1: 0,
                flags2: 0,
            },
        ])
    }

    fn safe_loc_row(id: u32, map_id: u32, x: f32, y: f32, z: f32) -> WorldSafeLocRow {
        WorldSafeLocRow {
            id,
            map_id,
            x,
            y,
            z,
            facing_degrees: 0.0,
        }
    }

    fn world_safe_locs() -> WorldSafeLocStore {
        let (store, report) = WorldSafeLocStore::from_rows_like_cpp(
            [
                safe_loc_row(4, 1, 0.0, 0.0, 0.0),
                safe_loc_row(10, 1, 100.0, 100.0, 0.0),
                safe_loc_row(100, 571, 0.0, 0.0, 0.0),
                safe_loc_row(101, 571, 5.0, 0.0, 0.0),
                safe_loc_row(102, 571, 50.0, 0.0, 0.0),
                safe_loc_row(200, 1, 0.0, 0.0, 0.0),
            ],
            &map_store(),
        );
        assert_eq!(report.loaded, 6);
        store
    }

    fn lookup_context(map_id: u32, team: u32) -> GraveyardLookupContextLikeCpp {
        GraveyardLookupContextLikeCpp {
            map_id,
            x: 4.0,
            y: 0.0,
            z: 0.0,
            team,
            parent_map_id: None,
            corpse_map_id: None,
            is_battleground_or_arena: false,
        }
    }

    #[test]
    fn graveyard_zones_load_validates_and_skips_like_cpp() {
        let mut store = GraveyardStore::default();

        let report = store.load_graveyard_zones_from_rows_like_cpp(
            [row(1, 10), row(2, 10), row(1, 10), row(3, 10), row(1, 30)],
            |safe_loc_id| safe_loc_id != 3,
            |zone_id| zone_id != 30,
        );

        assert_eq!(report.loaded, 2);
        assert_eq!(report.duplicates, vec![row(1, 10)]);
        assert_eq!(report.missing_safe_locs, vec![row(3, 10)]);
        assert_eq!(report.missing_zones, vec![row(1, 30)]);
        assert!(store.find_graveyard_data_like_cpp(1, 10).is_some());
        assert!(store.find_graveyard_data_like_cpp(3, 10).is_none());
    }

    #[test]
    fn graveyard_conditions_attach_by_zone_and_safe_loc_like_cpp() {
        let mut graveyards = GraveyardStore::default();
        graveyards.load_graveyard_zones_from_rows_like_cpp([row(1, 10)], |_| true, |_| true);
        let condition = Condition {
            source_type: ConditionSourceType::Graveyard,
            source_group: 10,
            source_entry: 1,
            condition_type: ConditionType::Team,
            condition_value1: 469,
            ..Condition::default()
        };
        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition]);

        let report = graveyards.attach_graveyard_conditions_like_cpp(&store);

        assert_eq!(report.attached_condition_count, 1);
        assert!(report.missing_graveyards.is_empty());
        let graveyard = graveyards.find_graveyard_data_like_cpp(1, 10).unwrap();
        assert_eq!(graveyard.conditions.upgrade().unwrap().len(), 1);
    }

    #[test]
    fn graveyard_conditions_report_missing_links_like_cpp() {
        let mut graveyards = GraveyardStore::default();
        graveyards.load_graveyard_zones_from_rows_like_cpp([row(1, 10)], |_| true, |_| true);
        let condition = Condition {
            source_type: ConditionSourceType::Graveyard,
            source_group: 10,
            source_entry: 2,
            condition_type: ConditionType::Team,
            condition_value1: 469,
            ..Condition::default()
        };
        let missing_id = condition.id_like_cpp();
        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition]);

        let report = graveyards.attach_graveyard_conditions_like_cpp(&store);

        assert_eq!(report.attached_condition_count, 0);
        assert_eq!(report.missing_graveyards, vec![missing_id]);
    }

    #[test]
    fn default_graveyard_ids_match_cpp() {
        assert_eq!(
            GraveyardStore::default_graveyard_safe_loc_id_like_cpp(TEAM_HORDE_LIKE_CPP),
            Some(10)
        );
        assert_eq!(
            GraveyardStore::default_graveyard_safe_loc_id_like_cpp(TEAM_ALLIANCE_LIKE_CPP),
            Some(4)
        );
        assert_eq!(
            GraveyardStore::default_graveyard_safe_loc_id_like_cpp(0),
            None
        );
    }

    #[test]
    fn closest_graveyard_in_zone_picks_nearest_same_map_like_cpp() {
        let mut graveyards = GraveyardStore::default();
        graveyards.load_graveyard_zones_from_rows_like_cpp(
            [row(100, 20), row(101, 20), row(102, 20)],
            |_| true,
            |_| true,
        );

        let selected = graveyards.closest_graveyard_in_zone_like_cpp(
            20,
            lookup_context(571, TEAM_HORDE_LIKE_CPP),
            &world_safe_locs(),
        );

        assert_eq!(selected, Some(101));
    }

    #[test]
    fn closest_graveyard_in_zone_applies_team_conditions_without_condition_object_like_cpp() {
        let mut graveyards = GraveyardStore::default();
        graveyards.load_graveyard_zones_from_rows_like_cpp(
            [row(100, 20), row(101, 20)],
            |_| true,
            |_| true,
        );
        let alliance_condition = Condition {
            source_type: ConditionSourceType::Graveyard,
            source_group: 20,
            source_entry: 101,
            condition_type: ConditionType::Team,
            condition_value1: TEAM_ALLIANCE_LIKE_CPP,
            ..Condition::default()
        };
        let condition_store =
            ConditionEntriesByTypeStore::from_conditions_like_cpp([alliance_condition]);
        graveyards.attach_graveyard_conditions_like_cpp(&condition_store);

        let selected = graveyards.closest_graveyard_in_zone_like_cpp(
            20,
            lookup_context(571, TEAM_HORDE_LIKE_CPP),
            &world_safe_locs(),
        );

        assert_eq!(selected, Some(100));
    }

    #[test]
    fn closest_graveyard_in_zone_falls_back_to_default_when_zone_has_no_links_like_cpp() {
        let selected = GraveyardStore::default().closest_graveyard_in_zone_like_cpp(
            20,
            lookup_context(571, TEAM_ALLIANCE_LIKE_CPP),
            &world_safe_locs(),
        );

        assert_eq!(selected, Some(ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP));
    }
}
