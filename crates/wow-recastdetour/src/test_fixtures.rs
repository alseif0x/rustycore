//! Deterministic navmesh fixtures for tests and the `test-fixtures` feature.
//!
//! Moved out of the crate root under #658; every fixture is unchanged.

use super::*;

/// Side length in cells of [`obstacle_ring_tile_blob`].
pub const OBSTACLE_TILE_CELLS: u16 = 3;
/// Cell that is left out of [`obstacle_ring_tile_blob`], i.e. the obstacle.
pub const OBSTACLE_TILE_HOLE: (u16, u16) = (1, 1);
/// World size of one cell. It has to comfortably exceed
/// [`SMOOTH_PATH_STEP_SIZE_LIKE_CPP`], otherwise one `FindSmoothPath` step
/// would jump across the whole fixture and the resulting point list would
/// say nothing about the route actually taken.
pub const OBSTACLE_TILE_CELL_SIZE: f32 = 10.0;
/// World extent of the fixture along one axis.
pub const OBSTACLE_TILE_EXTENT: f32 = OBSTACLE_TILE_CELL_SIZE * OBSTACLE_TILE_CELLS as f32;

/// Builds one navmesh tile shaped like a 3x3 grid of square quads with the
/// centre quad missing, so a straight line through the centre is not
/// walkable but a route around it is. This is the smallest navmesh that can
/// distinguish "queried Detour" from "shortcut straight to the
/// destination".
///
/// Vertices use the same winding as `rustycore_dt_create_square_tile_data`
/// (`(x,z) -> (x+1,z) -> (x+1,z+1) -> (x,z+1)`, y up), and edge `j` runs
/// from vertex `j` to vertex `j+1`, so the neighbour half of each
/// `rcPolyMesh` poly entry is `[-z, +x, +z, -x]`. `MESH_NULL_IDX` (0xffff)
/// marks a border edge, matching `DetourNavMeshBuilder.cpp:525-546`.
#[must_use]
pub fn obstacle_ring_tile_blob(tile_x: i32, tile_y: i32) -> MmapTileBlob {
    obstacle_ring_tile_blob_at(tile_x, tile_y, 0.0, 0.0)
}

/// Same fixture, but with its geometry offset to `(origin_detour_x,
/// origin_detour_z)` so it can be placed inside a navmesh tile other than
/// `(0, 0)`.
#[must_use]
pub fn obstacle_ring_tile_blob_at(
    tile_x: i32,
    tile_y: i32,
    origin_detour_x: f32,
    origin_detour_z: f32,
) -> MmapTileBlob {
    obstacle_ring_tile_blob_at_height(tile_x, tile_y, origin_detour_x, 0.0, origin_detour_z)
}

/// Height-aware form of [`obstacle_ring_tile_blob_at`].
///
/// Detour uses `(x, y, z)` with `y` vertical, while WoW uses `(x, y, z)`
/// with `z` vertical. A connected fixture placed in a real world-map grid
/// therefore has to preserve the live terrain height instead of silently
/// building its polygons at zero.
#[must_use]
pub fn obstacle_ring_tile_blob_at_height(
    tile_x: i32,
    tile_y: i32,
    origin_detour_x: f32,
    origin_detour_y: f32,
    origin_detour_z: f32,
) -> MmapTileBlob {
    const NVP: usize = 4;
    const MESH_NULL_IDX: u16 = 0xffff;
    let cells = OBSTACLE_TILE_CELLS;
    let lattice = cells + 1;

    let mut verts: Vec<u16> = Vec::with_capacity(usize::from(lattice) * usize::from(lattice) * 3);
    for z in 0..lattice {
        for x in 0..lattice {
            verts.extend_from_slice(&[x, 0, z]);
        }
    }
    let vert_index = |x: u16, z: u16| z * lattice + x;

    // Poly index per walkable cell; the hole has none.
    let mut poly_index_of_cell = vec![MESH_NULL_IDX; usize::from(cells) * usize::from(cells)];
    let mut next_poly_index = 0u16;
    for z in 0..cells {
        for x in 0..cells {
            if (x, z) == OBSTACLE_TILE_HOLE {
                continue;
            }
            poly_index_of_cell[usize::from(z) * usize::from(cells) + usize::from(x)] =
                next_poly_index;
            next_poly_index += 1;
        }
    }
    let cell_poly = |x: i32, z: i32| -> u16 {
        if x < 0 || z < 0 || x >= i32::from(cells) || z >= i32::from(cells) {
            return MESH_NULL_IDX;
        }
        poly_index_of_cell[z as usize * usize::from(cells) + x as usize]
    };

    let mut polys: Vec<u16> = Vec::with_capacity(usize::from(next_poly_index) * NVP * 2);
    for z in 0..cells {
        for x in 0..cells {
            if (x, z) == OBSTACLE_TILE_HOLE {
                continue;
            }
            polys.extend_from_slice(&[
                vert_index(x, z),
                vert_index(x + 1, z),
                vert_index(x + 1, z + 1),
                vert_index(x, z + 1),
            ]);
            let (xi, zi) = (i32::from(x), i32::from(z));
            polys.extend_from_slice(&[
                cell_poly(xi, zi - 1),
                cell_poly(xi + 1, zi),
                cell_poly(xi, zi + 1),
                cell_poly(xi - 1, zi),
            ]);
        }
    }

    let poly_count = usize::from(next_poly_index);
    // NAV_GROUND is the flag `create_path_query_filter_like_cpp` includes
    // for a walking creature; area 0 keeps the default cost.
    let poly_flags = vec![NavTerrainFlag::GROUND.bits(); poly_count];
    let poly_areas = vec![0u8; poly_count];
    let bmin = [origin_detour_x, origin_detour_y, origin_detour_z];
    let bmax = [
        origin_detour_x + OBSTACLE_TILE_EXTENT,
        origin_detour_y + OBSTACLE_TILE_CELL_SIZE,
        origin_detour_z + OBSTACLE_TILE_EXTENT,
    ];

    let mut data = std::ptr::null_mut();
    let mut data_size = 0;
    assert!(unsafe {
        rustycore_dt_create_poly_mesh_tile_data(
            tile_x,
            tile_y,
            verts.as_ptr(),
            (verts.len() / 3) as i32,
            polys.as_ptr(),
            poly_count as i32,
            NVP as i32,
            poly_flags.as_ptr(),
            poly_areas.as_ptr(),
            bmin.as_ptr(),
            bmax.as_ptr(),
            OBSTACLE_TILE_CELL_SIZE,
            OBSTACLE_TILE_CELL_SIZE,
            2.0,
            0.0,
            0.9,
            &mut data,
            &mut data_size,
        )
    });
    assert!(!data.is_null());
    assert!(data_size > 0);

    let bytes = unsafe { std::slice::from_raw_parts(data, data_size as usize) }.to_vec();
    unsafe { rustycore_dt_free(data.cast()) };

    MmapTileBlob {
        header: MmapTileHeader {
            mmap_magic: MMAP_MAGIC_LIKE_CPP,
            dt_version: DT_NAVMESH_VERSION_LIKE_CPP,
            mmap_version: MMAP_VERSION_LIKE_CPP,
            size: data_size as u32,
            uses_liquids: true,
            padding: [0, 0, 0],
        },
        data: bytes,
    }
}

/// Navmesh params under which [`obstacle_ring_tile_blob`] occupies exactly
/// tile `(0, 0)`.
#[must_use]
pub fn obstacle_ring_nav_mesh_params() -> DetourNavMeshParams {
    DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: OBSTACLE_TILE_EXTENT,
        tile_height: OBSTACLE_TILE_EXTENT,
        max_tiles: 4,
        max_polys: 256,
    }
}

/// Mesh whose only tile is [`obstacle_ring_tile_blob`].
#[must_use]
pub fn obstacle_ring_nav_mesh() -> DetourNavMesh {
    let params = obstacle_ring_nav_mesh_params();
    let mut mesh = DetourNavMesh::new(&params).unwrap();
    let tile = obstacle_ring_tile_blob(0, 0);
    assert_ne!(mesh.add_tile(&tile).unwrap(), 0);
    mesh
}

/// Walking-creature filter, i.e. the C++ `CreateFilter` result for
/// `CanWalk() == true` and `CanEnterWater() == false`.
#[must_use]
pub fn obstacle_ring_walk_filter() -> DetourQueryFilter {
    create_path_query_filter_like_cpp(PathQueryFilterContext::creature(true, false, false, false))
        .unwrap()
}

/// World bounds of the missing centre cell, as `x`/`z` in Detour space.
#[must_use]
pub fn obstacle_hole_bounds() -> (std::ops::RangeInclusive<f32>, std::ops::RangeInclusive<f32>) {
    let (hole_x, hole_z) = OBSTACLE_TILE_HOLE;
    let low_x = f32::from(hole_x) * OBSTACLE_TILE_CELL_SIZE;
    let low_z = f32::from(hole_z) * OBSTACLE_TILE_CELL_SIZE;
    (
        low_x..=(low_x + OBSTACLE_TILE_CELL_SIZE),
        low_z..=(low_z + OBSTACLE_TILE_CELL_SIZE),
    )
}

/// Navmesh params on the production 533.3333-unit tile grid, so fixture
/// tiles are placed and looked up by exactly the rules `MMapManager` uses
/// for real `.mmtile` data.
#[must_use]
pub fn obstacle_ring_world_nav_mesh_params() -> DetourNavMeshParams {
    obstacle_ring_world_nav_mesh_params_at(0.0, 0.0)
}

/// Production-grid params with an explicit Detour `(x, z)` origin.
///
/// Real `.mmap` files choose an origin that keeps tile indices inside the
/// navmesh's finite tile grid. Connected synthetic fixtures can live at
/// negative WoW coordinates, so using a hardcoded zero origin would create
/// a negative Detour tile index that the runtime correctly rejects.
#[must_use]
pub fn obstacle_ring_world_nav_mesh_params_at(
    origin_detour_x: f32,
    origin_detour_z: f32,
) -> DetourNavMeshParams {
    DetourNavMeshParams {
        origin: [origin_detour_x, 0.0, origin_detour_z],
        tile_width: SIZE_OF_GRIDS_LIKE_CPP,
        tile_height: SIZE_OF_GRIDS_LIKE_CPP,
        max_tiles: 4096,
        max_polys: 16_384,
    }
}

/// Writes the fixture into `base_path` as the on-disk mmaps layout
/// `MMapManager` loads — `mmaps/<map>.mmap` on the production tile grid plus
/// one `.mmtile` per requested WoW position — so a real runtime pathfinder
/// can serve queries for `map_id` around each of them.
///
/// Each tile is placed at the navmesh tile `dtNavMesh::calcTileLoc` derives
/// for its position, with its geometry offset to that tile's corner, and
/// named with the C++ 64x64 grid id for the same position.
pub fn write_obstacle_ring_mmaps_like_cpp(
    base_path: impl AsRef<Path>,
    map_id: u32,
    wow_positions: &[(f32, f32)],
) {
    let positions = wow_positions
        .iter()
        .map(|&(wow_x, wow_y)| (wow_x, wow_y, 0.0))
        .collect::<Vec<_>>();
    write_obstacle_ring_mmaps_at_height_like_cpp(base_path, map_id, &positions);
}

/// Height-aware on-disk fixture writer used by connected C++/Rust capture
/// fixtures. Each `(x, y, z)` tuple is in WoW coordinates.
pub fn write_obstacle_ring_mmaps_at_height_like_cpp(
    base_path: impl AsRef<Path>,
    map_id: u32,
    wow_positions: &[(f32, f32, f32)],
) {
    let base_path = base_path.as_ref();
    std::fs::create_dir_all(base_path.join("mmaps")).unwrap();
    let origin_tile_x = wow_positions
        .iter()
        .map(|&(_, wow_y, _)| (wow_y / SIZE_OF_GRIDS_LIKE_CPP).floor() as i32)
        .min()
        .unwrap_or(0);
    let origin_tile_z = wow_positions
        .iter()
        .map(|&(wow_x, _, _)| (wow_x / SIZE_OF_GRIDS_LIKE_CPP).floor() as i32)
        .min()
        .unwrap_or(0);
    let origin_detour_x = origin_tile_x as f32 * SIZE_OF_GRIDS_LIKE_CPP;
    let origin_detour_z = origin_tile_z as f32 * SIZE_OF_GRIDS_LIKE_CPP;
    std::fs::write(
        map_file_path_like_cpp(base_path, map_id),
        obstacle_ring_world_nav_mesh_params_at(origin_detour_x, origin_detour_z).to_bytes(),
    )
    .unwrap();

    for &(wow_x, wow_y, wow_z) in wow_positions {
        // `wow_position_to_detour_like_cpp` puts WoW y on the Detour x axis
        // and WoW x on the Detour z axis, which is what `calcTileLoc`
        // divides by the tile size.
        let tile_x = (wow_y / SIZE_OF_GRIDS_LIKE_CPP).floor();
        let tile_z = (wow_x / SIZE_OF_GRIDS_LIKE_CPP).floor();
        let tile = obstacle_ring_tile_blob_at_height(
            tile_x as i32 - origin_tile_x,
            tile_z as i32 - origin_tile_z,
            tile_x * SIZE_OF_GRIDS_LIKE_CPP,
            wow_z,
            tile_z * SIZE_OF_GRIDS_LIKE_CPP,
        );

        let (grid_x, grid_y) = mmap_tile_coords_for_wow_position_like_cpp(wow_x, wow_y);
        let mut bytes = tile.header.to_bytes().to_vec();
        bytes.extend_from_slice(&tile.data);
        std::fs::write(
            tile_file_path_like_cpp(base_path, map_id, grid_x, grid_y),
            bytes,
        )
        .unwrap();
    }
}
