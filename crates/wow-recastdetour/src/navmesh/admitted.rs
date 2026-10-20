//! Explicit checked admission and corridor queries for the in-memory manager.
//!
//! These operations reuse `MMapManager` and `MMapData` as their only native
//! state authority. They have no filesystem fallback and do not claim that
//! valid framing establishes geometric or gameplay correctness.

use super::{
    CHECKED_GROUND_MAX_ABS_COORDINATE, CheckedDetour7GroundPayload, DetourNavMeshError,
    DetourNavMeshParams, DetourNavMeshQuery, DetourNavMeshQueryError, DetourNearestPoly,
    DetourPathResult, DetourPolyRef, DetourQueryFilter, DetourTileError, MMapData, MMapManager,
    MMapNavMeshQuery, pack_tile_id_like_cpp,
};
use std::mem::ManuallyDrop;
use thiserror::Error;

pub const CHECKED_NATIVE_MAX_TILE_SLOTS: i32 = 4096;
pub const CHECKED_NATIVE_MAX_TILE_DIMENSION: f32 = 1_048_576.0;
pub const CHECKED_CORRIDOR_MAX_PATH: usize = 4096;

/// Caller-downward limits for native mesh parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckedNativeParamsLimits {
    max_tile_slots: i32,
    max_tile_dimension: f32,
    max_abs_coordinate: f32,
}

impl Default for CheckedNativeParamsLimits {
    fn default() -> Self {
        Self {
            max_tile_slots: CHECKED_NATIVE_MAX_TILE_SLOTS,
            max_tile_dimension: CHECKED_NATIVE_MAX_TILE_DIMENSION,
            max_abs_coordinate: CHECKED_GROUND_MAX_ABS_COORDINATE,
        }
    }
}

impl CheckedNativeParamsLimits {
    pub fn new(
        max_tile_slots: i32,
        max_tile_dimension: f32,
        max_abs_coordinate: f32,
    ) -> Result<Self, CheckedNativeParamsError> {
        let defaults = Self::default();
        if !(1..=defaults.max_tile_slots).contains(&max_tile_slots) {
            return Err(CheckedNativeParamsError::InvalidLimits {
                field: "max_tile_slots",
            });
        }
        if !max_tile_dimension.is_finite()
            || !(1.0..=defaults.max_tile_dimension).contains(&max_tile_dimension)
        {
            return Err(CheckedNativeParamsError::InvalidLimits {
                field: "max_tile_dimension",
            });
        }
        if !max_abs_coordinate.is_finite()
            || !(0.0..=defaults.max_abs_coordinate).contains(&max_abs_coordinate)
        {
            return Err(CheckedNativeParamsError::InvalidLimits {
                field: "max_abs_coordinate",
            });
        }
        Ok(Self {
            max_tile_slots,
            max_tile_dimension,
            max_abs_coordinate,
        })
    }
}

/// Validated native allocation parameters. Its private value cannot be
/// mutated after admission.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckedDetourNavMeshParams {
    params: DetourNavMeshParams,
}

impl CheckedDetourNavMeshParams {
    pub fn new(
        params: DetourNavMeshParams,
        limits: CheckedNativeParamsLimits,
    ) -> Result<Self, CheckedNativeParamsError> {
        for axis in 0..3 {
            let value = params.origin[axis];
            if !value.is_finite() || value.abs() > limits.max_abs_coordinate {
                return Err(CheckedNativeParamsError::InvalidOrigin { axis, value });
            }
        }
        for (field, value) in [
            ("tile_width", params.tile_width),
            ("tile_height", params.tile_height),
        ] {
            if !value.is_finite() || value < 1.0 || value > limits.max_tile_dimension {
                return Err(CheckedNativeParamsError::InvalidTileDimension { field, value });
            }
        }
        if !(1..=limits.max_tile_slots).contains(&params.max_tiles) {
            return Err(CheckedNativeParamsError::InvalidTileSlots {
                actual: params.max_tiles,
                maximum: limits.max_tile_slots,
            });
        }
        if params.max_polys != i32::MIN && params.max_polys <= 0 {
            return Err(CheckedNativeParamsError::InvalidMaxPolys {
                actual: params.max_polys,
            });
        }
        Ok(Self { params })
    }

    #[must_use]
    pub const fn params(self) -> DetourNavMeshParams {
        self.params
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CheckedNativeParamsError {
    #[error("invalid checked native-parameter limit for {field}")]
    InvalidLimits { field: &'static str },
    #[error("navmesh origin axis {axis} is invalid or exceeds its numeric limit: {value}")]
    InvalidOrigin { axis: usize, value: f32 },
    #[error("navmesh {field} is outside the admitted finite range: {value}")]
    InvalidTileDimension { field: &'static str, value: f32 },
    #[error("navmesh tile slots {actual} are outside 1..={maximum}")]
    InvalidTileSlots { actual: i32, maximum: i32 },
    #[error(
        "navmesh maxPolys must be positive or the Detour 64-bit sentinel i32::MIN, got {actual}"
    )]
    InvalidMaxPolys { actual: i32 },
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum MMapAdmissionError {
    #[error("checked admission requires initialize_thread_unsafe first")]
    ThreadUnsafeMapSetRequired,
    #[error("map {map_id} is not in the initialized thread-unsafe map set")]
    MapNotInThreadUnsafeSet { map_id: u32 },
    #[error("map {map_id} has no checked native parameters admitted yet")]
    NavMeshParamsNotAdmitted { map_id: u32 },
    #[error("map {map_id} already has different native parameters")]
    ConflictingNavMeshParams { map_id: u32 },
    #[error("failed to initialize checked native navmesh: {0}")]
    NavMesh(DetourNavMeshError),
    #[error("failed to add checked native ground tile: {0}")]
    Tile(DetourTileError),
    #[error("failed to initialize checked instance query: {0}")]
    Query(DetourNavMeshQueryError),
}

impl MMapData {
    /// Admits a checked payload under the caller's original WoW grid key.
    /// `tile_x`/`tile_y` in the native payload are separate Detour coordinates
    /// and are intentionally not used as this manager key.
    pub fn admit_checked_ground_tile(
        &mut self,
        packed_wow_grid_key: u32,
        payload: &CheckedDetour7GroundPayload<'_>,
    ) -> Result<bool, DetourTileError> {
        if self.loaded_tile_refs.contains_key(&packed_wow_grid_key) {
            return Ok(false);
        }
        let tile_ref = self.nav_mesh.add_checked_ground_tile(payload)?;
        self.loaded_tile_refs.insert(packed_wow_grid_key, tile_ref);
        Ok(true)
    }

    /// Finds the two nearest polygons and, when both exist, returns Detour's
    /// raw corridor and status bits. Positions and half-extents are already
    /// in Detour `(x, y, z)` space; axis projection belongs to the caller.
    #[allow(clippy::too_many_arguments)]
    pub fn find_detour_corridor_for_instance(
        &self,
        instance_map_id: u32,
        instance_id: u32,
        filter: &DetourQueryFilter,
        start_position: [f32; 3],
        end_position: [f32; 3],
        half_extents: [f32; 3],
        max_path: usize,
    ) -> Result<DetourCorridorQueryResult, DetourCorridorQueryError> {
        validate_position("start", start_position)?;
        validate_position("end", end_position)?;
        validate_extents(half_extents)?;
        if max_path == 0 || max_path > CHECKED_CORRIDOR_MAX_PATH {
            return Err(DetourCorridorQueryError::InvalidPathCapacity {
                actual: max_path,
                maximum: CHECKED_CORRIDOR_MAX_PATH,
            });
        }

        let retained_query = self
            .nav_mesh_queries
            .get(&(instance_map_id, instance_id))
            .ok_or(DetourCorridorQueryError::MissingInstanceQuery {
                instance_map_id,
                instance_id,
            })?;

        // MMapData owns both fields. The scoped adapter ties the temporary
        // non-owning query view to the mesh borrow and suppresses freeing the
        // retained native query when the operation returns.
        with_retained_query(&self.nav_mesh, retained_query, |query| {
            let start_native = query.find_nearest_poly(start_position, half_extents, filter)?;
            let end_native = query.find_nearest_poly(end_position, half_extents, filter)?;
            let start = nearest_fact(start_native);
            let end = nearest_fact(end_native);
            let path = match (start.poly_ref, end.poly_ref) {
                (Some(start_ref), Some(end_ref)) => Some(query.find_path_with_status(
                    start_ref,
                    end_ref,
                    start_position,
                    end_position,
                    filter,
                    max_path,
                )?),
                _ => None,
            };

            Ok(DetourCorridorQueryResult { start, end, path })
        })
    }
}

impl MMapManager {
    /// Creates a native mesh only for a map admitted by
    /// `initialize_thread_unsafe`. A repeated identical admission is a no-op;
    /// conflicting parameters never replace existing native state.
    pub fn admit_checked_nav_mesh_params(
        &mut self,
        map_id: u32,
        params: CheckedDetourNavMeshParams,
    ) -> Result<bool, MMapAdmissionError> {
        self.require_thread_unsafe_map(map_id)?;
        match self.loaded_mmaps.get(&map_id) {
            Some(Some(data)) if data.nav_mesh_params == params.params() => return Ok(false),
            Some(Some(_)) => {
                return Err(MMapAdmissionError::ConflictingNavMeshParams { map_id });
            }
            Some(None) => {}
            None => return Err(MMapAdmissionError::MapNotInThreadUnsafeSet { map_id }),
        }

        let data = MMapData::new(params.params()).map_err(MMapAdmissionError::NavMesh)?;
        self.loaded_mmaps.insert(map_id, Some(data));
        Ok(true)
    }

    /// Adds one checked ground payload using the original WoW grid coordinates
    /// for duplicate tracking and counters. No file is opened or inferred.
    pub fn admit_checked_ground_tile(
        &mut self,
        map_id: u32,
        wow_grid_x: i32,
        wow_grid_y: i32,
        payload: &CheckedDetour7GroundPayload<'_>,
    ) -> Result<bool, MMapAdmissionError> {
        self.require_thread_unsafe_map(map_id)?;
        let Some(data) = self.loaded_mmaps.get_mut(&map_id).and_then(Option::as_mut) else {
            return Err(MMapAdmissionError::NavMeshParamsNotAdmitted { map_id });
        };
        let packed_key = pack_tile_id_like_cpp(wow_grid_x, wow_grid_y);
        let added = data
            .admit_checked_ground_tile(packed_key, payload)
            .map_err(MMapAdmissionError::Tile)?;
        if added {
            self.loaded_tiles = self.loaded_tiles.saturating_add(1);
        }
        Ok(added)
    }

    /// Retains an instance query for a checked map using the caller's bounded
    /// node budget. Repeating the same budget is a no-op; changing it requires
    /// unloading the retained query first. This path never falls back to
    /// filesystem map loading.
    pub fn admit_instance_query(
        &mut self,
        mesh_map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
        max_nodes: i32,
    ) -> Result<bool, MMapAdmissionError> {
        self.require_thread_unsafe_map(mesh_map_id)?;
        let Some(data) = self
            .loaded_mmaps
            .get_mut(&mesh_map_id)
            .and_then(Option::as_mut)
        else {
            return Err(MMapAdmissionError::NavMeshParamsNotAdmitted {
                map_id: mesh_map_id,
            });
        };
        data.load_nav_mesh_query_with_max_nodes(instance_map_id, instance_id, max_nodes)
            .map_err(MMapAdmissionError::Query)
    }

    fn require_thread_unsafe_map(&self, map_id: u32) -> Result<(), MMapAdmissionError> {
        if self.thread_safe_environment {
            return Err(MMapAdmissionError::ThreadUnsafeMapSetRequired);
        }
        if !self.loaded_mmaps.contains_key(&map_id) {
            return Err(MMapAdmissionError::MapNotInThreadUnsafeSet { map_id });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetourNearestPolyFact {
    /// `None` means Detour returned reference zero for this nearest-poly call.
    pub poly_ref: Option<DetourPolyRef>,
    pub nearest_point: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetourCorridorQueryResult {
    pub start: DetourNearestPolyFact,
    pub end: DetourNearestPolyFact,
    /// `None` means at least one endpoint had no nearest polygon. The caller
    /// must not treat that outcome as a complete route.
    pub path: Option<DetourPathResult>,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum DetourCorridorQueryError {
    #[error("instance query ({instance_map_id}, {instance_id}) is not admitted")]
    MissingInstanceQuery {
        instance_map_id: u32,
        instance_id: u32,
    },
    #[error(
        "{endpoint} position axis {axis} is non-finite or outside the checked numeric bound: {value}"
    )]
    InvalidPosition {
        endpoint: &'static str,
        axis: usize,
        value: f32,
    },
    #[error(
        "query half-extent axis {axis} is non-finite, negative, or outside the checked numeric bound: {value}"
    )]
    InvalidHalfExtent { axis: usize, value: f32 },
    #[error("path capacity {actual} is outside 1..={maximum}")]
    InvalidPathCapacity { actual: usize, maximum: usize },
    #[error("Detour corridor query failed: {0}")]
    Native(DetourNavMeshQueryError),
}

impl From<DetourNavMeshQueryError> for DetourCorridorQueryError {
    fn from(value: DetourNavMeshQueryError) -> Self {
        Self::Native(value)
    }
}

fn nearest_fact(value: DetourNearestPoly) -> DetourNearestPolyFact {
    DetourNearestPolyFact {
        poly_ref: (value.poly_ref != 0).then_some(value.poly_ref),
        nearest_point: value.nearest_point,
    }
}

fn with_retained_query<'mesh, R>(
    _mesh: &'mesh super::DetourNavMesh,
    retained: &'mesh MMapNavMeshQuery,
    operation: impl FnOnce(&DetourNavMeshQuery<'mesh>) -> R,
) -> R {
    let query = ManuallyDrop::new(DetourNavMeshQuery {
        raw: retained.raw,
        _mesh_lifetime_and_thread_model: std::marker::PhantomData,
    });
    operation(&query)
}

fn validate_position(
    endpoint: &'static str,
    position: [f32; 3],
) -> Result<(), DetourCorridorQueryError> {
    for (axis, value) in position.into_iter().enumerate() {
        if !value.is_finite() || value.abs() > CHECKED_GROUND_MAX_ABS_COORDINATE {
            return Err(DetourCorridorQueryError::InvalidPosition {
                endpoint,
                axis,
                value,
            });
        }
    }
    Ok(())
}

fn validate_extents(extents: [f32; 3]) -> Result<(), DetourCorridorQueryError> {
    for (axis, value) in extents.into_iter().enumerate() {
        if !value.is_finite() || value < 0.0 || value > CHECKED_GROUND_MAX_ABS_COORDINATE {
            return Err(DetourCorridorQueryError::InvalidHalfExtent { axis, value });
        }
    }
    Ok(())
}
