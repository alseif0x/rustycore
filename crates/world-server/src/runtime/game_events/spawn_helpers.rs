//! Dormant owned admission for the existing loaded-grid materialization family.
//!
//! The Record builders remain the sole LoadFromDB/Create implementation. These
//! operations consume their complete output before any primary store insertion;
//! they never promote a Record already installed by a legacy pool/catalog driver.
//! Production still uses its original Record insertion and legacy mirror paths.
//! Prepared construction precedes Fresh admission; legacy mirror construction
//! currently follows Record AddToMap. This new dormant ownership contract does
//! not assert that switching those production timings is already equivalent.

use crate::LoadedGridCreatureRespawnCachesLikeCpp;
use crate::runtime::map::{LoadedGridCreaturePreparationError, build_creature_spawn_records};
use crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp;
use crate::spawn_store_loader::WaypointPathStoreLikeCpp;
use wow_entities::{AccessorObjectKind, MapObjectRecord};
use wow_map::map::{AddToMapError, AddToMapOutcome, LoadedGridRespawnRecordsLikeCpp};
use wow_map::{FreshCreatureActorAdmission, FreshCreatureActorAdmissionError};
use wow_world::map_manager::WorldCreature;

/// Owns one newly materialized actor and every preceding loaded-grid facet.
/// No Clone: the caller must admit it or explicitly retain/drop its payload.
#[derive(Debug)]
pub(crate) struct PreparedLoadedGridCreature {
    pre_add_records: Vec<MapObjectRecord>,
    actor: WorldCreature,
}

/// Pre-add outcomes retain their original order, including represented errors.
/// Fresh admission retains the complete incoming actor on rejection/duplicates.
#[derive(Debug)]
pub(crate) struct PreparedLoadedGridCreatureOutcome {
    pub(crate) pre_add: Vec<Result<AddToMapOutcome, AddToMapError>>,
    pub(crate) primary:
        Result<FreshCreatureActorAdmission, (FreshCreatureActorAdmissionError, WorldCreature)>,
}

/// Grid, GameEvent, pool and condition spawn use the same no-timer builder.
/// Their existing callers still own metadata/difficulty/grid/plan gates; this
/// operation is the exact loader boundary after those gates, before insertion.
pub(crate) fn prepare_spawn_creature(
    map: &mut wow_map::Map,
    spawn_id: wow_map::SpawnId,
    metadata: &CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Result<Option<PreparedLoadedGridCreature>, LoadedGridCreaturePreparationError> {
    let Some(records) = build_creature_spawn_records(
        map,
        wow_map::SpawnObjectType::Creature,
        spawn_id,
        metadata,
        caches,
    )?
    else {
        return Ok(None);
    };
    prepare_loaded_grid_creature(records, metadata.waypoint_paths_like_cpp())
        .map_err(LoadedGridCreaturePreparationError::NotCreature)
        .map(Some)
}

pub(crate) fn prepare_loaded_grid_creature(
    records: LoadedGridRespawnRecordsLikeCpp,
    waypoint_paths: &WaypointPathStoreLikeCpp,
) -> Result<PreparedLoadedGridCreature, LoadedGridRespawnRecordsLikeCpp> {
    // A generic Creature-kind WorldObject, Player, Pet or GameObject is not a
    // Creature factory output. Preserve all owned facets before any effects.
    if records.primary_record.kind() != AccessorObjectKind::Creature
        || records.primary_record.creature().is_none()
    {
        return Err(records);
    }
    let LoadedGridRespawnRecordsLikeCpp {
        pre_add_records,
        primary_record,
    } = records;
    let creature = match primary_record.into_creature() {
        Ok(creature) => creature,
        Err(primary_record) => {
            return Err(LoadedGridRespawnRecordsLikeCpp {
                pre_add_records,
                primary_record,
            });
        }
    };
    // Reuse the exact create_data/default-motion/waypoint constructor currently
    // used by mirror_loaded_grid_creature_to_legacy_like_cpp, moving the inner.
    let actor = WorldCreature::from_loaded_grid_canonical_like_cpp(creature, |path_id| {
        waypoint_paths.get(path_id).cloned()
    });
    Ok(PreparedLoadedGridCreature {
        pre_add_records,
        actor,
    })
}

impl PreparedLoadedGridCreature {
    pub(crate) fn admit(self, map: &mut wow_map::Map) -> PreparedLoadedGridCreatureOutcome {
        let (pre_add, primary) = map
            .admit_loaded_grid_materialization(wow_map::LoadedGridMaterialization::creature(
                self.pre_add_records,
                self.actor,
            ))
            .into_parts();
        let wow_map::LoadedGridPrimaryAdmission::CreatureActor(primary) = primary else {
            unreachable!("prepared creature constructs Actor materialization")
        };
        PreparedLoadedGridCreatureOutcome { pre_add, primary }
    }
}

#[cfg(test)]
mod tests;
