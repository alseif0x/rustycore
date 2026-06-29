//! `GridMapTerrain` — a real, file-backed terrain provider for a single map id.
//!
//! Faithful port of the height-query half of C++ `TerrainInfo` (`TerrainMgr.cpp`)
//! for WoW 3.4.3: it owns the per-grid `.map` tiles for one map and answers
//! `GetGridHeight` / `GetStaticHeight` / `Map::GetHeight`. It plugs into [`Map`]
//! through the [`TerrainGridLoader`] + [`MapWorldObjectEnvironment`] seams so the
//! `WorldObject -> WorldObjectEnvironment -> Map -> terrain` chain returns real
//! ground heights instead of the [`NoopTerrainGridLoader`] sentinel.
//!
//! Scope (issue [03]/#15): **grid terrain height only**. VMap, liquid level, the
//! dynamic GO-floor tree, and mmaps are out of scope here — exactly the branches
//! that, in C++, would otherwise contribute to `GetStaticHeight`/`GetGameObjectFloor`.
//! Those collapse to "no value" the same way C++ does when VMap/liquid are
//! disabled, so the height returned is the raw `.map` surface (+ holes).
//!
//! [`Map`]: crate::map::Map
//! [`NoopTerrainGridLoader`]: crate::map::NoopTerrainGridLoader

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use wow_core::Position;
use wow_entities::{INVALID_HEIGHT, LineOfSightQuery, WorldObject, WorldObjectHeightQuery};

use crate::grid_map::GridMap;
use crate::map::{MapWorldObjectEnvironment, TerrainGridLoader};

// C++ Grids/GridDefines.h
const SIZE_OF_GRIDS: f32 = 533.333_3;
const CENTER_GRID_ID: i32 = 32;
const MAX_NUMBER_OF_GRIDS: i32 = 64;

/// C++ `VMAP_INVALID_HEIGHT_VALUE` (`IVMapManager.h`): the value returned for an
/// unknown height (distinct from the `INVALID_HEIGHT` *check* threshold).
pub const VMAP_INVALID_HEIGHT_VALUE: f32 = -200_000.0;

/// C++ `GROUND_HEIGHT_TOLERANCE` (`SharedDefines.h`): slack when deciding whether
/// the probe `z` is at/above the raw ground surface.
const GROUND_HEIGHT_TOLERANCE: f32 = 0.05;

/// File-backed terrain for one map id: a lazily-populated `[64][64]` grid of
/// `.map` height tiles read from `<data_dir>/maps/{mapId:04}_{gx:02}_{gy:02}.map`.
///
/// Mirrors `TerrainInfo`'s ownership: one instance per map id, shared across that
/// map's instances. Tiles load on first touch (C++ `GetGrid(..., loadIfMissing)`)
/// and also eagerly from [`TerrainGridLoader::load_map_and_vmap`] when the owning
/// [`Map`](crate::map::Map) activates a grid.
#[derive(Debug)]
pub struct GridMapTerrain {
    map_id: u32,
    data_dir: PathBuf,
    /// Key = raw tile index `(gx, gy)`; value `Some` = loaded, `None` = tried and
    /// absent/invalid (cached negative so we do not re-stat every query). An
    /// absent key means "not yet attempted".
    grids: Mutex<HashMap<(i32, i32), Option<GridMap>>>,
}

impl GridMapTerrain {
    /// Build terrain for `map_id` rooted at `data_dir` (the `DataDir` config; the
    /// tiles live under `<data_dir>/maps/`). No I/O happens until the first query
    /// or grid load.
    #[must_use]
    pub fn new(map_id: u32, data_dir: impl AsRef<Path>) -> Self {
        Self {
            map_id,
            data_dir: data_dir.as_ref().to_path_buf(),
            grids: Mutex::new(HashMap::new()),
        }
    }

    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    /// Raw TrinityCore tile index for world `(x, y)` — `TerrainInfo::GetGrid`
    /// (`TerrainMgr.cpp:284`): `gx = (int)(CENTER_GRID_ID - x / SIZE_OF_GRIDS)`.
    fn tile_index(x: f32, y: f32) -> (i32, i32) {
        let gx = (CENTER_GRID_ID as f32 - x / SIZE_OF_GRIDS) as i32;
        let gy = (CENTER_GRID_ID as f32 - y / SIZE_OF_GRIDS) as i32;
        (gx, gy)
    }

    fn tile_path(&self, gx: i32, gy: i32) -> PathBuf {
        self.data_dir
            .join("maps")
            .join(format!("{:04}_{gx:02}_{gy:02}.map", self.map_id))
    }

    fn load_tile(&self, gx: i32, gy: i32) -> Option<GridMap> {
        if !(0..MAX_NUMBER_OF_GRIDS).contains(&gx) || !(0..MAX_NUMBER_OF_GRIDS).contains(&gy) {
            return None;
        }
        let bytes = std::fs::read(self.tile_path(gx, gy)).ok()?;
        GridMap::parse(&bytes)
    }

    /// Ensure the tile covering `(gx, gy)` is in the cache (positive or negative),
    /// then run `f` against it. Centralises the lazy-load + lock discipline.
    fn with_tile<R>(&self, gx: i32, gy: i32, f: impl FnOnce(Option<&GridMap>) -> R) -> R {
        let mut grids = self.grids.lock().expect("terrain grid cache poisoned");
        let entry = grids
            .entry((gx, gy))
            .or_insert_with(|| self.load_tile(gx, gy));
        f(entry.as_ref())
    }

    /// `TerrainInfo::GetGridHeight` (`TerrainMgr.cpp:687`): the raw `.map` surface
    /// height at `(x, y)`, or [`VMAP_INVALID_HEIGHT_VALUE`] when there is no tile.
    #[must_use]
    pub fn grid_height(&self, x: f32, y: f32) -> f32 {
        let (gx, gy) = Self::tile_index(x, y);
        self.with_tile(gx, gy, |tile| {
            tile.map_or(VMAP_INVALID_HEIGHT_VALUE, |gm| gm.get_height(x, y))
        })
    }

    /// `TerrainInfo::GetStaticHeight` (`TerrainMgr.cpp:695`) with VMap disabled.
    ///
    /// The raw ground is only accepted when the probe `z` is at/above it (within
    /// [`GROUND_HEIGHT_TOLERANCE`]); otherwise C++ leaves `mapHeight` at the
    /// invalid sentinel. With no VMap there is no second candidate, so the result
    /// is the map surface or [`VMAP_INVALID_HEIGHT_VALUE`].
    #[must_use]
    pub fn static_height(&self, x: f32, y: f32, z: f32) -> f32 {
        let grid_height = self.grid_height(x, y);
        if z >= grid_height - GROUND_HEIGHT_TOLERANCE {
            grid_height
        } else {
            VMAP_INVALID_HEIGHT_VALUE
        }
    }
}

impl TerrainGridLoader for GridMapTerrain {
    fn load_map_and_vmap(&mut self, grid_x: u32, grid_y: u32) {
        // `Map::ensure_grid_created` passes the un-flipped raw tile indices
        // (`terrain_grid_coords`), i.e. the same `(gx, gy)` the lazy path derives
        // from world `(x, y)`. Populate the cache eagerly (C++ `LoadMapAndVMap`).
        let (gx, gy) = (grid_x as i32, grid_y as i32);
        let mut grids = self.grids.lock().expect("terrain grid cache poisoned");
        grids
            .entry((gx, gy))
            .or_insert_with(|| self.load_tile(gx, gy));
    }

    fn unload_map(&mut self, grid_x: u32, grid_y: u32) {
        let mut grids = self.grids.lock().expect("terrain grid cache poisoned");
        grids.remove(&(grid_x as i32, grid_y as i32));
    }
}

impl MapWorldObjectEnvironment for GridMapTerrain {
    fn line_of_sight(&self, _query: LineOfSightQuery<'_>) -> bool {
        // No VMap/dynamic tree in scope: C++ `Map::isInLineOfSight` with VMap
        // disabled reports clear LOS. Real occlusion arrives with the VMap port.
        true
    }

    fn map_height(
        &self,
        _object: &WorldObject,
        x: f32,
        y: f32,
        z: f32,
        _query: WorldObjectHeightQuery,
    ) -> f32 {
        // C++ `Map::GetHeight = max(GetStaticHeight, GetGameObjectFloor)`; without
        // the dynamic tree the GO floor is `VMAP_INVALID_HEIGHT_VALUE`, so the max
        // is the static (raw `.map`) height. `query.vmap` is moot with no VMap.
        self.static_height(x, y, z)
    }

    fn floor_z(&self, _object: &WorldObject, _position: Position, _max_search_dist: f32) -> f32 {
        // `Map::GetGameObjectFloor` needs the dynamic GO collision tree, which is
        // not ported. Match the noop sentinel; `WorldObject::get_floor_z` already
        // maxes this against the object's static floor.
        INVALID_HEIGHT
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use wow_constants::{TypeId, TypeMask};

    const Z_PROBE: f32 = 100_000.0; // MAX_HEIGHT-ish: always "above ground".

    /// Unique scratch dir per test; cleaned on drop.
    struct TempMapsDir(PathBuf);

    impl TempMapsDir {
        fn new() -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir()
                .join(format!("rustycore_terrain_test_{}_{n}", std::process::id()));
            std::fs::create_dir_all(dir.join("maps")).expect("create temp maps dir");
            Self(dir)
        }

        fn write_tile(&self, map_id: u32, gx: i32, gy: i32, bytes: &[u8]) {
            let path = self
                .0
                .join("maps")
                .join(format!("{map_id:04}_{gx:02}_{gy:02}.map"));
            std::fs::write(path, bytes).expect("write tile");
        }
    }

    impl Drop for TempMapsDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A constant-height float `.map` tile (flat surface at `height`).
    fn constant_float_tile(height: f32) -> Vec<u8> {
        const V9: usize = 129 * 129;
        const V8: usize = 128 * 128;
        let height_off = 44u32;
        let mut b = Vec::new();
        b.extend_from_slice(b"MAPS");
        b.extend_from_slice(&10u32.to_le_bytes()); // version
        b.extend_from_slice(&0u32.to_le_bytes()); // build
        b.extend_from_slice(&0u32.to_le_bytes()); // areaMapOffset
        b.extend_from_slice(&0u32.to_le_bytes()); // areaMapSize
        b.extend_from_slice(&height_off.to_le_bytes()); // heightMapOffset
        for _ in 0..5 {
            b.extend_from_slice(&0u32.to_le_bytes()); // height/liquid/holes offsets+sizes
        }
        b.extend_from_slice(b"MHGT");
        b.extend_from_slice(&0u32.to_le_bytes()); // flags = float
        b.extend_from_slice(&height.to_le_bytes()); // gridHeight
        b.extend_from_slice(&height.to_le_bytes()); // gridMaxHeight
        for _ in 0..V9 {
            b.extend_from_slice(&height.to_le_bytes());
        }
        for _ in 0..V8 {
            b.extend_from_slice(&height.to_le_bytes());
        }
        b
    }

    #[test]
    fn missing_tile_returns_invalid_sentinel() {
        let dir = TempMapsDir::new();
        let terrain = GridMapTerrain::new(0, &dir.0);
        // No file on disk → GetGridHeight returns the VMap-invalid sentinel.
        assert_eq!(terrain.grid_height(0.0, 0.0), VMAP_INVALID_HEIGHT_VALUE);
        // static_height likewise has no candidate.
        assert_eq!(
            terrain.static_height(0.0, 0.0, Z_PROBE),
            VMAP_INVALID_HEIGHT_VALUE
        );
    }

    #[test]
    fn loads_correct_tile_for_world_position() {
        // World (0,0) → raw tile index (32,32).
        let dir = TempMapsDir::new();
        dir.write_tile(0, 32, 32, &constant_float_tile(57.25));
        let terrain = GridMapTerrain::new(0, &dir.0);
        assert!((terrain.grid_height(0.0, 0.0) - 57.25).abs() < 1e-2);
    }

    #[test]
    fn static_height_rejects_probe_below_ground() {
        let dir = TempMapsDir::new();
        dir.write_tile(0, 32, 32, &constant_float_tile(50.0));
        let terrain = GridMapTerrain::new(0, &dir.0);
        // Probe well above ground → ground accepted.
        assert!((terrain.static_height(0.0, 0.0, 60.0) - 50.0).abs() < 1e-2);
        // Probe well below ground → C++ leaves mapHeight invalid.
        assert_eq!(
            terrain.static_height(0.0, 0.0, 10.0),
            VMAP_INVALID_HEIGHT_VALUE
        );
        // Within tolerance band → still accepted.
        assert!((terrain.static_height(0.0, 0.0, 50.0 - 0.04) - 50.0).abs() < 1e-2);
    }

    #[test]
    fn eager_load_then_unload_via_loader_seam() {
        let dir = TempMapsDir::new();
        dir.write_tile(0, 32, 32, &constant_float_tile(12.0));
        let mut terrain = GridMapTerrain::new(0, &dir.0);
        // Eager load (raw indices, as Map::ensure_grid_created supplies).
        terrain.load_map_and_vmap(32, 32);
        assert!((terrain.grid_height(0.0, 0.0) - 12.0).abs() < 1e-2);
        // Unload drops the cache entry; a fresh query re-reads from disk.
        terrain.unload_map(32, 32);
        assert!((terrain.grid_height(0.0, 0.0) - 12.0).abs() < 1e-2);
    }

    #[test]
    fn map_height_env_seam_returns_ground() {
        let dir = TempMapsDir::new();
        dir.write_tile(0, 32, 32, &constant_float_tile(33.0));
        let terrain = GridMapTerrain::new(0, &dir.0);
        let object = WorldObject::new(true, TypeId::Unit, TypeMask::UNIT);
        let h = terrain.map_height(
            &object,
            0.0,
            0.0,
            Z_PROBE,
            WorldObjectHeightQuery::default(),
        );
        assert!((h - 33.0).abs() < 1e-2);
    }

    /// Real-data smoke check (not run in CI — needs the server's `DataDir`).
    /// Run with: `cargo test -p wow-map --lib terrain::tests::real_map -- --ignored --nocapture`.
    /// Validates the parser + height query against actual extracted `.map` tiles:
    /// the human start in Elwynn Forest (map 0, ~(-8949.95, -132.49)) sits on
    /// ground near Z≈83.5 in retail/TC data.
    #[test]
    #[ignore = "requires real DataDir at /home/server/woltk-server-core/Data"]
    fn real_map_tile_returns_plausible_ground_height() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let terrain = GridMapTerrain::new(0, data_dir);
        let (x, y) = (-8949.95_f32, -132.49_f32);
        let h = terrain.grid_height(x, y);
        eprintln!("REAL map0 grid_height({x}, {y}) = {h}");
        assert!(
            h > 70.0 && h < 100.0,
            "Elwynn human-start ground should be ~83.5, got {h}"
        );
        // GetStaticHeight from above ground accepts the surface.
        let sh = terrain.static_height(x, y, h + 5.0);
        eprintln!("REAL map0 static_height(probe={}) = {sh}", h + 5.0);
        assert!((sh - h).abs() < 1e-2);
    }

    #[test]
    fn out_of_range_world_position_is_safe() {
        let dir = TempMapsDir::new();
        let terrain = GridMapTerrain::new(0, &dir.0);
        // Absurd coordinate → tile index outside [0,64); must not panic.
        assert_eq!(
            terrain.grid_height(1.0e9, -1.0e9),
            VMAP_INVALID_HEIGHT_VALUE
        );
    }
}
