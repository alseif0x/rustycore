//! MapManager skeleton.
//!
//! C++ references:
//! - `game/Maps/MapManager.h`
//! - `game/Maps/MapManager.cpp`

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use crate::DEFAULT_VISIBILITY_NOTIFY_PERIOD;
use crate::MapKey;
use crate::map::{
    AreaTriggersUpdateSummaryLikeCpp, ConversationsUpdateSummaryLikeCpp,
    CreatureUpdateSummaryLikeCpp, DynamicMapTreeUpdateSummaryLikeCpp,
    DynamicObjectsUpdateSummaryLikeCpp, FarSpellCallbackDrainSummaryLikeCpp,
    GameObjectsUpdateSummaryLikeCpp, GridStatesUpdateSummaryLikeCpp,
    LoadedGridRespawnRecordsLikeCpp, Map, MapUpdateMetricsSummaryLikeCpp,
    MoveListDrainSummaryLikeCpp, NoopGridLifecycle, NoopTerrainGridLoader,
    PersonalPhaseTrackerUpdateSummaryLikeCpp, ProcessRelocationNotifiesOutcome,
    SceneObjectUpdateContextLikeCpp, SceneObjectsUpdateSummaryLikeCpp,
    ScriptScheduleProcessSummaryLikeCpp, SendObjectUpdatesSummaryLikeCpp,
    TransportsUpdateSummaryLikeCpp, WeatherUpdateSummaryLikeCpp,
};
use crate::pool::PoolMgrLikeCpp;
use crate::spawn::{Difficulty, SpawnId, SpawnObjectType, SpawnStore};
use wow_core::GameTime;
use wow_entities::CreatureRuntimeUpdateContext;

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

#[derive(Debug)]
pub struct ManagedMap {
    map: Map<NoopTerrainGridLoader, NoopGridLifecycle>,
    kind: ManagedMapKind,
    can_unload: bool,
    player_count: u32,
    instance_lock_token: Option<u64>,
    update_calls: Vec<u32>,
    delayed_update_calls: Vec<u32>,
    last_dynamic_tree_update_summary_like_cpp: DynamicMapTreeUpdateSummaryLikeCpp,
    last_dynamic_objects_update_summary: DynamicObjectsUpdateSummaryLikeCpp,
    last_creatures_update_summary: CreatureUpdateSummaryLikeCpp,
    last_game_objects_update_summary: GameObjectsUpdateSummaryLikeCpp,
    last_transports_update_summary: TransportsUpdateSummaryLikeCpp,
    last_area_triggers_update_summary: AreaTriggersUpdateSummaryLikeCpp,
    last_conversations_update_summary: ConversationsUpdateSummaryLikeCpp,
    last_scene_objects_update_summary: SceneObjectsUpdateSummaryLikeCpp,
    last_send_object_updates_summary_like_cpp: SendObjectUpdatesSummaryLikeCpp,
    last_script_schedule_process_summary_like_cpp: ScriptScheduleProcessSummaryLikeCpp,
    last_weather_update_summary_like_cpp: WeatherUpdateSummaryLikeCpp,
    last_personal_phase_tracker_update_summary: PersonalPhaseTrackerUpdateSummaryLikeCpp,
    last_live_move_list_drain_summary: LiveMoveListDrainSummaryLikeCpp,
    last_far_spell_callback_drain_summary_like_cpp: FarSpellCallbackDrainSummaryLikeCpp,
    last_grid_states_update_summary_like_cpp: GridStatesUpdateSummaryLikeCpp,
    last_process_relocation_notifies_outcome_like_cpp: ProcessRelocationNotifiesOutcome,
    last_map_update_tail_summary_like_cpp: MapUpdateTailSummaryLikeCpp,
    unload_all_calls: u32,
}

fn game_time_now_secs_i64() -> i64 {
    let now_secs = GameTime::now().as_secs();
    now_secs.min(i64::MAX as u64) as i64
}

fn game_time_now_ms_u64() -> u64 {
    static PROCESS_START: OnceLock<Instant> = OnceLock::new();
    game_time_elapsed_ms_u64(PROCESS_START.get_or_init(Instant::now).elapsed())
}

fn game_time_elapsed_ms_u64(elapsed: Duration) -> u64 {
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
            map: Map::new(map_id, instance_id, difficulty, grid_expiry_ms),
            kind,
            can_unload: false,
            player_count: 0,
            instance_lock_token: None,
            update_calls: Vec::new(),
            delayed_update_calls: Vec::new(),
            last_dynamic_tree_update_summary_like_cpp: DynamicMapTreeUpdateSummaryLikeCpp::default(
            ),
            last_dynamic_objects_update_summary: DynamicObjectsUpdateSummaryLikeCpp::default(),
            last_creatures_update_summary: CreatureUpdateSummaryLikeCpp::default(),
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
        self.map.map_id()
    }

    pub const fn instance_id(&self) -> u32 {
        self.map.instance_id()
    }

    pub const fn kind(&self) -> ManagedMapKind {
        self.kind
    }

    pub fn map(&self) -> &Map<NoopTerrainGridLoader, NoopGridLifecycle> {
        &self.map
    }

    pub fn map_mut(&mut self) -> &mut Map<NoopTerrainGridLoader, NoopGridLifecycle> {
        &mut self.map
    }

    pub fn set_can_unload(&mut self, can_unload: bool) {
        self.can_unload = can_unload;
    }

    pub fn set_player_count(&mut self, player_count: u32) {
        self.player_count = player_count;
    }

    pub const fn player_count(&self) -> u32 {
        self.player_count
    }

    pub const fn instance_lock_token(&self) -> Option<u64> {
        self.instance_lock_token
    }

    pub fn set_instance_lock_token(&mut self, token: Option<u64>) {
        self.instance_lock_token = token;
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

    fn can_unload(&self, _diff_ms: u32) -> bool {
        self.can_unload
    }

    fn remove_all_players(&mut self) {
        self.player_count = 0;
    }

    fn have_players(&self) -> bool {
        self.player_count > 0
    }

    fn update(&mut self, diff_ms: u32) {
        self.update_with_optional_pool_update_context::<fn(
            &mut Map,
            SpawnObjectType,
            SpawnId,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(diff_ms, None, None);
    }

    fn update_with_pool_update_context(
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

    fn update_with_pool_update_loaded_grid_records_context<L>(
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

    fn update_with_optional_pool_update_context<L>(
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
            self.map.update_dynamic_tree_like_cpp(diff_ms);
        self.update_calls.push(diff_ms);
        self.last_dynamic_objects_update_summary =
            self.map.update_dynamic_objects_like_cpp(diff_ms);
        let now_secs = game_time_now_secs_i64();
        // Partial C++ ObjectUpdater seam: after DynamicObject, visit only the
        // represented map-owned Creature family in this slice. Default context is
        // honest represented runtime only: no real AI/combat/threat/fanout.
        self.last_creatures_update_summary =
            self.map
                .update_creatures_like_cpp(diff_ms, now_secs, |_guid, _creature| {
                    CreatureRuntimeUpdateContext::default()
                });
        // Partial C++ ObjectUpdater seam: after Creature, visit represented
        // map-owned GameObject records. C++ real order is TypeContainerVisitor
        // nearby-cell/active-object traversal; this Rust insertion only adds the
        // missing family and leaves AI/go-type/per-player/packet/DB gaps open.
        self.last_game_objects_update_summary = match (pool_update, load_record) {
            (Some((spawn_store, pool_mgr)), Some(load_record)) => self
                .map
                .update_game_objects_with_pool_update_loaded_grid_records_like_cpp(
                    diff_ms,
                    now_secs,
                    spawn_store,
                    pool_mgr,
                    load_record,
                ),
            (Some((spawn_store, pool_mgr)), None) => {
                self.map.update_game_objects_with_pool_update_like_cpp(
                    diff_ms,
                    now_secs,
                    spawn_store,
                    pool_mgr,
                )
            }
            (None, _) => self.map.update_game_objects_like_cpp(diff_ms, now_secs),
        };
        // Partial C++ transport seam: after the represented GameObject/ObjectUpdater
        // family and before later represented families, visit typed canonical
        // Transports. This does not reproduce exact C++ cell visitor ordering nor
        // full `_transports` runtime (AI/scripts/spline/teleport/fanout/passengers).
        let now_ms = game_time_now_ms_u64();
        self.last_transports_update_summary = self.map.update_transports_like_cpp(diff_ms, now_ms);
        // Partial C++ ObjectUpdater seam: after Transport, visit only the
        // represented map-owned AreaTrigger family in this slice. Other families,
        // nearby-cell traversal, player/session updates, fanout and scripts stay
        // explicit remaining gaps.
        self.last_area_triggers_update_summary = self.map.update_area_triggers_like_cpp(diff_ms);
        // Partial C++ ObjectUpdater seam: visit represented map-owned
        // Conversation records after AreaTrigger for this Rust slice. Exact
        // TypeContainerVisitor ordering/cell traversal, real scripts,
        // SendObjectUpdates and fanout remain explicit gaps.
        self.last_conversations_update_summary = self.map.update_conversations_like_cpp(diff_ms);
        // Partial C++ ObjectUpdater seam: visit represented map-owned
        // SceneObject records after Conversation for this Rust slice. Real
        // ObjectAccessor::GetUnit and Aura lookup by spell/cast id are not present
        // yet, so the live manager default is conservative and does not remove
        // SceneObjects merely because that runtime is absent.
        self.last_scene_objects_update_summary =
            self.map
                .update_scene_objects_like_cpp(diff_ms, |_guid, scene_object| {
                    SceneObjectUpdateContextLikeCpp::represented_default_for(scene_object)
                });
        // C++ calls `Map::SendObjectUpdates()` after ObjectUpdater/Transport/
        // SceneObject-style visitation and before scripts/weather/personal phase
        // (`Map.cpp:777-798`). Rust consumes only represented map-owned
        // `m_objectUpdated`/changed-mask state here; no `UpdateDataMapType`,
        // session packets, visible-player iteration, or direct fanout is built.
        self.last_send_object_updates_summary_like_cpp = self.map.send_object_updates_like_cpp();
        // C++ then drains `m_scriptSchedule` under `i_scriptLock` before weather
        // and personal phase (`Map.cpp:777-798`, `MapScripts.cpp:311-321`).
        // Rust records due represented actions only; no script commands,
        // ObjectAccessor/session/fanout/DB/weather side effects are executed.
        self.last_script_schedule_process_summary_like_cpp = self
            .map
            .process_script_schedule_update_order_like_cpp(now_secs);
        // C++ updates `_weatherUpdateTimer` immediately after script schedule and
        // before `GetMultiPersonalPhaseTracker().Update(this, t_diff)`
        // (`Map.cpp:777-798`). Rust represents only the map-owned timer and
        // `_zoneDynamicInfo.DefaultWeather` update/reset seam; WeatherMgr, RNG,
        // script hooks, player fanout, packets, DB and zone messages remain gaps.
        self.last_weather_update_summary_like_cpp = self.map.update_weather_like_cpp(diff_ms);
        // C++ calls `GetMultiPersonalPhaseTracker().Update(this, t_diff)` after
        // SendObjectUpdates/scripts/weather and before later move/remove drains.
        // Rust consumes the existing map-owned tracker here as a represented seam
        // only: GUID expiry -> AddObjectToRemoveList, without claiming exact full
        // update ordering, visibility fanout, DB, scripts, or dynamic-tree parity.
        self.last_personal_phase_tracker_update_summary =
            self.map.update_personal_phase_tracker_like_cpp(diff_ms);
        // C++ `Map::Update` immediately drains Creature, GameObject, and
        // AreaTrigger move-lists after personal-phase tracker update and before
        // `ProcessRelocationNotifies(t_diff)` (`Map.cpp:797-805`). Rust keeps
        // `Map` as the sole owner of canonical object and queue state here; the
        // live manager only orchestrates order and stores summaries. DynamicObject
        // is intentionally not drained by this live path because C++ `Map::Update`
        // does not call a DynamicObject move-list drain.
        self.last_live_move_list_drain_summary = LiveMoveListDrainSummaryLikeCpp {
            creature: self.map.move_all_creatures_in_move_list_like_cpp(),
            game_object: self.map.move_all_game_objects_in_move_list_like_cpp(),
            area_trigger: self.map.move_all_area_triggers_in_move_list_like_cpp(),
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
            .map
            .process_live_relocation_notifies_like_cpp(diff_ms, DEFAULT_VISIBILITY_NOTIFY_PERIOD);
        // C++ `Map::Update` tail immediately follows ProcessRelocationNotifies:
        // `sScriptMgr->OnMapUpdate(this, t_diff)` then the `map_creatures` and
        // `map_gameobjects` metrics (`Map.cpp:804-815`). Rust records only the
        // boundary invocation and typed canonical counts from `Map::map_objects`;
        // no real ScriptMgr dispatch, script callbacks, Prometheus/telemetry,
        // ObjectAccessor, DB, or fanout side effects are claimed.
        self.last_map_update_tail_summary_like_cpp = MapUpdateTailSummaryLikeCpp {
            script_hook: MapUpdateScriptHookSummaryLikeCpp {
                invoked: true,
                diff_ms,
                map_id: self.map.map_id(),
                instance_id: self.map.instance_id(),
                kind: self.kind,
                script_dispatch_represented: false,
            },
            metrics: self.map.map_update_metrics_like_cpp(),
        };
    }

    fn delayed_update(&mut self, diff_ms: u32) {
        self.delayed_update_calls.push(diff_ms);
        // C++ `Map::DelayedUpdate` drains `_farSpellCallbacks` before
        // `RemoveAllObjectsInRemoveList()`, then updates grid states unless BG/arena
        // (`Map.cpp:2519-2544`). Rust keeps callback ownership inside `Map`; the
        // manager only orchestrates the live order and records the drain summary.
        self.last_far_spell_callback_drain_summary_like_cpp =
            self.map.drain_far_spell_callbacks_like_cpp();
        self.map.remove_all_objects_in_remove_list_like_cpp();
        self.last_grid_states_update_summary_like_cpp = if self.kind.is_battleground_or_arena() {
            GridStatesUpdateSummaryLikeCpp {
                diff_ms,
                skipped_battleground_or_arena: true,
                ..GridStatesUpdateSummaryLikeCpp::default()
            }
        } else {
            self.map.update_loaded_grid_states_like_cpp(diff_ms)
        };
    }

    fn unload_all(&mut self) {
        self.unload_all_calls += 1;
    }
}

pub type SpawnGroupInitializerLikeCpp = Arc<dyn Fn(&mut ManagedMap) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceIdAllocator {
    free_instance_ids: Vec<bool>,
    next_instance_id: u32,
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

    fn ensure_len(&mut self, len: usize) {
        if self.free_instance_ids.len() < len {
            self.free_instance_ids.resize(len, true);
            self.free_instance_ids[0] = false;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntervalTimer {
    interval_ms: u32,
    current_ms: u32,
}

impl IntervalTimer {
    const fn new(interval_ms: u32) -> Self {
        Self {
            interval_ms,
            current_ms: 0,
        }
    }

    fn set_interval(&mut self, interval_ms: u32) {
        self.interval_ms = interval_ms.max(MIN_MAP_UPDATE_DELAY_MS);
    }

    fn reset(&mut self) {
        self.current_ms = 0;
    }

    fn update(&mut self, diff_ms: u32) {
        self.current_ms = self.current_ms.saturating_add(diff_ms);
    }

    const fn passed(self) -> bool {
        self.current_ms >= self.interval_ms
    }

    const fn current(self) -> u32 {
        self.current_ms
    }

    fn set_current(&mut self, current_ms: u32) {
        self.current_ms = current_ms;
    }
}

pub struct MapManager {
    grid_cleanup_delay_ms: u32,
    maps: BTreeMap<MapKey, ManagedMap>,
    timer: IntervalTimer,
    instance_ids: InstanceIdAllocator,
    updater: MapUpdater,
    scheduled_scripts: usize,
    spawn_group_initializer_like_cpp: Option<SpawnGroupInitializerLikeCpp>,
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

    fn update_with_optional_pool_update_context<L>(
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

        for key in destroyed {
            self.maps.remove(&key);
        }

        for map in self.maps.values_mut() {
            map.delayed_update(current);
        }

        self.timer.set_current(0);
        Some(current)
    }

    pub fn destroy_map(&mut self, map_id: u32, instance_id: u32) -> bool {
        let key = MapKey::new(map_id, instance_id);
        let Some(map) = self.maps.get_mut(&key) else {
            return false;
        };

        if Self::destroy_map_inner(map, &mut self.instance_ids) {
            self.maps.remove(&key);
            true
        } else {
            false
        }
    }

    fn destroy_map_inner(map: &mut ManagedMap, instance_ids: &mut InstanceIdAllocator) -> bool {
        map.remove_all_players();
        if map.have_players() {
            return false;
        }

        map.unload_all();

        if map.kind().frees_instance_id_on_destroy() {
            instance_ids.free_instance_id(map.instance_id());
        }

        true
    }

    pub fn unload_all(&mut self) {
        for map in self.maps.values_mut() {
            map.unload_all();
        }
        self.maps.clear();
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
    worker_threads: usize,
    pending_requests: usize,
    scheduled_updates: usize,
    wait_calls: usize,
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

    fn update_finished(&mut self) {
        self.pending_requests = self.pending_requests.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::coords::GridCoord;
    use crate::grid::GridStateKind;
    use crate::map::{
        MapObjectMoveListFamilyLikeCpp, RepresentedFarSpellCallbackActionLikeCpp,
        RepresentedFarSpellCallbackLikeCpp,
    };
    use crate::pool::{
        PoolGroupLikeCpp, PoolMemberKindLikeCpp, PoolObjectLikeCpp, PoolTemplateDataLikeCpp,
    };
    use crate::spawn::{SpawnData, SpawnGroupFlags, SpawnGroupTemplateData, SpawnObjectType};
    use wow_constants::{DeathState, TypeId, TypeMask};
    use wow_core::{ObjectGuid, Position, guid::HighGuid};
    use wow_entities::{
        AccessorObjectKind, AreaTrigger, Conversation, Creature, DynamicObject,
        GAMEOBJECT_TYPE_GENERIC, GameObject, LootState, MapObjectRecord, ObjectNotifyFlags, Player,
        SceneObject, Transport, TransportPathLeg, TransportTemplate, WorldObject,
    };

    #[test]
    fn game_time_now_ms_uses_runtime_monotonic_subsecond_counter() {
        assert_eq!(game_time_elapsed_ms_u64(Duration::from_millis(999)), 999);
        assert_eq!(
            game_time_elapsed_ms_u64(Duration::from_millis(1_001)),
            1_001
        );

        let first = game_time_now_ms_u64();
        let second = game_time_now_ms_u64();
        assert!(second >= first);
    }

    #[test]
    fn map_manager_update_consumes_dynamic_tree_before_instrumented_tail_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let map = manager.find_map_mut(1, 0).unwrap();
        map.map_mut()
            .set_dynamic_tree_model_count_for_tests_like_cpp(1);
        map.map_mut()
            .mark_dynamic_tree_unbalanced_for_tests_like_cpp(2);

        manager.update(199);
        let map = manager.find_map(1, 0).unwrap();
        let first = map.last_dynamic_tree_update_summary_like_cpp();
        assert_eq!(first.diff_ms, 199);
        assert!(!first.empty);
        assert_eq!(first.timer_before_ms, 200);
        assert_eq!(first.timer_after_ms, 1);
        assert!(!first.timer_passed);
        assert_eq!(first.unbalanced_before, 2);
        assert_eq!(first.unbalanced_after, 2);
        assert_eq!(map.update_calls(), &[199]);

        manager.update(1);
        let map = manager.find_map(1, 0).unwrap();
        let second = map.last_dynamic_tree_update_summary_like_cpp();
        assert_eq!(second.diff_ms, 1);
        assert_eq!(second.timer_before_ms, 1);
        assert_eq!(second.timer_after_ms, 200);
        assert!(second.timer_passed);
        assert_eq!(second.timer_reset_to_ms, Some(200));
        assert_eq!(second.unbalanced_before, 2);
        assert!(second.balanced);
        assert_eq!(second.unbalanced_after, 0);
        assert_eq!(map.update_calls(), &[199, 1]);
    }

    #[test]
    fn delays_are_clamped_like_map_manager_h() {
        let manager = MapManager::new(1, 0);

        assert_eq!(manager.grid_cleanup_delay_ms(), MIN_GRID_DELAY_MS);
    }

    #[test]
    fn create_and_find_map_uses_cpp_map_key_shape() {
        let mut manager = MapManager::default();

        manager.create_world_map(1, 0);

        let map = manager.find_map(1, 0).unwrap();
        assert_eq!(map.map_id(), 1);
        assert_eq!(map.instance_id(), 0);
        assert!(manager.find_map(1, 1).is_none());
    }

    #[test]
    fn map_manager_init_spawn_group_state_hook_runs_once_for_new_maps_only() {
        let mut manager = MapManager::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let hook_calls = Arc::clone(&calls);
        manager.set_spawn_group_initializer_like_cpp(move |map| {
            hook_calls.fetch_add(1, Ordering::SeqCst);
            map.set_player_count(7);
        });

        manager.create_world_map(571, 0);
        manager.create_world_map(571, 0);
        manager.create_map_entry(571, 0, 0, ManagedMapKind::World);

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(manager.find_map(571, 0).unwrap().player_count(), 7);

        manager.create_map_entry(
            571,
            9,
            1,
            ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        manager.clear_spawn_group_initializer_like_cpp();
        manager.create_world_map(1, 0);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn map_manager_init_spawn_group_state_hook_can_mutate_managed_map_spawn_groups() {
        let manual = SpawnGroupTemplateData {
            group_id: 10,
            name: "manual".to_string(),
            map_id: 571,
            flags: SpawnGroupFlags::MANUAL_SPAWN,
        };
        let automatic = SpawnGroupTemplateData {
            group_id: 11,
            name: "automatic".to_string(),
            map_id: 571,
            flags: SpawnGroupFlags::NONE,
        };
        let system = SpawnGroupTemplateData {
            group_id: 12,
            name: "system".to_string(),
            map_id: 571,
            flags: SpawnGroupFlags::SYSTEM,
        };
        let groups = Arc::new(vec![manual.clone(), automatic.clone(), system.clone()]);

        let mut manager = MapManager::default();
        manager.set_spawn_group_initializer_like_cpp({
            let groups = Arc::clone(&groups);
            move |managed_map| {
                managed_map
                    .map_mut()
                    .init_spawn_group_state_like_cpp(groups.iter(), |group| {
                        group.group_id == manual.group_id
                    });
            }
        });

        manager.create_world_map(571, 0);
        let map = manager.find_map(571, 0).unwrap().map();

        assert!(map.is_spawn_group_active_like_cpp(Some(&groups[0])));
        assert!(!map.is_spawn_group_active_like_cpp(Some(&groups[1])));
        assert!(map.is_spawn_group_active_like_cpp(Some(&groups[2])));
    }

    #[test]
    fn do_for_all_maps_with_map_id_uses_ordered_pair_range() {
        let mut manager = MapManager::default();
        manager.create_world_map(1, 0);
        manager.create_world_map(2, 0);
        manager.create_map_entry(1, 3, 0, ManagedMapKind::World);

        let mut keys = Vec::new();
        manager.do_for_all_maps_with_map_id(1, |map| {
            keys.push((map.map_id(), map.instance_id()));
        });

        assert_eq!(keys, vec![(1, 0), (1, 3)]);
    }

    #[test]
    fn do_for_all_maps_mut_visits_maps_in_btreemap_order_and_allows_mutation() {
        let mut manager = MapManager::default();
        manager.create_world_map(2, 0);
        manager.create_map_entry(1, 3, 0, ManagedMapKind::World);
        manager.create_world_map(1, 0);

        let mut keys = Vec::new();
        manager.do_for_all_maps_mut(|map| {
            keys.push((map.map_id(), map.instance_id()));
            map.set_player_count(map.map_id() + map.instance_id());
        });

        assert_eq!(keys, vec![(1, 0), (1, 3), (2, 0)]);
        assert_eq!(manager.find_map(1, 0).unwrap().player_count(), 1);
        assert_eq!(manager.find_map(1, 3).unwrap().player_count(), 4);
        assert_eq!(manager.find_map(2, 0).unwrap().player_count(), 2);
    }

    #[test]
    fn update_waits_for_interval_then_updates_and_delayed_updates_maps() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 10);
        manager.create_world_map(1, 0);

        assert_eq!(manager.update(9), None);
        assert!(manager.find_map(1, 0).unwrap().update_calls().is_empty());

        assert_eq!(manager.update(1), Some(10));

        let map = manager.find_map(1, 0).unwrap();
        assert_eq!(map.update_calls(), &[10]);
        assert_eq!(map.delayed_update_calls(), &[10]);
    }

    #[test]
    fn live_delayed_update_consumes_removal_grid_state_after_remove_list_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let position = Position::xyz(3_000.0, 3_000.0, 0.0);
        {
            let managed_map = manager.find_map_mut(1, 0).unwrap();
            assert!(managed_map.map_mut().load_grid(position.x, position.y));
            let cell = crate::map::cell_from_world(position.x, position.y);
            let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
            let grid = managed_map.map_mut().get_ngrid_mut(coord).unwrap();
            grid.set_state(GridStateKind::Removal);
            grid.info_mut().reset_time_tracker(1);
            assert!(managed_map.map().get_ngrid(coord).is_some());
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        let cell = crate::map::cell_from_world(position.x, position.y);
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        let summary = managed_map.last_grid_states_update_summary_like_cpp();
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(summary.diff_ms, 1);
        assert_eq!(summary.visited, 1);
        assert_eq!(summary.updated, 1);
        assert_eq!(summary.unloaded, 1);
        assert_eq!(summary.removal_unloaded, 1);
        assert!(!summary.skipped_battleground_or_arena);
        assert!(managed_map.map().get_ngrid(coord).is_none());
    }

    #[test]
    fn battleground_delayed_update_skips_grid_state_update_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_map_entry(1, 33, 0, ManagedMapKind::Battleground);
        let position = Position::xyz(3_100.0, 3_100.0, 0.0);
        {
            let managed_map = manager.find_map_mut(1, 33).unwrap();
            assert!(managed_map.map_mut().load_grid(position.x, position.y));
            let cell = crate::map::cell_from_world(position.x, position.y);
            let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
            let grid = managed_map.map_mut().get_ngrid_mut(coord).unwrap();
            grid.set_state(GridStateKind::Removal);
            grid.info_mut().reset_time_tracker(1);
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 33).unwrap();
        let cell = crate::map::cell_from_world(position.x, position.y);
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        let summary = managed_map.last_grid_states_update_summary_like_cpp();
        assert_eq!(summary.diff_ms, 1);
        assert!(summary.skipped_battleground_or_arena);
        assert_eq!(summary.visited, 0);
        assert_eq!(summary.unloaded, 0);
        assert_eq!(
            managed_map.map().get_ngrid(coord).unwrap().state(),
            GridStateKind::Removal
        );
    }

    #[test]
    fn live_update_delayed_update_drains_dynamic_object_remove_list_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let dynamic_object_guid = guid(HighGuid::DynamicObject, 4320101, 1, 0);
        let mut dynamic_object = DynamicObject::new(true);
        dynamic_object
            .world_mut()
            .object_mut()
            .create(dynamic_object_guid);
        dynamic_object.world_mut().set_map(1, 0).unwrap();
        dynamic_object
            .world_mut()
            .relocate(Position::xyz(11.0, 21.0, 31.0));
        dynamic_object.world_mut().object_mut().add_to_world();

        {
            let managed_map = manager.find_map_mut(1, 0).unwrap();
            managed_map
                .map_mut()
                .add_map_object_record_to_map_like_cpp(
                    MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
                )
                .unwrap();
            let queued = managed_map
                .map_mut()
                .add_object_to_remove_list_like_cpp(dynamic_object_guid);
            assert!(queued.queued);
            assert!(
                managed_map
                    .map()
                    .map_object_record(dynamic_object_guid)
                    .is_some()
            );
            assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 1);
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert!(
            managed_map
                .map()
                .map_object_record(dynamic_object_guid)
                .is_none()
        );
    }

    #[test]
    fn map_manager_update_far_spell_callback_queues_remove_before_delayed_remove_drain_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let dynamic_object_guid = guid(HighGuid::DynamicObject, 487_0101, 1, 0);
        let mut dynamic_object = DynamicObject::new(true);
        dynamic_object
            .world_mut()
            .object_mut()
            .create(dynamic_object_guid);
        dynamic_object.world_mut().set_map(1, 0).unwrap();
        dynamic_object
            .world_mut()
            .relocate(Position::xyz(11.0, 21.0, 31.0));
        dynamic_object.world_mut().object_mut().add_to_world();
        dynamic_object.set_duration(10_000);

        {
            let managed_map = manager.find_map_mut(1, 0).unwrap();
            managed_map
                .map_mut()
                .add_map_object_record_to_map_like_cpp(
                    MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
                )
                .unwrap();
            managed_map.map_mut().add_far_spell_callback_like_cpp(
                RepresentedFarSpellCallbackLikeCpp {
                    id: 487,
                    action: RepresentedFarSpellCallbackActionLikeCpp::QueueObjectRemove {
                        guid: dynamic_object_guid,
                    },
                },
            );
            assert_eq!(managed_map.map().far_spell_callbacks_count_like_cpp(), 1);
            assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
            assert!(
                managed_map
                    .map()
                    .map_object_record(dynamic_object_guid)
                    .is_some()
            );
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        let far_spell = managed_map.last_far_spell_callback_drain_summary_like_cpp();
        assert_eq!(far_spell.processed, 1);
        assert_eq!(far_spell.remove_queued, 1);
        assert_eq!(far_spell.queued_after, 0);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert!(
            managed_map
                .map()
                .map_object_record(dynamic_object_guid)
                .is_none()
        );
        assert_eq!(
            managed_map
                .last_grid_states_update_summary_like_cpp()
                .diff_ms,
            1
        );
    }

    #[test]
    fn map_manager_update_visits_live_dynamic_object_without_expiry_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4330101, 10, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_dynamic_objects_update_summary(),
            DynamicObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                expired_remove_queued: 0,
                missing_or_stale: 0,
                not_dynamic_object: 0,
                not_in_world: 0,
            }
        );
        let dynamic_object = managed_map
            .map()
            .get_typed_dynamic_object(dynamic_object_guid)
            .unwrap();
        assert_eq!(dynamic_object.duration_ms(), 9);
    }

    #[test]
    fn map_manager_update_expires_dynamic_object_then_delayed_update_drains_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4330201, 1, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_dynamic_objects_update_summary(),
            DynamicObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                expired_remove_queued: 1,
                missing_or_stale: 0,
                not_dynamic_object: 0,
                not_in_world: 0,
            }
        );
        assert!(
            managed_map
                .map()
                .map_object_record(dynamic_object_guid)
                .is_none()
        );
    }

    #[test]
    fn map_manager_update_personal_phase_expiry_enqueues_and_delayed_update_drains_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let creature_guid = insert_creature_for_update(&mut manager, 4400301, true);
        let owner = ObjectGuid::create_player(1, 44003);
        {
            let map = manager.find_map_mut(1, 0).unwrap().map_mut();
            map.register_personal_phase_object_for_test(44, owner, creature_guid);
            map.mark_personal_phases_for_deletion_for_test(owner);
        }

        assert_eq!(manager.update(60_000), Some(60_000));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[60_000]);
        assert_eq!(managed_map.delayed_update_calls(), &[60_000]);
        assert_eq!(
            managed_map.last_personal_phase_tracker_update_summary(),
            PersonalPhaseTrackerUpdateSummaryLikeCpp {
                expired_objects: 1,
                remove_queued: 1,
                missing_or_stale: 0,
                unsupported_kinds: 0,
                duplicate_queued: 0,
            }
        );
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert!(managed_map.map().map_object_record(creature_guid).is_none());
    }

    #[test]
    fn map_manager_update_processes_weather_between_scripts_and_personal_phase_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let creature_guid = insert_creature_for_update(&mut manager, 4460101, true);
        let owner = ObjectGuid::create_player(1, 44601);
        {
            let map = manager.find_map_mut(1, 0).unwrap().map_mut();
            let schedule = map.schedule_represented_script_action_like_cpp(
                0,
                1,
                ObjectGuid::create_player(1, 44602),
                creature_guid,
                owner,
                446,
            );
            assert!(schedule.immediate_process.is_none());
            assert_eq!(map.represented_script_schedule_count_like_cpp(), 1);
            map.register_represented_zone_default_weather_for_test(447);
            map.register_personal_phase_object_for_test(446, owner, creature_guid);
            map.mark_personal_phases_for_deletion_for_test(owner);
        }

        assert_eq!(manager.update(60_000), Some(60_000));

        let managed_map = manager.find_map(1, 0).unwrap();
        let script_summary = managed_map.last_script_schedule_process_summary_like_cpp();
        assert_eq!(script_summary.queued_before, 1);
        assert_eq!(script_summary.processed, 1);
        assert_eq!(script_summary.remaining, 0);
        assert!(script_summary.lock_entered);
        assert_eq!(script_summary.processed_actions[0].command_id, 446);
        assert_eq!(
            managed_map
                .map()
                .represented_script_schedule_count_like_cpp(),
            0
        );
        let weather_summary = managed_map.last_weather_update_summary_like_cpp();
        assert!(weather_summary.timer_passed);
        assert_eq!(weather_summary.timer_current_before, 0);
        assert_eq!(weather_summary.timer_current_after_update, 60_000);
        assert_eq!(weather_summary.timer_current_after_reset, 0);
        assert_eq!(weather_summary.zones_seen, 1);
        assert_eq!(weather_summary.default_weather_updated, 1);
        assert_eq!(weather_summary.weather_update_call_diff_ms, Some(1_000));
        assert_eq!(
            managed_map
                .map()
                .represented_zone_default_weather_update_diffs_like_cpp(447),
            Some([1_000].as_slice())
        );
        assert_eq!(
            managed_map.last_personal_phase_tracker_update_summary(),
            PersonalPhaseTrackerUpdateSummaryLikeCpp {
                expired_objects: 1,
                remove_queued: 1,
                missing_or_stale: 0,
                unsupported_kinds: 0,
                duplicate_queued: 0,
            }
        );
        assert_eq!(
            managed_map
                .last_live_move_list_drain_summary_like_cpp()
                .creature
                .processed,
            0
        );
        assert!(managed_map.map().map_object_record(creature_guid).is_none());
    }

    #[test]
    fn map_update_tail_empty_map_records_hook_and_zero_metrics_like_cpp() {
        let mut managed_map = ManagedMap::new(
            571,
            77,
            0,
            MIN_GRID_DELAY_MS.into(),
            ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
        );

        assert_eq!(
            managed_map.last_map_update_tail_summary_like_cpp(),
            MapUpdateTailSummaryLikeCpp::default()
        );

        managed_map.update(37);

        assert_eq!(
            managed_map.last_map_update_tail_summary_like_cpp(),
            MapUpdateTailSummaryLikeCpp {
                script_hook: MapUpdateScriptHookSummaryLikeCpp {
                    invoked: true,
                    diff_ms: 37,
                    map_id: 571,
                    instance_id: 77,
                    kind: ManagedMapKind::Dungeon {
                        has_reset_schedule: true,
                    },
                    script_dispatch_represented: false,
                },
                metrics: MapUpdateMetricsSummaryLikeCpp {
                    creature_count: 0,
                    gameobject_count: 0,
                    map_id: 571,
                    instance_id: 77,
                },
            }
        );
        assert_eq!(
            managed_map.last_process_relocation_notifies_outcome_like_cpp(),
            ProcessRelocationNotifiesOutcome::default()
        );
    }

    #[test]
    fn map_update_tail_metrics_count_only_typed_creature_and_gameobject_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);

        let _creature_guid = insert_creature_for_update(&mut manager, 4480101, true);
        let _game_object_guid = insert_game_object_for_update(&mut manager, 4480102, 0, true);
        let _dynamic_object_guid =
            insert_dynamic_object_for_update(&mut manager, 4480103, 10, true);
        let _area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4480104, 10, true);
        let _transport_guid = insert_transport_for_update(&mut manager, 4480105, true, 1);
        let _player_guid = insert_player_for_relocation_notify(
            &mut manager,
            4480106,
            Position::xyz(16.0, 26.0, 36.0),
        );
        insert_generic_world_object_record_for_metrics(
            &mut manager,
            AccessorObjectKind::Creature,
            guid(HighGuid::Creature, 4480107, 1, 0),
            TypeId::Unit,
            TypeMask::UNIT,
        );
        insert_generic_world_object_record_for_metrics(
            &mut manager,
            AccessorObjectKind::GameObject,
            guid(HighGuid::GameObject, 4480108, 1, 0),
            TypeId::GameObject,
            TypeMask::GAME_OBJECT,
        );

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        let tail = managed_map.last_map_update_tail_summary_like_cpp();
        assert_eq!(
            tail.script_hook,
            MapUpdateScriptHookSummaryLikeCpp {
                invoked: true,
                diff_ms: 1,
                map_id: 1,
                instance_id: 0,
                kind: ManagedMapKind::World,
                script_dispatch_represented: false,
            }
        );
        assert_eq!(
            tail.metrics,
            MapUpdateMetricsSummaryLikeCpp {
                creature_count: 1,
                gameobject_count: 1,
                map_id: 1,
                instance_id: 0,
            }
        );
    }

    #[test]
    fn map_manager_update_empty_script_schedule_reports_noop_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        let summary = managed_map.last_script_schedule_process_summary_like_cpp();
        assert!(summary.empty_noop);
        assert_eq!(summary.queued_before, 0);
        assert_eq!(summary.processed, 0);
        assert_eq!(summary.remaining, 0);
        assert!(!summary.lock_entered);
    }

    #[test]
    fn map_manager_update_skips_not_in_world_dynamic_object_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let dynamic_object_guid =
            insert_dynamic_object_for_update(&mut manager, 4330301, 10, false);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_dynamic_objects_update_summary(),
            DynamicObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                expired_remove_queued: 0,
                missing_or_stale: 0,
                not_dynamic_object: 0,
                not_in_world: 1,
            }
        );
        let dynamic_object = managed_map
            .map()
            .get_typed_dynamic_object(dynamic_object_guid)
            .unwrap();
        assert_eq!(dynamic_object.duration_ms(), 10);
    }

    #[test]
    fn map_manager_game_object_update_visits_live_game_object_without_expiry_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let game_object_guid = insert_game_object_for_update(&mut manager, 4380101, 0, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_game_objects_update_summary(),
            GameObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                despawn_remove_queued: 0,
                missing_or_stale: 0,
                not_game_object: 0,
                not_in_world: 0,
                linked_traps_removed: 0,
                loot_cleared: 0,
                goober_spell_casts_represented: 0,
                goober_users_cleared: 0,
                goober_state_reset: 0,
                goober_nodespawn_returns: 0,
                ..GameObjectsUpdateSummaryLikeCpp::default()
            }
        );
        let game_object = managed_map
            .map()
            .get_typed_game_object(game_object_guid)
            .unwrap();
        assert_eq!(game_object.despawn_delay(), 0);
    }

    #[test]
    fn map_manager_game_object_update_expired_despawn_then_delayed_update_drains_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let game_object_guid = insert_game_object_for_update(&mut manager, 4380201, 1, true);
        {
            let game_object = manager
                .find_map_mut(1, 0)
                .unwrap()
                .map_mut()
                .get_typed_game_object_mut(game_object_guid)
                .unwrap();
            game_object.set_loot_state(LootState::Activated, Some(ObjectGuid::create_player(1, 2)));
            assert_eq!(game_object.loot_state(), LootState::Activated);
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_game_objects_update_summary(),
            GameObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                despawn_remove_queued: 1,
                missing_or_stale: 0,
                not_game_object: 0,
                not_in_world: 0,
                linked_traps_removed: 0,
                loot_cleared: 0,
                goober_spell_casts_represented: 0,
                goober_users_cleared: 0,
                goober_state_reset: 0,
                goober_nodespawn_returns: 0,
                ..GameObjectsUpdateSummaryLikeCpp::default()
            }
        );
        assert!(
            managed_map
                .map()
                .map_object_record(game_object_guid)
                .is_none()
        );
    }

    #[test]
    fn map_manager_game_object_update_without_pool_context_keeps_remove_list_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let game_object_guid =
            insert_pool_compatible_owner_created_game_object_for_update(&mut manager, 4642101, 464);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map.last_game_objects_update_summary(),
            GameObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                despawn_remove_queued: 1,
                despawn_pool_updated: 0,
                loot_cleared: 1,
                summoned_expired_deletes: 1,
                summoned_expired_respawn_time_zeroed: 1,
                summoned_expired_despawn_represented: 1,
                summoned_expired_go_state_ready: 1,
                ..GameObjectsUpdateSummaryLikeCpp::default()
            }
        );
        // `MapManager::update` immediately follows `Map::Update` with
        // `Map::DelayedUpdate`, which drains the represented remove-list.
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert!(
            managed_map
                .map()
                .map_object_record(game_object_guid)
                .is_none()
        );
        assert!(
            !managed_map
                .map()
                .pool_data_like_cpp()
                .is_spawned_gameobject_like_cpp(4642102)
        );
    }

    #[test]
    fn map_manager_game_object_update_with_pool_context_updates_pool_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let game_object_guid =
            insert_pool_compatible_owner_created_game_object_for_update(&mut manager, 4642201, 464);
        let (spawn_store, pool_mgr) =
            gameobject_pool_update_context_for_manager_like_cpp(464, 4642201, 4642202);

        assert_eq!(
            manager.update_with_pool_update_context(1, &spawn_store, &pool_mgr),
            Some(1)
        );

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map.last_game_objects_update_summary(),
            GameObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                despawn_remove_queued: 0,
                despawn_pool_updated: 1,
                loot_cleared: 1,
                summoned_expired_deletes: 1,
                summoned_expired_respawn_time_zeroed: 1,
                summoned_expired_despawn_represented: 1,
                summoned_expired_go_state_ready: 1,
                ..GameObjectsUpdateSummaryLikeCpp::default()
            }
        );
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        // Without the loaded-grid loader from 031dx, pool-data advances and the
        // represented `Despawn1Object<GameObject>` removes the trigger, but the
        // live replacement is not materialized.
        assert!(
            managed_map
                .map()
                .map_object_record(game_object_guid)
                .is_none()
        );
        assert!(
            managed_map
                .map()
                .map_object_record(guid(HighGuid::GameObject, 4642202, 1, 0))
                .is_none()
        );
        assert!(
            managed_map
                .map()
                .pool_data_like_cpp()
                .is_spawned_gameobject_like_cpp(4642202)
        );
    }

    #[test]
    fn map_manager_game_object_update_with_pool_loaded_grid_context_adds_replacement_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let game_object_guid =
            insert_pool_compatible_owner_created_game_object_for_update(&mut manager, 4642301, 464);
        manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .ensure_grid_loaded(&crate::map::cell_from_world(0.0, 0.0));
        let (spawn_store, pool_mgr) =
            gameobject_pool_update_context_for_manager_like_cpp(464, 4642301, 4642302);

        let replacement_guid = guid(HighGuid::GameObject, 4642302, 1, 0);
        let mut loader_calls = 0usize;
        assert_eq!(
            manager.update_with_pool_update_loaded_grid_records_context(
                1,
                &spawn_store,
                &pool_mgr,
                |_, object_type, spawn_id| {
                    loader_calls += 1;
                    assert_eq!(object_type, SpawnObjectType::GameObject);
                    assert_eq!(spawn_id, 4642302);
                    let mut game_object = GameObject::new();
                    game_object
                        .world_mut()
                        .object_mut()
                        .create(replacement_guid);
                    game_object.world_mut().set_map(1, 0).unwrap();
                    game_object
                        .world_mut()
                        .relocate(Position::xyz(0.0, 0.0, 0.0));
                    game_object.world_mut().object_mut().add_to_world();
                    game_object.set_spawn_id(spawn_id);
                    Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                        MapObjectRecord::new_game_object(game_object).unwrap(),
                    ))
                },
            ),
            Some(1)
        );

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(loader_calls, 1);
        assert_eq!(
            managed_map.last_game_objects_update_summary(),
            GameObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                despawn_remove_queued: 0,
                despawn_pool_updated: 1,
                loot_cleared: 1,
                summoned_expired_deletes: 1,
                summoned_expired_respawn_time_zeroed: 1,
                summoned_expired_despawn_represented: 1,
                summoned_expired_go_state_ready: 1,
                ..GameObjectsUpdateSummaryLikeCpp::default()
            }
        );
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        // Loaded-grid replacement materialization is wired here. With the
        // spawn-id index populated before AddToMap, the represented
        // `Despawn1Object<GameObject>` removes the trigger before inserting the
        // replacement.
        assert!(
            managed_map
                .map()
                .map_object_record(game_object_guid)
                .is_none()
        );
        assert!(
            managed_map
                .map()
                .map_object_record(replacement_guid)
                .is_some()
        );
        assert_eq!(
            managed_map
                .map()
                .gameobject_spawn_id_store_count_like_cpp(4642302),
            1
        );
        assert!(
            managed_map
                .map()
                .pool_data_like_cpp()
                .is_spawned_gameobject_like_cpp(4642302)
        );
    }

    #[test]
    fn map_manager_game_object_update_skips_not_in_world_and_keeps_delay_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let game_object_guid = insert_game_object_for_update(&mut manager, 4380301, 10, false);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_game_objects_update_summary(),
            GameObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                despawn_remove_queued: 0,
                missing_or_stale: 0,
                not_game_object: 0,
                not_in_world: 1,
                linked_traps_removed: 0,
                loot_cleared: 0,
                goober_spell_casts_represented: 0,
                goober_users_cleared: 0,
                goober_state_reset: 0,
                goober_nodespawn_returns: 0,
                ..GameObjectsUpdateSummaryLikeCpp::default()
            }
        );
        let game_object = managed_map
            .map()
            .get_typed_game_object(game_object_guid)
            .unwrap();
        assert_eq!(game_object.despawn_delay(), 10);
    }

    #[test]
    fn map_manager_game_object_update_summary_ignores_non_game_objects_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let game_object_guid = insert_game_object_for_update(&mut manager, 4380401, 0, true);
        let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4380402, 10, true);
        let creature_guid = insert_creature_for_update(&mut manager, 4380403, true);
        let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4380404, 10, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map.last_game_objects_update_summary(),
            GameObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                despawn_remove_queued: 0,
                missing_or_stale: 0,
                not_game_object: 0,
                not_in_world: 0,
                linked_traps_removed: 0,
                loot_cleared: 0,
                goober_spell_casts_represented: 0,
                goober_users_cleared: 0,
                goober_state_reset: 0,
                goober_nodespawn_returns: 0,
                ..GameObjectsUpdateSummaryLikeCpp::default()
            }
        );
        assert!(
            managed_map
                .map()
                .map_object_record(game_object_guid)
                .is_some()
        );
        assert!(
            managed_map
                .map()
                .map_object_record(dynamic_object_guid)
                .is_some()
        );
        assert!(managed_map.map().map_object_record(creature_guid).is_some());
        assert!(
            managed_map
                .map()
                .map_object_record(area_trigger_guid)
                .is_some()
        );
    }

    #[test]
    fn map_manager_transport_update_visits_live_transport_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let transport_guid = insert_transport_for_update(&mut manager, 4390101, true, 1);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(
            managed_map.last_transports_update_summary(),
            TransportsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                unsupported_no_period: 0,
                missing_or_stale: 0,
                not_transport: 0,
                not_in_world: 0,
                position_updates_represented: 1,
                just_stopped: 0,
            }
        );
        let transport = managed_map
            .map()
            .map_object_record(transport_guid)
            .and_then(MapObjectRecord::transport)
            .unwrap();
        assert_eq!(transport.path_progress_ms(), 101);
    }

    #[test]
    fn map_manager_transport_update_visits_not_in_world_transport_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let transport_guid = insert_transport_for_update(&mut manager, 4390201, false, 2);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map.last_transports_update_summary(),
            TransportsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                unsupported_no_period: 0,
                missing_or_stale: 0,
                not_transport: 0,
                not_in_world: 0,
                position_updates_represented: 0,
                just_stopped: 0,
            }
        );
        let transport = managed_map
            .map()
            .map_object_record(transport_guid)
            .and_then(MapObjectRecord::transport)
            .unwrap();
        assert_eq!(transport.path_progress_ms(), 101);
    }

    #[test]
    fn map_manager_update_dynamic_object_slice_ignores_creature_and_gameobject_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4330401, 10, true);
        let creature_guid = guid(HighGuid::Creature, 4330402, 1, 0);
        let game_object_guid = guid(HighGuid::GameObject, 4330403, 1, 0);
        {
            let managed_map = manager.find_map_mut(1, 0).unwrap();
            let mut creature = Creature::new(false);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(creature_guid);
            creature.unit_mut().world_mut().set_map(1, 0).unwrap();
            creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(12.0, 22.0, 32.0));
            creature.unit_mut().world_mut().object_mut().add_to_world();
            managed_map
                .map_mut()
                .add_map_object_record_to_map_like_cpp(
                    MapObjectRecord::new_creature(creature).unwrap(),
                )
                .unwrap();

            let mut game_object = GameObject::new();
            game_object
                .world_mut()
                .object_mut()
                .create(game_object_guid);
            game_object.world_mut().set_map(1, 0).unwrap();
            game_object
                .world_mut()
                .relocate(Position::xyz(13.0, 23.0, 33.0));
            game_object.world_mut().object_mut().add_to_world();
            managed_map
                .map_mut()
                .add_map_object_record_to_map_like_cpp(
                    MapObjectRecord::new_game_object(game_object).unwrap(),
                )
                .unwrap();
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map.last_dynamic_objects_update_summary(),
            DynamicObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                expired_remove_queued: 0,
                missing_or_stale: 0,
                not_dynamic_object: 0,
                not_in_world: 0,
            }
        );
        assert!(managed_map.map().map_object_record(creature_guid).is_some());
        assert!(
            managed_map
                .map()
                .map_object_record(game_object_guid)
                .is_some()
        );
        assert_eq!(
            managed_map
                .map()
                .get_typed_dynamic_object(dynamic_object_guid)
                .unwrap()
                .duration_ms(),
            9
        );
    }

    #[test]
    fn map_manager_update_consumes_send_object_updates_before_personal_phase_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let game_object_guid = guid(HighGuid::GameObject, 4450401, 1, 0);
        {
            let managed_map = manager.find_map_mut(1, 0).unwrap();
            let mut game_object = GameObject::new();
            game_object
                .world_mut()
                .object_mut()
                .create(game_object_guid);
            game_object.world_mut().set_map(1, 0).unwrap();
            game_object
                .world_mut()
                .relocate(Position::xyz(13.0, 23.0, 33.0));
            game_object.world_mut().object_mut().add_to_world();
            game_object.world_mut().object_mut().set_scale(2.0);
            managed_map
                .map_mut()
                .add_map_object_record_to_map_like_cpp(
                    MapObjectRecord::new_game_object(game_object).unwrap(),
                )
                .unwrap();
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map.last_send_object_updates_summary_like_cpp(),
            SendObjectUpdatesSummaryLikeCpp {
                queued_before: 1,
                processed: 1,
                cleared_update_masks: 1,
                skipped_not_in_world: 0,
                missing_or_stale: 0,
                fanout_not_represented: 1,
                dynamic_object_values_updates: Vec::new(),
            }
        );
        let object = managed_map
            .map()
            .map_object(game_object_guid)
            .unwrap()
            .object();
        assert!(!object.is_object_updated());
        assert!(object.changed_fields().is_empty());
    }

    #[test]
    fn map_manager_update_visits_live_creature_with_default_context_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let creature_guid = insert_creature_for_update(&mut manager, 4350101, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(
            managed_map.last_creatures_update_summary(),
            CreatureUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                skipped_missing: 0,
                skipped_non_creature: 0,
                skipped_not_in_world: 0,
                actions_recorded: 3,
            }
        );
        assert!(
            !managed_map
                .map()
                .map_object_record(creature_guid)
                .unwrap()
                .creature()
                .unwrap()
                .trigger_just_appeared()
        );
    }

    #[test]
    fn map_manager_update_skips_not_in_world_creature_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let creature_guid = insert_creature_for_update(&mut manager, 4350201, false);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map.last_creatures_update_summary(),
            CreatureUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                skipped_missing: 0,
                skipped_non_creature: 0,
                skipped_not_in_world: 1,
                actions_recorded: 0,
            }
        );
        assert!(
            managed_map
                .map()
                .map_object_record(creature_guid)
                .unwrap()
                .creature()
                .unwrap()
                .trigger_just_appeared()
        );
    }

    #[test]
    fn map_manager_update_creature_slice_ignores_other_families_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let creature_guid = insert_creature_for_update(&mut manager, 4350301, true);
        let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4350302, 10, true);
        let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4350303, 10, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.last_creatures_update_summary().visited, 1);
        assert_eq!(managed_map.last_creatures_update_summary().updated, 1);
        assert!(
            !managed_map
                .map()
                .map_object_record(creature_guid)
                .unwrap()
                .creature()
                .unwrap()
                .trigger_just_appeared()
        );
        assert_eq!(
            managed_map
                .map()
                .get_typed_dynamic_object(dynamic_object_guid)
                .unwrap()
                .duration_ms(),
            9
        );
        assert_eq!(
            managed_map
                .map()
                .map_object_record(area_trigger_guid)
                .unwrap()
                .area_trigger()
                .unwrap()
                .duration_ms(),
            9
        );
    }

    #[test]
    fn map_manager_update_visits_live_area_trigger_without_expiry_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4340101, 10, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_area_triggers_update_summary(),
            AreaTriggersUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                expired_remove_queued: 0,
                missing_or_stale: 0,
                not_area_trigger: 0,
                not_in_world: 0,
            }
        );
        let area_trigger = managed_map
            .map()
            .map_object_record(area_trigger_guid)
            .unwrap()
            .area_trigger()
            .unwrap();
        assert_eq!(area_trigger.duration_ms(), 9);
        assert_eq!(area_trigger.time_since_created_ms(), 1);
    }

    #[test]
    fn map_manager_update_expires_area_trigger_then_delayed_update_drains_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4340201, 1, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_area_triggers_update_summary(),
            AreaTriggersUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                expired_remove_queued: 1,
                missing_or_stale: 0,
                not_area_trigger: 0,
                not_in_world: 0,
            }
        );
        assert!(
            managed_map
                .map()
                .map_object_record(area_trigger_guid)
                .is_none()
        );
    }

    #[test]
    fn map_manager_update_skips_not_in_world_area_trigger_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4340301, 10, false);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_area_triggers_update_summary(),
            AreaTriggersUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                expired_remove_queued: 0,
                missing_or_stale: 0,
                not_area_trigger: 0,
                not_in_world: 1,
            }
        );
        let area_trigger = managed_map
            .map()
            .map_object_record(area_trigger_guid)
            .unwrap()
            .area_trigger()
            .unwrap();
        assert_eq!(area_trigger.duration_ms(), 10);
        assert_eq!(area_trigger.time_since_created_ms(), 0);
    }

    #[test]
    fn map_manager_update_area_trigger_slice_ignores_other_families_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4340401, 10, true);
        let dynamic_object_guid = insert_dynamic_object_for_update(&mut manager, 4340402, 10, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map.last_area_triggers_update_summary(),
            AreaTriggersUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                expired_remove_queued: 0,
                missing_or_stale: 0,
                not_area_trigger: 0,
                not_in_world: 0,
            }
        );
        assert_eq!(
            managed_map
                .map()
                .map_object_record(area_trigger_guid)
                .unwrap()
                .area_trigger()
                .unwrap()
                .duration_ms(),
            9
        );
        assert_eq!(
            managed_map
                .map()
                .get_typed_dynamic_object(dynamic_object_guid)
                .unwrap()
                .duration_ms(),
            9
        );
    }

    #[test]
    fn map_manager_update_visits_live_conversation_without_expiry_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let conversation_guid = insert_conversation_for_update(&mut manager, 4360101, 10, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_conversations_update_summary(),
            ConversationsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                expired_remove_queued: 0,
                missing_or_stale: 0,
                not_conversation: 0,
                not_in_world: 0,
            }
        );
        let conversation = managed_map
            .map()
            .map_object_record(conversation_guid)
            .unwrap()
            .conversation()
            .unwrap();
        assert_eq!(conversation.duration_ms(), 9);
        assert!(!conversation.is_removed());
    }

    #[test]
    fn map_manager_update_expires_conversation_then_delayed_update_drains_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let conversation_guid = insert_conversation_for_update(&mut manager, 4360201, 1, true);

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_conversations_update_summary(),
            ConversationsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 0,
                expired_remove_queued: 1,
                missing_or_stale: 0,
                not_conversation: 0,
                not_in_world: 0,
            }
        );
        assert!(
            managed_map
                .map()
                .map_object_record(conversation_guid)
                .is_none()
        );
    }

    #[test]
    fn map_manager_update_visits_live_scene_object_without_removal_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let scene_object_guid = insert_scene_object_for_update(
            &mut manager,
            4370101,
            true,
            Some(guid(HighGuid::Cast, 4370102, 1, 0)),
        );

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.update_calls(), &[1]);
        assert_eq!(managed_map.delayed_update_calls(), &[1]);
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert_eq!(
            managed_map.last_scene_objects_update_summary(),
            SceneObjectsUpdateSummaryLikeCpp {
                visited: 1,
                updated: 1,
                remove_queued: 0,
                missing_or_stale: 0,
                not_scene_object: 0,
                not_in_world: 0,
            }
        );
        assert!(
            managed_map
                .map()
                .map_object_record(scene_object_guid)
                .unwrap()
                .scene_object()
                .is_some()
        );
    }

    #[test]
    fn map_manager_update_queues_scene_object_removal_then_delayed_update_drains_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let scene_object_guid =
            insert_scene_object_for_update(&mut manager, 4370201, true, ObjectGuid::EMPTY.into());
        {
            let managed_map = manager.find_map_mut(1, 0).unwrap();
            let summary =
                managed_map
                    .map_mut()
                    .update_scene_objects_like_cpp(1, |_guid, _scene| {
                        SceneObjectUpdateContextLikeCpp {
                            creator_exists: false,
                            linked_aura_exists: true,
                        }
                    });
            assert_eq!(summary.visited, 1);
            assert_eq!(summary.remove_queued, 1);
            assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 1);
            assert!(
                managed_map
                    .map()
                    .map_object_record(scene_object_guid)
                    .is_some()
            );
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(managed_map.map().objects_to_remove_count_like_cpp(), 0);
        assert!(
            managed_map
                .map()
                .map_object_record(scene_object_guid)
                .is_none()
        );
    }

    #[test]
    fn update_destroys_unloadable_maps_before_delayed_update() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_map_entry(
            33,
            7,
            1,
            ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
        );
        manager.find_map_mut(33, 7).unwrap().set_can_unload(true);
        manager.init_instance_ids(10);
        for instance_id in 1..=7 {
            manager.register_instance_id(instance_id);
        }

        assert_eq!(manager.update(1), Some(1));

        assert!(manager.find_map(33, 7).is_none());
        assert_eq!(manager.next_instance_id(), 7);
    }

    #[test]
    fn destroy_map_removes_players_then_unloads_and_removes_entry() {
        let mut manager = MapManager::default();
        manager.create_world_map(1, 0).set_player_count(2);

        assert!(manager.destroy_map(1, 0));
        assert!(manager.find_map(1, 0).is_none());
    }

    #[test]
    fn num_instances_and_players_match_dungeon_filter() {
        let mut manager = MapManager::default();
        manager.create_world_map(1, 0).set_player_count(10);
        manager
            .create_map_entry(
                33,
                7,
                1,
                ManagedMapKind::Dungeon {
                    has_reset_schedule: true,
                },
            )
            .set_player_count(3);

        assert_eq!(manager.num_instances(), 1);
        assert_eq!(manager.num_players_in_instances(), 3);
    }

    #[test]
    fn instance_id_allocator_reuses_lowest_freed_id() {
        let mut allocator = InstanceIdAllocator::new();
        allocator.init_instance_ids(3);
        allocator.register_instance_id(1);
        allocator.register_instance_id(2);

        assert_eq!(allocator.generate_instance_id(), Some(3));
        allocator.free_instance_id(2);
        assert_eq!(allocator.generate_instance_id(), Some(2));
    }

    #[test]
    fn scheduled_script_counter_saturates_on_decrease() {
        let mut manager = MapManager::default();

        manager.increase_scheduled_scripts_count();
        assert!(manager.is_script_scheduled());
        manager.decrease_scheduled_script_count_by(2);
        assert!(!manager.is_script_scheduled());
    }

    #[test]
    fn activated_map_updater_uses_schedule_and_wait_path() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        manager.map_updater_mut().activate(2);

        assert_eq!(manager.update(1), Some(1));

        let map = manager.find_map(1, 0).unwrap();
        assert_eq!(map.update_calls(), &[1]);
        assert_eq!(manager.map_updater().scheduled_updates(), 1);
        assert_eq!(manager.map_updater().wait_calls(), 1);
        assert!(manager.map_updater().activated());

        manager.map_updater_mut().deactivate();
        assert!(!manager.map_updater().activated());
    }

    #[test]
    fn map_manager_update_drains_live_creature_gameobject_area_trigger_move_lists_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let creature_guid = insert_creature_for_update(&mut manager, 4420101, true);
        let game_object_guid = insert_game_object_for_update(&mut manager, 4420102, 0, true);
        let area_trigger_guid = insert_area_trigger_for_update(&mut manager, 4420103, 100, true);
        let creature_position = Position::xyz(10.5, 20.5, 30.5);
        let game_object_position = Position::xyz(13.5, 23.5, 33.5);
        let area_trigger_position = Position::xyz(12.5, 22.5, 32.5);

        {
            let map = manager.find_map_mut(1, 0).unwrap().map_mut();
            assert_eq!(
                map.add_creature_to_move_list_like_cpp(creature_guid, creature_position),
                crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
            );
            assert_eq!(
                map.add_game_object_to_move_list_like_cpp(game_object_guid, game_object_position),
                crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
            );
            assert_eq!(
                map.add_area_trigger_to_move_list_like_cpp(
                    area_trigger_guid,
                    area_trigger_position
                ),
                crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
            );
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        let map = managed_map.map();
        assert_eq!(
            map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
            0
        );
        assert_eq!(
            map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
            0
        );
        assert_eq!(
            map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
            0
        );
        assert_eq!(
            map.map_object(creature_guid).unwrap().position(),
            creature_position
        );
        assert_eq!(
            map.map_object(game_object_guid).unwrap().position(),
            game_object_position
        );
        assert_eq!(
            map.map_object(area_trigger_guid).unwrap().position(),
            area_trigger_position
        );
        assert_eq!(
            managed_map.last_live_move_list_drain_summary_like_cpp(),
            LiveMoveListDrainSummaryLikeCpp {
                creature: MoveListDrainSummaryLikeCpp {
                    family: Some(MapObjectMoveListFamilyLikeCpp::Creature),
                    processed: 1,
                    relocated: 1,
                    ..Default::default()
                },
                game_object: MoveListDrainSummaryLikeCpp {
                    family: Some(MapObjectMoveListFamilyLikeCpp::GameObject),
                    processed: 1,
                    relocated: 1,
                    ..Default::default()
                },
                area_trigger: MoveListDrainSummaryLikeCpp {
                    family: Some(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
                    processed: 1,
                    relocated: 1,
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn map_manager_update_leaves_dynamic_object_move_list_queued_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let dynamic_object_guid =
            insert_dynamic_object_for_update(&mut manager, 4420201, 100, true);
        let original_position = manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .map_object(dynamic_object_guid)
            .unwrap()
            .position();
        let queued_position = Position::xyz(11.5, 21.5, 31.5);

        {
            let map = manager.find_map_mut(1, 0).unwrap().map_mut();
            assert_eq!(
                map.add_dynamic_object_to_move_list_like_cpp(dynamic_object_guid, queued_position),
                crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
            );
        }

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();
        let map = managed_map.map();
        assert_eq!(
            map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::DynamicObject),
            1
        );
        assert_eq!(
            map.map_object(dynamic_object_guid).unwrap().position(),
            original_position
        );
        assert_eq!(
            managed_map.last_live_move_list_drain_summary_like_cpp(),
            LiveMoveListDrainSummaryLikeCpp {
                creature: MoveListDrainSummaryLikeCpp {
                    family: Some(MapObjectMoveListFamilyLikeCpp::Creature),
                    ..Default::default()
                },
                game_object: MoveListDrainSummaryLikeCpp {
                    family: Some(MapObjectMoveListFamilyLikeCpp::GameObject),
                    ..Default::default()
                },
                area_trigger: MoveListDrainSummaryLikeCpp {
                    family: Some(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn map_manager_update_processes_live_relocation_notifies_for_player_source_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let player_guid = insert_player_for_relocation_notify(
            &mut manager,
            4430101,
            Position::xyz(10.0, 20.0, 30.0),
        );
        let creature_guid = insert_creature_at_for_relocation_notify(
            &mut manager,
            4430102,
            Position::xyz(10.5, 20.5, 30.5),
            true,
        );
        let player_normal_guid = insert_player_for_relocation_notify(
            &mut manager,
            4440103,
            Position::xyz(11.0, 21.0, 31.0),
        );
        let other_creature_guid = insert_creature_at_for_relocation_notify(
            &mut manager,
            4440104,
            Position::xyz(11.5, 21.5, 31.5),
            false,
        );

        {
            let map = manager.find_map_mut(1, 0).unwrap().map_mut();
            map.get_typed_player_mut(player_guid)
                .unwrap()
                .unit_mut()
                .world_mut()
                .object_mut()
                .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
            map.get_typed_creature_mut(creature_guid)
                .unwrap()
                .unit_mut()
                .world_mut()
                .object_mut()
                .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
            assert!(
                map.map_object(player_guid)
                    .unwrap()
                    .object()
                    .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
            );
            assert!(test_object_needs_notify_visibility(map, creature_guid));
        }

        assert_eq!(
            manager.update(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32),
            Some(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32)
        );

        let managed_map = manager.find_map(1, 0).unwrap();
        let outcome = managed_map.last_process_relocation_notifies_outcome_like_cpp();
        assert_eq!(
            outcome.process_plan.diff_ms,
            DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32
        );
        assert!(!outcome.process_plan.delayed_relocation_cells.is_empty());
        assert!(!outcome.process_plan.reset_timer_grids.is_empty());
        assert!(
            outcome
                .delayed_plan
                .cell_plans
                .iter()
                .any(|cell| cell.plan.player_relocations.contains(&player_guid))
        );
        assert!(
            outcome
                .delayed_plan
                .cell_plans
                .iter()
                .any(|cell| cell.plan.creature_relocations.contains(&creature_guid))
        );
        let creature_visibility_plan = outcome
            .visibility_plans
            .creature_plans
            .iter()
            .find(|plan| plan.creature_guid == creature_guid)
            .expect("live DelayedUnitRelocation creature visibility plan");
        assert!(
            creature_visibility_plan
                .visibility_plan
                .player_visibility_updates
                .contains(&player_normal_guid)
        );
        assert!(
            creature_visibility_plan
                .visibility_plan
                .ai_relocation_checks
                .contains(&(creature_guid, player_guid))
        );
        assert!(
            creature_visibility_plan
                .visibility_plan
                .ai_relocation_checks
                .contains(&(creature_guid, other_creature_guid))
        );
        assert!(
            creature_visibility_plan
                .visibility_plan
                .ai_relocation_checks
                .contains(&(other_creature_guid, creature_guid))
        );
        assert!(
            outcome
                .visibility_plans
                .player_plans
                .iter()
                .any(|plan| plan.player_guid == player_guid && plan.viewpoint_guid == player_guid)
        );
        assert!(
            outcome
                .reset_outcome
                .reset_player_guids
                .contains(&player_guid)
        );
        assert!(
            outcome
                .reset_outcome
                .reset_creature_guids
                .contains(&creature_guid)
        );
        assert!(!test_object_needs_notify_visibility(
            managed_map.map(),
            player_guid
        ));
        assert!(!test_object_needs_notify_visibility(
            managed_map.map(),
            creature_guid
        ));
    }

    #[test]
    fn map_manager_update_skips_process_relocation_notifies_without_sources_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let creature_guid = insert_creature_at_for_relocation_notify(
            &mut manager,
            4430201,
            Position::xyz(10.5, 20.5, 30.5),
            false,
        );

        {
            let map = manager.find_map_mut(1, 0).unwrap().map_mut();
            map.get_typed_creature_mut(creature_guid)
                .unwrap()
                .unit_mut()
                .world_mut()
                .object_mut()
                .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        }

        assert_eq!(
            manager.update(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32),
            Some(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32)
        );

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map.last_process_relocation_notifies_outcome_like_cpp(),
            ProcessRelocationNotifiesOutcome::default()
        );
        assert!(test_object_needs_notify_visibility(
            managed_map.map(),
            creature_guid
        ));
    }

    #[test]
    fn map_manager_update_process_relocation_notifies_uses_post_drain_position_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let moving_active_guid = insert_creature_at_for_relocation_notify(
            &mut manager,
            4430301,
            Position::xyz(10.0, 20.0, 30.0),
            true,
        );
        let notify_guid = insert_creature_at_for_relocation_notify(
            &mut manager,
            4430302,
            Position::xyz(1500.0, 1500.0, 30.0),
            false,
        );
        let post_drain_position = Position::xyz(1500.5, 1500.5, 30.5);

        {
            let map = manager.find_map_mut(1, 0).unwrap().map_mut();
            map.get_typed_creature_mut(notify_guid)
                .unwrap()
                .unit_mut()
                .world_mut()
                .object_mut()
                .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
            assert_eq!(
                map.add_creature_to_move_list_like_cpp(moving_active_guid, post_drain_position),
                crate::map::AddObjectToMoveListOutcomeLikeCpp::Queued
            );
        }

        assert_eq!(
            manager.update(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32),
            Some(DEFAULT_VISIBILITY_NOTIFY_PERIOD as u32)
        );

        let managed_map = manager.find_map(1, 0).unwrap();
        assert_eq!(
            managed_map
                .map()
                .map_object(moving_active_guid)
                .unwrap()
                .position(),
            post_drain_position
        );
        assert_eq!(
            managed_map
                .last_live_move_list_drain_summary_like_cpp()
                .creature
                .relocated,
            1
        );
        let outcome = managed_map.last_process_relocation_notifies_outcome_like_cpp();
        assert!(
            outcome
                .reset_outcome
                .reset_creature_guids
                .contains(&notify_guid)
        );
        assert!(!test_object_needs_notify_visibility(
            managed_map.map(),
            notify_guid
        ));
    }

    fn guid(high: HighGuid, counter: i64, map_id: u32, instance_id: u32) -> ObjectGuid {
        if high == HighGuid::Player {
            ObjectGuid::create_global(high, 0, counter)
        } else if high == HighGuid::Transport {
            ObjectGuid::create_transport(high, counter)
        } else {
            ObjectGuid::create_world_object(high, 0, 1, map_id as u16, instance_id, 100, counter)
        }
    }

    fn test_object_needs_notify_visibility(
        map: &Map<NoopTerrainGridLoader, NoopGridLifecycle>,
        guid: ObjectGuid,
    ) -> bool {
        map.map_object(guid).is_some_and(|object| {
            object
                .object()
                .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
        })
    }

    fn insert_player_for_relocation_notify(
        manager: &mut MapManager,
        counter: i64,
        position: Position,
    ) -> ObjectGuid {
        let player_guid = guid(HighGuid::Player, counter, 1, 0);
        let mut player = Player::new(None, false);
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(player_guid);
        player.unit_mut().world_mut().set_map(1, 0).unwrap();
        player.unit_mut().world_mut().relocate(position);
        let record = MapObjectRecord::new_player(player).unwrap();
        manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .add_map_object_record_to_map_like_cpp(record)
            .unwrap();
        player_guid
    }

    fn insert_creature_at_for_relocation_notify(
        manager: &mut MapManager,
        counter: i64,
        position: Position,
        active: bool,
    ) -> ObjectGuid {
        let creature_guid = guid(HighGuid::Creature, counter, 1, 0);
        let mut creature = Creature::new(false);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(creature_guid);
        creature.unit_mut().world_mut().set_map(1, 0).unwrap();
        creature.unit_mut().world_mut().relocate(position);
        creature.unit_mut().world_mut().set_active(active);
        creature.unit_mut().set_death_state(DeathState::Alive);
        creature.unit_mut().set_max_health(100);
        creature.unit_mut().set_health(100);
        let record = MapObjectRecord::new_creature(creature).unwrap();
        manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .add_map_object_record_to_map_like_cpp(record)
            .unwrap();
        creature_guid
    }

    fn insert_creature_for_update(
        manager: &mut MapManager,
        counter: i64,
        in_world: bool,
    ) -> ObjectGuid {
        let creature_guid = guid(HighGuid::Creature, counter, 1, 0);
        let mut creature = Creature::new(false);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(creature_guid);
        creature.unit_mut().world_mut().set_map(1, 0).unwrap();
        creature
            .unit_mut()
            .world_mut()
            .relocate(Position::xyz(10.0, 20.0, 30.0));
        if in_world {
            creature.unit_mut().world_mut().object_mut().add_to_world();
        }
        let record = MapObjectRecord::new_creature(creature).unwrap();
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        if in_world {
            map.add_map_object_record_to_map_like_cpp(record).unwrap();
        } else {
            map.insert_map_object_record(record).unwrap();
        }
        creature_guid
    }

    fn insert_dynamic_object_for_update(
        manager: &mut MapManager,
        counter: i64,
        duration_ms: i32,
        in_world: bool,
    ) -> ObjectGuid {
        let dynamic_object_guid = guid(HighGuid::DynamicObject, counter, 1, 0);
        let mut dynamic_object = DynamicObject::new(true);
        dynamic_object
            .world_mut()
            .object_mut()
            .create(dynamic_object_guid);
        dynamic_object.world_mut().set_map(1, 0).unwrap();
        dynamic_object
            .world_mut()
            .relocate(Position::xyz(11.0, 21.0, 31.0));
        if in_world {
            dynamic_object.world_mut().object_mut().add_to_world();
        }
        dynamic_object.set_duration(duration_ms);
        let record = MapObjectRecord::new_dynamic_object(dynamic_object).unwrap();
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        if in_world {
            map.add_map_object_record_to_map_like_cpp(record).unwrap();
        } else {
            map.insert_map_object_record(record).unwrap();
        }
        dynamic_object_guid
    }

    fn insert_pool_compatible_owner_created_game_object_for_update(
        manager: &mut MapManager,
        spawn_id: u64,
        pool_id: u32,
    ) -> ObjectGuid {
        let game_object_guid = guid(HighGuid::GameObject, spawn_id as i64, 1, 0);
        let mut game_object = GameObject::new();
        game_object
            .world_mut()
            .object_mut()
            .create(game_object_guid);
        game_object.world_mut().set_map(1, 0).unwrap();
        game_object
            .world_mut()
            .relocate(Position::xyz(13.0, 23.0, 33.0));
        game_object.world_mut().object_mut().add_to_world();
        game_object.set_go_type(GAMEOBJECT_TYPE_GENERIC as u8);
        game_object.world_mut().object_mut().set_entry(190_011);
        game_object.set_spawn_id(spawn_id);
        game_object.set_represented_gameobject_data_present_like_cpp(true);
        game_object.set_respawn_compatibility_mode(true);
        game_object.set_created_by(guid(HighGuid::Player, spawn_id as i64 + 99, 1, 0));
        game_object.set_respawn_time(0);
        game_object.set_loot_state(LootState::JustDeactivated, None);

        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        map.add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(game_object).unwrap(),
        )
        .unwrap();
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::GameObject, spawn_id, pool_id)
            .expect("spawned pool state");
        game_object_guid
    }

    fn gameobject_pool_update_context_for_manager_like_cpp(
        pool_id: u32,
        despawned_spawn_id: u64,
        replacement_spawn_id: u64,
    ) -> (SpawnStore, PoolMgrLikeCpp) {
        let mut store = SpawnStore::new();
        let active = spawn_group_for_manager_like_cpp(pool_id + 100, SpawnGroupFlags::NONE);
        let mut despawned_spawn = spawn_data_for_manager_like_cpp(
            SpawnObjectType::GameObject,
            despawned_spawn_id,
            active.clone(),
        );
        despawned_spawn.pool_id = pool_id;
        let mut replacement_spawn = spawn_data_for_manager_like_cpp(
            SpawnObjectType::GameObject,
            replacement_spawn_id,
            active,
        );
        replacement_spawn.pool_id = pool_id;
        store.add_object_spawn(&despawned_spawn, |_| false);
        store.add_object_spawn(&replacement_spawn, |_| false);

        let mut pool_mgr = PoolMgrLikeCpp::new();
        pool_mgr.insert_template_like_cpp(pool_id, PoolTemplateDataLikeCpp::new(1, 1));
        let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, pool_id);
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(replacement_spawn_id, 0.0), 1);
        group.add_entry_like_cpp(PoolObjectLikeCpp::new(despawned_spawn_id, 0.0), 1);
        pool_mgr
            .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, pool_id, group)
            .expect("test pool group");
        pool_mgr
            .register_spawn_pool_relation_like_cpp(
                PoolMemberKindLikeCpp::GameObject,
                despawned_spawn_id,
                pool_id,
            )
            .expect("test pool relation");

        (store, pool_mgr)
    }

    fn spawn_group_for_manager_like_cpp(
        group_id: u32,
        flags: SpawnGroupFlags,
    ) -> SpawnGroupTemplateData {
        SpawnGroupTemplateData {
            group_id,
            name: format!("manager-group-{group_id}"),
            map_id: 1,
            flags,
        }
    }

    fn spawn_data_for_manager_like_cpp(
        object_type: SpawnObjectType,
        spawn_id: u64,
        spawn_group: SpawnGroupTemplateData,
    ) -> SpawnData {
        SpawnData {
            object_type,
            spawn_id,
            map_id: 1,
            db_data: true,
            spawn_group,
            id: 99,
            spawn_point: crate::spawn::SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
            phase_use_flags: 0,
            phase_id: 0,
            phase_group: 0,
            terrain_swap_map: 0,
            pool_id: 0,
            spawn_time_secs: 0,
            spawn_difficulties: vec![1],
            script_id: 0,
            string_id: String::new(),
        }
    }

    fn insert_game_object_for_update(
        manager: &mut MapManager,
        counter: i64,
        despawn_delay_ms: u32,
        in_world: bool,
    ) -> ObjectGuid {
        let game_object_guid = guid(HighGuid::GameObject, counter, 1, 0);
        let mut game_object = GameObject::new();
        game_object
            .world_mut()
            .object_mut()
            .create(game_object_guid);
        game_object.world_mut().set_map(1, 0).unwrap();
        game_object
            .world_mut()
            .relocate(Position::xyz(13.0, 23.0, 33.0));
        if in_world {
            game_object.world_mut().object_mut().add_to_world();
        }
        if despawn_delay_ms != 0 {
            assert!(game_object.schedule_despawn_or_unsummon_like_cpp(despawn_delay_ms, 77));
        }
        let record = MapObjectRecord::new_game_object(game_object).unwrap();
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        if in_world {
            map.add_map_object_record_to_map_like_cpp(record).unwrap();
        } else {
            map.insert_map_object_record(record).unwrap();
        }
        game_object_guid
    }

    fn insert_generic_world_object_record_for_metrics(
        manager: &mut MapManager,
        kind: AccessorObjectKind,
        guid: ObjectGuid,
        type_id: TypeId,
        type_mask: TypeMask,
    ) {
        let mut object = WorldObject::new(true, type_id, type_mask);
        object.object_mut().create(guid);
        object.set_map(1, 0).unwrap();
        object.relocate(Position::xyz(17.0, 27.0, 37.0));
        object.object_mut().add_to_world();
        let record = MapObjectRecord::new(kind, object).unwrap();
        manager
            .find_map_mut(1, 0)
            .unwrap()
            .map_mut()
            .add_map_object_record_to_map_like_cpp(record)
            .unwrap();
    }

    fn insert_transport_for_update(
        manager: &mut MapManager,
        counter: i64,
        in_world: bool,
        expected_map_id: u32,
    ) -> ObjectGuid {
        let transport_guid = guid(HighGuid::Transport, counter, 1, 0);
        let template = TransportTemplate {
            total_path_time_ms: 1_000,
            path_legs: vec![TransportPathLeg {
                map_id: expected_map_id,
                start_timestamp_ms: 0,
                duration_ms: 1_000,
                segments: vec![],
            }],
            ..TransportTemplate::default()
        };
        let mut transport = Transport::with_template(template);
        transport.world_mut().object_mut().create(transport_guid);
        transport.world_mut().set_map(1, 0).unwrap();
        transport
            .world_mut()
            .relocate(Position::xyz(15.0, 25.0, 35.0));
        transport.set_path_progress_ms(100);
        if in_world {
            transport.world_mut().object_mut().add_to_world();
        }
        let record = MapObjectRecord::new_transport(transport).unwrap();
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        if in_world {
            map.add_map_object_record_to_map_like_cpp(record).unwrap();
        } else {
            map.insert_map_object_record(record).unwrap();
        }
        transport_guid
    }

    fn insert_area_trigger_for_update(
        manager: &mut MapManager,
        counter: i64,
        duration_ms: i32,
        in_world: bool,
    ) -> ObjectGuid {
        let area_trigger_guid = guid(HighGuid::AreaTrigger, counter, 1, 0);
        let mut area_trigger = AreaTrigger::new();
        area_trigger
            .world_mut()
            .object_mut()
            .create(area_trigger_guid);
        area_trigger.world_mut().set_map(1, 0).unwrap();
        area_trigger
            .world_mut()
            .relocate(Position::xyz(12.0, 22.0, 32.0));
        if in_world {
            area_trigger.world_mut().object_mut().add_to_world();
        }
        area_trigger.set_duration(duration_ms);
        let record = MapObjectRecord::new_area_trigger(area_trigger).unwrap();
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        if in_world {
            map.add_map_object_record_to_map_like_cpp(record).unwrap();
        } else {
            map.insert_map_object_record(record).unwrap();
        }
        area_trigger_guid
    }

    fn insert_conversation_for_update(
        manager: &mut MapManager,
        counter: i64,
        duration_ms: i32,
        in_world: bool,
    ) -> ObjectGuid {
        let conversation_guid = guid(HighGuid::Conversation, counter, 1, 0);
        let mut conversation = Conversation::new();
        conversation
            .world_mut()
            .object_mut()
            .create(conversation_guid);
        conversation.world_mut().set_map(1, 0).unwrap();
        conversation
            .world_mut()
            .relocate(Position::xyz(13.0, 23.0, 33.0));
        if in_world {
            conversation.world_mut().object_mut().add_to_world();
        }
        conversation.set_duration_ms(duration_ms);
        let record = MapObjectRecord::new_conversation(conversation).unwrap();
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        if in_world {
            map.add_map_object_record_to_map_like_cpp(record).unwrap();
        } else {
            map.insert_map_object_record(record).unwrap();
        }
        conversation_guid
    }

    fn insert_scene_object_for_update(
        manager: &mut MapManager,
        counter: i64,
        in_world: bool,
        created_by_spell_cast: Option<ObjectGuid>,
    ) -> ObjectGuid {
        let scene_object_guid = guid(HighGuid::SceneObject, counter, 1, 0);
        let mut scene_object = SceneObject::new();
        scene_object
            .world_mut()
            .object_mut()
            .create(scene_object_guid);
        scene_object.world_mut().set_map(1, 0).unwrap();
        scene_object
            .world_mut()
            .relocate(Position::xyz(14.0, 24.0, 34.0));
        scene_object.set_created_by(guid(HighGuid::Player, counter + 1000, 1, 0));
        if let Some(cast_guid) = created_by_spell_cast {
            scene_object.set_created_by_spell_cast(cast_guid);
        }
        if in_world {
            scene_object.world_mut().object_mut().add_to_world();
        }
        let record = MapObjectRecord::new_scene_object(scene_object).unwrap();
        let map = manager.find_map_mut(1, 0).unwrap().map_mut();
        if in_world {
            map.add_map_object_record_to_map_like_cpp(record).unwrap();
        } else {
            map.insert_map_object_record(record).unwrap();
        }
        scene_object_guid
    }

    fn world_entry(map_id: u32) -> CreateMapEntryContext {
        CreateMapEntryContext {
            map_id,
            kind: CreateMapEntryKind::World,
            split_by_faction: false,
            flex_locking: false,
        }
    }

    fn dungeon_entry(map_id: u32, flex_locking: bool) -> CreateMapEntryContext {
        CreateMapEntryContext {
            map_id,
            kind: CreateMapEntryKind::Dungeon,
            split_by_faction: false,
            flex_locking,
        }
    }

    fn player() -> CreateMapPlayerContext {
        CreateMapPlayerContext {
            guid_counter: 77,
            team_id: 469,
            battleground_id: 0,
            has_battleground: false,
            player_difficulty_id: 1,
            player_recent_instance_id: 0,
            group: None,
        }
    }

    fn difficulty(
        difficulty_id: Difficulty,
        has_reset_schedule: bool,
        is_instance_id_bound: bool,
    ) -> CreateMapDifficultyContext {
        CreateMapDifficultyContext {
            difficulty_id,
            has_reset_schedule,
            is_instance_id_bound,
        }
    }

    #[test]
    fn create_map_decision_rejects_missing_player_or_map_entry_like_cpp() {
        let mut manager = MapManager::default();

        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(world_entry(1)),
                None,
                |_, _| None,
                None,
                |_, _| None,
            ),
            CreateMapDecision::Reject {
                side_effects: Vec::new(),
            }
        );
        assert_eq!(
            manager.create_map_decision_like_cpp(
                None,
                Some(player()),
                |_, _| None,
                None,
                |_, _| None
            ),
            CreateMapDecision::Reject {
                side_effects: Vec::new(),
            }
        );
    }

    #[test]
    fn create_map_decision_world_uses_zero_or_team_instance_like_cpp() {
        let mut manager = MapManager::default();
        let mut split_entry = world_entry(530);
        split_entry.split_by_faction = true;

        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(world_entry(0)),
                Some(player()),
                |_, _| None,
                None,
                |_, _| None,
            ),
            CreateMapDecision::Create {
                key: MapKey::new(0, 0),
                difficulty_id: 0,
                kind: ManagedMapKind::World,
                side_effects: Vec::new(),
            }
        );
        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(split_entry),
                Some(player()),
                |_, _| None,
                None,
                |_, _| None,
            ),
            CreateMapDecision::Create {
                key: MapKey::new(530, 469),
                difficulty_id: 0,
                kind: ManagedMapKind::World,
                side_effects: Vec::new(),
            }
        );
    }

    #[test]
    fn create_map_decision_battleground_requires_instance_and_bg_pointer_like_cpp() {
        let mut manager = MapManager::default();
        let entry = CreateMapEntryContext {
            map_id: 489,
            kind: CreateMapEntryKind::BattlegroundOrArena,
            split_by_faction: false,
            flex_locking: false,
        };

        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(entry),
                Some(player()),
                |_, _| None,
                None,
                |_, _| None,
            ),
            CreateMapDecision::Reject {
                side_effects: Vec::new(),
            }
        );

        let mut bg_player = player();
        bg_player.battleground_id = 12;
        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(entry),
                Some(bg_player),
                |_, _| None,
                None,
                |_, _| None,
            ),
            CreateMapDecision::Reject {
                side_effects: vec![CreateMapSideEffect::TeleportToBattlegroundEntryPoint],
            }
        );

        bg_player.has_battleground = true;
        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(entry),
                Some(bg_player),
                |_, _| None,
                None,
                |_, _| None,
            ),
            CreateMapDecision::Create {
                key: MapKey::new(489, 12),
                difficulty_id: 0,
                kind: ManagedMapKind::Battleground,
                side_effects: Vec::new(),
            }
        );
    }

    #[test]
    fn create_map_decision_dungeon_uses_active_lock_and_resets_difficulty_like_cpp() {
        let mut manager = MapManager::default();
        manager
            .create_map_entry(
                33,
                42,
                2,
                ManagedMapKind::Dungeon {
                    has_reset_schedule: true,
                },
            )
            .set_instance_lock_token(Some(9));

        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(dungeon_entry(33, false)),
                Some(player()),
                |_, requested| Some(difficulty(requested, true, true)),
                Some(CreateMapInstanceLockContext {
                    instance_id: 42,
                    difficulty_id: 2,
                    token: 9,
                }),
                |_, _| None,
            ),
            CreateMapDecision::Existing {
                key: MapKey::new(33, 42),
                difficulty_id: 2,
                side_effects: Vec::new(),
            }
        );
    }

    #[test]
    fn create_map_decision_normal_dungeon_reuses_recent_instance_like_cpp() {
        let mut manager = MapManager::default();
        let mut player = player();
        player.player_recent_instance_id = 7;

        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(dungeon_entry(33, false)),
                Some(player),
                |_, requested| Some(difficulty(requested, false, true)),
                None,
                |_, _| None,
            ),
            CreateMapDecision::Create {
                key: MapKey::new(33, 7),
                difficulty_id: 1,
                kind: ManagedMapKind::Dungeon {
                    has_reset_schedule: false,
                },
                side_effects: vec![CreateMapSideEffect::SetPlayerRecentInstance { instance_id: 7 }],
            }
        );
    }

    #[test]
    fn create_map_decision_dungeon_generates_instance_and_lock_side_effect_like_cpp() {
        let mut manager = MapManager::default();
        manager.init_instance_ids(3);
        manager.register_instance_id(1);

        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(dungeon_entry(631, false)),
                Some(player()),
                |_, requested| Some(difficulty(requested, true, true)),
                None,
                |_, _| None,
            ),
            CreateMapDecision::Create {
                key: MapKey::new(631, 2),
                difficulty_id: 1,
                kind: ManagedMapKind::Dungeon {
                    has_reset_schedule: true,
                },
                side_effects: vec![
                    CreateMapSideEffect::CreateInstanceLockForNewInstance {
                        owner_guid_counter: 77,
                        instance_id: 2,
                    },
                    CreateMapSideEffect::SetPlayerRecentInstance { instance_id: 2 },
                ],
            }
        );
    }

    #[test]
    fn create_map_decision_flex_lock_conflict_regenerates_instance_like_cpp() {
        let mut manager = MapManager::default();
        manager.init_instance_ids(50);
        for instance_id in 1..=42 {
            manager.register_instance_id(instance_id);
        }
        manager
            .create_map_entry(
                631,
                42,
                3,
                ManagedMapKind::Dungeon {
                    has_reset_schedule: true,
                },
            )
            .set_instance_lock_token(Some(100));

        assert_eq!(
            manager.create_map_decision_like_cpp(
                Some(dungeon_entry(631, true)),
                Some(player()),
                |_, requested| Some(difficulty(requested, true, false)),
                Some(CreateMapInstanceLockContext {
                    instance_id: 42,
                    difficulty_id: 3,
                    token: 200,
                }),
                |_, _| None,
            ),
            CreateMapDecision::Create {
                key: MapKey::new(631, 43),
                difficulty_id: 1,
                kind: ManagedMapKind::Dungeon {
                    has_reset_schedule: true,
                },
                side_effects: vec![
                    CreateMapSideEffect::SetInstanceLockInstanceId { instance_id: 43 },
                    CreateMapSideEffect::SetPlayerRecentInstance { instance_id: 43 },
                ],
            }
        );
    }

    #[test]
    fn find_instance_id_for_player_matches_cpp_world_bg_and_garrison_branches() {
        let manager = MapManager::default();
        let mut split = world_entry(609);
        split.split_by_faction = true;
        let bg = CreateMapEntryContext {
            map_id: 489,
            kind: CreateMapEntryKind::BattlegroundOrArena,
            split_by_faction: false,
            flex_locking: false,
        };
        let garrison = CreateMapEntryContext {
            map_id: 1152,
            kind: CreateMapEntryKind::Garrison,
            split_by_faction: false,
            flex_locking: false,
        };
        let mut player = player();
        player.team_id = 1;
        player.battleground_id = 12;

        assert_eq!(
            manager.find_instance_id_for_player_like_cpp(
                Some(world_entry(0)),
                Some(player),
                |_, _| None,
                None,
                |_, _| None,
            ),
            0
        );
        assert_eq!(
            manager.find_instance_id_for_player_like_cpp(
                Some(split),
                Some(player),
                |_, _| None,
                None,
                |_, _| None,
            ),
            1
        );
        assert_eq!(
            manager.find_instance_id_for_player_like_cpp(
                Some(bg),
                Some(player),
                |_, _| None,
                None,
                |_, _| None,
            ),
            12
        );
        assert_eq!(
            manager.find_instance_id_for_player_like_cpp(
                Some(garrison),
                Some(player),
                |_, _| None,
                None,
                |_, _| None,
            ),
            77
        );
    }

    #[test]
    fn find_instance_id_for_player_matches_cpp_dungeon_lock_and_recent_rules() {
        let mut manager = MapManager::default();
        manager
            .create_map_entry(
                631,
                42,
                3,
                ManagedMapKind::Dungeon {
                    has_reset_schedule: true,
                },
            )
            .set_instance_lock_token(Some(100));
        let mut player = player();
        player.player_recent_instance_id = 7;

        assert_eq!(
            manager.find_instance_id_for_player_like_cpp(
                Some(dungeon_entry(33, false)),
                Some(player),
                |_, requested| Some(difficulty(requested, false, true)),
                None,
                |_, _| None,
            ),
            7
        );
        assert_eq!(
            manager.find_instance_id_for_player_like_cpp(
                Some(dungeon_entry(631, true)),
                Some(player),
                |_, requested| Some(difficulty(requested, true, false)),
                Some(CreateMapInstanceLockContext {
                    instance_id: 42,
                    difficulty_id: 3,
                    token: 200,
                }),
                |_, _| None,
            ),
            0
        );
        assert_eq!(
            manager.find_instance_id_for_player_like_cpp(
                Some(dungeon_entry(631, true)),
                Some(player),
                |_, requested| Some(difficulty(requested, true, false)),
                Some(CreateMapInstanceLockContext {
                    instance_id: 42,
                    difficulty_id: 3,
                    token: 100,
                }),
                |_, _| None,
            ),
            42
        );
    }

    // Anchor: canonical motor B (map.rs:6286-6295) calls runtime_update_plan and
    // stores actions_recorded but has NO match that dispatches AiUpdateTick/
    // MeleeAttackIfReady — no real AI/combat/threat/fanout. Characterizes the split
    // vs. legacy motor A (WorldSession::tick_creatures_sync / tick_combat_sync).
    #[test]
    fn canonical_map_update_visits_creature_with_no_real_ai_combat_effect_like_cpp() {
        let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 1);
        manager.create_world_map(1, 0);
        let creature_guid = insert_creature_for_update(&mut manager, 9990001, true);

        let position_before = manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .map_object(creature_guid)
            .unwrap()
            .position();
        let health_before = manager
            .find_map(1, 0)
            .unwrap()
            .map()
            .map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .unit()
            .data()
            .health;

        assert_eq!(manager.update(1), Some(1));

        let managed_map = manager.find_map(1, 0).unwrap();

        let summary = managed_map.last_creatures_update_summary();
        assert!(
            summary.visited >= 1,
            "canonical tick must visit the creature"
        );
        assert!(
            summary.actions_recorded > 0,
            "canonical tick must record plan actions (runtime_update_plan was called)"
        );

        let position_after = managed_map
            .map()
            .map_object(creature_guid)
            .unwrap()
            .position();
        let health_after = managed_map
            .map()
            .map_object_record(creature_guid)
            .unwrap()
            .creature()
            .unwrap()
            .unit()
            .data()
            .health;

        assert_eq!(
            position_after, position_before,
            "canonical tick must not change creature position — no real AI/combat executed"
        );
        assert_eq!(
            health_after, health_before,
            "canonical tick must not change creature health — no real AI/combat executed"
        );
    }
}
