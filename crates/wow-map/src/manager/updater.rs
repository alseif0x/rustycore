//! MapUpdater scheduling and worker accounting.

use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapUpdater {
    pub(super) worker_threads: usize,
    pub(super) pending_requests: usize,
    pub(super) scheduled_updates: usize,
    pub(super) wait_calls: usize,
}

impl MapUpdater {
    pub fn activate(&mut self, num_threads: usize) {
        self.worker_threads = self.worker_threads.saturating_add(num_threads);
    }

    pub fn deactivate(&mut self) {
        self.wait();
        self.worker_threads = 0;
    }

    pub const fn activated(&self) -> bool {
        self.worker_threads > 0
    }

    pub fn schedule_update(&mut self, map: &mut ManagedMap, diff_ms: u32) {
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        map.update(diff_ms);
        self.update_finished();
    }

    pub fn schedule_update_with_pool_update_context(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
    ) {
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        map.update_with_pool_update_context(diff_ms, spawn_store, pool_mgr);
        self.update_finished();
    }

    pub fn schedule_update_with_pool_update_loaded_grid_records_context<L>(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        load_record: &mut L,
    ) where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        map.update_with_pool_update_loaded_grid_records_context(
            diff_ms,
            spawn_store,
            pool_mgr,
            load_record,
        );
        self.update_finished();
    }

    /// The post-session half of one scheduled map update (#787). The updater
    /// still runs inline; the split only changes which phases this call covers.
    pub fn schedule_after_sessions(&mut self, map: &mut ManagedMap, diff_ms: u32) {
        self.schedule_after_sessions_with_owner_like_cpp(
            map,
            diff_ms,
            MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
        );
    }

    pub fn schedule_after_sessions_with_owner_like_cpp(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
    ) {
        self.schedule_after_sessions_with_owner_and_selection_like_cpp(
            map,
            diff_ms,
            creature_update_owner,
            MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
        );
    }

    pub fn schedule_after_sessions_with_owner_and_selection_like_cpp(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
        object_update_selection: MapObjectUpdateSelectionLikeCpp,
    ) {
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        map.update_after_sessions_with_creature_owner_and_selection_like_cpp(
            diff_ms,
            None,
            None::<
                &mut fn(
                    &mut Map,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
            creature_update_owner,
            object_update_selection,
        );
        self.update_finished();
    }

    /// As [`Self::schedule_after_sessions`], with the pool update context.
    pub fn schedule_after_sessions_with_pool_update_context(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
    ) {
        self.schedule_after_sessions_with_pool_update_context_and_owner_like_cpp(
            map,
            diff_ms,
            spawn_store,
            pool_mgr,
            MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
        );
    }

    pub fn schedule_after_sessions_with_pool_update_context_and_owner_like_cpp(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
    ) {
        self.schedule_after_sessions_with_pool_update_context_owner_and_selection_like_cpp(
            map,
            diff_ms,
            spawn_store,
            pool_mgr,
            creature_update_owner,
            MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
        );
    }

    pub fn schedule_after_sessions_with_pool_update_context_owner_and_selection_like_cpp(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
        object_update_selection: MapObjectUpdateSelectionLikeCpp,
    ) {
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        map.update_after_sessions_with_creature_owner_and_selection_like_cpp(
            diff_ms,
            Some((spawn_store, pool_mgr)),
            None::<
                &mut fn(
                    &mut Map,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
            creature_update_owner,
            object_update_selection,
        );
        self.update_finished();
    }

    /// As [`Self::schedule_after_sessions`], with the pool update context and
    /// the loaded-grid respawn record hook.
    pub fn schedule_after_sessions_with_pool_update_loaded_grid_records_context<L>(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        load_record: &mut L,
    ) where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.schedule_after_sessions_with_pool_update_loaded_grid_records_context_and_owner_like_cpp(
            map,
            diff_ms,
            spawn_store,
            pool_mgr,
            MapCreatureUpdateOwnerLikeCpp::CanonicalMap,
            load_record,
        );
    }

    pub fn schedule_after_sessions_with_pool_update_loaded_grid_records_context_and_owner_like_cpp<
        L,
    >(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
        load_record: &mut L,
    ) where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.schedule_after_sessions_with_pool_update_loaded_grid_records_context_owner_and_selection_like_cpp(
            map,
            diff_ms,
            spawn_store,
            pool_mgr,
            creature_update_owner,
            MapObjectUpdateSelectionLikeCpp::WholeTypedStores,
            load_record,
        );
    }

    pub fn schedule_after_sessions_with_pool_update_loaded_grid_records_context_owner_and_selection_like_cpp<
        L,
    >(
        &mut self,
        map: &mut ManagedMap,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
        object_update_selection: MapObjectUpdateSelectionLikeCpp,
        load_record: &mut L,
    ) where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        map.update_after_sessions_with_creature_owner_and_selection_like_cpp(
            diff_ms,
            Some((spawn_store, pool_mgr)),
            Some(load_record),
            creature_update_owner,
            object_update_selection,
        );
        self.update_finished();
    }

    pub fn wait(&mut self) {
        self.wait_calls += 1;
        debug_assert_eq!(self.pending_requests, 0);
    }

    pub(super) fn begin_staged_object_map(&mut self) -> bool {
        if !self.activated() {
            return false;
        }
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        true
    }

    pub(super) fn finish_staged_object_map(&mut self, accounting_started: bool) {
        if accounting_started {
            self.update_finished();
        }
    }

    pub const fn scheduled_updates(&self) -> usize {
        self.scheduled_updates
    }

    pub const fn wait_calls(&self) -> usize {
        self.wait_calls
    }

    pub(super) fn update_finished(&mut self) {
        self.pending_requests = self.pending_requests.saturating_sub(1);
    }
}
