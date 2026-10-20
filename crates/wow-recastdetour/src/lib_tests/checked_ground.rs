use super::*;

fn square_data() -> Vec<u8> {
    generated_square_tile_blob(0, 0).data
}

fn read_i32_at(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn write_i32_at(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u32_at(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u16_at(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_f32_at(bytes: &mut [u8], at: usize, value: f32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn section_offsets(bytes: &[u8]) -> (usize, usize, usize, usize, usize) {
    let polygon_count = read_i32_at(bytes, 24) as usize;
    let vertex_count = read_i32_at(bytes, 28) as usize;
    let link_count = read_i32_at(bytes, 32) as usize;
    let detail_mesh_count = read_i32_at(bytes, 36) as usize;
    let detail_vertex_count = read_i32_at(bytes, 40) as usize;
    let detail_triangle_count = read_i32_at(bytes, 44) as usize;
    let vertices = 100;
    let polygons = vertices + vertex_count * 12;
    let links = polygons + polygon_count * 32;
    let details = links + link_count * 16;
    let detail_vertices = details + detail_mesh_count * 12;
    let detail_triangles = detail_vertices + detail_vertex_count * 12;
    let bv_nodes = detail_triangles + detail_triangle_count * 4;
    (
        polygons,
        details,
        detail_vertices,
        detail_triangles,
        bv_nodes,
    )
}

#[test]
fn checked_ground_borrows_generated_native_bytes_and_exposes_metadata() {
    let tile = generated_square_tile_blob(3, 4);
    let checked = CheckedDetour7GroundPayload::parse(&tile.data).unwrap();

    assert_eq!(checked.bytes().as_ptr(), tile.data.as_ptr());
    assert_eq!(checked.bytes().len(), tile.data.len());
    assert_eq!(
        checked.metadata(),
        CheckedGroundMetadata {
            tile_x: 3,
            tile_y: 4,
            tile_layer: 0,
            polygon_count: 1,
            vertex_count: 4,
            link_count: read_i32_at(&tile.data, 32) as usize,
            detail_vertex_count: read_i32_at(&tile.data, 40) as usize,
            detail_triangle_count: read_i32_at(&tile.data, 44) as usize,
            bv_node_count: read_i32_at(&tile.data, 48) as usize,
        }
    );
}

#[test]
fn checked_ground_rejects_truncation_trailing_bytes_and_wrong_header() {
    let tile = square_data();
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&tile[..99]),
        Err(CheckedGroundPayloadError::HeaderTooShort { .. })
    ));
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&tile[..tile.len() - 1]),
        Err(CheckedGroundPayloadError::PayloadLengthMismatch { .. })
    ));
    let mut trailing = tile.clone();
    trailing.push(0);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&trailing),
        Err(CheckedGroundPayloadError::PayloadLengthMismatch { .. })
    ));

    let mut bad_magic = tile.clone();
    write_u32_at(&mut bad_magic, 0, 0);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&bad_magic),
        Err(CheckedGroundPayloadError::WrongMagic { .. })
    ));
    let mut bad_version = tile;
    write_i32_at(&mut bad_version, 4, 8);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&bad_version),
        Err(CheckedGroundPayloadError::WrongVersion { .. })
    ));
}

#[test]
fn checked_ground_rejects_counts_tile_coordinate_overflow_and_offmesh() {
    let tile = square_data();

    let mut negative_count = tile.clone();
    write_i32_at(&mut negative_count, 24, -1);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&negative_count),
        Err(CheckedGroundPayloadError::InvalidCount {
            field: "polyCount",
            ..
        })
    ));

    let mut over_limit = tile.clone();
    write_i32_at(&mut over_limit, 28, i32::MAX);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&over_limit),
        Err(CheckedGroundPayloadError::CountExceedsLimit {
            field: "vertCount",
            ..
        })
    ));

    let mut no_links = tile.clone();
    write_i32_at(&mut no_links, 32, 0);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&no_links),
        Err(CheckedGroundPayloadError::InvalidCount {
            field: "maxLinkCount",
            ..
        })
    ));

    let mut invalid_tile_edge = tile.clone();
    write_i32_at(&mut invalid_tile_edge, 8, i32::MAX);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&invalid_tile_edge),
        Err(CheckedGroundPayloadError::TileCoordinateOverflowMargin { .. })
    ));

    let mut wrong_layer = tile.clone();
    write_i32_at(&mut wrong_layer, 16, 1);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&wrong_layer),
        Err(CheckedGroundPayloadError::UnsupportedTileLayer { .. })
    ));

    let mut offmesh = tile;
    write_i32_at(&mut offmesh, 52, 1);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&offmesh),
        Err(CheckedGroundPayloadError::OffMeshOutsideProfile { .. })
    ));
}

#[test]
fn checked_ground_rejects_bad_polygon_vertex_and_internal_neighbor_indices() {
    let tile = square_data();
    let (polygons, ..) = section_offsets(&tile);

    let mut bad_vertex = tile.clone();
    write_u16_at(&mut bad_vertex, polygons + 4, u16::MAX);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&bad_vertex),
        Err(CheckedGroundPayloadError::PolygonVertexOutOfRange { .. })
    ));

    let mut duplicate_vertex = tile.clone();
    let first = u16::from_le_bytes(
        duplicate_vertex[polygons + 4..polygons + 6]
            .try_into()
            .unwrap(),
    );
    write_u16_at(&mut duplicate_vertex, polygons + 6, first);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&duplicate_vertex),
        Err(CheckedGroundPayloadError::PolygonDuplicateVertex { .. })
    ));

    let mut bad_neighbor = tile;
    write_u16_at(&mut bad_neighbor, polygons + 16, 2);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&bad_neighbor),
        Err(CheckedGroundPayloadError::PolygonNeighborOutOfRange { .. })
    ));
}

#[test]
fn checked_ground_rejects_detail_ranges_and_triangle_indices() {
    let tile = square_data();
    let (_, details, _, detail_triangles, _) = section_offsets(&tile);

    let mut bad_vertex_range = tile.clone();
    write_u32_at(&mut bad_vertex_range, details, u32::MAX);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&bad_vertex_range),
        Err(CheckedGroundPayloadError::DetailMeshRangeInvalid { .. })
    ));

    let mut bad_triangle_range = tile.clone();
    write_u32_at(&mut bad_triangle_range, details + 4, u32::MAX);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&bad_triangle_range),
        Err(CheckedGroundPayloadError::DetailMeshRangeInvalid { .. })
    ));

    let mut bad_triangle_index = tile;
    bad_triangle_index[detail_triangles] = u8::MAX;
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&bad_triangle_index),
        Err(CheckedGroundPayloadError::DetailTriangleIndexOutOfRange { .. })
    ));
}

#[test]
fn checked_ground_rejects_bv_indices_bounds_and_unrepresentable_quantization() {
    let tile = square_data();
    let (_, _, _, _, bv_nodes) = section_offsets(&tile);
    assert!(read_i32_at(&tile, 48) > 0);

    let mut bad_leaf = tile.clone();
    write_i32_at(&mut bad_leaf, bv_nodes + 12, 1);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&bad_leaf),
        Err(CheckedGroundPayloadError::BvLeafOutOfRange { .. })
    ));

    let mut bad_escape = tile.clone();
    write_i32_at(&mut bad_escape, bv_nodes + 12, i32::MIN);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&bad_escape),
        Err(CheckedGroundPayloadError::BvEscapeOutOfRange { .. })
    ));

    let mut inverted_bounds = tile.clone();
    write_u16_at(&mut inverted_bounds, bv_nodes, u16::MAX);
    write_u16_at(&mut inverted_bounds, bv_nodes + 6, 0);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&inverted_bounds),
        Err(CheckedGroundPayloadError::InvertedBvNodeBounds { .. })
    ));

    let mut unrepresentable = tile;
    write_f32_at(&mut unrepresentable, 96, 65_535.0);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&unrepresentable),
        Err(CheckedGroundPayloadError::BvQuantizationOutOfRange { .. })
    ));
}

#[test]
fn checked_ground_rejects_unsupported_and_degenerate_portals() {
    let tile = square_data();
    let (polygons, ..) = section_offsets(&tile);

    let mut diagonal = tile.clone();
    write_u16_at(&mut diagonal, polygons + 16, 0x8001);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&diagonal),
        Err(CheckedGroundPayloadError::UnsupportedPortalDirection { .. })
    ));

    let mut zero_tangent = tile.clone();
    write_u16_at(&mut zero_tangent, polygons + 16, 0x8000);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&zero_tangent),
        Err(CheckedGroundPayloadError::ZeroPortalTangent { .. })
    ));

    let mut finite_portal = tile.clone();
    write_u16_at(&mut finite_portal, polygons + 18, 0x8000);
    assert!(CheckedDetour7GroundPayload::parse(&finite_portal).is_ok());

    let mut nonfinite_slope = tile;
    write_u16_at(&mut nonfinite_slope, polygons + 18, 0x8000);
    write_f32_at(&mut nonfinite_slope, 100 + 2 * 12 + 4, 1.0);
    write_f32_at(&mut nonfinite_slope, 100 + 2 * 12 + 8, f32::from_bits(1));
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&nonfinite_slope),
        Err(CheckedGroundPayloadError::InvalidPortalArithmetic { .. })
    ));
}

#[test]
fn checked_ground_rejects_nonfinite_and_overbound_numeric_data() {
    let tile = square_data();

    let mut nonfinite_header = tile.clone();
    write_f32_at(&mut nonfinite_header, 60, f32::INFINITY);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&nonfinite_header),
        Err(CheckedGroundPayloadError::InvalidHeaderFloat { .. })
    ));

    let mut nonfinite_vertex = tile.clone();
    write_f32_at(&mut nonfinite_vertex, 100, f32::NAN);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&nonfinite_vertex),
        Err(CheckedGroundPayloadError::InvalidCoordinate { kind: "vertex", .. })
    ));

    let mut overbound_vertex = tile.clone();
    write_f32_at(
        &mut overbound_vertex,
        100,
        CHECKED_GROUND_MAX_ABS_COORDINATE * 2.0,
    );
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&overbound_vertex),
        Err(CheckedGroundPayloadError::InvalidCoordinate { kind: "vertex", .. })
    ));

    let (_, details, detail_vertices, _, _) = section_offsets(&tile);
    let mut with_detail_vertex = tile[..detail_vertices].to_vec();
    for _ in 0..3 {
        with_detail_vertex.extend_from_slice(&0.0f32.to_le_bytes());
    }
    with_detail_vertex.extend_from_slice(&tile[detail_vertices..]);
    write_i32_at(&mut with_detail_vertex, 40, 1);
    with_detail_vertex[details + 8] = 1;
    assert!(CheckedDetour7GroundPayload::parse(&with_detail_vertex).is_ok());

    let new_detail_vertex_at = detail_vertices;
    write_f32_at(&mut with_detail_vertex, new_detail_vertex_at + 4, f32::NAN);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&with_detail_vertex),
        Err(CheckedGroundPayloadError::InvalidCoordinate {
            kind: "detail vertex",
            ..
        })
    ));
}

#[test]
fn checked_ground_limits_only_allow_downward_overrides() {
    assert!(
        CheckedGroundLimits::new(
            CHECKED_GROUND_MAX_PAYLOAD_BYTES - 1,
            CHECKED_GROUND_MAX_POLYGONS,
            CHECKED_GROUND_MAX_VERTICES,
            CHECKED_GROUND_MAX_LINKS,
            CHECKED_GROUND_MAX_DETAIL_TRIANGLES,
            CHECKED_GROUND_MAX_BV_NODES,
            CHECKED_GROUND_MAX_ABS_COORDINATE,
        )
        .is_ok()
    );
    assert!(matches!(
        CheckedGroundLimits::new(
            CHECKED_GROUND_MAX_PAYLOAD_BYTES + 1,
            CHECKED_GROUND_MAX_POLYGONS,
            CHECKED_GROUND_MAX_VERTICES,
            CHECKED_GROUND_MAX_LINKS,
            CHECKED_GROUND_MAX_DETAIL_TRIANGLES,
            CHECKED_GROUND_MAX_BV_NODES,
            CHECKED_GROUND_MAX_ABS_COORDINATE,
        ),
        Err(CheckedGroundPayloadError::InvalidLimits {
            field: "max_payload_bytes"
        })
    ));

    let tile = square_data();
    let reduced = CheckedGroundLimits::new(
        CHECKED_GROUND_MAX_PAYLOAD_BYTES,
        CHECKED_GROUND_MAX_POLYGONS,
        3,
        CHECKED_GROUND_MAX_LINKS,
        CHECKED_GROUND_MAX_DETAIL_TRIANGLES,
        CHECKED_GROUND_MAX_BV_NODES,
        CHECKED_GROUND_MAX_ABS_COORDINATE,
    )
    .unwrap();
    assert!(matches!(
        CheckedDetour7GroundPayload::parse_with_limits(&tile, reduced),
        Err(CheckedGroundPayloadError::CountExceedsLimit {
            field: "vertCount",
            ..
        })
    ));

    let byte_limited = CheckedGroundLimits::new(
        tile.len() - 1,
        CHECKED_GROUND_MAX_POLYGONS,
        CHECKED_GROUND_MAX_VERTICES,
        CHECKED_GROUND_MAX_LINKS,
        CHECKED_GROUND_MAX_DETAIL_TRIANGLES,
        CHECKED_GROUND_MAX_BV_NODES,
        CHECKED_GROUND_MAX_ABS_COORDINATE,
    )
    .unwrap();
    assert!(matches!(
        CheckedDetour7GroundPayload::parse_with_limits(&tile, byte_limited),
        Err(CheckedGroundPayloadError::PayloadTooLarge { .. })
    ));

    let mut huge_count = tile;
    write_i32_at(&mut huge_count, 44, i32::MAX);
    assert!(matches!(
        CheckedDetour7GroundPayload::parse(&huge_count),
        Err(CheckedGroundPayloadError::CountExceedsLimit {
            field: "detailTriCount",
            ..
        })
    ));
}
