//! Bounded, immutable ingress for the selected Detour7 ground-tile profile.
//!
//! This parser proves that the exact little-endian byte layout and every
//! index consumed by the vendored 64-bit Detour7 add/query paths fit this
//! profile. It is not a geometry validator and does not establish gameplay,
//! producer authenticity, or route completeness.

use super::{DT_NAVMESH_MAGIC_LIKE_CPP, DT_NAVMESH_VERSION_LIKE_CPP};
use thiserror::Error;

pub const CHECKED_GROUND_MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
pub const CHECKED_GROUND_MAX_POLYGONS: usize = 100_000;
pub const CHECKED_GROUND_MAX_VERTICES: usize = 250_000;
pub const CHECKED_GROUND_MAX_LINKS: usize = 1_000_000;
pub const CHECKED_GROUND_MAX_DETAIL_TRIANGLES: usize = 1_000_000;
pub const CHECKED_GROUND_MAX_BV_NODES: usize = 1_000_000;
pub const CHECKED_GROUND_MAX_ABS_COORDINATE: f32 = 1_048_576.0;

const TILE_HEADER_SIZE: usize = 100;
const POLY_SIZE: usize = 32;
const LINK_SIZE: usize = 16;
const DETAIL_MESH_SIZE: usize = 12;
const BV_NODE_SIZE: usize = 16;
const DETOUR_VERTS_PER_POLYGON: usize = 6;
const DT_EXT_LINK: u16 = 0x8000;

/// Caller-downward policy for checked ground payload admission.
///
/// These are logical byte/count limits, not process RSS limits. Construction
/// rejects every override above the built-in ceilings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckedGroundLimits {
    max_payload_bytes: usize,
    max_polygons: usize,
    max_vertices: usize,
    max_links: usize,
    max_detail_triangles: usize,
    max_bv_nodes: usize,
    max_abs_coordinate: f32,
}

impl Default for CheckedGroundLimits {
    fn default() -> Self {
        Self {
            max_payload_bytes: CHECKED_GROUND_MAX_PAYLOAD_BYTES,
            max_polygons: CHECKED_GROUND_MAX_POLYGONS,
            max_vertices: CHECKED_GROUND_MAX_VERTICES,
            max_links: CHECKED_GROUND_MAX_LINKS,
            max_detail_triangles: CHECKED_GROUND_MAX_DETAIL_TRIANGLES,
            max_bv_nodes: CHECKED_GROUND_MAX_BV_NODES,
            max_abs_coordinate: CHECKED_GROUND_MAX_ABS_COORDINATE,
        }
    }
}

impl CheckedGroundLimits {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        max_payload_bytes: usize,
        max_polygons: usize,
        max_vertices: usize,
        max_links: usize,
        max_detail_triangles: usize,
        max_bv_nodes: usize,
        max_abs_coordinate: f32,
    ) -> Result<Self, CheckedGroundPayloadError> {
        let defaults = Self::default();
        for (field, value) in [
            ("max_payload_bytes", max_payload_bytes),
            ("max_polygons", max_polygons),
            ("max_vertices", max_vertices),
            ("max_links", max_links),
        ] {
            if value == 0 {
                return Err(CheckedGroundPayloadError::InvalidLimits { field });
            }
        }
        for (field, value, ceiling) in [
            (
                "max_payload_bytes",
                max_payload_bytes,
                defaults.max_payload_bytes,
            ),
            ("max_polygons", max_polygons, defaults.max_polygons),
            ("max_vertices", max_vertices, defaults.max_vertices),
            ("max_links", max_links, defaults.max_links),
            (
                "max_detail_triangles",
                max_detail_triangles,
                defaults.max_detail_triangles,
            ),
            ("max_bv_nodes", max_bv_nodes, defaults.max_bv_nodes),
        ] {
            if value > ceiling {
                return Err(CheckedGroundPayloadError::InvalidLimits { field });
            }
        }
        if !max_abs_coordinate.is_finite()
            || !(0.0..=defaults.max_abs_coordinate).contains(&max_abs_coordinate)
        {
            return Err(CheckedGroundPayloadError::InvalidLimits {
                field: "max_abs_coordinate",
            });
        }

        Ok(Self {
            max_payload_bytes,
            max_polygons,
            max_vertices,
            max_links,
            max_detail_triangles,
            max_bv_nodes,
            max_abs_coordinate,
        })
    }
}

/// Read-only facts proven by [`CheckedDetour7GroundPayload::parse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedGroundMetadata {
    pub tile_x: i32,
    pub tile_y: i32,
    pub tile_layer: i32,
    pub polygon_count: usize,
    pub vertex_count: usize,
    pub link_count: usize,
    pub detail_vertex_count: usize,
    pub detail_triangle_count: usize,
    pub bv_node_count: usize,
}

/// An immutable borrow of a payload admitted by the Detour7 ground-only
/// profile. Fields are private so native add can only receive bytes that have
/// completed the bounded structural and index checks.
pub struct CheckedDetour7GroundPayload<'a> {
    bytes: &'a [u8],
    metadata: CheckedGroundMetadata,
}

impl std::fmt::Debug for CheckedDetour7GroundPayload<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckedDetour7GroundPayload")
            .field("byte_len", &self.bytes.len())
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl<'a> CheckedDetour7GroundPayload<'a> {
    /// Parses the built-in bounded profile without copying or mutating bytes.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, CheckedGroundPayloadError> {
        Self::parse_with_limits(bytes, CheckedGroundLimits::default())
    }

    /// Parses with caller-reduced logical file/count/numeric limits.
    pub fn parse_with_limits(
        bytes: &'a [u8],
        limits: CheckedGroundLimits,
    ) -> Result<Self, CheckedGroundPayloadError> {
        if !cfg!(target_endian = "little") {
            return Err(CheckedGroundPayloadError::UnsupportedTargetEndian);
        }
        if bytes.len() > limits.max_payload_bytes {
            return Err(CheckedGroundPayloadError::PayloadTooLarge {
                actual: bytes.len(),
                maximum: limits.max_payload_bytes,
            });
        }
        if bytes.len() < TILE_HEADER_SIZE {
            return Err(CheckedGroundPayloadError::HeaderTooShort {
                actual: bytes.len(),
            });
        }

        let magic = read_u32(bytes, 0).ok_or(CheckedGroundPayloadError::HeaderTooShort {
            actual: bytes.len(),
        })?;
        if magic != DT_NAVMESH_MAGIC_LIKE_CPP {
            return Err(CheckedGroundPayloadError::WrongMagic {
                actual: magic,
                expected: DT_NAVMESH_MAGIC_LIKE_CPP,
            });
        }
        let version = read_i32(bytes, 4).ok_or(CheckedGroundPayloadError::HeaderTooShort {
            actual: bytes.len(),
        })?;
        if version != DT_NAVMESH_VERSION_LIKE_CPP as i32 {
            return Err(CheckedGroundPayloadError::WrongVersion {
                actual: version,
                expected: DT_NAVMESH_VERSION_LIKE_CPP as i32,
            });
        }

        let tile_x = read_i32(bytes, 8).ok_or(CheckedGroundPayloadError::HeaderTooShort {
            actual: bytes.len(),
        })?;
        let tile_y = read_i32(bytes, 12).ok_or(CheckedGroundPayloadError::HeaderTooShort {
            actual: bytes.len(),
        })?;
        let tile_layer = read_i32(bytes, 16).ok_or(CheckedGroundPayloadError::HeaderTooShort {
            actual: bytes.len(),
        })?;
        if tile_layer != 0 {
            return Err(CheckedGroundPayloadError::UnsupportedTileLayer { actual: tile_layer });
        }
        if tile_x == i32::MIN || tile_x == i32::MAX || tile_y == i32::MIN || tile_y == i32::MAX {
            return Err(CheckedGroundPayloadError::TileCoordinateOverflowMargin { tile_x, tile_y });
        }

        let polygon_count = checked_count(bytes, 24, "polyCount", limits.max_polygons, false)?;
        let vertex_count = checked_count(bytes, 28, "vertCount", limits.max_vertices, false)?;
        let link_count = checked_count(bytes, 32, "maxLinkCount", limits.max_links, false)?;
        let detail_mesh_count =
            checked_count(bytes, 36, "detailMeshCount", limits.max_polygons, true)?;
        let detail_vertex_count =
            checked_count(bytes, 40, "detailVertCount", limits.max_vertices, true)?;
        let detail_triangle_count = checked_count(
            bytes,
            44,
            "detailTriCount",
            limits.max_detail_triangles,
            true,
        )?;
        let bv_node_count = checked_count(bytes, 48, "bvNodeCount", limits.max_bv_nodes, true)?;
        let off_mesh_count =
            checked_count(bytes, 52, "offMeshConCount", limits.max_polygons, true)?;
        let off_mesh_base =
            read_i32(bytes, 56).ok_or(CheckedGroundPayloadError::HeaderTooShort {
                actual: bytes.len(),
            })?;
        if detail_mesh_count != polygon_count {
            return Err(CheckedGroundPayloadError::DetailMeshCountMismatch {
                polygons: polygon_count,
                detail_meshes: detail_mesh_count,
            });
        }
        if off_mesh_count != 0 || off_mesh_base != polygon_count as i32 {
            return Err(CheckedGroundPayloadError::OffMeshOutsideProfile {
                count: off_mesh_count,
                base: off_mesh_base,
                expected_base: polygon_count,
            });
        }

        let walkable_height = header_float(bytes, 60, "walkableHeight", limits)?;
        let walkable_radius = header_float(bytes, 64, "walkableRadius", limits)?;
        let walkable_climb = header_float(bytes, 68, "walkableClimb", limits)?;
        if walkable_height < 0.0 || walkable_radius < 0.0 || walkable_climb < 0.0 {
            return Err(CheckedGroundPayloadError::NegativeWalkableDimension);
        }
        let mut bounds_min = [0.0; 3];
        let mut bounds_max = [0.0; 3];
        for axis in 0..3 {
            bounds_min[axis] = header_float(bytes, 72 + axis * 4, "bmin", limits)?;
            bounds_max[axis] = header_float(bytes, 84 + axis * 4, "bmax", limits)?;
            if bounds_min[axis] > bounds_max[axis] {
                return Err(CheckedGroundPayloadError::InvertedHeaderBounds { axis });
            }
        }
        let bv_quant_factor = header_float(bytes, 96, "bvQuantFactor", limits)?;
        if bv_quant_factor < 0.0 || (bv_node_count > 0 && bv_quant_factor <= 0.0) {
            return Err(CheckedGroundPayloadError::InvalidBvQuantFactor {
                value: bv_quant_factor,
            });
        }
        if bv_node_count > 0 {
            for axis in 0..3 {
                let span = bounds_max[axis] - bounds_min[axis];
                let quantized_span = span * bv_quant_factor;
                let quantized_max_with_native_rounding = quantized_span + 1.0;
                if !span.is_finite()
                    || !quantized_span.is_finite()
                    || !quantized_max_with_native_rounding.is_finite()
                    || quantized_max_with_native_rounding > f32::from(u16::MAX)
                {
                    return Err(CheckedGroundPayloadError::BvQuantizationOutOfRange {
                        axis,
                        span,
                        quant_factor: bv_quant_factor,
                    });
                }
            }
        }

        let vertices_at = TILE_HEADER_SIZE;
        let polygons_at = advance(vertices_at, vertex_count, 12, "vertices")?;
        let links_at = advance(polygons_at, polygon_count, POLY_SIZE, "polygons")?;
        let details_at = advance(links_at, link_count, LINK_SIZE, "links")?;
        let detail_vertices_at = advance(
            details_at,
            detail_mesh_count,
            DETAIL_MESH_SIZE,
            "detail meshes",
        )?;
        let detail_triangles_at = advance(
            detail_vertices_at,
            detail_vertex_count,
            12,
            "detail vertices",
        )?;
        let bv_nodes_at = advance(
            detail_triangles_at,
            detail_triangle_count,
            4,
            "detail triangles",
        )?;
        let payload_end = advance(bv_nodes_at, bv_node_count, BV_NODE_SIZE, "BV nodes")?;
        if payload_end != bytes.len() {
            return Err(CheckedGroundPayloadError::PayloadLengthMismatch {
                expected: payload_end,
                actual: bytes.len(),
            });
        }

        for vertex_index in 0..vertex_count {
            for axis in 0..3 {
                let value = read_f32(bytes, vertices_at + vertex_index * 12 + axis * 4).ok_or(
                    CheckedGroundPayloadError::PayloadLengthMismatch {
                        expected: payload_end,
                        actual: bytes.len(),
                    },
                )?;
                validate_coordinate(
                    value,
                    limits.max_abs_coordinate,
                    "vertex",
                    vertex_index,
                    axis,
                )?;
            }
        }

        for polygon_index in 0..polygon_count {
            let at = polygons_at + polygon_index * POLY_SIZE;
            // addTile rebuilds every poly firstLink before traversing links.
            // It resets the link freelist `next` fields, and each link's
            // reference/edge/portal fields are assigned before use. The
            // serialized pointer-like values are never trusted as topology.
            let vertex_count_for_poly = bytes[at + 30] as usize;
            let area_and_type = bytes[at + 31];
            let polygon_type = area_and_type >> 6;
            if !(3..=DETOUR_VERTS_PER_POLYGON).contains(&vertex_count_for_poly) || polygon_type != 0
            {
                return Err(CheckedGroundPayloadError::PolygonShapeOutsideProfile {
                    polygon_index,
                    vertex_count: vertex_count_for_poly,
                    polygon_type,
                });
            }

            let mut unique_vertices = [u16::MAX; DETOUR_VERTS_PER_POLYGON];
            for vertex_slot in 0..vertex_count_for_poly {
                let vertex_index = read_u16(bytes, at + 4 + vertex_slot * 2).ok_or(
                    CheckedGroundPayloadError::PayloadLengthMismatch {
                        expected: payload_end,
                        actual: bytes.len(),
                    },
                )?;
                if usize::from(vertex_index) >= vertex_count {
                    return Err(CheckedGroundPayloadError::PolygonVertexOutOfRange {
                        polygon_index,
                        vertex_slot,
                        vertex_index,
                        vertex_count,
                    });
                }
                if unique_vertices[..vertex_slot].contains(&vertex_index) {
                    return Err(CheckedGroundPayloadError::PolygonDuplicateVertex {
                        polygon_index,
                        vertex_index,
                    });
                }
                unique_vertices[vertex_slot] = vertex_index;
            }

            for edge in 0..vertex_count_for_poly {
                let neighbor = read_u16(bytes, at + 16 + edge * 2).ok_or(
                    CheckedGroundPayloadError::PayloadLengthMismatch {
                        expected: payload_end,
                        actual: bytes.len(),
                    },
                )?;
                if neighbor == 0 {
                    continue;
                }
                if neighbor & DT_EXT_LINK == 0 {
                    if usize::from(neighbor) > polygon_count {
                        return Err(CheckedGroundPayloadError::PolygonNeighborOutOfRange {
                            polygon_index,
                            edge,
                            neighbor,
                            polygon_count,
                        });
                    }
                    continue;
                }

                let direction = neighbor & 0x00ff;
                if !matches!(direction, 0 | 2 | 4 | 6) || neighbor != (DT_EXT_LINK | direction) {
                    return Err(CheckedGroundPayloadError::UnsupportedPortalDirection {
                        polygon_index,
                        edge,
                        neighbor,
                    });
                }
                validate_portal_edge(
                    bytes,
                    vertices_at,
                    at,
                    polygon_index,
                    edge,
                    vertex_count_for_poly,
                    direction,
                    limits.max_abs_coordinate,
                )?;
            }
        }

        for polygon_index in 0..detail_mesh_count {
            let at = details_at + polygon_index * DETAIL_MESH_SIZE;
            let vertex_base =
                read_u32(bytes, at).ok_or(CheckedGroundPayloadError::PayloadLengthMismatch {
                    expected: payload_end,
                    actual: bytes.len(),
                })? as usize;
            let triangle_base =
                read_u32(bytes, at + 4).ok_or(CheckedGroundPayloadError::PayloadLengthMismatch {
                    expected: payload_end,
                    actual: bytes.len(),
                })? as usize;
            let detail_vertices = bytes[at + 8] as usize;
            let detail_triangles = bytes[at + 9] as usize;
            let vertex_end = vertex_base.checked_add(detail_vertices);
            let triangle_end = triangle_base.checked_add(detail_triangles);
            if vertex_end.is_none_or(|end| end > detail_vertex_count)
                || triangle_end.is_none_or(|end| end > detail_triangle_count)
                || detail_triangles == 0
            {
                return Err(CheckedGroundPayloadError::DetailMeshRangeInvalid {
                    polygon_index,
                    vertex_base,
                    detail_vertices,
                    triangle_base,
                    detail_triangles,
                });
            }

            let poly_vertex_count = bytes[polygons_at + polygon_index * POLY_SIZE + 30] as usize;
            let maximum_local_vertex = poly_vertex_count + detail_vertices;
            for triangle_index in triangle_base..triangle_end.unwrap_or(triangle_base) {
                let triangle_at = detail_triangles_at + triangle_index * 4;
                for vertex_slot in 0..3 {
                    let local_index = bytes[triangle_at + vertex_slot] as usize;
                    if local_index >= maximum_local_vertex {
                        return Err(CheckedGroundPayloadError::DetailTriangleIndexOutOfRange {
                            polygon_index,
                            triangle_index,
                            vertex_slot,
                            local_index,
                            maximum_local_vertex,
                        });
                    }
                }
            }
        }

        for detail_vertex_index in 0..detail_vertex_count {
            for axis in 0..3 {
                let value = read_f32(
                    bytes,
                    detail_vertices_at + detail_vertex_index * 12 + axis * 4,
                )
                .ok_or(CheckedGroundPayloadError::PayloadLengthMismatch {
                    expected: payload_end,
                    actual: bytes.len(),
                })?;
                validate_coordinate(
                    value,
                    limits.max_abs_coordinate,
                    "detail vertex",
                    detail_vertex_index,
                    axis,
                )?;
            }
        }

        for node_index in 0..bv_node_count {
            let at = bv_nodes_at + node_index * BV_NODE_SIZE;
            for axis in 0..3 {
                let minimum = read_u16(bytes, at + axis * 2).ok_or(
                    CheckedGroundPayloadError::PayloadLengthMismatch {
                        expected: payload_end,
                        actual: bytes.len(),
                    },
                )?;
                let maximum = read_u16(bytes, at + 6 + axis * 2).ok_or(
                    CheckedGroundPayloadError::PayloadLengthMismatch {
                        expected: payload_end,
                        actual: bytes.len(),
                    },
                )?;
                if minimum > maximum {
                    return Err(CheckedGroundPayloadError::InvertedBvNodeBounds {
                        node_index,
                        axis,
                    });
                }
            }
            let leaf_or_escape = read_i32(bytes, at + 12).ok_or(
                CheckedGroundPayloadError::PayloadLengthMismatch {
                    expected: payload_end,
                    actual: bytes.len(),
                },
            )?;
            if leaf_or_escape >= 0 {
                if leaf_or_escape as usize >= polygon_count {
                    return Err(CheckedGroundPayloadError::BvLeafOutOfRange {
                        node_index,
                        polygon_index: leaf_or_escape,
                        polygon_count,
                    });
                }
            } else {
                let escape = leaf_or_escape.checked_neg().ok_or(
                    CheckedGroundPayloadError::BvEscapeOutOfRange {
                        node_index,
                        escape: leaf_or_escape,
                        node_count: bv_node_count,
                    },
                )? as usize;
                let escape_end = node_index.checked_add(escape);
                if escape == 0 || escape_end.is_none_or(|end| end > bv_node_count) {
                    return Err(CheckedGroundPayloadError::BvEscapeOutOfRange {
                        node_index,
                        escape: leaf_or_escape,
                        node_count: bv_node_count,
                    });
                }
            }
        }

        Ok(Self {
            bytes,
            metadata: CheckedGroundMetadata {
                tile_x,
                tile_y,
                tile_layer,
                polygon_count,
                vertex_count,
                link_count,
                detail_vertex_count,
                detail_triangle_count,
                bv_node_count,
            },
        })
    }

    #[must_use]
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    #[must_use]
    pub const fn metadata(&self) -> CheckedGroundMetadata {
        self.metadata
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CheckedGroundPayloadError {
    #[error("invalid checked-ground limit for {field}")]
    InvalidLimits { field: &'static str },
    #[error("checked-ground ingress requires a little-endian target")]
    UnsupportedTargetEndian,
    #[error("ground payload is too large: got {actual} bytes, maximum is {maximum}")]
    PayloadTooLarge { actual: usize, maximum: usize },
    #[error("Detour ground payload header is truncated: got {actual} bytes, expected at least 100")]
    HeaderTooShort { actual: usize },
    #[error("wrong Detour tile magic 0x{actual:08x}, expected 0x{expected:08x}")]
    WrongMagic { actual: u32, expected: u32 },
    #[error("wrong Detour tile version {actual}, expected {expected}")]
    WrongVersion { actual: i32, expected: i32 },
    #[error("invalid {field} count {value}")]
    InvalidCount { field: &'static str, value: i32 },
    #[error("{field} count {actual} exceeds its limit {maximum}")]
    CountExceedsLimit {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    #[error("only Detour tile layer 0 is admitted, got {actual}")]
    UnsupportedTileLayer { actual: i32 },
    #[error("tile coordinates must leave room for native neighbor +/-1: ({tile_x}, {tile_y})")]
    TileCoordinateOverflowMargin { tile_x: i32, tile_y: i32 },
    #[error("detail mesh count {detail_meshes} does not equal polygon count {polygons}")]
    DetailMeshCountMismatch {
        polygons: usize,
        detail_meshes: usize,
    },
    #[error(
        "off-mesh data is outside the selected ground profile: count={count}, base={base}, expected base={expected_base}"
    )]
    OffMeshOutsideProfile {
        count: usize,
        base: i32,
        expected_base: usize,
    },
    #[error("header float {field} is non-finite or outside the admitted numeric bound")]
    InvalidHeaderFloat { field: &'static str },
    #[error("walkable dimensions must be nonnegative")]
    NegativeWalkableDimension,
    #[error("header bounds are inverted on axis {axis}")]
    InvertedHeaderBounds { axis: usize },
    #[error("invalid BV quantization factor {value}")]
    InvalidBvQuantFactor { value: f32 },
    #[error(
        "BV quantization on axis {axis} is not representable for span {span} and factor {quant_factor}"
    )]
    BvQuantizationOutOfRange {
        axis: usize,
        span: f32,
        quant_factor: f32,
    },
    #[error("payload layout size overflow in {section}")]
    PayloadLayoutOverflow { section: &'static str },
    #[error("payload extent mismatch: layout ends at {expected}, input has {actual} bytes")]
    PayloadLengthMismatch { expected: usize, actual: usize },
    #[error("{kind} {index} coordinate {axis} is non-finite or outside the admitted numeric bound")]
    InvalidCoordinate {
        kind: &'static str,
        index: usize,
        axis: usize,
    },
    #[error(
        "polygon {polygon_index} has unsupported shape: {vertex_count} vertices, type {polygon_type}"
    )]
    PolygonShapeOutsideProfile {
        polygon_index: usize,
        vertex_count: usize,
        polygon_type: u8,
    },
    #[error(
        "polygon {polygon_index} vertex slot {vertex_slot} references {vertex_index}, but only {vertex_count} vertices exist"
    )]
    PolygonVertexOutOfRange {
        polygon_index: usize,
        vertex_slot: usize,
        vertex_index: u16,
        vertex_count: usize,
    },
    #[error("polygon {polygon_index} repeats vertex {vertex_index}")]
    PolygonDuplicateVertex {
        polygon_index: usize,
        vertex_index: u16,
    },
    #[error(
        "polygon {polygon_index} edge {edge} has invalid internal neighbor {neighbor} for {polygon_count} polygons"
    )]
    PolygonNeighborOutOfRange {
        polygon_index: usize,
        edge: usize,
        neighbor: u16,
        polygon_count: usize,
    },
    #[error(
        "polygon {polygon_index} edge {edge} has unsupported external portal encoding 0x{neighbor:04x}"
    )]
    UnsupportedPortalDirection {
        polygon_index: usize,
        edge: usize,
        neighbor: u16,
    },
    #[error("polygon {polygon_index} portal edge {edge} has zero tangent span")]
    ZeroPortalTangent { polygon_index: usize, edge: usize },
    #[error(
        "polygon {polygon_index} portal edge {edge} has non-finite or excessive slope arithmetic"
    )]
    InvalidPortalArithmetic { polygon_index: usize, edge: usize },
    #[error("detail range for polygon {polygon_index} is invalid")]
    DetailMeshRangeInvalid {
        polygon_index: usize,
        vertex_base: usize,
        detail_vertices: usize,
        triangle_base: usize,
        detail_triangles: usize,
    },
    #[error(
        "detail triangle {triangle_index}, vertex slot {vertex_slot} for polygon {polygon_index} references {local_index}, maximum local vertex count is {maximum_local_vertex}"
    )]
    DetailTriangleIndexOutOfRange {
        polygon_index: usize,
        triangle_index: usize,
        vertex_slot: usize,
        local_index: usize,
        maximum_local_vertex: usize,
    },
    #[error("BV node {node_index} bounds are inverted on axis {axis}")]
    InvertedBvNodeBounds { node_index: usize, axis: usize },
    #[error(
        "BV leaf node {node_index} references polygon {polygon_index}, but only {polygon_count} polygons exist"
    )]
    BvLeafOutOfRange {
        node_index: usize,
        polygon_index: i32,
        polygon_count: usize,
    },
    #[error("BV escape at node {node_index} is invalid ({escape}) for {node_count} nodes")]
    BvEscapeOutOfRange {
        node_index: usize,
        escape: i32,
        node_count: usize,
    },
}

fn checked_count(
    bytes: &[u8],
    at: usize,
    field: &'static str,
    maximum: usize,
    allow_zero: bool,
) -> Result<usize, CheckedGroundPayloadError> {
    let value = read_i32(bytes, at).ok_or(CheckedGroundPayloadError::HeaderTooShort {
        actual: bytes.len(),
    })?;
    if value < 0 || (!allow_zero && value == 0) {
        return Err(CheckedGroundPayloadError::InvalidCount { field, value });
    }
    let value = value as usize;
    if value > maximum {
        return Err(CheckedGroundPayloadError::CountExceedsLimit {
            field,
            actual: value,
            maximum,
        });
    }
    Ok(value)
}

fn header_float(
    bytes: &[u8],
    at: usize,
    field: &'static str,
    limits: CheckedGroundLimits,
) -> Result<f32, CheckedGroundPayloadError> {
    let value = read_f32(bytes, at).ok_or(CheckedGroundPayloadError::HeaderTooShort {
        actual: bytes.len(),
    })?;
    if !value.is_finite() || value.abs() > limits.max_abs_coordinate {
        return Err(CheckedGroundPayloadError::InvalidHeaderFloat { field });
    }
    Ok(value)
}

fn validate_coordinate(
    value: f32,
    maximum: f32,
    kind: &'static str,
    index: usize,
    axis: usize,
) -> Result<(), CheckedGroundPayloadError> {
    if !value.is_finite() || value.abs() > maximum {
        return Err(CheckedGroundPayloadError::InvalidCoordinate { kind, index, axis });
    }
    Ok(())
}

fn advance(
    at: usize,
    count: usize,
    stride: usize,
    section: &'static str,
) -> Result<usize, CheckedGroundPayloadError> {
    at.checked_add(
        count
            .checked_mul(stride)
            .ok_or(CheckedGroundPayloadError::PayloadLayoutOverflow { section })?,
    )
    .ok_or(CheckedGroundPayloadError::PayloadLayoutOverflow { section })
}

#[allow(clippy::too_many_arguments)]
fn validate_portal_edge(
    bytes: &[u8],
    vertices_at: usize,
    polygon_at: usize,
    polygon_index: usize,
    edge: usize,
    vertex_count: usize,
    direction: u16,
    maximum: f32,
) -> Result<(), CheckedGroundPayloadError> {
    let left_index = read_u16(bytes, polygon_at + 4 + edge * 2).ok_or(
        CheckedGroundPayloadError::PayloadLengthMismatch {
            expected: bytes.len(),
            actual: bytes.len(),
        },
    )? as usize;
    let right_slot = (edge + 1) % vertex_count;
    let right_index = read_u16(bytes, polygon_at + 4 + right_slot * 2).ok_or(
        CheckedGroundPayloadError::PayloadLengthMismatch {
            expected: bytes.len(),
            actual: bytes.len(),
        },
    )? as usize;
    let mut left = [0.0; 3];
    let mut right = [0.0; 3];
    for axis in 0..3 {
        left[axis] = read_f32(bytes, vertices_at + left_index * 12 + axis * 4).ok_or(
            CheckedGroundPayloadError::PayloadLengthMismatch {
                expected: bytes.len(),
                actual: bytes.len(),
            },
        )?;
        right[axis] = read_f32(bytes, vertices_at + right_index * 12 + axis * 4).ok_or(
            CheckedGroundPayloadError::PayloadLengthMismatch {
                expected: bytes.len(),
                actual: bytes.len(),
            },
        )?;
    }

    // Detour's slab code uses z as the tangent for x-side portals (0/4),
    // x for z-side portals (2/6), and y as the slab height. A zero or
    // non-finite divisor reaches division/byte quantization in connectExtLinks.
    let tangent_axis = if matches!(direction, 0 | 4) { 2 } else { 0 };
    let tangent_start = left[tangent_axis];
    let tangent_end = right[tangent_axis];
    let tangent_delta = tangent_end - tangent_start;
    if !tangent_delta.is_finite() || tangent_delta == 0.0 {
        return Err(CheckedGroundPayloadError::ZeroPortalTangent {
            polygon_index,
            edge,
        });
    }

    let vertical_start = left[1];
    let vertical_end = right[1];
    let slope = (vertical_end - vertical_start) / tangent_delta;
    let intercept = vertical_start - slope * tangent_start;
    let end_projection = slope * tangent_end;
    if !slope.is_finite()
        || slope.abs() > maximum
        || !intercept.is_finite()
        || !end_projection.is_finite()
        || intercept.abs() > maximum * maximum * 2.0
        || end_projection.abs() > maximum * maximum * 2.0
    {
        return Err(CheckedGroundPayloadError::InvalidPortalArithmetic {
            polygon_index,
            edge,
        });
    }
    Ok(())
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn read_i32(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn read_f32(bytes: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_bits(read_u32(bytes, at)?))
}
