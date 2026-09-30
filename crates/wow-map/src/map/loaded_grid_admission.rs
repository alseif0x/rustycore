//! One ordered admission operation for loaded-grid Record and Actor payloads.
//!
//! Planning, timer decisions and materialization remain with their current
//! callers. Only the compatibility Record branch takes its existing snapshot;
//! the Actor branch moves its complete motor through Fresh admission.

use super::{AddToMapError, AddToMapOutcome, FreshCreatureActorAdmission,
    FreshCreatureActorAdmissionError, GridLifecycle, LoadedGridRespawnRecordsLikeCpp,
    Map, TerrainGridLoader};
use crate::map_manager::WorldCreature;
use wow_entities::MapObjectRecord;

mod owned;
pub use owned::{LoadedGridAttemptPlan, LoadedGridPoolOutcome,
    LoadedGridRespawnOutcome,
    LoadedGridConditionOutcome, LoadedGridSpawnAttempt,
    LoadedGridSpawnAttemptResult, LoadedGridSpawnOutcome};
pub(in crate::map) use owned::LoadedGridReceipts;

#[derive(Debug)]
pub struct LoadedGridMaterialization {
    pre_add_records: Vec<MapObjectRecord>,
    primary: LoadedGridPrimary,
}

#[derive(Debug)]
enum LoadedGridPrimary {
    Record(MapObjectRecord),
    CreatureActor(WorldCreature),
}

impl LoadedGridMaterialization {
    pub fn records(records: LoadedGridRespawnRecordsLikeCpp) -> Self {
        Self {
            pre_add_records: records.pre_add_records,
            primary: LoadedGridPrimary::Record(records.primary_record),
        }
    }

    pub fn creature(pre_add_records: Vec<MapObjectRecord>, actor: WorldCreature) -> Self {
        Self { pre_add_records, primary: LoadedGridPrimary::CreatureActor(actor) }
    }
}

#[derive(Debug)]
pub enum LoadedGridPrimaryAdmission {
    Record {
        snapshot: MapObjectRecord,
        result: Result<AddToMapOutcome, AddToMapError>,
    },
    CreatureActor(Result<FreshCreatureActorAdmission, (FreshCreatureActorAdmissionError, WorldCreature)>),
}

#[derive(Debug)]
pub struct LoadedGridAdmission {
    pre_add: Vec<Result<AddToMapOutcome, AddToMapError>>,
    primary: LoadedGridPrimaryAdmission,
}

impl LoadedGridAdmission {
    pub fn into_parts(self) -> (
        Vec<Result<AddToMapOutcome, AddToMapError>>, LoadedGridPrimaryAdmission,
    ) {
        (self.pre_add, self.primary)
    }

    // These three compatibility drivers construct only Record materialization.
    // Keep this projection inside Map; no external flow uses its invariant.
    pub(in crate::map) fn into_record_parts(self) -> (
        Vec<Result<AddToMapOutcome, AddToMapError>>, MapObjectRecord,
        Result<AddToMapOutcome, AddToMapError>,
    ) {
        let (pre_add, primary) = self.into_parts();
        match primary {
            LoadedGridPrimaryAdmission::Record { snapshot, result } => (pre_add, snapshot, result),
            LoadedGridPrimaryAdmission::CreatureActor(_) => {
                unreachable!("Record compatibility drivers construct Record materialization")
            }
        }
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn admit_loaded_grid_materialization(
        &mut self,
        materialization: LoadedGridMaterialization,
    ) -> LoadedGridAdmission {
        let mut pre_add = Vec::with_capacity(materialization.pre_add_records.len());
        for record in materialization.pre_add_records {
            // Failure consumes/drops this facet just as before; subsequent
            // facets and the primary still execute. Return every ordered result.
            pre_add.push(self.add_map_object_record_to_map_like_cpp(record));
        }
        let primary = match materialization.primary {
            LoadedGridPrimary::Record(record) => {
                // Original timing: AFTER all pre-adds, BEFORE primary AddToMap.
                let snapshot = record.clone();
                let result = self.add_map_object_record_to_map_like_cpp(record);
                LoadedGridPrimaryAdmission::Record { snapshot, result }
            }
            LoadedGridPrimary::CreatureActor(actor) => {
                LoadedGridPrimaryAdmission::CreatureActor(self.admit_fresh_creature_actor(actor))
            }
        };
        LoadedGridAdmission { pre_add, primary }
    }
}

#[cfg(test)]
mod tests;
