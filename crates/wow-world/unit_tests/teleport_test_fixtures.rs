//! Explicit catalog inputs for teleport scenarios, not a production fallback.
use std::sync::Arc;
use wow_data::{MapEntry, MapStore};

pub(crate) fn world_maps<const N: usize>(ids: [u32; N]) -> Arc<MapStore> {
    Arc::new(MapStore::from_entries(ids.map(|id| MapEntry {
        id,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    })))
}
