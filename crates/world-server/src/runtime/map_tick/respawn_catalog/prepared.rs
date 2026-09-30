//! Dormant owned factory boundary for Catalog Creature respawns.
//!
//! The existing Catalog driver still inserts Records and mirrors legacy actors.
//! Its future admission consumer must call this before insertion, retaining its
//! original timer/pool/linked-respawn decisions and statement ownership.

use crate::LoadedGridCreatureRespawnCachesLikeCpp;
use crate::runtime::game_events::spawn_helpers::{
    PreparedLoadedGridCreature, prepare_loaded_grid_creature,
};
use crate::runtime::map::{LoadedGridCreaturePreparationError, build_creature_respawn_records};
use crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp;

pub(crate) fn prepare_catalog_creature_respawn(
    map: &mut wow_map::Map,
    spawn_id: wow_map::SpawnId,
    metadata: &CanonicalSpawnMetadataLikeCpp,
    caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Result<Option<PreparedLoadedGridCreature>, LoadedGridCreaturePreparationError> {
    // Unlike grid/event/pool/condition spawn, this loader requires the current
    // map-owned respawn timer. Do not default its time to zero or sample a clock.
    let Some(records) = build_creature_respawn_records(
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
