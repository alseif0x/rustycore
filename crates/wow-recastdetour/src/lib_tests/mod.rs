//! Navmesh FFI regressions.
//!
//! Separated from the lib_tests.rs root under #658.

//! Behaviour tests for [`super`].
//!
//! Extracted from `lib.rs`, which was 5,664 lines of which
//! 2,470 — 44% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::*;

fn disconnected_two_island_nav_mesh() -> DetourNavMesh {
    const NVP: usize = 4;
    const MESH_NULL_IDX: u16 = 0xffff;
    // Two ten-yard quads separated by a ten-yard void.
    let verts: [u16; 24] = [
        0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, // first island
        2, 0, 0, 3, 0, 0, 3, 0, 1, 2, 0, 1, // second island
    ];
    let polys: [u16; 16] = [
        0,
        1,
        2,
        3,
        MESH_NULL_IDX,
        MESH_NULL_IDX,
        MESH_NULL_IDX,
        MESH_NULL_IDX,
        4,
        5,
        6,
        7,
        MESH_NULL_IDX,
        MESH_NULL_IDX,
        MESH_NULL_IDX,
        MESH_NULL_IDX,
    ];
    let poly_flags = [NavTerrainFlag::GROUND.bits(); 2];
    let poly_areas = [0_u8; 2];
    let bmin = [0.0, 0.0, 0.0];
    let bmax = [30.0, 10.0, 10.0];
    let mut data = std::ptr::null_mut();
    let mut data_size = 0;
    assert!(unsafe {
        rustycore_dt_create_poly_mesh_tile_data(
            0,
            0,
            verts.as_ptr(),
            (verts.len() / 3) as i32,
            polys.as_ptr(),
            2,
            NVP as i32,
            poly_flags.as_ptr(),
            poly_areas.as_ptr(),
            bmin.as_ptr(),
            bmax.as_ptr(),
            10.0,
            10.0,
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

    let params = DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 30.0,
        tile_height: 10.0,
        max_tiles: 1,
        max_polys: 16,
    };
    let mut mesh = DetourNavMesh::new(&params).unwrap();
    let tile = MmapTileBlob {
        header: MmapTileHeader {
            mmap_magic: MMAP_MAGIC_LIKE_CPP,
            dt_version: DT_NAVMESH_VERSION_LIKE_CPP,
            mmap_version: MMAP_VERSION_LIKE_CPP,
            size: data_size as u32,
            uses_liquids: true,
            padding: [0; 3],
        },
        data: bytes,
    };
    assert_ne!(mesh.add_tile(&tile).unwrap(), 0);
    mesh
}

fn unique_test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rustycore-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ))
}

fn generated_square_tile_blob(tile_x: i32, tile_y: i32) -> MmapTileBlob {
    let mut data = std::ptr::null_mut();
    let mut data_size = 0;
    assert!(unsafe {
        rustycore_dt_create_square_tile_data(tile_x, tile_y, &mut data, &mut data_size)
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

use crate::test_fixtures::*;

fn write_mmap_tile_blob(path: &std::path::Path, tile: &MmapTileBlob) {
    let mut bytes = tile.header.to_bytes().to_vec();
    bytes.extend_from_slice(&tile.data);
    std::fs::write(path, bytes).unwrap();
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
