//! Instance scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn live_terrain_wires_static_vmap_los_provider_into_map_cache_like_cpp() {
    let dir = unique_temp_data_dir("live-vmap-los-provider");
    let provider = Arc::new(RecordingLiveStaticVMapLos::new(false));
    let shared_provider: SharedStaticVMapLineOfSightProvider = provider.clone();
    let terrain_cache =
        LiveTerrainHeights::new_with_static_vmap_line_of_sight(&dir, shared_provider);

    let terrain = terrain_cache.terrain_for_map(1);
    let mut source = wow_entities::WorldObject::new(
        false,
        wow_constants::TypeId::Unit,
        wow_constants::TypeMask::UNIT,
    );
    source.relocate(Position::new(10.0, 10.0, 1.0, 0.0));
    let query = wow_entities::LineOfSightQuery::to_position_like_cpp(
        &source,
        Position::new(20.0, 10.0, 1.0, 0.0),
        wow_entities::LineOfSightOptions::default(),
    );

    assert!(
        !terrain.line_of_sight(query),
        "live terrain must not bypass an installed static VMAP LOS provider"
    );
    let calls = provider
        .calls
        .lock()
        .expect("recording live vmap LOS calls poisoned");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].map_id, 1);

    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn terrain_grid_area_map_decodes_cpp_area_cell_and_zone_parent() {
    let data_dir = unique_temp_data_dir("terrain-area-map");
    let map_id = 571;
    let x = 0.0;
    let y = 0.0;
    let (gx, gy) = terrain_grid_coords_for_wow_position_like_cpp(x, y);
    let area_offset = MAP_FILE_HEADER_SIZE_LIKE_CPP as u32;
    let area_size = (MAP_AREA_HEADER_SIZE_LIKE_CPP
        + MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * std::mem::size_of::<u16>()) as u32;

    let mut bytes = map_file_header_with_area_like_cpp(area_offset, area_size);
    bytes.extend_from_slice(MAP_AREA_MAGIC_LIKE_CPP);
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&4395_u16.to_le_bytes());
    let mut cells = [0_u16; MAP_AREA_CELLS_PER_GRID_LIKE_CPP * MAP_AREA_CELLS_PER_GRID_LIKE_CPP];
    cells[0] = 4613;
    for cell in cells {
        bytes.extend_from_slice(&cell.to_le_bytes());
    }
    fs::write(
        data_dir
            .join("maps")
            .join(format!("{map_id:04}_{gx:02}_{gy:02}.map")),
        bytes,
    )
    .expect("write test map");

    let area_store = wow_data::AreaTableStore::from_entries([
        test_area_entry(4395, 0, 0),
        test_area_entry(4613, 4395, 0x4000_0000),
    ]);

    assert_eq!(
        zone_and_area_for_position_like_cpp(&data_dir, map_id, x, y, Some(&area_store), |_| {
            9999
        },)
        .expect("resolve terrain zone area"),
        (4395, 4613)
    );
}
#[test]
fn terrain_area_file_uses_cpp_reversed_grid_coordinates_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-area-grid-reversal");
    let map_id = 530;
    let x = 2_933.0;
    let y = -5_600.0;
    let (file_x, file_y) = terrain_grid_coords_for_wow_position_like_cpp(x, y);
    assert_eq!((file_x, file_y), (26, 42));

    let area_offset = MAP_FILE_HEADER_SIZE_LIKE_CPP as u32;
    let mut bytes =
        map_file_header_with_area_like_cpp(area_offset, MAP_AREA_HEADER_SIZE_LIKE_CPP as u32);
    bytes.extend_from_slice(MAP_AREA_MAGIC_LIKE_CPP);
    bytes.extend_from_slice(&MAP_AREA_HEADER_FLAG_NO_AREA_LIKE_CPP.to_le_bytes());
    bytes.extend_from_slice(&3_697_u16.to_le_bytes());
    fs::write(
        data_dir
            .join("maps")
            .join(format!("{map_id:04}_{file_x:02}_{file_y:02}.map")),
        bytes,
    )
    .expect("write reversed C++ terrain tile");

    assert_eq!(
        terrain_grid_area_id_for_position_like_cpp(&data_dir, map_id, x, y)
            .expect("read reversed C++ terrain tile"),
        Some(3_697)
    );
}
#[test]
fn terrain_zone_area_falls_back_to_map_area_when_grid_missing_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-area-fallback");
    assert_eq!(
        zone_and_area_for_position_like_cpp(&data_dir, 571, 0.0, 0.0, None, |_| 4395)
            .expect("resolve fallback terrain zone area"),
        (4395, 4395)
    );
}
#[test]
fn terrain_grid_coords_match_cpp_compute_grid_coord_reversal() {
    assert_eq!(
        terrain_grid_coords_for_wow_position_like_cpp(0.0, 0.0),
        (31, 31)
    );
    assert_eq!(
        terrain_grid_coords_for_wow_position_like_cpp(SIZE_OF_GRIDS_LIKE_CPP, 0.0),
        (30, 31)
    );
    assert_eq!(
        terrain_grid_coords_for_wow_position_like_cpp(-SIZE_OF_GRIDS_LIKE_CPP, 0.0),
        (32, 31)
    );
}
#[test]
fn terrain_grid_files_read_cpp_tilelist_bitset_string_order() {
    let data_dir = unique_temp_data_dir("terrain-grid-tilelist");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write tilelist");

    let terrain = TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 609, &HashMap::new())
        .expect("load terrain grid files");

    assert!(terrain.has_grid_file_like_cpp(31, 31));
    assert!(!terrain.has_grid_file_like_cpp(31, 30));
    fs::remove_dir_all(data_dir).expect("remove test dir");
}
#[test]
fn terrain_grid_files_fallback_validates_map_header_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-grid-map-header");
    fs::write(
        data_dir.join("maps").join("0609_31_31.map"),
        map_file_header_like_cpp(),
    )
    .expect("write map file");
    fs::write(
        data_dir.join("maps").join("0609_31_30.map"),
        b"not a valid map header",
    )
    .expect("write invalid map file");

    let terrain = TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 609, &HashMap::new())
        .expect("load terrain grid files");

    assert!(terrain.has_grid_file_like_cpp(31, 31));
    assert!(!terrain.has_grid_file_like_cpp(31, 30));
    fs::remove_dir_all(data_dir).expect("remove test dir");
}
#[test]
fn terrain_grid_files_has_child_terrain_grid_file_like_cpp() {
    let data_dir = unique_temp_data_dir("terrain-grid-child");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0571.tilelist"),
        tilelist_like_cpp([]),
    )
    .expect("write parent tilelist");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write child tilelist");
    let parent_child_map_data = HashMap::from([(571, vec![609]), (609, Vec::new())]);

    let terrain =
        TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 571, &parent_child_map_data)
            .expect("load terrain grid files");

    assert!(terrain.has_child_terrain_grid_file_like_cpp(609, 31, 31));
    assert!(!terrain.has_child_terrain_grid_file_like_cpp(609, 31, 30));
    assert!(!terrain.has_child_terrain_grid_file_like_cpp(700, 31, 31));
    fs::remove_dir_all(data_dir).expect("remove test dir");
}
