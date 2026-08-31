//! SQLx-free World source contract for immutable C++ phasing metadata.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainWorldMapPersistenceRowLikeCpp {
    pub terrain_swap_map: u32,
    pub ui_map_phase_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainSwapDefaultPersistenceRowLikeCpp {
    pub map_id: u32,
    pub terrain_swap_map: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseAreaPersistenceRowLikeCpp {
    pub area_id: u32,
    pub phase_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseNamePersistenceRowLikeCpp {
    pub phase_id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseWorldCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// C++ `ObjectMgr` World-table source for immutable phasing metadata.
///
/// The four operations remain independent because production currently
/// publishes their domain stores at distinct startup fences. The port exposes
/// the capability, not table-generic query access.
pub trait PhaseWorldCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_phase_area_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PhaseWorldCatalogLoadOutcomeLikeCpp<PhaseAreaPersistenceRowLikeCpp>,
    >;

    fn load_phase_name_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PhaseWorldCatalogLoadOutcomeLikeCpp<PhaseNamePersistenceRowLikeCpp>,
    >;

    fn load_terrain_world_map_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PhaseWorldCatalogLoadOutcomeLikeCpp<TerrainWorldMapPersistenceRowLikeCpp>,
    >;

    fn load_terrain_swap_default_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PhaseWorldCatalogLoadOutcomeLikeCpp<TerrainSwapDefaultPersistenceRowLikeCpp>,
    >;
}
