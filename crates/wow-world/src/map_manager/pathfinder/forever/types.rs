// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: selected Forever navigation package consumer.
//! Public request, result, limit, and diagnostic types for selected navigation.

use std::{error::Error as StdError, fmt};

use wow_core::Position;
use wow_entities::PhaseShift;
use wow_recastdetour::{
    CheckedGroundPayloadError, CheckedNativeParamsError, DetourCorridorQueryError,
    DetourNearestPolyFact, DetourPathResult, DetourQueryFilterError, MMapAdmissionError,
    PathQueryFilterContext,
};

pub use forever_navigation::package_v23::{
    Context as ForeverNavigationContext, Expected as ForeverNavigationExpected,
    Locale as ForeverNavigationLocale, Summary as ForeverNavigationSummary,
};
pub use forever_navigation::tilebuilder::Coordinate as ForeverNavigationTile;

pub const MAX_FOREVER_NAVIGATION_TILES: usize = 4096;
pub const MAX_FOREVER_NAVIGATION_TILES_PER_BATCH: usize = 64;
pub const MAX_FOREVER_NAVIGATION_BATCH_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_FOREVER_NAVIGATION_LOADED_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_FOREVER_NAVIGATION_FILE_BYTES: usize = 64 * 1024 * 1024 + 20;
pub const MAX_FOREVER_NAVIGATION_INSTANCE_QUERIES: usize = 64;
pub const FOREVER_NAVIGATION_QUERY_NODE_BUDGET: i32 = 4096;
pub const MAX_FOREVER_NAVIGATION_PATH_POLYS: usize = 256;

/// Downward-only product limits for one selected-navigation worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeverNavigationLimits {
    max_loaded_tiles: usize,
    max_loaded_bytes: usize,
    max_tiles_per_batch: usize,
    max_batch_bytes: usize,
    max_file_bytes: usize,
    max_instance_queries: usize,
}

impl Default for ForeverNavigationLimits {
    fn default() -> Self {
        Self {
            max_loaded_tiles: MAX_FOREVER_NAVIGATION_TILES,
            max_loaded_bytes: MAX_FOREVER_NAVIGATION_LOADED_BYTES,
            max_tiles_per_batch: MAX_FOREVER_NAVIGATION_TILES_PER_BATCH,
            max_batch_bytes: MAX_FOREVER_NAVIGATION_BATCH_BYTES,
            max_file_bytes: MAX_FOREVER_NAVIGATION_FILE_BYTES,
            max_instance_queries: MAX_FOREVER_NAVIGATION_INSTANCE_QUERIES,
        }
    }
}

impl ForeverNavigationLimits {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        max_loaded_tiles: usize,
        max_loaded_bytes: usize,
        max_tiles_per_batch: usize,
        max_batch_bytes: usize,
        max_file_bytes: usize,
        max_instance_queries: usize,
    ) -> Result<Self, ForeverNavigationError> {
        let defaults = Self::default();
        for (field, value, maximum) in [
            (
                "max_loaded_tiles",
                max_loaded_tiles,
                defaults.max_loaded_tiles,
            ),
            (
                "max_loaded_bytes",
                max_loaded_bytes,
                defaults.max_loaded_bytes,
            ),
            (
                "max_tiles_per_batch",
                max_tiles_per_batch,
                defaults.max_tiles_per_batch,
            ),
            ("max_batch_bytes", max_batch_bytes, defaults.max_batch_bytes),
            ("max_file_bytes", max_file_bytes, defaults.max_file_bytes),
            (
                "max_instance_queries",
                max_instance_queries,
                defaults.max_instance_queries,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(ForeverNavigationError::InvalidLimit { field });
            }
        }
        if max_file_bytes < forever_navigation::mmap::TILE_HEADER_BYTES + 100 {
            return Err(ForeverNavigationError::InvalidLimit {
                field: "max_file_bytes",
            });
        }

        Ok(Self {
            max_loaded_tiles,
            max_loaded_bytes,
            max_tiles_per_batch,
            max_batch_bytes,
            max_file_bytes,
            max_instance_queries,
        })
    }

    pub const fn max_loaded_tiles(self) -> usize {
        self.max_loaded_tiles
    }

    pub const fn max_loaded_bytes(self) -> usize {
        self.max_loaded_bytes
    }

    pub const fn max_tiles_per_batch(self) -> usize {
        self.max_tiles_per_batch
    }

    pub const fn max_batch_bytes(self) -> usize {
        self.max_batch_bytes
    }

    pub const fn max_file_bytes(self) -> usize {
        self.max_file_bytes
    }

    pub const fn max_instance_queries(self) -> usize {
        self.max_instance_queries
    }
}

/// One explicit raw corridor query against the selected package.
#[derive(Clone, Debug)]
pub struct ForeverNavigationQueryRequest {
    pub map_id: u32,
    pub instance_map_id: u32,
    pub instance_id: u32,
    pub start: Position,
    pub destination: Position,
    pub phase_shift: PhaseShift,
    pub filter_context: PathQueryFilterContext,
    /// Detour-space `(x, y, z)` half-extents for nearest-poly lookup.
    pub half_extents: [f32; 3],
    /// Caller-reduced corridor capacity. The product maximum is 256 polys.
    pub max_path: usize,
}

/// The unmodified Detour corridor response, including nearest-poly facts.
#[derive(Clone, Debug, PartialEq)]
pub struct ForeverNavigationCorridor {
    pub start: DetourNearestPolyFact,
    pub destination: DetourNearestPolyFact,
    pub path: DetourPathResult,
}

/// Counts derived from the one authoritative native manager and immutable package metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeverNavigationStats {
    pub map_id: u32,
    pub loaded_tiles: usize,
    pub loaded_tile_artifact_bytes: u64,
    pub global_artifact_bytes: u64,
    pub loaded_artifact_bytes: u64,
    pub instance_queries: usize,
    pub poisoned: bool,
}

/// Typed failures for package admission, bounded native state, and raw queries.
#[derive(Debug)]
pub enum ForeverNavigationError {
    InvalidExpected {
        field: &'static str,
    },
    InvalidLimit {
        field: &'static str,
    },
    NotConfigured,
    Poisoned,
    WrongMap {
        actual: u32,
        expected: u32,
    },
    UnsupportedPhase,
    InvalidPosition {
        endpoint: &'static str,
        axis: usize,
    },
    PositionOutsideMap {
        endpoint: &'static str,
    },
    PackageChanged,
    NativeStateDiverged {
        packed_tile_key: u32,
    },
    DuplicateTile(ForeverNavigationTile),
    BatchTooLarge {
        actual: usize,
        maximum: usize,
    },
    TileOutsideCoverage(ForeverNavigationTile),
    DeclaredEmptyTile(ForeverNavigationTile),
    TileNotPrepared(ForeverNavigationTile),
    LoadedTileLimit {
        actual: usize,
        maximum: usize,
    },
    LoadedByteLimit {
        actual: u64,
        maximum: usize,
    },
    InstanceQueryLimit {
        actual: usize,
        maximum: usize,
    },
    Package(forever_navigation::package_v23::Error),
    Mmap(forever_navigation::mmap::Error),
    CheckedGround(CheckedGroundPayloadError),
    CheckedParams(CheckedNativeParamsError),
    Filter(DetourQueryFilterError),
    NativeAdmission(MMapAdmissionError),
    NativeCorridor(DetourCorridorQueryError),
    NoNearestPolygon {
        start: DetourNearestPolyFact,
        destination: DetourNearestPolyFact,
    },
    NonCanonicalGlobalParams,
    TilePlacementMismatch {
        tile: ForeverNavigationTile,
        expected: (i32, i32),
        actual: (i32, i32),
    },
    TileBoundsHeaderUnavailable(ForeverNavigationTile),
    TileBoundsMismatch {
        tile: ForeverNavigationTile,
        expected: [f32; 4],
        actual: [f32; 4],
    },
    RollbackFailed {
        add: String,
        rollback: String,
    },
    NativeUnloadFailed(String),
    WorkerSpawn(String),
    WorkerStopped,
    UnexpectedWorkerResponse,
    #[cfg(feature = "test-fixtures")]
    TestInjectedNativeFailure,
}

impl fmt::Display for ForeverNavigationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl StdError for ForeverNavigationError {}

impl From<forever_navigation::package_v23::Error> for ForeverNavigationError {
    fn from(value: forever_navigation::package_v23::Error) -> Self {
        Self::Package(value)
    }
}

impl From<forever_navigation::mmap::Error> for ForeverNavigationError {
    fn from(value: forever_navigation::mmap::Error) -> Self {
        Self::Mmap(value)
    }
}

impl From<CheckedGroundPayloadError> for ForeverNavigationError {
    fn from(value: CheckedGroundPayloadError) -> Self {
        Self::CheckedGround(value)
    }
}

impl From<CheckedNativeParamsError> for ForeverNavigationError {
    fn from(value: CheckedNativeParamsError) -> Self {
        Self::CheckedParams(value)
    }
}

impl From<DetourQueryFilterError> for ForeverNavigationError {
    fn from(value: DetourQueryFilterError) -> Self {
        Self::Filter(value)
    }
}

impl From<MMapAdmissionError> for ForeverNavigationError {
    fn from(value: MMapAdmissionError) -> Self {
        Self::NativeAdmission(value)
    }
}

impl From<DetourCorridorQueryError> for ForeverNavigationError {
    fn from(value: DetourCorridorQueryError) -> Self {
        Self::NativeCorridor(value)
    }
}

#[derive(Debug)]
pub(in crate::map_manager::pathfinder) enum ForeverNavigationWorkerAction {
    PrepareTiles {
        map_id: u32,
        tiles: Vec<ForeverNavigationTile>,
    },
    UnloadTiles {
        map_id: u32,
        tiles: Vec<ForeverNavigationTile>,
    },
    PrepareInstance {
        map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
    },
    UnloadInstance {
        map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
    },
    Query(ForeverNavigationQueryRequest),
    Stats,
    #[cfg(feature = "test-fixtures")]
    SetFailurePlan(ForeverNavigationTestFailurePlan),
}

#[derive(Debug)]
pub(in crate::map_manager::pathfinder) enum ForeverNavigationWorkerResponse {
    Unit,
    Stats(ForeverNavigationStats),
    Corridor(ForeverNavigationCorridor),
}

#[cfg(feature = "test-fixtures")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeverNavigationTestFailurePlan {
    /// Inject a failure after this many real new native tile admissions.
    pub fail_after_native_adds: usize,
    /// Simulate rollback failure before native unload for the first new tile.
    pub fail_rollback: bool,
}
