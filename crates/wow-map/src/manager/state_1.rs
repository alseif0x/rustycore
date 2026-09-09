//! Managed-map lifecycle and updater state definitions, part 1 of 2.
//!
//! Separated from the manager.rs root under #644. Behaviour is preserved.

use super::*;

pub const MIN_GRID_DELAY_MS: u32 = 60_000;

pub const MIN_MAP_UPDATE_DELAY_MS: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedMapKind {
    World,
    Dungeon { has_reset_schedule: bool },
    Battleground,
}

impl ManagedMapKind {
    pub const fn is_dungeon(self) -> bool {
        matches!(self, Self::Dungeon { .. })
    }

    pub const fn is_battleground_or_arena(self) -> bool {
        matches!(self, Self::Battleground)
    }

    pub const fn frees_instance_id_on_destroy(self) -> bool {
        match self {
            Self::Battleground => true,
            Self::Dungeon { has_reset_schedule } => !has_reset_schedule,
            Self::World => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateMapEntryKind {
    World,
    Dungeon,
    BattlegroundOrArena,
    Garrison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateMapEntryContext {
    pub map_id: u32,
    pub kind: CreateMapEntryKind,
    pub split_by_faction: bool,
    pub flex_locking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateMapDifficultyContext {
    pub difficulty_id: Difficulty,
    pub has_reset_schedule: bool,
    pub is_instance_id_bound: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateMapInstanceLockContext {
    pub instance_id: u32,
    pub difficulty_id: Difficulty,
    pub token: u64,
    pub owner_guid_counter: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateMapPlayerContext {
    pub guid_counter: u64,
    pub team_id: u32,
    pub battleground_id: u32,
    pub has_battleground: bool,
    pub player_difficulty_id: Difficulty,
    pub player_recent_instance_id: u32,
    pub group: Option<CreateMapGroupContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateMapGroupContext {
    pub difficulty_id: Difficulty,
    pub recent_instance_owner_guid_counter: u64,
    pub recent_instance_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateMapSideEffect {
    TeleportToBattlegroundEntryPoint,
    CreateInstanceLockForNewInstance {
        owner_guid_counter: u64,
        instance_id: u32,
    },
    SetInstanceLockInstanceId {
        instance_id: u32,
    },
    SetGroupRecentInstance {
        owner_guid_counter: u64,
        instance_id: u32,
    },
    SetPlayerRecentInstance {
        instance_id: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateMapDecision {
    Existing {
        key: MapKey,
        difficulty_id: Difficulty,
        side_effects: Vec<CreateMapSideEffect>,
    },
    Create {
        key: MapKey,
        difficulty_id: Difficulty,
        kind: ManagedMapKind,
        side_effects: Vec<CreateMapSideEffect>,
    },
    Reject {
        side_effects: Vec<CreateMapSideEffect>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExistingInstanceMapContext {
    pub instance_lock_token: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveMoveListDrainSummaryLikeCpp {
    pub creature: MoveListDrainSummaryLikeCpp,
    pub game_object: MoveListDrainSummaryLikeCpp,
    pub area_trigger: MoveListDrainSummaryLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapUpdateScriptHookSummaryLikeCpp {
    pub invoked: bool,
    pub diff_ms: u32,
    pub map_id: u32,
    pub instance_id: u32,
    pub kind: ManagedMapKind,
    pub script_dispatch_represented: bool,
}

impl Default for MapUpdateScriptHookSummaryLikeCpp {
    fn default() -> Self {
        Self {
            invoked: false,
            diff_ms: 0,
            map_id: 0,
            instance_id: 0,
            kind: ManagedMapKind::World,
            script_dispatch_represented: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapUpdateTailSummaryLikeCpp {
    pub script_hook: MapUpdateScriptHookSummaryLikeCpp,
    pub metrics: MapUpdateMetricsSummaryLikeCpp,
}

/// The concrete `Map` a canonical `ManagedMap` owns.
///
/// Named so code outside `wow-map` can take `&mut` to it without spelling the
/// loader/lifecycle parameters. #28 needs it because helpers that used to take
/// the canonical lock themselves now take the map, so the caller acquires the
/// lock once instead of once per helper.
pub type ManagedMapInnerLikeCpp = Map<NoopTerrainGridLoader, NoopGridLifecycle>;

#[derive(Debug)]
pub struct ManagedMap {
    pub(super) runtime: MapRuntime,
    pub(super) kind: ManagedMapKind,
    pub(super) can_unload: bool,
    pub(super) instance_encounter_in_progress: bool,
    pub(super) instance_lock_token: Option<u64>,
    pub(super) instance_lock_context: Option<CreateMapInstanceLockContext>,
    pub(super) update_calls: Vec<u32>,
    pub(super) delayed_update_calls: Vec<u32>,
    pub(super) last_dynamic_tree_update_summary_like_cpp: DynamicMapTreeUpdateSummaryLikeCpp,
    pub(super) last_dynamic_objects_update_summary: DynamicObjectsUpdateSummaryLikeCpp,
    pub(super) last_creatures_update_summary: CreatureUpdateSummaryLikeCpp,
    pub(super) last_expired_pvp_combat_refs_like_cpp: Vec<(ObjectGuid, ObjectGuid)>,
    pub(super) last_game_objects_update_summary: GameObjectsUpdateSummaryLikeCpp,
    pub(super) last_transports_update_summary: TransportsUpdateSummaryLikeCpp,
    pub(super) last_area_triggers_update_summary: AreaTriggersUpdateSummaryLikeCpp,
    pub(super) last_conversations_update_summary: ConversationsUpdateSummaryLikeCpp,
    pub(super) last_scene_objects_update_summary: SceneObjectsUpdateSummaryLikeCpp,
    pub(super) last_send_object_updates_summary_like_cpp: SendObjectUpdatesSummaryLikeCpp,
    pub(super) last_script_schedule_process_summary_like_cpp: ScriptScheduleProcessSummaryLikeCpp,
    pub(super) last_weather_update_summary_like_cpp: WeatherUpdateSummaryLikeCpp,
    pub(super) last_personal_phase_tracker_update_summary: PersonalPhaseTrackerUpdateSummaryLikeCpp,
    pub(super) last_live_move_list_drain_summary: LiveMoveListDrainSummaryLikeCpp,
    pub(super) last_far_spell_callback_drain_summary_like_cpp: FarSpellCallbackDrainSummaryLikeCpp,
    pub(super) last_grid_states_update_summary_like_cpp: GridStatesUpdateSummaryLikeCpp,
    pub(super) last_process_relocation_notifies_outcome_like_cpp: ProcessRelocationNotifiesOutcome,
    pub(super) last_map_update_tail_summary_like_cpp: MapUpdateTailSummaryLikeCpp,
    pub(super) unload_all_calls: u32,
}

pub(super) fn game_time_now_secs_i64() -> i64 {
    let now_secs = GameTime::now().as_secs();
    now_secs.min(i64::MAX as u64) as i64
}

pub(super) fn game_time_now_ms_u64() -> u64 {
    static PROCESS_START: OnceLock<Instant> = OnceLock::new();
    game_time_elapsed_ms_u64(PROCESS_START.get_or_init(Instant::now).elapsed())
}

pub(super) fn game_time_elapsed_ms_u64(elapsed: Duration) -> u64 {
    let elapsed_ms = elapsed.as_millis();
    elapsed_ms.min(u128::from(u64::MAX)) as u64
}

impl ManagedMap {
    pub fn new(
        map_id: u32,
        instance_id: u32,
        difficulty: Difficulty,
        grid_expiry_ms: i64,
        kind: ManagedMapKind,
    ) -> Self {
        Self {
            runtime: MapRuntime::new(Map::new(map_id, instance_id, difficulty, grid_expiry_ms)),
            kind,
            can_unload: false,
            instance_encounter_in_progress: false,
            instance_lock_token: None,
            instance_lock_context: None,
            update_calls: Vec::new(),
            delayed_update_calls: Vec::new(),
            last_dynamic_tree_update_summary_like_cpp: DynamicMapTreeUpdateSummaryLikeCpp::default(
            ),
            last_dynamic_objects_update_summary: DynamicObjectsUpdateSummaryLikeCpp::default(),
            last_creatures_update_summary: CreatureUpdateSummaryLikeCpp::default(),
            last_expired_pvp_combat_refs_like_cpp: Vec::new(),
            last_game_objects_update_summary: GameObjectsUpdateSummaryLikeCpp::default(),
            last_transports_update_summary: TransportsUpdateSummaryLikeCpp::default(),
            last_area_triggers_update_summary: AreaTriggersUpdateSummaryLikeCpp::default(),
            last_conversations_update_summary: ConversationsUpdateSummaryLikeCpp::default(),
            last_scene_objects_update_summary: SceneObjectsUpdateSummaryLikeCpp::default(),
            last_send_object_updates_summary_like_cpp: SendObjectUpdatesSummaryLikeCpp::default(),
            last_script_schedule_process_summary_like_cpp:
                ScriptScheduleProcessSummaryLikeCpp::default(),
            last_weather_update_summary_like_cpp: WeatherUpdateSummaryLikeCpp::default(),
            last_personal_phase_tracker_update_summary:
                PersonalPhaseTrackerUpdateSummaryLikeCpp::default(),
            last_live_move_list_drain_summary: LiveMoveListDrainSummaryLikeCpp::default(),
            last_far_spell_callback_drain_summary_like_cpp:
                FarSpellCallbackDrainSummaryLikeCpp::default(),
            last_grid_states_update_summary_like_cpp: GridStatesUpdateSummaryLikeCpp::default(),
            last_process_relocation_notifies_outcome_like_cpp:
                ProcessRelocationNotifiesOutcome::default(),
            last_map_update_tail_summary_like_cpp: MapUpdateTailSummaryLikeCpp::default(),
            unload_all_calls: 0,
        }
    }

    pub const fn map_id(&self) -> u32 {
        self.runtime.map.map_id()
    }

    pub const fn instance_id(&self) -> u32 {
        self.runtime.map.instance_id()
    }

    pub const fn difficulty(&self) -> Difficulty {
        self.runtime.map.spawn_mode()
    }

    pub const fn kind(&self) -> ManagedMapKind {
        self.kind
    }

    pub fn map(&self) -> &Map<NoopTerrainGridLoader, NoopGridLifecycle> {
        &self.runtime.map
    }

    pub fn map_mut(&mut self) -> &mut Map<NoopTerrainGridLoader, NoopGridLifecycle> {
        &mut self.runtime.map
    }

    pub fn set_can_unload(&mut self, can_unload: bool) {
        self.can_unload = can_unload;
    }

    pub fn player_count(&self) -> u32 {
        self.runtime.map.typed_player_counts_like_cpp().0
    }

    pub fn players_count_except_gms_like_cpp(&self) -> u32 {
        self.runtime.map.typed_player_counts_like_cpp().1
    }

    pub fn set_instance_encounter_in_progress_like_cpp(&mut self, in_progress: bool) {
        self.instance_encounter_in_progress = in_progress;
    }

    pub const fn instance_encounter_in_progress_like_cpp(&self) -> bool {
        self.instance_encounter_in_progress
    }

    pub const fn instance_lock_token(&self) -> Option<u64> {
        self.instance_lock_token
    }

    pub const fn instance_lock_context(&self) -> Option<CreateMapInstanceLockContext> {
        self.instance_lock_context
    }

    pub fn set_instance_lock_token(&mut self, token: Option<u64>) {
        self.instance_lock_token = token;
        if token.is_none() {
            self.instance_lock_context = None;
        }
    }

    pub fn set_instance_lock_context(&mut self, context: Option<CreateMapInstanceLockContext>) {
        self.instance_lock_token = context.map(|context| context.token);
        self.instance_lock_context = context;
    }

    pub fn update_calls(&self) -> &[u32] {
        &self.update_calls
    }

    pub fn delayed_update_calls(&self) -> &[u32] {
        &self.delayed_update_calls
    }

    pub const fn last_dynamic_tree_update_summary_like_cpp(
        &self,
    ) -> DynamicMapTreeUpdateSummaryLikeCpp {
        self.last_dynamic_tree_update_summary_like_cpp
    }

    pub const fn last_dynamic_objects_update_summary(&self) -> DynamicObjectsUpdateSummaryLikeCpp {
        self.last_dynamic_objects_update_summary
    }

    pub const fn last_creatures_update_summary(&self) -> CreatureUpdateSummaryLikeCpp {
        self.last_creatures_update_summary
    }

    pub fn last_expired_pvp_combat_refs_like_cpp(&self) -> &[(ObjectGuid, ObjectGuid)] {
        &self.last_expired_pvp_combat_refs_like_cpp
    }

    pub fn last_game_objects_update_summary(&self) -> GameObjectsUpdateSummaryLikeCpp {
        self.last_game_objects_update_summary.clone()
    }

    pub const fn last_transports_update_summary(&self) -> TransportsUpdateSummaryLikeCpp {
        self.last_transports_update_summary
    }

    pub const fn last_area_triggers_update_summary(&self) -> AreaTriggersUpdateSummaryLikeCpp {
        self.last_area_triggers_update_summary
    }

    pub const fn last_conversations_update_summary(&self) -> ConversationsUpdateSummaryLikeCpp {
        self.last_conversations_update_summary
    }

    pub const fn last_scene_objects_update_summary(&self) -> SceneObjectsUpdateSummaryLikeCpp {
        self.last_scene_objects_update_summary
    }

    pub const fn last_personal_phase_tracker_update_summary(
        &self,
    ) -> PersonalPhaseTrackerUpdateSummaryLikeCpp {
        self.last_personal_phase_tracker_update_summary
    }

    pub fn last_send_object_updates_summary_like_cpp(&self) -> SendObjectUpdatesSummaryLikeCpp {
        self.last_send_object_updates_summary_like_cpp.clone()
    }

    pub fn last_script_schedule_process_summary_like_cpp(
        &self,
    ) -> ScriptScheduleProcessSummaryLikeCpp {
        self.last_script_schedule_process_summary_like_cpp.clone()
    }

    pub const fn last_weather_update_summary_like_cpp(&self) -> WeatherUpdateSummaryLikeCpp {
        self.last_weather_update_summary_like_cpp
    }

    pub fn last_live_move_list_drain_summary_like_cpp(&self) -> LiveMoveListDrainSummaryLikeCpp {
        self.last_live_move_list_drain_summary.clone()
    }

    pub const fn last_far_spell_callback_drain_summary_like_cpp(
        &self,
    ) -> FarSpellCallbackDrainSummaryLikeCpp {
        self.last_far_spell_callback_drain_summary_like_cpp
    }

    pub const fn last_grid_states_update_summary_like_cpp(&self) -> GridStatesUpdateSummaryLikeCpp {
        self.last_grid_states_update_summary_like_cpp
    }

    pub fn last_process_relocation_notifies_outcome_like_cpp(
        &self,
    ) -> ProcessRelocationNotifiesOutcome {
        self.last_process_relocation_notifies_outcome_like_cpp
            .clone()
    }

    pub const fn last_map_update_tail_summary_like_cpp(&self) -> MapUpdateTailSummaryLikeCpp {
        self.last_map_update_tail_summary_like_cpp
    }

    pub const fn unload_all_calls(&self) -> u32 {
        self.unload_all_calls
    }

    pub(super) fn can_unload(&self, _diff_ms: u32) -> bool {
        self.can_unload
    }

    pub(super) fn have_players(&self) -> bool {
        self.player_count() > 0
    }

    pub(super) fn update(&mut self, diff_ms: u32) {
        self.update_with_optional_pool_update_context::<fn(
            &mut Map,
            SpawnObjectType,
            SpawnId,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(diff_ms, None, None);
    }

    pub(super) fn update_with_pool_update_context(
        &mut self,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
    ) {
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
        );
    }

    pub(super) fn update_with_pool_update_loaded_grid_records_context<L>(
        &mut self,
        diff_ms: u32,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        load_record: &mut L,
    ) where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.update_with_optional_pool_update_context(
            diff_ms,
            Some((spawn_store, pool_mgr)),
            Some(load_record),
        );
    }

    pub(super) fn update_with_optional_pool_update_context<L>(
        &mut self,
        diff_ms: u32,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        // C++ `Map::Update` starts with `_dynamicTree.update(t_diff)` before
        // world sessions, respawns, ObjectUpdater families, SendObjectUpdates,
        // scripts, weather, personal phase, move lists, relocation notifies, and
        // tail ScriptMgr/metrics (`Map.cpp:666-668`). Rust exposes only the
        // represented map-owned timer/unbalanced seam here; `update_calls` below
        // remains manager instrumentation, not a C++ phase.
        self.last_dynamic_tree_update_summary_like_cpp =
            self.runtime.map.update_dynamic_tree_like_cpp(diff_ms);
        self.update_calls.push(diff_ms);
        self.last_dynamic_objects_update_summary =
            self.runtime.map.update_dynamic_objects_like_cpp(diff_ms);
        let now_secs = game_time_now_secs_i64();
        // Partial C++ ObjectUpdater seam: after DynamicObject, visit only the
        // represented map-owned Creature family in this slice. Default context is
        // honest represented runtime only: no real AI/combat/threat/fanout.
        self.last_creatures_update_summary =
            self.runtime
                .map
                .update_creatures_like_cpp(diff_ms, now_secs, |_guid, _creature| {
                    CreatureRuntimeUpdateContext::default()
                });
        // C++ Unit::Update advances timed PvP combat references for both
        // players and creatures. The canonical map owns both sides here, so
        // expire them once per map tick and purge the reciprocal relation.
        self.last_expired_pvp_combat_refs_like_cpp = self
            .runtime
            .map
            .update_all_pvp_combat_refs_like_cpp(diff_ms);
        // Partial C++ ObjectUpdater seam: after Creature, visit represented
        // map-owned GameObject records. C++ real order is TypeContainerVisitor
        // nearby-cell/active-object traversal; this Rust insertion only adds the
        // missing family and leaves AI/go-type/per-player/packet/DB gaps open.
        self.last_game_objects_update_summary = match (pool_update, load_record) {
            (Some((spawn_store, pool_mgr)), Some(load_record)) => self
                .runtime
                .map
                .update_game_objects_with_pool_update_loaded_grid_records_like_cpp(
                    diff_ms,
                    now_secs,
                    spawn_store,
                    pool_mgr,
                    load_record,
                ),
            (Some((spawn_store, pool_mgr)), None) => self
                .runtime
                .map
                .update_game_objects_with_pool_update_like_cpp(
                    diff_ms,
                    now_secs,
                    spawn_store,
                    pool_mgr,
                ),
            (None, _) => self
                .runtime
                .map
                .update_game_objects_like_cpp(diff_ms, now_secs),
        };
        // Partial C++ transport seam: after the represented GameObject/ObjectUpdater
        // family and before later represented families, visit typed canonical
        // Transports. This does not reproduce exact C++ cell visitor ordering nor
        // full `_transports` runtime (AI/scripts/spline/teleport/fanout/passengers).
        let now_ms = game_time_now_ms_u64();
        self.last_transports_update_summary =
            self.runtime.map.update_transports_like_cpp(diff_ms, now_ms);
        // Partial C++ ObjectUpdater seam: after Transport, visit only the
        // represented map-owned AreaTrigger family in this slice. Other families,
        // nearby-cell traversal, player/session updates, fanout and scripts stay
        // explicit remaining gaps.
        self.last_area_triggers_update_summary =
            self.runtime.map.update_area_triggers_like_cpp(diff_ms);
        // Partial C++ ObjectUpdater seam: visit represented map-owned
        // Conversation records after AreaTrigger for this Rust slice. Exact
        // TypeContainerVisitor ordering/cell traversal, real scripts,
        // SendObjectUpdates and fanout remain explicit gaps.
        self.last_conversations_update_summary =
            self.runtime.map.update_conversations_like_cpp(diff_ms);
        // Partial C++ ObjectUpdater seam: visit represented map-owned
        // SceneObject records after Conversation for this Rust slice. Real
        // ObjectAccessor::GetUnit and Aura lookup by spell/cast id are not present
        // yet, so the live manager default is conservative and does not remove
        // SceneObjects merely because that runtime is absent.
        self.last_scene_objects_update_summary =
            self.runtime
                .map
                .update_scene_objects_like_cpp(diff_ms, |_guid, scene_object| {
                    SceneObjectUpdateContextLikeCpp::represented_default_for(scene_object)
                });
        // C++ calls `Map::SendObjectUpdates()` after ObjectUpdater/Transport/
        // SceneObject-style visitation and before scripts/weather/personal phase
        // (`Map.cpp:777-798`). Rust consumes only represented map-owned
        // `m_objectUpdated`/changed-mask state here; no `UpdateDataMapType`,
        // session packets, visible-player iteration, or direct fanout is built.
        self.last_send_object_updates_summary_like_cpp =
            self.runtime.map.send_object_updates_like_cpp();
        // C++ then drains `m_scriptSchedule` under `i_scriptLock` before weather
        // and personal phase (`Map.cpp:777-798`, `MapScripts.cpp:311-321`).
        // Rust records due represented actions only; no script commands,
        // ObjectAccessor/session/fanout/DB/weather side effects are executed.
        self.last_script_schedule_process_summary_like_cpp = self
            .runtime
            .map
            .process_script_schedule_update_order_like_cpp(now_secs);
        // C++ updates `_weatherUpdateTimer` immediately after script schedule and
        // before `GetMultiPersonalPhaseTracker().Update(this, t_diff)`
        // (`Map.cpp:777-798`). Rust represents only the map-owned timer and
        // `_zoneDynamicInfo.DefaultWeather` update/reset seam; WeatherMgr, RNG,
        // script hooks, player fanout, packets, DB and zone messages remain gaps.
        self.last_weather_update_summary_like_cpp =
            self.runtime.map.update_weather_like_cpp(diff_ms);
        // C++ calls `GetMultiPersonalPhaseTracker().Update(this, t_diff)` after
        // SendObjectUpdates/scripts/weather and before later move/remove drains.
        // Rust consumes the existing map-owned tracker here as a represented seam
        // only: GUID expiry -> AddObjectToRemoveList, without claiming exact full
        // update ordering, visibility fanout, DB, scripts, or dynamic-tree parity.
        self.last_personal_phase_tracker_update_summary = self
            .runtime
            .map
            .update_personal_phase_tracker_like_cpp(diff_ms);
        // C++ `Map::Update` immediately drains Creature, GameObject, and
        // AreaTrigger move-lists after personal-phase tracker update and before
        // `ProcessRelocationNotifies(t_diff)` (`Map.cpp:797-805`). Rust keeps
        // `Map` as the sole owner of canonical object and queue state here; the
        // live manager only orchestrates order and stores summaries. DynamicObject
        // is intentionally not drained by this live path because C++ `Map::Update`
        // does not call a DynamicObject move-list drain.
        self.last_live_move_list_drain_summary = LiveMoveListDrainSummaryLikeCpp {
            creature: self.runtime.map.move_all_creatures_in_move_list_like_cpp(),
            game_object: self
                .runtime
                .map
                .move_all_game_objects_in_move_list_like_cpp(),
            area_trigger: self
                .runtime
                .map
                .move_all_area_triggers_in_move_list_like_cpp(),
        };
        // C++ `Map::Update` calls `ProcessRelocationNotifies(t_diff)`
        // immediately after the live Creature/GameObject/AreaTrigger move-list
        // drains and only when player/active-non-player sources exist
        // (`Map.cpp:797-805`). Rust consumes only the existing map-owned
        // represented helper here: marked-cell selection, relocation timer
        // selection/reset, delayed relocation plan selection, and notify flag
        // reset over canonical map state. It does not claim real notifier side
        // effects, packets, ObjectAccessor/session fanout, AI, dynamic tree, or
        // exact full visitor parity.
        self.last_process_relocation_notifies_outcome_like_cpp = self
            .runtime
            .map
            .process_live_relocation_notifies_like_cpp(diff_ms, DEFAULT_VISIBILITY_NOTIFY_PERIOD);
        // C++ `Map::Update` tail immediately follows ProcessRelocationNotifies:
        // `sScriptMgr->OnMapUpdate(this, t_diff)` then the `map_creatures` and
        // `map_gameobjects` metrics (`Map.cpp:804-815`). Rust records only the
        // boundary invocation and typed canonical counts from `Map::entity_world`;
        // no real ScriptMgr dispatch, script callbacks, Prometheus/telemetry,
        // ObjectAccessor, DB, or fanout side effects are claimed.
        self.last_map_update_tail_summary_like_cpp = MapUpdateTailSummaryLikeCpp {
            script_hook: MapUpdateScriptHookSummaryLikeCpp {
                invoked: true,
                diff_ms,
                map_id: self.runtime.map.map_id(),
                instance_id: self.runtime.map.instance_id(),
                kind: self.kind,
                script_dispatch_represented: false,
            },
            metrics: self.runtime.map.map_update_metrics_like_cpp(),
        };
    }

    pub(super) fn delayed_update(&mut self, diff_ms: u32) {
        self.delayed_update_calls.push(diff_ms);
        // C++ `Map::DelayedUpdate` drains `_farSpellCallbacks` before
        // `RemoveAllObjectsInRemoveList()`, then updates grid states unless BG/arena
        // (`Map.cpp:2519-2544`). Rust keeps callback ownership inside `Map`; the
        // manager only orchestrates the live order and records the drain summary.
        self.last_far_spell_callback_drain_summary_like_cpp =
            self.runtime.map.drain_far_spell_callbacks_like_cpp();
        self.runtime
            .map
            .remove_all_objects_in_remove_list_like_cpp();
        self.last_grid_states_update_summary_like_cpp = if self.kind.is_battleground_or_arena() {
            GridStatesUpdateSummaryLikeCpp {
                diff_ms,
                skipped_battleground_or_arena: true,
                ..GridStatesUpdateSummaryLikeCpp::default()
            }
        } else {
            self.runtime.map.update_loaded_grid_states_like_cpp(diff_ms)
        };
    }

    pub(super) fn unload_all(&mut self) {
        self.unload_all_calls += 1;
    }
}

pub type SpawnGroupInitializerLikeCpp = Arc<dyn Fn(&mut ManagedMap) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceIdAllocator {
    pub(super) free_instance_ids: Vec<bool>,
    pub(super) next_instance_id: u32,
}

impl Default for InstanceIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceIdAllocator {
    pub fn new() -> Self {
        Self {
            free_instance_ids: vec![false, true],
            next_instance_id: 1,
        }
    }

    pub fn init_instance_ids(&mut self, max_existing_instance_id: u64) {
        self.next_instance_id = 1;
        self.free_instance_ids = vec![true; max_existing_instance_id as usize + 2];
        self.free_instance_ids[0] = false;
    }

    pub fn register_instance_id(&mut self, instance_id: u32) {
        self.ensure_len(instance_id as usize + 1);
        self.free_instance_ids[instance_id as usize] = false;
        if self.next_instance_id == instance_id {
            self.next_instance_id += 1;
        }
    }

    pub fn generate_instance_id(&mut self) -> Option<u32> {
        if self.next_instance_id == u32::MAX {
            return None;
        }

        let new_instance_id = self.next_instance_id;
        self.ensure_len(new_instance_id as usize + 1);
        self.free_instance_ids[new_instance_id as usize] = false;

        let search_start = self.next_instance_id.saturating_add(1) as usize;
        if let Some(next) = self
            .free_instance_ids
            .iter()
            .enumerate()
            .skip(search_start)
            .find_map(|(index, free)| (*free).then_some(index as u32))
        {
            self.next_instance_id = next;
        } else {
            self.next_instance_id = self.free_instance_ids.len() as u32;
            self.free_instance_ids.push(true);
        }

        Some(new_instance_id)
    }

    pub fn free_instance_id(&mut self, instance_id: u32) {
        self.ensure_len(instance_id as usize + 1);
        self.next_instance_id = self.next_instance_id.min(instance_id);
        self.free_instance_ids[instance_id as usize] = true;
    }

    pub const fn next_instance_id(&self) -> u32 {
        self.next_instance_id
    }

    pub(super) fn ensure_len(&mut self, len: usize) {
        if self.free_instance_ids.len() < len {
            self.free_instance_ids.resize(len, true);
            self.free_instance_ids[0] = false;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IntervalTimer {
    pub(super) interval_ms: u32,
    pub(super) current_ms: u32,
}

impl IntervalTimer {
    pub(super) const fn new(interval_ms: u32) -> Self {
        Self {
            interval_ms,
            current_ms: 0,
        }
    }

    pub(super) fn set_interval(&mut self, interval_ms: u32) {
        self.interval_ms = interval_ms.max(MIN_MAP_UPDATE_DELAY_MS);
    }

    pub(super) fn reset(&mut self) {
        self.current_ms = 0;
    }

    pub(super) fn update(&mut self, diff_ms: u32) {
        self.current_ms = self.current_ms.saturating_add(diff_ms);
    }

    pub(super) const fn passed(self) -> bool {
        self.current_ms >= self.interval_ms
    }

    pub(super) const fn current(self) -> u32 {
        self.current_ms
    }

    pub(super) fn set_current(&mut self, current_ms: u32) {
        self.current_ms = current_ms;
    }
}

pub struct MapManager {
    pub(super) grid_cleanup_delay_ms: u32,
    pub(super) maps: BTreeMap<MapKey, ManagedMap>,
    pub(super) timer: IntervalTimer,
    pub(super) instance_ids: InstanceIdAllocator,
    pub(super) updater: MapUpdater,
    pub(super) scheduled_scripts: usize,
    pub(super) spawn_group_initializer_like_cpp: Option<SpawnGroupInitializerLikeCpp>,
    pub(super) player_owners_like_cpp: BTreeMap<ObjectGuid, player_owner::PlayerOwnershipLikeCpp>,
    pub(super) detached_players_like_cpp: BTreeMap<ObjectGuid, Box<wow_entities::Player>>,
    pub(super) next_player_generation_like_cpp: u64,
}

impl fmt::Debug for MapManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapManager")
            .field("grid_cleanup_delay_ms", &self.grid_cleanup_delay_ms)
            .field("maps", &self.maps)
            .field("timer", &self.timer)
            .field("instance_ids", &self.instance_ids)
            .field("updater", &self.updater)
            .field("scheduled_scripts", &self.scheduled_scripts)
            .field(
                "spawn_group_initializer_like_cpp",
                &self
                    .spawn_group_initializer_like_cpp
                    .as_ref()
                    .map(|_| "<hook>"),
            )
            .finish()
    }
}

impl Default for MapManager {
    fn default() -> Self {
        Self::new(MIN_GRID_DELAY_MS, MIN_MAP_UPDATE_DELAY_MS)
    }
}
