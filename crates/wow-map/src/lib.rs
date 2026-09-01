pub mod cell;
pub mod coords;
pub mod grid;
pub mod grid_map;
pub mod grid_unload;
pub mod manager;
pub mod map;
pub mod object_grid_loader;
pub mod personal_phase;
pub mod pool;
pub mod spawn;
pub mod terrain;
pub mod vmap;

use std::fmt;

pub use cell::{Cell, CellArea, GridObjectGuids, WorldObjectGuids, calculate_cell_area_like_cpp};
pub use coords::{
    CellCoord, GridCoord, MAX_NUMBER_OF_CELLS, MAX_NUMBER_OF_GRIDS, SIZE_OF_GRID_CELL,
    SIZE_OF_GRIDS, TOTAL_NUMBER_OF_CELLS_PER_MAP, cell_to_grid_local, compute_cell_coord,
    compute_cell_coord_with_offset, compute_grid_coord, compute_grid_coord_simple,
    is_valid_map_coord, normalize_map_coord,
};
pub use grid::{
    DEFAULT_VISIBILITY_NOTIFY_PERIOD, GridInfo, GridStateKind, MapGridHost, NGrid, PeriodicTimer,
    TimeTracker, update_grid_state,
};
pub use grid_unload::{
    GridObjectKind, GridUnloadAction, GridUnloadApplyOutcome, GridUnloadEntityStore,
    GuidGridUnloadLifecycle, apply_grid_unload_action, apply_grid_unload_actions,
    object_grid_cleaner, object_grid_evacuator, object_grid_stoper, object_grid_unloader,
};
pub use manager::{
    CreateMapDecision, CreateMapDifficultyContext, CreateMapEntryContext, CreateMapEntryKind,
    CreateMapGroupContext, CreateMapInstanceLockContext, CreateMapPlayerContext,
    CreateMapSideEffect, ExistingInstanceMapContext, InstanceIdAllocator, MIN_GRID_DELAY_MS,
    MIN_MAP_UPDATE_DELAY_MS, ManagedMap, ManagedMapInnerLikeCpp, ManagedMapKind, MapManager,
    MapUpdater, PlayerHandle, PlayerOwnerError, PlayerResidenceLikeCpp,
};
pub use map::{
    AIRelocationPlan, ActiveObjectKind, AddToMapError, AddToMapOutcome,
    CheckRespawnCompositeOutcomeLikeCpp, CheckRespawnLinkedRespawnGuardOutcomeLikeCpp,
    CreatureDelayedRelocationVisibilityPlan, CreatureRelocationVisibilityPlan,
    DelayedCreatureRelocationContext, DelayedPlayerRelocationContext,
    DelayedUnitRelocationCellPlan, DelayedUnitRelocationForCellsPlan, DelayedUnitRelocationPlan,
    DelayedUnitRelocationVisibilityPlans, GameEventNpcFlagValuesUpdateLikeCpp, GridLifecycle, Map,
    MapObjectCellMoveState, MapObjectMoveListEntry, MapObjectMoveListPlan,
    MapObjectRelocationError, MapObjectRelocationOutcome, MapObjectStoreError,
    MapUpdatePlayerSources, MapUpdateVisitPlan, NearbyCellGuids, NearbyCellVisitCenter,
    NearbyCellVisitPlan, NoopGridLifecycle, NoopTerrainGridLoader, ObjectUpdatePlan,
    PlayerDelayedRelocationVisibilityPlan, PlayerRelocationVisibilityPlan,
    ProcessRelocationNotifiesOutcome, RelocationNotifyProcessPlan, RemoveFromMapError,
    RemoveFromMapOutcome, ResetNotifyFlagsOutcome, SpawnedPoolDataErrorLikeCpp,
    SpawnedPoolDataLikeCpp, TerrainGridLoader, cell_from_grid_center, cell_from_world,
    is_grid_id_loaded,
};
pub use object_grid_loader::{
    CorpseCellStore, CorpseGridObject, GridSpawnLoadFilter, LoadAllGridSpawns,
    ObjectGridLoadCounts, ObjectGridLoader, SpawnGridLifecycle,
};
pub use personal_phase::{
    MultiPersonalPhaseTracker, PERSONAL_PHASE_DELETE_TIME_DEFAULT_MS, PersonalPhaseSpawns,
    PhaseRef, PhaseShift, PlayerPersonalPhasesTracker,
};
pub use pool::{
    PoolDespawnObjectPlanLikeCpp, PoolDespawnPoolPlanLikeCpp, PoolGroupLikeCpp,
    PoolInitForMapErrorKindLikeCpp, PoolInitForMapErrorLikeCpp, PoolInitForMapPlanLikeCpp,
    PoolMemberKindLikeCpp, PoolMgrLikeCpp, PoolMgrPlanErrorLikeCpp, PoolMgrPlanSkipReasonLikeCpp,
    PoolObjectLikeCpp, PoolRelationRemovalLikeCpp, PoolSpawnObjectActionLikeCpp,
    PoolSpawnObjectPlanLikeCpp, PoolSpawnPoolPlanLikeCpp, PoolTemplateDataLikeCpp,
    PoolTypedDespawnPlanLikeCpp, PoolTypedSpawnPlanLikeCpp,
};
pub use spawn::{
    AddRespawnInfoOutcomeLikeCpp, CellSpawnGuids, CheckRespawnOutcomeLikeCpp,
    CheckRespawnSpawnGroupGuardOutcomeLikeCpp, Difficulty, LinkedRespawnLoadIssueKindLikeCpp,
    LinkedRespawnLoadIssueLikeCpp, LinkedRespawnLoadReportLikeCpp, LinkedRespawnRowLikeCpp,
    LinkedRespawnStoreLikeCpp, LinkedRespawnTypeLikeCpp, PersonalSpawnMapKey,
    ProcessRespawnActionLikeCpp, RespawnInfoLikeCpp, RespawnStoreLikeCpp, SpawnData,
    SpawnGridLoadStateLikeCpp, SpawnGroupActiveChange, SpawnGroupFlags, SpawnGroupRuntimeState,
    SpawnGroupTemplateData, SpawnId, SpawnMapKey, SpawnObjectType, SpawnPosition, SpawnStore,
};
pub use terrain::{GridMapTerrain, VMAP_INVALID_HEIGHT_VALUE};
pub use vmap::{
    SharedStaticVMapLineOfSightProvider, StaticVMapLineOfSightProvider, VMapLineOfSightEndpoint,
    VMapLineOfSightQuery, VMapPosition,
};

/// Key used by `MapManager` for world and instance map lookup.
///
/// C++ ref: `Maps/MapManager.h`, `using MapKey = std::pair<uint32, uint32>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MapKey {
    pub map_id: u32,
    pub instance_id: u32,
}

impl MapKey {
    pub const fn new(map_id: u32, instance_id: u32) -> Self {
        Self {
            map_id,
            instance_id,
        }
    }
}

impl fmt::Display for MapKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MapKey({}, {})", self.map_id, self.instance_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_key_keeps_trinity_uint32_shape() {
        let key = MapKey::new(u32::MAX, 42);
        assert_eq!(key.map_id, u32::MAX);
        assert_eq!(key.instance_id, 42);
        assert_eq!(key.to_string(), "MapKey(4294967295, 42)");
    }

    #[test]
    fn map_key_orders_like_std_pair_for_range_queries() {
        let mut keys = [
            MapKey::new(571, 3),
            MapKey::new(0, 0),
            MapKey::new(571, 1),
            MapKey::new(1, 0),
        ];
        keys.sort();

        assert_eq!(
            keys,
            [
                MapKey::new(0, 0),
                MapKey::new(1, 0),
                MapKey::new(571, 1),
                MapKey::new(571, 3),
            ]
        );
    }
}
