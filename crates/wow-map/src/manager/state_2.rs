//! Managed-map lifecycle and updater state definitions, part 2 of 2.
//!
//! Separated from the manager.rs root under #644. Behaviour is preserved.

use super::*;

impl MapManager {
    pub fn new(grid_cleanup_delay_ms: u32, map_update_interval_ms: u32) -> Self {
        let mut manager = Self {
            grid_cleanup_delay_ms: MIN_GRID_DELAY_MS,
            maps: BTreeMap::new(),
            timer: IntervalTimer::new(MIN_MAP_UPDATE_DELAY_MS),
            instance_ids: InstanceIdAllocator::new(),
            updater: MapUpdater::default(),
            scheduled_scripts: 0,
            spawn_group_initializer_like_cpp: None,
            player_owners_like_cpp: BTreeMap::new(),
            detached_players_like_cpp: BTreeMap::new(),
            next_player_generation_like_cpp: 1,
        };
        manager.set_grid_cleanup_delay(grid_cleanup_delay_ms);
        manager.set_map_update_interval(map_update_interval_ms);
        manager
    }

    pub const fn grid_cleanup_delay_ms(&self) -> u32 {
        self.grid_cleanup_delay_ms
    }

    pub fn set_grid_cleanup_delay(&mut self, delay_ms: u32) {
        self.grid_cleanup_delay_ms = delay_ms.max(MIN_GRID_DELAY_MS);
    }

    pub fn set_map_update_interval(&mut self, interval_ms: u32) {
        self.timer.set_interval(interval_ms);
        self.timer.reset();
    }

    pub fn set_spawn_group_initializer_like_cpp(
        &mut self,
        initializer: impl Fn(&mut ManagedMap) + Send + Sync + 'static,
    ) {
        self.spawn_group_initializer_like_cpp = Some(Arc::new(initializer));
    }

    pub fn clear_spawn_group_initializer_like_cpp(&mut self) {
        self.spawn_group_initializer_like_cpp = None;
    }

    pub fn create_world_map(&mut self, map_id: u32, instance_id: u32) -> &mut ManagedMap {
        self.create_map_entry(map_id, instance_id, 0, ManagedMapKind::World)
    }

    pub fn create_map_entry(
        &mut self,
        map_id: u32,
        instance_id: u32,
        difficulty: Difficulty,
        kind: ManagedMapKind,
    ) -> &mut ManagedMap {
        let key = MapKey::new(map_id, instance_id);
        let grid_cleanup_delay_ms = self.grid_cleanup_delay_ms;
        let spawn_group_initializer_like_cpp = self.spawn_group_initializer_like_cpp.clone();
        match self.maps.entry(key) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let mut map = ManagedMap::new(
                    map_id,
                    instance_id,
                    difficulty,
                    i64::from(grid_cleanup_delay_ms),
                    kind,
                );
                if let Some(initializer) = spawn_group_initializer_like_cpp {
                    initializer(&mut map);
                }
                if instance_id != 0 && (kind.is_dungeon() || kind.is_battleground_or_arena()) {
                    self.instance_ids.register_instance_id(instance_id);
                }
                entry.insert(map)
            }
        }
    }

    pub fn create_map_decision_like_cpp(
        &mut self,
        entry: Option<CreateMapEntryContext>,
        player: Option<CreateMapPlayerContext>,
        map_difficulty: impl FnOnce(u32, Difficulty) -> Option<CreateMapDifficultyContext>,
        active_instance_lock: Option<CreateMapInstanceLockContext>,
        existing_instance_map: impl FnOnce(u32, u32) -> Option<ExistingInstanceMapContext>,
    ) -> CreateMapDecision {
        let Some(player) = player else {
            return CreateMapDecision::Reject {
                side_effects: Vec::new(),
            };
        };
        let Some(entry) = entry else {
            return CreateMapDecision::Reject {
                side_effects: Vec::new(),
            };
        };

        match entry.kind {
            CreateMapEntryKind::BattlegroundOrArena => {
                let instance_id = player.battleground_id;
                if instance_id == 0 {
                    return CreateMapDecision::Reject {
                        side_effects: Vec::new(),
                    };
                }

                let key = MapKey::new(entry.map_id, instance_id);
                if self.find_map(entry.map_id, instance_id).is_some() {
                    return CreateMapDecision::Existing {
                        key,
                        difficulty_id: 0,
                        side_effects: Vec::new(),
                    };
                }

                if !player.has_battleground {
                    return CreateMapDecision::Reject {
                        side_effects: vec![CreateMapSideEffect::TeleportToBattlegroundEntryPoint],
                    };
                }

                CreateMapDecision::Create {
                    key,
                    difficulty_id: 0,
                    kind: ManagedMapKind::Battleground,
                    side_effects: Vec::new(),
                }
            }
            CreateMapEntryKind::Dungeon => {
                let group = player.group;
                let mut difficulty = group
                    .map(|group| group.difficulty_id)
                    .unwrap_or(player.player_difficulty_id);
                let Some(difficulty_context) = map_difficulty(entry.map_id, difficulty) else {
                    return CreateMapDecision::Reject {
                        side_effects: Vec::new(),
                    };
                };

                let owner_guid_counter = group
                    .map(|group| group.recent_instance_owner_guid_counter)
                    .unwrap_or(player.guid_counter);
                let mut side_effects = Vec::new();
                let instance_lock = active_instance_lock;
                let mut instance_id = 0;

                if let Some(lock) = instance_lock {
                    instance_id = lock.instance_id;
                    if !entry.flex_locking {
                        difficulty = lock.difficulty_id;
                    }
                } else {
                    if !difficulty_context.has_reset_schedule {
                        instance_id = group
                            .map(|group| group.recent_instance_id)
                            .unwrap_or(player.player_recent_instance_id);
                    }

                    if instance_id == 0 {
                        let Some(generated) = self.generate_instance_id() else {
                            return CreateMapDecision::Reject {
                                side_effects: Vec::new(),
                            };
                        };
                        instance_id = generated;
                    }

                    if difficulty_context.has_reset_schedule {
                        side_effects.push(CreateMapSideEffect::CreateInstanceLockForNewInstance {
                            owner_guid_counter,
                            instance_id,
                        });
                    }
                }

                let existing = self.find_map(entry.map_id, instance_id).map(|map| {
                    ExistingInstanceMapContext {
                        instance_lock_token: map.instance_lock_token(),
                    }
                });
                let existing =
                    existing.or_else(|| existing_instance_map(entry.map_id, instance_id));

                if !difficulty_context.is_instance_id_bound
                    && let (Some(lock), Some(existing)) = (instance_lock, existing)
                    && existing.instance_lock_token != Some(lock.token)
                {
                    let Some(generated) = self.generate_instance_id() else {
                        return CreateMapDecision::Reject { side_effects };
                    };
                    instance_id = generated;
                    side_effects
                        .push(CreateMapSideEffect::SetInstanceLockInstanceId { instance_id });
                }

                let key = MapKey::new(entry.map_id, instance_id);
                if self.find_map(entry.map_id, instance_id).is_some() {
                    return CreateMapDecision::Existing {
                        key,
                        difficulty_id: difficulty,
                        side_effects,
                    };
                }

                if let Some(group) = group {
                    side_effects.push(CreateMapSideEffect::SetGroupRecentInstance {
                        owner_guid_counter: group.recent_instance_owner_guid_counter,
                        instance_id,
                    });
                } else {
                    side_effects.push(CreateMapSideEffect::SetPlayerRecentInstance { instance_id });
                }

                CreateMapDecision::Create {
                    key,
                    difficulty_id: difficulty,
                    kind: ManagedMapKind::Dungeon {
                        has_reset_schedule: difficulty_context.has_reset_schedule,
                    },
                    side_effects,
                }
            }
            CreateMapEntryKind::Garrison => CreateMapDecision::Create {
                key: MapKey::new(entry.map_id, player.guid_counter as u32),
                difficulty_id: 0,
                kind: ManagedMapKind::World,
                side_effects: Vec::new(),
            },
            CreateMapEntryKind::World => {
                let instance_id = if entry.split_by_faction {
                    player.team_id
                } else {
                    0
                };
                let key = MapKey::new(entry.map_id, instance_id);
                if self.find_map(entry.map_id, instance_id).is_some() {
                    CreateMapDecision::Existing {
                        key,
                        difficulty_id: 0,
                        side_effects: Vec::new(),
                    }
                } else {
                    CreateMapDecision::Create {
                        key,
                        difficulty_id: 0,
                        kind: ManagedMapKind::World,
                        side_effects: Vec::new(),
                    }
                }
            }
        }
    }

    pub fn find_instance_id_for_player_like_cpp(
        &self,
        entry: Option<CreateMapEntryContext>,
        player: Option<CreateMapPlayerContext>,
        map_difficulty: impl FnOnce(u32, Difficulty) -> Option<CreateMapDifficultyContext>,
        active_instance_lock: Option<CreateMapInstanceLockContext>,
        existing_instance_map: impl FnOnce(u32, u32) -> Option<ExistingInstanceMapContext>,
    ) -> u32 {
        let Some(player) = player else {
            return 0;
        };
        let Some(entry) = entry else {
            return 0;
        };

        match entry.kind {
            CreateMapEntryKind::BattlegroundOrArena => player.battleground_id,
            CreateMapEntryKind::Dungeon => {
                let group = player.group;
                let difficulty = group
                    .map(|group| group.difficulty_id)
                    .unwrap_or(player.player_difficulty_id);
                let Some(difficulty_context) = map_difficulty(entry.map_id, difficulty) else {
                    return 0;
                };

                let mut instance_id = 0;
                if let Some(lock) = active_instance_lock {
                    instance_id = lock.instance_id;
                } else if !difficulty_context.has_reset_schedule {
                    instance_id = group
                        .map(|group| group.recent_instance_id)
                        .unwrap_or(player.player_recent_instance_id);
                }

                if instance_id == 0 {
                    return 0;
                }

                let existing = self.find_map(entry.map_id, instance_id).map(|map| {
                    ExistingInstanceMapContext {
                        instance_lock_token: map.instance_lock_token(),
                    }
                });
                let existing =
                    existing.or_else(|| existing_instance_map(entry.map_id, instance_id));
                if !difficulty_context.is_instance_id_bound
                    && let (Some(lock), Some(existing)) = (active_instance_lock, existing)
                    && existing.instance_lock_token != Some(lock.token)
                {
                    return 0;
                }

                instance_id
            }
            CreateMapEntryKind::Garrison => player.guid_counter as u32,
            CreateMapEntryKind::World => {
                if entry.split_by_faction {
                    player.team_id
                } else {
                    0
                }
            }
        }
    }

    pub fn find_map(&self, map_id: u32, instance_id: u32) -> Option<&ManagedMap> {
        self.maps.get(&MapKey::new(map_id, instance_id))
    }

    pub fn find_map_mut(&mut self, map_id: u32, instance_id: u32) -> Option<&mut ManagedMap> {
        self.maps.get_mut(&MapKey::new(map_id, instance_id))
    }

    /// Execute one synchronous gameplay command inside the single writer for
    /// this exact map instance and return only owned evidence.
    ///
    /// External synchronization remains the caller's responsibility during
    /// the staged migration. No map/entity borrow crosses this boundary, so a
    /// later actor-backed driver can preserve the same command contract.
    pub fn execute_map_command_like_cpp(
        &mut self,
        map_id: u32,
        instance_id: u32,
        command: MapCommandLikeCpp,
    ) -> MapCommandOutcomeLikeCpp {
        let key = MapKey::new(map_id, instance_id);
        let Some(managed) = self.maps.get_mut(&key) else {
            return MapCommandOutcomeLikeCpp::missing_map(key, command);
        };
        managed.runtime.execute(command)
    }

    pub fn do_for_all_maps<F>(&self, mut worker: F)
    where
        F: FnMut(&ManagedMap),
    {
        for map in self.maps.values() {
            worker(map);
        }
    }

    pub fn do_for_all_maps_mut<F>(&mut self, mut worker: F)
    where
        F: FnMut(&mut ManagedMap),
    {
        for map in self.maps.values_mut() {
            worker(map);
        }
    }

    pub fn do_for_all_maps_with_map_id<F>(&self, map_id: u32, mut worker: F)
    where
        F: FnMut(&ManagedMap),
    {
        let start = MapKey::new(map_id, 0);
        let end = MapKey::new(map_id, u32::MAX);
        for (_, map) in self.maps.range(start..=end) {
            worker(map);
        }
    }

    pub fn update(&mut self, diff_ms: u32) -> Option<u32> {
        self.update_with_optional_pool_update_context::<fn(
            &mut Map,
            SpawnObjectType,
            SpawnId,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(diff_ms, None, None)
    }

    pub fn update_with_pool_update_context(
        &mut self,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
    ) -> Option<u32> {
        self.update_with_optional_pool_update_context(
            diff_ms,
            Some((spawn_store, pool_mgr)),
            None::<
                &mut fn(
                    &mut Map,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
        )
    }

    pub fn update_with_pool_update_loaded_grid_records_context<L>(
        &mut self,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        mut load_record: L,
    ) -> Option<u32>
    where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.update_with_optional_pool_update_context(
            diff_ms,
            Some((spawn_store, pool_mgr)),
            Some(&mut load_record),
        )
    }

    pub(super) fn update_with_optional_pool_update_context<L>(
        &mut self,
        diff_ms: u32,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        mut load_record: Option<&mut L>,
    ) -> Option<u32>
    where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.timer.update(diff_ms);
        if !self.timer.passed() {
            return None;
        }

        let current = self.timer.current();
        let keys: Vec<MapKey> = self.maps.keys().copied().collect();
        let mut destroyed = Vec::new();
        let mut updated = Vec::new();

        for key in keys {
            let Some(map) = self.maps.get_mut(&key) else {
                continue;
            };

            if map.can_unload(diff_ms) {
                if Self::destroy_map_inner(map, &mut self.instance_ids) {
                    destroyed.push(key);
                }
                continue;
            }

            updated.push(key);

            if self.updater.activated() {
                match pool_update {
                    Some((spawn_store, pool_mgr)) => {
                        if let Some(load_record) = load_record.as_mut() {
                            self.updater
                                .schedule_update_with_pool_update_loaded_grid_records_context(
                                    map,
                                    current,
                                    spawn_store,
                                    pool_mgr,
                                    &mut **load_record,
                                )
                        } else {
                            self.updater.schedule_update_with_pool_update_context(
                                map,
                                current,
                                spawn_store,
                                pool_mgr,
                            )
                        }
                    }
                    None => self.updater.schedule_update(map, current),
                }
            } else {
                match pool_update {
                    Some((spawn_store, pool_mgr)) => {
                        if let Some(load_record) = load_record.as_mut() {
                            map.update_with_pool_update_loaded_grid_records_context(
                                current,
                                spawn_store,
                                pool_mgr,
                                &mut **load_record,
                            );
                        } else {
                            map.update_with_pool_update_context(current, spawn_store, pool_mgr);
                        }
                    }
                    None => map.update(current),
                }
            }
        }

        if self.updater.activated() {
            self.updater.wait();
        }

        // Export only this tick's selected player notifiers, after every map
        // update completes and before delayed removal can change residence.
        self.retain_selected_player_visibility_refreshes_like_cpp(updated);

        for key in destroyed {
            self.maps.remove(&key);
        }

        for map in self.maps.values_mut() {
            map.delayed_update(current);
        }

        self.timer.set_current(0);
        Some(current)
    }

    pub fn num_instances(&self) -> u32 {
        self.maps
            .values()
            .filter(|map| map.kind().is_dungeon())
            .count() as u32
    }

    pub fn num_players_in_instances(&self) -> u32 {
        self.maps
            .values()
            .filter(|map| map.kind().is_dungeon())
            .map(ManagedMap::player_count)
            .sum()
    }

    pub fn init_instance_ids(&mut self, max_existing_instance_id: u64) {
        self.instance_ids
            .init_instance_ids(max_existing_instance_id);
    }

    pub fn register_instance_id(&mut self, instance_id: u32) {
        self.instance_ids.register_instance_id(instance_id);
    }

    pub fn generate_instance_id(&mut self) -> Option<u32> {
        self.instance_ids.generate_instance_id()
    }

    pub fn free_instance_id(&mut self, instance_id: u32) {
        self.instance_ids.free_instance_id(instance_id);
    }

    pub fn next_instance_id(&self) -> u32 {
        self.instance_ids.next_instance_id()
    }

    pub fn map_updater(&self) -> &MapUpdater {
        &self.updater
    }

    pub fn map_updater_mut(&mut self) -> &mut MapUpdater {
        &mut self.updater
    }

    pub fn increase_scheduled_scripts_count(&mut self) {
        self.scheduled_scripts += 1;
    }

    pub fn decrease_scheduled_script_count(&mut self) {
        self.scheduled_scripts = self.scheduled_scripts.saturating_sub(1);
    }

    pub fn decrease_scheduled_script_count_by(&mut self, count: usize) {
        self.scheduled_scripts = self.scheduled_scripts.saturating_sub(count);
    }

    pub const fn is_script_scheduled(&self) -> bool {
        self.scheduled_scripts > 0
    }
}

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

    pub fn wait(&mut self) {
        self.wait_calls += 1;
        debug_assert_eq!(self.pending_requests, 0);
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
