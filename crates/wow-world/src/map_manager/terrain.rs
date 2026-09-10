//! Terrain items of mod.
//!
//! Separated from mod.rs under #711; every item keeps its name,
//! signature and body.

use super::*;

/// TrinityCore `TerrainInfo::GetMinHeight` fallback when no terrain grid is loaded.
///
/// Real grid-backed min-height data belongs to the terrain/map-data port; exposing
/// the fallback here lets movement preserve the C++ under-map branch without
/// inventing terrain values.
pub const DEFAULT_MIN_HEIGHT_LIKE_CPP: f32 = -500.0;

pub(super) const MAP_MAGIC_LIKE_CPP: &[u8; 4] = b"MAPS";

pub(super) const MAP_AREA_MAGIC_LIKE_CPP: &[u8; 4] = b"AREA";

pub(super) const MAP_VERSION_MAGIC_LIKE_CPP: u32 = 10;

pub(super) const MAP_FILE_HEADER_SIZE_LIKE_CPP: usize = 44;

pub(super) const MAP_AREA_HEADER_SIZE_LIKE_CPP: usize = 8;

pub(super) const MAP_AREA_HEADER_FLAG_NO_AREA_LIKE_CPP: u16 = 0x0001;

pub(super) const MAP_AREA_CELLS_PER_GRID_LIKE_CPP: usize = 16;

pub(super) const TERRAIN_GRID_COUNT_LIKE_CPP: usize =
    MAX_NUMBER_OF_GRIDS_LIKE_CPP as usize * MAX_NUMBER_OF_GRIDS_LIKE_CPP as usize;

pub fn terrain_grid_coords_for_wow_position_like_cpp(x: f32, y: f32) -> (i32, i32) {
    let center_grid_offset = SIZE_OF_GRIDS_LIKE_CPP / 2.0;
    let x_offset = (x - center_grid_offset) / SIZE_OF_GRIDS_LIKE_CPP;
    let y_offset = (y - center_grid_offset) / SIZE_OF_GRIDS_LIKE_CPP;
    let grid_x = (x_offset + CENTER_GRID_ID_LIKE_CPP as f32 + 0.5) as i32;
    let grid_y = (y_offset + CENTER_GRID_ID_LIKE_CPP as f32 + 0.5) as i32;

    (
        (MAX_NUMBER_OF_GRIDS_LIKE_CPP - 1) - grid_x,
        (MAX_NUMBER_OF_GRIDS_LIKE_CPP - 1) - grid_y,
    )
}

pub fn terrain_map_id_for_phase_shift_like_cpp(
    phase_shift: &PhaseShift,
    map_id: u32,
    x: f32,
    y: f32,
    mut has_child_terrain_grid_file: impl FnMut(u32, i32, i32) -> bool,
) -> u32 {
    match phase_shift.visible_map_id_count_like_cpp() {
        0 => map_id,
        1 => phase_shift
            .visible_map_ids_like_cpp()
            .next()
            .unwrap_or(map_id),
        _ => {
            let (grid_x, grid_y) = terrain_grid_coords_for_wow_position_like_cpp(x, y);
            phase_shift
                .visible_map_ids_like_cpp()
                .find(|visible_map_id| has_child_terrain_grid_file(*visible_map_id, grid_x, grid_y))
                .unwrap_or(map_id)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerrainGridFilesLikeCpp {
    map_id: u32,
    grid_file_exists: Vec<bool>,
    child_terrain: Vec<TerrainGridFilesLikeCpp>,
}

impl TerrainGridFilesLikeCpp {
    pub fn load_root_like_cpp(
        data_dir: impl AsRef<Path>,
        map_id: u32,
        parent_child_map_data: &HashMap<u32, Vec<u32>>,
    ) -> io::Result<Self> {
        Self::load_impl_like_cpp(data_dir.as_ref(), map_id, parent_child_map_data)
    }

    fn load_impl_like_cpp(
        data_dir: &Path,
        map_id: u32,
        parent_child_map_data: &HashMap<u32, Vec<u32>>,
    ) -> io::Result<Self> {
        let grid_file_exists = discover_grid_map_files_like_cpp(data_dir, map_id)?;
        let mut child_terrain = Vec::new();
        if let Some(child_map_ids) = parent_child_map_data.get(&map_id) {
            for child_map_id in child_map_ids {
                child_terrain.push(Self::load_impl_like_cpp(
                    data_dir,
                    *child_map_id,
                    parent_child_map_data,
                )?);
            }
        }

        Ok(Self {
            map_id,
            grid_file_exists,
            child_terrain,
        })
    }

    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    pub fn has_grid_file_like_cpp(&self, gx: i32, gy: i32) -> bool {
        terrain_grid_bitset_index_like_cpp(gx, gy)
            .and_then(|idx| self.grid_file_exists.get(idx).copied())
            .unwrap_or(false)
    }

    pub fn has_child_terrain_grid_file_like_cpp(&self, map_id: u32, gx: i32, gy: i32) -> bool {
        self.child_terrain
            .iter()
            .find(|child_terrain| child_terrain.map_id == map_id)
            .is_some_and(|child_terrain| child_terrain.has_grid_file_like_cpp(gx, gy))
    }

    pub fn terrain_map_id_for_phase_shift_like_cpp(
        &self,
        phase_shift: &PhaseShift,
        source_map_id: u32,
        x: f32,
        y: f32,
    ) -> u32 {
        terrain_map_id_for_phase_shift_like_cpp(
            phase_shift,
            source_map_id,
            x,
            y,
            |map_id, gx, gy| self.has_child_terrain_grid_file_like_cpp(map_id, gx, gy),
        )
    }
}

#[derive(Debug)]
pub struct TerrainGridFileIndexLikeCpp {
    data_dir: PathBuf,
    parent_child_map_data: HashMap<u32, Vec<u32>>,
    parent_map_ids: HashMap<u32, u32>,
    terrain_maps: HashMap<u32, TerrainGridFilesLikeCpp>,
}

impl TerrainGridFileIndexLikeCpp {
    pub fn new(
        data_dir: impl AsRef<Path>,
        parent_child_map_data: impl IntoIterator<Item = (u32, Vec<u32>)>,
    ) -> Self {
        let parent_child_map_data: HashMap<u32, Vec<u32>> =
            parent_child_map_data.into_iter().collect();
        let mut parent_map_ids = HashMap::new();
        for (parent_map_id, child_map_ids) in &parent_child_map_data {
            for child_map_id in child_map_ids {
                parent_map_ids.insert(*child_map_id, *parent_map_id);
            }
        }

        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            parent_child_map_data,
            parent_map_ids,
            terrain_maps: HashMap::new(),
        }
    }

    pub fn root_map_id_like_cpp(&self, map_id: u32) -> u32 {
        let mut root_map_id = map_id;
        while let Some(parent_map_id) = self.parent_map_ids.get(&root_map_id).copied() {
            root_map_id = parent_map_id;
        }
        root_map_id
    }

    pub fn terrain_for_map_like_cpp(
        &mut self,
        map_id: u32,
    ) -> io::Result<&TerrainGridFilesLikeCpp> {
        let root_map_id = self.root_map_id_like_cpp(map_id);
        if !self.terrain_maps.contains_key(&root_map_id) {
            let terrain = TerrainGridFilesLikeCpp::load_root_like_cpp(
                &self.data_dir,
                root_map_id,
                &self.parent_child_map_data,
            )?;
            self.terrain_maps.insert(root_map_id, terrain);
        }

        Ok(self
            .terrain_maps
            .get(&root_map_id)
            .expect("terrain root inserted"))
    }

    pub fn terrain_map_id_for_phase_shift_like_cpp(
        &mut self,
        phase_shift: &PhaseShift,
        source_map_id: u32,
        x: f32,
        y: f32,
    ) -> u32 {
        if phase_shift.visible_map_id_count_like_cpp() == 0 {
            return source_map_id;
        }

        self.terrain_for_map_like_cpp(source_map_id)
            .map(|terrain| {
                terrain.terrain_map_id_for_phase_shift_like_cpp(phase_shift, source_map_id, x, y)
            })
            .unwrap_or(source_map_id)
    }
}

pub(super) fn discover_grid_map_files_like_cpp(
    data_dir: &Path,
    map_id: u32,
) -> io::Result<Vec<bool>> {
    let tile_list_name = data_dir.join("maps").join(format!("{map_id:04}.tilelist"));
    if let Ok(mut tile_list) = File::open(tile_list_name) {
        let mut map_magic = [0_u8; 4];
        let mut version_magic = [0_u8; 4];
        let mut build = [0_u8; 4];
        let mut tiles_data = vec![0_u8; TERRAIN_GRID_COUNT_LIKE_CPP];
        if tile_list.read_exact(&mut map_magic).is_ok()
            && map_magic == *MAP_MAGIC_LIKE_CPP
            && tile_list.read_exact(&mut version_magic).is_ok()
            && u32::from_le_bytes(version_magic) == MAP_VERSION_MAGIC_LIKE_CPP
            && tile_list.read_exact(&mut build).is_ok()
            && tile_list.read_exact(&mut tiles_data).is_ok()
        {
            return Ok(terrain_grid_bitset_from_cpp_string_like_cpp(&tiles_data));
        }
    }

    let mut grid_file_exists = vec![false; TERRAIN_GRID_COUNT_LIKE_CPP];
    for gx in 0..MAX_NUMBER_OF_GRIDS_LIKE_CPP {
        for gy in 0..MAX_NUMBER_OF_GRIDS_LIKE_CPP {
            let idx = terrain_grid_bitset_index_like_cpp(gx, gy).expect("valid terrain grid index");
            grid_file_exists[idx] = exist_map_like_cpp(data_dir, map_id, gx, gy);
        }
    }
    Ok(grid_file_exists)
}

pub(super) fn terrain_grid_bitset_index_like_cpp(gx: i32, gy: i32) -> Option<usize> {
    if !(0..MAX_NUMBER_OF_GRIDS_LIKE_CPP).contains(&gx)
        || !(0..MAX_NUMBER_OF_GRIDS_LIKE_CPP).contains(&gy)
    {
        return None;
    }

    Some(gx as usize * MAX_NUMBER_OF_GRIDS_LIKE_CPP as usize + gy as usize)
}

pub(super) fn terrain_grid_bitset_from_cpp_string_like_cpp(tiles_data: &[u8]) -> Vec<bool> {
    let mut grid_file_exists = vec![false; TERRAIN_GRID_COUNT_LIKE_CPP];
    for (idx, exists) in grid_file_exists.iter_mut().enumerate() {
        let string_idx = TERRAIN_GRID_COUNT_LIKE_CPP - 1 - idx;
        *exists = tiles_data.get(string_idx).copied() == Some(b'1');
    }
    grid_file_exists
}

pub(super) fn exist_map_like_cpp(data_dir: &Path, map_id: u32, gx: i32, gy: i32) -> bool {
    let file_name = data_dir
        .join("maps")
        .join(format!("{map_id:04}_{gx:02}_{gy:02}.map"));
    let Ok(mut file) = File::open(file_name) else {
        return false;
    };

    let mut header = [0_u8; MAP_FILE_HEADER_SIZE_LIKE_CPP];
    if file.read_exact(&mut header).is_err() {
        return false;
    }

    header[..4] == MAP_MAGIC_LIKE_CPP[..]
        && u32::from_le_bytes([header[4], header[5], header[6], header[7]])
            == MAP_VERSION_MAGIC_LIKE_CPP
}

pub fn terrain_grid_area_id_for_position_like_cpp(
    data_dir: impl AsRef<Path>,
    map_id: u32,
    x: f32,
    y: f32,
) -> io::Result<Option<u32>> {
    // `terrain_grid_coords_for_wow_position_like_cpp` already includes the
    // axis reversal performed by C++ `Map::EnsureGridCreated` before it asks
    // TerrainMgr for the extracted map tile (`Map.cpp:338-343`).
    let (gx, gy) = terrain_grid_coords_for_wow_position_like_cpp(x, y);
    let file_name = data_dir
        .as_ref()
        .join("maps")
        .join(format!("{map_id:04}_{gx:02}_{gy:02}.map"));
    let mut file = match File::open(&file_name) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    let mut header = [0_u8; MAP_FILE_HEADER_SIZE_LIKE_CPP];
    file.read_exact(&mut header)?;
    if header[..4] != MAP_MAGIC_LIKE_CPP[..]
        || u32::from_le_bytes([header[4], header[5], header[6], header[7]])
            != MAP_VERSION_MAGIC_LIKE_CPP
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid C++ terrain map header in {}", file_name.display()),
        ));
    }

    let area_map_offset = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
    if area_map_offset == 0 {
        return Ok(None);
    }

    file.seek(SeekFrom::Start(u64::from(area_map_offset)))?;
    let mut area_header = [0_u8; MAP_AREA_HEADER_SIZE_LIKE_CPP];
    file.read_exact(&mut area_header)?;
    if area_header[..4] != MAP_AREA_MAGIC_LIKE_CPP[..] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid C++ terrain area header in {}", file_name.display()),
        ));
    }

    let flags = u16::from_le_bytes([area_header[4], area_header[5]]);
    let grid_area = u16::from_le_bytes([area_header[6], area_header[7]]);
    if flags & MAP_AREA_HEADER_FLAG_NO_AREA_LIKE_CPP != 0 {
        return Ok(Some(u32::from(grid_area)));
    }

    let mut area_map = [0_u16; MAP_AREA_CELLS_PER_GRID_LIKE_CPP * MAP_AREA_CELLS_PER_GRID_LIKE_CPP];
    let mut area_map_bytes = [0_u8;
        MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * std::mem::size_of::<u16>()];
    file.read_exact(&mut area_map_bytes)?;
    for (idx, chunk) in area_map_bytes.chunks_exact(2).enumerate() {
        area_map[idx] = u16::from_le_bytes([chunk[0], chunk[1]]);
    }

    let x = MAP_AREA_CELLS_PER_GRID_LIKE_CPP as f32
        * (CENTER_GRID_ID_LIKE_CPP as f32 - x / SIZE_OF_GRIDS_LIKE_CPP);
    let y = MAP_AREA_CELLS_PER_GRID_LIKE_CPP as f32
        * (CENTER_GRID_ID_LIKE_CPP as f32 - y / SIZE_OF_GRIDS_LIKE_CPP);
    let lx = (x as i32 & 15) as usize;
    let ly = (y as i32 & 15) as usize;
    Ok(Some(u32::from(
        area_map[lx * MAP_AREA_CELLS_PER_GRID_LIKE_CPP + ly],
    )))
}

pub fn zone_and_area_for_position_like_cpp(
    data_dir: impl AsRef<Path>,
    map_id: u32,
    x: f32,
    y: f32,
    area_store: Option<&wow_data::AreaTableStore>,
    map_area_id_fallback: impl FnOnce(u32) -> u32,
) -> io::Result<(u32, u32)> {
    let area_id = terrain_grid_area_id_for_position_like_cpp(data_dir, map_id, x, y)?
        .filter(|area_id| *area_id != 0)
        .unwrap_or_else(|| map_area_id_fallback(map_id));

    let zone_id = area_store
        .and_then(|store| store.get(area_id))
        .filter(|area| area.parent_area_id != 0 && area.is_subzone_like_cpp())
        .map(|area| u32::from(area.parent_area_id))
        .unwrap_or(area_id);

    Ok((zone_id, area_id))
}

/// Live, file-backed terrain height for the legacy world runtime.
///
/// One [`GridMapTerrain`] per map id (created lazily, shared across that map's
/// instances), mirroring C++ `TerrainInfo` ownership. Rooted at the server's
/// `DataDir`; tiles load on first query. This is the seam that lets the live
/// spawn/respawn path ground-snap creatures with real `.map` heights.
#[derive(Debug)]
pub struct LiveTerrainHeights {
    data_dir: PathBuf,
    static_vmap_los: Option<SharedStaticVMapLineOfSightProvider>,
    per_map: Mutex<HashMap<u32, Arc<GridMapTerrain>>>,
}

impl LiveTerrainHeights {
    #[must_use]
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self::new_with_optional_static_vmap_line_of_sight(data_dir, None)
    }

    /// Build the live terrain cache with a static VMAP LOS provider wired into
    /// every lazily-created map terrain.
    ///
    /// C++ startup creates/configures one `VMapManager2` and each map's
    /// `TerrainInfo`/`Map::isInLineOfSight` consults it for static geometry. This
    /// keeps the Rust live cache usable by a real provider once the VMAP
    /// `StaticMapTree` parser owns model geometry; without a provider, LOS
    /// remains C++'s disabled/missing-tree clear fallback.
    #[must_use]
    pub fn new_with_static_vmap_line_of_sight(
        data_dir: impl AsRef<Path>,
        provider: SharedStaticVMapLineOfSightProvider,
    ) -> Self {
        Self::new_with_optional_static_vmap_line_of_sight(data_dir, Some(provider))
    }

    #[must_use]
    fn new_with_optional_static_vmap_line_of_sight(
        data_dir: impl AsRef<Path>,
        static_vmap_los: Option<SharedStaticVMapLineOfSightProvider>,
    ) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            static_vmap_los,
            per_map: Mutex::new(HashMap::new()),
        }
    }

    pub(super) fn terrain_for_map(&self, map_id: u32) -> Arc<GridMapTerrain> {
        let mut per_map = self.per_map.lock().expect("live terrain cache poisoned");
        Arc::clone(per_map.entry(map_id).or_insert_with(|| {
            let terrain = GridMapTerrain::new(map_id, &self.data_dir);
            let terrain = match &self.static_vmap_los {
                Some(provider) => terrain.with_static_vmap_line_of_sight(Arc::clone(provider)),
                None => terrain,
            };
            Arc::new(terrain)
        }))
    }

    /// C++ `Map::GetHeight` (no VMap/GO-floor): the raw `.map` ground at `(x, y)`,
    /// accepted only when the probe `z` is at/above it. The caller supplies the
    /// already-offset probe `z` (matching `WorldObject::GetMapHeight`).
    #[must_use]
    pub fn static_height_like_cpp(&self, map_id: u32, x: f32, y: f32, z: f32) -> f32 {
        self.terrain_for_map(map_id).static_height(x, y, z)
    }

    /// Raw `.map` surface height at `(x, y)`, without the C++ probe-Z acceptance
    /// gate. This is intentionally not a general `Map::GetHeight` replacement;
    /// creature path normalization uses it only as a guard for Rust MMap points
    /// that arrived below the client-visible terrain surface.
    #[must_use]
    pub fn grid_height_like_cpp(&self, map_id: u32, x: f32, y: f32) -> f32 {
        self.terrain_for_map(map_id).grid_height(x, y)
    }

    /// C++ `WorldObject::IsWithinLOSInMap` static-VMAP portion for two objects
    /// already known to belong to the same legacy map instance.
    #[must_use]
    pub fn is_within_los_like_cpp(
        &self,
        map_id: u32,
        source: &wow_entities::WorldObject,
        target: &wow_entities::WorldObject,
    ) -> bool {
        self.terrain_for_map(map_id).line_of_sight(
            wow_entities::LineOfSightQuery::to_object_like_cpp(
                source,
                target,
                wow_entities::LineOfSightOptions::default(),
            ),
        )
    }
}
