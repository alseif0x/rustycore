use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use tracing::info;
use wow_database::{WorldDatabase, WorldStatements};

use crate::MapStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainSwapInfo {
    pub id: u32,
    pub ui_map_phase_ids: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct TerrainSwapStore {
    terrain_swap_info_by_id: HashMap<u32, TerrainSwapInfo>,
    terrain_swap_ids_by_map: BTreeMap<u32, Vec<u32>>,
}

impl TerrainSwapStore {
    pub fn from_rows_like_cpp(
        map_store: &MapStore,
        terrain_world_maps: impl IntoIterator<Item = (u32, u32)>,
        terrain_swap_defaults: impl IntoIterator<Item = (u32, u32)>,
        mut is_ui_map_phase: impl FnMut(u32) -> bool,
    ) -> Self {
        let mut store = Self::default();

        for map in map_store.entries() {
            if map.parent_map_id != -1 {
                store
                    .terrain_swap_info_by_id
                    .entry(map.id)
                    .or_insert_with(|| TerrainSwapInfo {
                        id: map.id,
                        ui_map_phase_ids: Vec::new(),
                    });
            }
        }

        for (terrain_swap_map, ui_map_phase_id) in terrain_world_maps {
            if map_store.get(terrain_swap_map).is_none() || !is_ui_map_phase(ui_map_phase_id) {
                continue;
            }

            let terrain_swap_info = store
                .terrain_swap_info_by_id
                .entry(terrain_swap_map)
                .or_insert_with(|| TerrainSwapInfo {
                    id: terrain_swap_map,
                    ui_map_phase_ids: Vec::new(),
                });
            terrain_swap_info.ui_map_phase_ids.push(ui_map_phase_id);
        }

        for (map_id, terrain_swap_map) in terrain_swap_defaults {
            if map_store.get(map_id).is_none() || map_store.get(terrain_swap_map).is_none() {
                continue;
            }

            let terrain_swap_info = store
                .terrain_swap_info_by_id
                .entry(terrain_swap_map)
                .or_insert_with(|| TerrainSwapInfo {
                    id: terrain_swap_map,
                    ui_map_phase_ids: Vec::new(),
                });
            store
                .terrain_swap_ids_by_map
                .entry(map_id)
                .or_default()
                .push(terrain_swap_info.id);
        }

        store
    }

    pub fn terrain_swap_info(&self, terrain_swap_id: u32) -> Option<&TerrainSwapInfo> {
        self.terrain_swap_info_by_id.get(&terrain_swap_id)
    }

    pub fn terrain_swaps_for_map(&self, map_id: u32) -> &[u32] {
        self.terrain_swap_ids_by_map
            .get(&map_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn terrain_swaps_by_map_like_cpp(&self) -> impl Iterator<Item = (u32, &[u32])> + '_ {
        self.terrain_swap_ids_by_map
            .iter()
            .map(|(map_id, terrain_swaps)| (*map_id, terrain_swaps.as_slice()))
    }

    pub fn validate_spawn_terrain_swap_like_cpp(
        &self,
        map_store: &MapStore,
        spawn_map_id: u32,
        terrain_swap_map: i32,
    ) -> Option<u32> {
        let terrain_swap_map = u32::try_from(terrain_swap_map).ok()?;
        let terrain_swap_entry = map_store.get(terrain_swap_map)?;
        if terrain_swap_entry.parent_map_id != spawn_map_id as i16 {
            return None;
        }
        Some(terrain_swap_map)
    }

    pub fn terrain_swap_count(&self) -> usize {
        self.terrain_swap_info_by_id.len()
    }
}

pub async fn load_terrain_swaps(
    db: &WorldDatabase,
    map_store: &MapStore,
    is_ui_map_phase: impl FnMut(u32) -> bool,
) -> Result<TerrainSwapStore> {
    let mut terrain_world_maps = Vec::new();
    let stmt = db.prepare(WorldStatements::SEL_TERRAIN_WORLD_MAPS);
    let result = db.query(&stmt).await?;
    if !result.is_empty() {
        let mut result = result;
        loop {
            terrain_world_maps.push((result.read(0), result.read(1)));
            if !result.next_row() {
                break;
            }
        }
    }

    let mut terrain_swap_defaults = Vec::new();
    let stmt = db.prepare(WorldStatements::SEL_TERRAIN_SWAP_DEFAULTS);
    let result = db.query(&stmt).await?;
    if !result.is_empty() {
        let mut result = result;
        loop {
            terrain_swap_defaults.push((result.read(0), result.read(1)));
            if !result.next_row() {
                break;
            }
        }
    }

    let store = TerrainSwapStore::from_rows_like_cpp(
        map_store,
        terrain_world_maps,
        terrain_swap_defaults,
        is_ui_map_phase,
    );
    info!(
        "Loaded {} terrain swap definitions",
        store.terrain_swap_count()
    );
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MapEntry;

    fn map(id: u32, parent_map_id: i16) -> MapEntry {
        MapEntry {
            id,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id,
            cosmetic_parent_map_id: -1,
            flags1: 0,
        }
    }

    #[test]
    fn terrain_swap_store_initializes_parent_maps_like_cpp() {
        let map_store = MapStore::from_entries([map(571, -1), map(609, 571)]);

        let store = TerrainSwapStore::from_rows_like_cpp(&map_store, [], [], |_| true);

        assert_eq!(store.terrain_swap_count(), 1);
        assert_eq!(store.terrain_swap_info(609).map(|info| info.id), Some(609));
    }

    #[test]
    fn terrain_swap_store_loads_worldmap_ui_phase_ids_like_cpp() {
        let map_store = MapStore::from_entries([map(571, -1), map(609, 571)]);

        let store =
            TerrainSwapStore::from_rows_like_cpp(&map_store, [(609, 42), (999, 43)], [], |id| {
                id == 42
            });

        assert_eq!(
            store
                .terrain_swap_info(609)
                .map(|info| info.ui_map_phase_ids.as_slice()),
            Some([42].as_slice())
        );
        assert!(store.terrain_swap_info(999).is_none());
    }

    #[test]
    fn terrain_swap_store_loads_defaults_by_map_like_cpp() {
        let map_store = MapStore::from_entries([map(1, -1), map(571, -1), map(609, 571)]);

        let store = TerrainSwapStore::from_rows_like_cpp(
            &map_store,
            [(609, 42)],
            [(571, 609), (999, 609), (571, 998)],
            |_| true,
        );

        assert_eq!(store.terrain_swaps_for_map(571), &[609]);
        assert!(store.terrain_swaps_for_map(999).is_empty());
        assert_eq!(
            store.terrain_swaps_by_map_like_cpp().collect::<Vec<_>>(),
            vec![(571, [609].as_slice())]
        );
    }

    #[test]
    fn terrain_swap_store_validates_spawn_terrain_swap_parent_like_cpp() {
        let map_store = MapStore::from_entries([map(571, -1), map(609, 571), map(700, -1)]);
        let store = TerrainSwapStore::from_rows_like_cpp(&map_store, [], [], |_| true);

        assert_eq!(
            store.validate_spawn_terrain_swap_like_cpp(&map_store, 571, 609),
            Some(609)
        );
        assert_eq!(
            store.validate_spawn_terrain_swap_like_cpp(&map_store, 700, 609),
            None
        );
        assert_eq!(
            store.validate_spawn_terrain_swap_like_cpp(&map_store, 571, 999),
            None
        );
        assert_eq!(
            store.validate_spawn_terrain_swap_like_cpp(&map_store, 571, -1),
            None
        );
    }
}
