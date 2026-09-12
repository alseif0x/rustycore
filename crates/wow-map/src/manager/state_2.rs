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
            map_incarnations_like_cpp: BTreeMap::new(),
            next_map_incarnation_like_cpp: 1,
            tick_coordination_like_cpp: MapTickCoordinationStateLikeCpp::Idle,
            next_tick_epoch_like_cpp: 1,
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
                let incarnation = self.next_map_incarnation_like_cpp;
                self.next_map_incarnation_like_cpp =
                    self.next_map_incarnation_like_cpp.saturating_add(1);
                self.map_incarnations_like_cpp.insert(key, incarnation);
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
        load_record: Option<&mut L>,
    ) -> Option<u32>
    where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let plan = self.begin_tick_like_cpp(diff_ms).into_started()?;
        let effective = plan.effective_diff_ms();
        self.resume_tick_like_cpp(plan, pool_update, load_record)
            .expect_resumed_like_cpp();
        Some(effective)
    }

    /// C++ `MapManager::Update` up to the point where each map would update its
    /// world sessions (`Maps/MapManager.cpp:287-311`, `Maps/Map.cpp:666-680`):
    /// advance the shared timer, refuse the tick until it passes, decide
    /// `CanUnload` and destroy before updating, and run the dynamic-tree phase
    /// of every surviving map.
    ///
    /// Answers the plan the coordinator resumes with, or `None` when the timer
    /// has not passed. Destroyed maps are removed when the tick resumes, so a
    /// caller that drops the plan leaves the manager exactly as C++ leaves a
    /// map it decided to destroy but has not erased yet.
    /// [`Self::resume_tick_like_cpp`] with the pool update context and the
    /// loaded-grid respawn record hook, for the canonical world-server loop.
    pub fn resume_tick_with_pool_update_loaded_grid_records_context<L>(
        &mut self,
        plan: MapTickPlanLikeCpp,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        mut load_record: L,
    ) -> MapTickResumeLikeCpp
    where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.resume_tick_like_cpp(plan, Some((spawn_store, pool_mgr)), Some(&mut load_record))
    }

    /// The players whose sessions C++ `Map::Update` drives for one map, in
    /// `m_mapRefManager` order and filtered to those in world
    /// (`Maps/Map.cpp:669-680`: `player && player->IsInWorld()`).
    ///
    /// The order is the map's link order, not a sorted GUID list: see
    /// `Map::map_reference_order_like_cpp`.
    #[must_use]
    pub fn map_session_pass_participants_like_cpp(
        &self,
        key: MapKey,
    ) -> Vec<MapSessionPassParticipantLikeCpp> {
        let Some(map) = self.maps.get(&key) else {
            return Vec::new();
        };
        let mut participants = Vec::new();
        for guid in map.map().map_reference_order_like_cpp().iter().copied() {
            if !map
                .map()
                .get_typed_player(guid)
                .is_some_and(|player| player.unit().world().object().is_in_world())
            {
                continue;
            }
            // Freeze the identity this tick admitted, under the guard that is
            // about to be released: a replacement incarnation or a transfer
            // after the split must not satisfy this request.
            let Some((handle, residence_revision)) = self.current_player_admission_like_cpp(guid)
            else {
                continue;
            };
            participants.push(MapSessionPassParticipantLikeCpp {
                guid,
                handle,
                residence_revision,
            });
        }
        participants
    }

    pub fn begin_tick_like_cpp(&mut self, diff_ms: u32) -> MapTickBeginLikeCpp {
        // A tick already split is not allowed to consume this diff: advancing the
        // shared timer here would give the pending resumption a foreign current
        // value and silently start a second overlapping tick.
        match self.tick_coordination_like_cpp {
            MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
            | MapTickCoordinationStateLikeCpp::Resuming(epoch) => {
                return MapTickBeginLikeCpp::Busy { epoch };
            }
            MapTickCoordinationStateLikeCpp::Idle => {}
        }

        self.timer.update(diff_ms);
        if !self.timer.passed() {
            return MapTickBeginLikeCpp::TimerNotPassed;
        }

        let current = self.timer.current();
        let keys: Vec<MapKey> = self.maps.keys().copied().collect();
        let mut destroyed = Vec::new();
        let mut updated = Vec::new();

        for key in keys {
            let incarnation = self.map_incarnation_like_cpp(key).unwrap_or_default();
            let Some(map) = self.maps.get_mut(&key) else {
                continue;
            };

            if map.can_unload(diff_ms) {
                if Self::destroy_map_inner(map, &mut self.instance_ids) {
                    destroyed.push(MapTickParticipantLikeCpp { key, incarnation });
                }
                continue;
            }

            map.update_dynamic_tree_phase_like_cpp(current);
            updated.push(MapTickParticipantLikeCpp { key, incarnation });
        }

        let epoch = self.next_tick_epoch_like_cpp;
        self.next_tick_epoch_like_cpp = self.next_tick_epoch_like_cpp.saturating_add(1);
        self.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch);

        MapTickBeginLikeCpp::Started(MapTickPlanLikeCpp {
            epoch,
            effective_diff_ms: current,
            updated,
            destroyed,
        })
    }

    /// The incarnation of the map currently registered under `key`.
    ///
    /// A `MapKey` is reusable: destroying an instance and creating another one
    /// with the same id yields the same key but a different map. The tick plan
    /// records the incarnation it admitted so the resumption can tell them apart
    /// (`Maps/MapManager.cpp:296-318` operates on the same object it admitted).
    #[must_use]
    pub fn map_incarnation_like_cpp(&self, key: MapKey) -> Option<u64> {
        self.map_incarnations_like_cpp.get(&key).copied()
    }

    /// Where the canonical tick currently is; `Idle` between ticks.
    #[must_use]
    pub const fn tick_coordination_like_cpp(&self) -> MapTickCoordinationStateLikeCpp {
        self.tick_coordination_like_cpp
    }

    /// Give up an admitted tick without running its remaining phases.
    ///
    /// Used when the coordinator is torn down between the split and the
    /// resumption: the plan is consumed, the manager returns to `Idle` and the
    /// shared timer keeps the current it had, so the next tick re-decides
    /// admission from a clean state instead of inheriting a half-run one.
    pub fn abandon_tick_like_cpp(&mut self, plan: MapTickPlanLikeCpp) -> MapTickResumeLikeCpp {
        if self.tick_coordination_like_cpp
            != MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch)
        {
            return MapTickResumeLikeCpp::Rejected {
                state: self.tick_coordination_like_cpp,
                plan_epoch: plan.epoch,
            };
        }
        self.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Idle;
        MapTickResumeLikeCpp::Resumed
    }

    /// The rest of the same canonical tick: every phase C++ `Map::Update` runs
    /// after the map's world sessions, the updater barrier, the retained
    /// visibility export, the removal of the maps destroyed at admission and
    /// `DelayedUpdate` for every survivor (`Maps/MapManager.cpp:311-318`).
    ///
    /// Runs exactly once per plan and uses the diff saved at the split, never a
    /// freshly measured one.
    pub fn resume_tick_like_cpp<L>(
        &mut self,
        plan: MapTickPlanLikeCpp,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        mut load_record: Option<&mut L>,
    ) -> MapTickResumeLikeCpp
    where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        // The plan is consumable, so a second resumption cannot even be spelled;
        // the state still has to be checked because a plan can outlive the
        // manager state it was admitted under.
        if self.tick_coordination_like_cpp
            != MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch)
        {
            return MapTickResumeLikeCpp::Rejected {
                state: self.tick_coordination_like_cpp,
                plan_epoch: plan.epoch,
            };
        }
        self.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(plan.epoch);

        let current = plan.effective_diff_ms;
        let mut resumed_keys = Vec::with_capacity(plan.updated.len());

        for participant in &plan.updated {
            let key = participant.key;
            // A map created under a reused key during the session pass is not the
            // map this tick admitted: it has run no dynamic-tree phase and must
            // not receive the phases that follow one.
            if self.map_incarnation_like_cpp(key) != Some(participant.incarnation) {
                continue;
            }
            let Some(map) = self.maps.get_mut(&key) else {
                continue;
            };
            resumed_keys.push(key);

            if self.updater.activated() {
                match pool_update {
                    Some((spawn_store, pool_mgr)) => {
                        if let Some(load_record) = load_record.as_mut() {
                            self.updater
                                .schedule_after_sessions_with_pool_update_loaded_grid_records_context(
                                    map,
                                    current,
                                    spawn_store,
                                    pool_mgr,
                                    &mut **load_record,
                                )
                        } else {
                            self.updater
                                .schedule_after_sessions_with_pool_update_context(
                                    map,
                                    current,
                                    spawn_store,
                                    pool_mgr,
                                )
                        }
                    }
                    None => self.updater.schedule_after_sessions(map, current),
                }
            } else {
                match pool_update {
                    Some((spawn_store, pool_mgr)) => {
                        if let Some(load_record) = load_record.as_mut() {
                            map.update_after_sessions_like_cpp(
                                current,
                                Some((spawn_store, pool_mgr)),
                                Some(&mut **load_record),
                            );
                        } else {
                            map.update_after_sessions_like_cpp(
                                current,
                                Some((spawn_store, pool_mgr)),
                                None::<&mut L>,
                            );
                        }
                    }
                    None => map.update_after_sessions_like_cpp(current, None, None::<&mut L>),
                }
            }
        }

        if self.updater.activated() {
            self.updater.wait();
        }

        // Export only this tick's selected player notifiers, after every map
        // update completes and before delayed removal can change residence.
        self.retain_selected_player_visibility_refreshes_like_cpp(resumed_keys);

        for participant in &plan.destroyed {
            if self.map_incarnation_like_cpp(participant.key) != Some(participant.incarnation) {
                continue;
            }
            self.maps.remove(&participant.key);
            self.map_incarnations_like_cpp.remove(&participant.key);
        }

        for map in self.maps.values_mut() {
            map.delayed_update(current);
        }

        self.timer.set_current(0);
        self.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Idle;
        MapTickResumeLikeCpp::Resumed
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

/// The part of one canonical map tick that survives the guard release.
///
/// #787 splits `MapManager::Update` after admission/unload and the dynamic-tree
/// phase so the coordinator can release every synchronous guard, let each
/// session run its admitted map pass, and resume the same tick. The effective
/// diff is captured at the split, as C++ passes one `i_timer.GetCurrent()` to
/// every map of that tick (`Maps/MapManager.cpp:307-311`).
///
/// The plan is deliberately neither `Clone` nor `Copy` and is consumed by
/// [`MapManager::resume_tick_like_cpp`]: the remaining phases of one tick run
/// exactly once because a second call has nothing to pass.
#[derive(Debug)]
pub struct MapTickPlanLikeCpp {
    epoch: u64,
    effective_diff_ms: u32,
    updated: Vec<MapTickParticipantLikeCpp>,
    destroyed: Vec<MapTickParticipantLikeCpp>,
}

/// One session a map's tick drives, with the identity frozen at admission.
///
/// C++ runs these passes while holding the map that owns the player
/// (`Maps/Map.cpp:669-680`). RustyCore releases that guard, so the request must
/// carry enough identity for the session to prove it is still the same player,
/// the same incarnation and the same residence before running any effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapSessionPassParticipantLikeCpp {
    pub guid: ObjectGuid,
    pub handle: PlayerHandle,
    pub residence_revision: u64,
}

/// One map admitted by a tick, together with the incarnation that was admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapTickParticipantLikeCpp {
    pub key: MapKey,
    pub incarnation: u64,
}

/// What [`MapManager::begin_tick_like_cpp`] decided for this diff.
#[derive(Debug)]
pub enum MapTickBeginLikeCpp {
    /// The tick was admitted; its remaining phases are owed exactly one
    /// [`MapManager::resume_tick_like_cpp`] or [`MapManager::abandon_tick_like_cpp`].
    Started(MapTickPlanLikeCpp),
    /// The shared timer has not passed; nothing was updated
    /// (`Maps/MapManager.cpp:289-295`).
    TimerNotPassed,
    /// A tick admitted earlier has not been resumed. The shared timer was not
    /// advanced, so this diff is still owed to the next accepted tick.
    Busy { epoch: u64 },
}

impl MapTickBeginLikeCpp {
    /// The admitted plan, or `None` for both refusals.
    #[must_use]
    pub fn into_started(self) -> Option<MapTickPlanLikeCpp> {
        match self {
            Self::Started(plan) => Some(plan),
            Self::TimerNotPassed | Self::Busy { .. } => None,
        }
    }

    #[must_use]
    pub const fn is_busy(&self) -> bool {
        matches!(self, Self::Busy { .. })
    }

    #[must_use]
    pub const fn is_timer_not_passed(&self) -> bool {
        matches!(self, Self::TimerNotPassed)
    }
}

/// Whether a resumption matched the tick the manager is actually holding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapTickResumeLikeCpp {
    Resumed,
    /// The manager is not awaiting this plan's epoch: the remaining phases were
    /// not run and nothing was mutated.
    Rejected {
        state: MapTickCoordinationStateLikeCpp,
        plan_epoch: u64,
    },
}

impl MapTickResumeLikeCpp {
    /// Panics when the resumption did not match, for the one-shot composition
    /// where `begin` and `resume` are adjacent and a mismatch is a bug.
    pub fn expect_resumed_like_cpp(self) {
        assert_eq!(
            self,
            Self::Resumed,
            "the adjacent resumption of a just-admitted tick must match"
        );
    }
}

impl MapTickPlanLikeCpp {
    /// The tick this plan belongs to; the manager refuses any other.
    #[must_use]
    pub const fn epoch_like_cpp(&self) -> u64 {
        self.epoch
    }

    /// The diff every map of this tick receives, saved at the split.
    #[must_use]
    pub fn effective_diff_ms(&self) -> u32 {
        self.effective_diff_ms
    }

    /// The maps that survived admission and will run their remaining phases,
    /// in the manager's iteration order.
    #[must_use]
    pub fn updated_maps_like_cpp(&self) -> &[MapTickParticipantLikeCpp] {
        &self.updated
    }

    /// The maps C++ destroys before updating (`MapManager.cpp:296-305`); they
    /// are removed when the tick resumes.
    #[must_use]
    pub fn destroyed_maps_like_cpp(&self) -> &[MapTickParticipantLikeCpp] {
        &self.destroyed
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

    /// The post-session half of one scheduled map update (#787). The updater
    /// still runs inline; the split only changes which phases this call covers.
    pub fn schedule_after_sessions(&mut self, map: &mut ManagedMap, diff_ms: u32) {
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        map.update_after_sessions_like_cpp(
            diff_ms,
            None,
            None::<
                &mut fn(
                    &mut Map,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
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
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        map.update_after_sessions_like_cpp(
            diff_ms,
            Some((spawn_store, pool_mgr)),
            None::<
                &mut fn(
                    &mut Map,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
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
        self.pending_requests += 1;
        self.scheduled_updates += 1;
        map.update_after_sessions_like_cpp(
            diff_ms,
            Some((spawn_store, pool_mgr)),
            Some(load_record),
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
