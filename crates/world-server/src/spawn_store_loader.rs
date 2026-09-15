//! Canonical spawn metadata loader for `world-server` startup.
//!
//! Scope: metadata/index dependency only. This builds an in-memory
//! `wow_map::SpawnStore` from DB rows and applies `spawn_group`; it does not
//! create live entities, activate spawn groups, run respawn/pool logic, or fan
//! out to sessions.
//!
//! C++ anchors used by this module/tests:
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:2138-2165`
//!   `ObjectMgr::ParseSpawnDifficulties`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:2167-2242`
//!   `ObjectMgr::LoadCreatures` query fields and default/legacy spawn group.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:2413-2485`
//!   game-event gate and `AddSpawnDataToGrid` / `AddCreatureToGrid`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:2492-2618`
//!   `ObjectMgr::LoadGameObjects` query fields, difficulties/event/pool.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:2676-2736`
//!   validation tail and `AddGameobjectToGrid`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/AreaTriggerDataStore.cpp:321-425`
//!   `LoadAreaTriggerSpawns` query, create-properties validation, parse, and indexing.
//! - Existing Rust DB statements:
//!   `/home/server/rustycore/crates/wow-database/src/statements/world.rs:467-529`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:2798-2862`
//!   `ObjectMgr::LoadSpawnGroups` mutates spawn-group template map metadata and indexes
//!   `_spawnGroupsByMap` / `_spawnGroupMapStore` for non-system groups.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp:2455-2468`
//!   `Map::InitSpawnGroupState` reads `GetSpawnGroupsForMap(GetId())`, resolves each
//!   `GetSpawnGroupData(groupId)`, skips system groups, checks conditions, and toggles the map.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Conditions/ConditionMgr.cpp:1142-1145`
//!   future map-condition consumer entry point; conditions are not evaluated in this loader.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:874-916`
//!   `game_event_pool` query, signed event-id internal index and `CheckPool` gate.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:937-956`
//!   `MAX(eventEntry)` sizing for `mGameEventCreatureGuids`, `mGameEventGameobjectGuids`, and `mGameEventPoolIds`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:379-475`
//!   `game_event_creature` / `game_event_gameobject` GUID metadata loading.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.h:33-78`
//!   `GameEventState`, `GameEventData` defaults and `isValid()` predicate.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:215-285`
//!   `game_event` master metadata load, reserved id 0, normal zero-length validation,
//!   and deferred holiday DB2 validation / `SetHolidayEventTime`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:44-80`
//!   `GameEventMgr::CheckOneGameEvent(uint16)` pure timing/state decision helper.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:331-374`
//!   `game_event_prerequisite` load into `GameEventData::prerequisite_events`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:646-726`
//!   `game_event_condition` and `game_event_condition_save` load into `mGameEvent[event].conditions`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:82-119`
//!   `GameEventMgr::NextCheck(uint16)` pure delay decision helper.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:994-1062`
//!   `GameEventMgr::Update()` consumes the helpers before Start/Stop side effects;
//!   those scheduler/runtime side effects remain out of scope here.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.h:102-110,122-123,169`
//!   `m_ActiveEvents` is a `std::set<uint16>` with membership insert/erase helpers.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:1763-1782`
//!   global `IsHolidayActive` / `IsEventActive` read the active-event set only.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:478-531`
//!   `game_event_model_equip` load, event-id range check, previous model/equipment defaults,
//!   and `GetEquipmentInfo(entry, equipId)` validation for positive equipment ids.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:1478-1502,1508-1542`
//!   `GetEquipmentInfo` lookup by `(CreatureID, ID)` backed by `creature_equip_template`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:730-761`
//!   `game_event_npcflag` load into `mGameEventNPCFlags` with event range skip.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:920-935`
//!   `GameEventMgr::GetNPCFlag(Creature*)` ORs matching spawn-id flags over active events.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:1149-1161`
//!   `UpdateEventNPCVendor(event_id, activate)` adds/removes event vendor items.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:1530-1587`
//!   represented condition progress and `CheckOneGameEventConditions`.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Events/GameEventMgr.cpp:1606-1615`
//!   world-state metadata values for future `SendWorldStateUpdate` fanout.
//! - `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.cpp:39-176`
//!   `WorldStateMgr::LoadFromDB` templates/defaults plus saved-value overlay.
//! - `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.cpp:183-228`
//!   `WorldStateMgr::GetValue`/`SetValue` realm-wide vs map-specific branching.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:9737-9777`
//!   `AddVendorItem`/`RemoveVendorItem(..., persist=false)` mutate only ObjectMgr cache.
//! - `/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Creature.cpp:85-95`
//!   `VendorItemData::RemoveItem` erases all matching `(item, Type)` records.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::CreatureFormationInfoLikeCpp;
use wow_map::pool::{
    PoolGroupLikeCpp, PoolMemberKindLikeCpp, PoolMgrLikeCpp, PoolObjectLikeCpp,
    PoolTemplateDataLikeCpp,
};
use wow_map::spawn::{
    LinkedRespawnLoadIssueKindLikeCpp, LinkedRespawnLoadIssueLikeCpp,
    LinkedRespawnLoadReportLikeCpp, LinkedRespawnRowLikeCpp, LinkedRespawnTypeLikeCpp,
    SPAWNGROUP_MAP_UNSET, SpawnGroupApplyReport, SpawnGroupMemberRow,
};
use wow_map::{
    Difficulty, LinkedRespawnStoreLikeCpp, SpawnData, SpawnGroupFlags, SpawnGroupTemplateData,
    SpawnId, SpawnObjectType, SpawnPosition, SpawnStore,
};
use wow_persistence::{
    AreaTriggerSpawnPersistenceRowLikeCpp as AreaTriggerSpawnRow,
    CanonicalSpawnCatalogLoadOutcomeLikeCpp, CanonicalSpawnCatalogPersistencePortLikeCpp,
    CreatureEquipmentIdPersistenceRowLikeCpp,
    CreatureFormationPersistenceRowLikeCpp as CreatureFormationRowLikeCpp,
    CreatureSpawnPersistenceRowLikeCpp as CreatureSpawnRow,
    GameEventConditionPersistenceRowLikeCpp as GameEventConditionRowLikeCpp,
    GameEventDataPersistenceRowLikeCpp as GameEventDataRowLikeCpp,
    GameEventModelEquipPersistenceRowLikeCpp as GameEventModelEquipRowLikeCpp,
    GameEventNpcFlagPersistenceRowLikeCpp as GameEventNpcFlagRowLikeCpp,
    GameEventNpcVendorPersistenceRowLikeCpp as GameEventNpcVendorRowLikeCpp,
    GameEventObjectGuidPersistenceRowLikeCpp as GameEventObjectGuidRowLikeCpp,
    GameEventPoolPersistenceRowLikeCpp as GameEventPoolRowLikeCpp,
    GameEventPrerequisitePersistenceRowLikeCpp as GameEventPrerequisiteRowLikeCpp,
    GameEventQuestConditionPersistenceRowLikeCpp as GameEventQuestConditionRowLikeCpp,
    GameEventQuestRelationPersistenceRowLikeCpp as GameEventQuestRelationRowLikeCpp,
    GameEventWorldCatalogLoadOutcomeLikeCpp, GameEventWorldCatalogPersistencePortLikeCpp,
    GameEventWorldCatalogPrefixLikeCpp, GameEventWorldCatalogSuffixLikeCpp,
    GameObjectSpawnPersistenceRowLikeCpp as GameObjectSpawnRow,
    LinkedRespawnPersistenceRowLikeCpp as LinkedRespawnDbRow,
    PoolAutospawnCandidatePersistenceRowLikeCpp as PoolAutospawnCandidateRowLikeCpp,
    PoolMemberKindPersistenceLikeCpp, PoolMemberPersistenceRowLikeCpp as PoolMemberRowLikeCpp,
    PoolTemplatePersistenceRowLikeCpp as PoolTemplateRowLikeCpp,
    SpawnGroupMemberPersistenceRowLikeCpp,
    WaypointPathNodePersistenceRowLikeCpp as WaypointPathNodeRowLikeCpp,
    WaypointPathPersistenceRowLikeCpp as WaypointPathRowLikeCpp,
    WorldStateStartupLoadOutcomeLikeCpp, WorldStateStartupPersistencePortLikeCpp,
    WorldStateTemplatePersistenceRowLikeCpp as WorldStateDbTemplateRowLikeCpp,
};

const DIFFICULTY_NONE_LIKE_CPP: Difficulty = 0;
const PERSONAL_PHASE_FLAG_LIKE_CPP: u32 = 0x8000_0000;
const TRANSPORT_MAP_IDS_REPRESENTED: &[u32] = &[];
const GAME_EVENT_MINUTE_SECS_LIKE_CPP: u64 = 60;
/// C++ `#define max_ge_check_delay DAY` in `GameEventMgr.h:31`.
pub const MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnKindLoadReport {
    pub rows: usize,
    pub indexed: usize,
    pub skipped_event: usize,
    pub skipped_empty_difficulties: usize,
    pub skipped_missing_map: usize,
    pub skipped_invalid_position: usize,
    pub validation_skipped: usize,
    pub script_id_unresolved: usize,
    pub skipped_invalid_create_properties: Vec<(SpawnId, u32, bool)>,
    pub skipped_nonzero_create_properties_flags: Vec<(SpawnId, u32, bool)>,
    pub skipped_create_properties_curves: Vec<(SpawnId, u32, bool)>,
    pub skipped_create_properties_time_to_target: Vec<(SpawnId, u32, bool)>,
    pub skipped_create_properties_orbit: Vec<(SpawnId, u32, bool)>,
    pub skipped_create_properties_splines: Vec<(SpawnId, u32, bool)>,
    pub corrected_invalid_spell_for_visuals: Vec<(SpawnId, i32)>,
}

#[derive(Debug, Clone, Default)]
pub struct CanonicalSpawnStoreLoadReport {
    pub creature: SpawnKindLoadReport,
    pub gameobject: SpawnKindLoadReport,
    pub area_trigger: SpawnKindLoadReport,
    pub spawn_group_rows: usize,
    pub spawn_group_apply: SpawnGroupApplyReport,
    pub linked_respawn: LinkedRespawnLoadReportLikeCpp,
    pub pool_mgr: PoolMgrLoadReportLikeCpp,
    pub game_events: GameEventDataLoadReportLikeCpp,
    pub game_event_prerequisites: GameEventPrerequisiteLoadReportLikeCpp,
    pub game_event_conditions: GameEventConditionLoadReportLikeCpp,
    pub game_event_condition_saves: GameEventConditionSaveLoadReportLikeCpp,
    pub game_event_quest_conditions: GameEventQuestConditionLoadReportLikeCpp,
    pub game_event_pools: GameEventPoolLoadReportLikeCpp,
    pub game_event_spawn_guids: GameEventSpawnGuidLoadReportLikeCpp,
    pub game_event_model_equip: GameEventModelEquipLoadReportLikeCpp,
    pub game_event_quest_relations: GameEventQuestRelationsLoadReportLikeCpp,
    pub game_event_npc_flags: GameEventNpcFlagLoadReportLikeCpp,
    pub game_event_npc_vendors: GameEventNpcVendorLoadReportLikeCpp,
    pub creature_formations: CreatureFormationLoadReportLikeCpp,
    pub waypoint_paths: WaypointPathLoadReportLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WaypointPathLoadReportLikeCpp {
    pub path_rows: usize,
    pub paths_loaded: usize,
    pub skipped_invalid_move_type: usize,
    pub node_rows: usize,
    pub nodes_loaded: usize,
    pub skipped_missing_path: usize,
    pub empty_paths: usize,
    pub backwards_too_short: usize,
    pub clamped_delay: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WaypointPathStoreLikeCpp {
    paths: BTreeMap<u32, wow_movement::WaypointPath>,
}

impl WaypointPathStoreLikeCpp {
    pub fn from_rows_like_cpp(
        path_rows: impl IntoIterator<Item = WaypointPathRowLikeCpp>,
        node_rows: impl IntoIterator<Item = WaypointPathNodeRowLikeCpp>,
    ) -> (Self, WaypointPathLoadReportLikeCpp) {
        let mut report = WaypointPathLoadReportLikeCpp::default();
        let mut paths = BTreeMap::new();

        for row in path_rows {
            report.path_rows += 1;
            let Some(move_type) = waypoint_move_type_from_db_like_cpp(row.move_type) else {
                // C++ logs and returns after `_pathStore[pathId]` has already inserted an
                // invalid enum value. Rust keeps the store typed, so invalid paths are skipped.
                report.skipped_invalid_move_type += 1;
                continue;
            };
            let mut path = wow_movement::WaypointPath::new(row.path_id, Vec::new());
            path.move_type = move_type;
            path.follow_path_backwards_from_end_to_start = row.flags & 0x01 != 0;
            paths.insert(row.path_id, path);
            report.paths_loaded += 1;
        }

        for row in node_rows {
            report.node_rows += 1;
            let Some(path) = paths.get_mut(&row.path_id) else {
                report.skipped_missing_path += 1;
                continue;
            };
            let mut x = row.x;
            let mut y = row.y;
            wow_map::normalize_map_coord(&mut x);
            wow_map::normalize_map_coord(&mut y);
            let delay_ms = match i32::try_from(row.delay) {
                Ok(delay) => delay,
                Err(_) => {
                    report.clamped_delay += 1;
                    i32::MAX
                }
            };
            let mut node = wow_movement::WaypointNode::new(row.node_id, x, y, row.z);
            node.delay_ms = delay_ms;
            if let Some(orientation) = row.orientation {
                node.orientation = Some(orientation);
            }
            path.nodes.push(node);
            report.nodes_loaded += 1;
        }

        for path in paths.values() {
            if path.nodes.is_empty() {
                report.empty_paths += 1;
            }
            if path.follow_path_backwards_from_end_to_start
                && path.nodes.len()
                    < wow_movement::WAYPOINT_PATH_FLAG_FOLLOW_PATH_BACKWARDS_MINIMUM_NODES_LIKE_CPP
            {
                report.backwards_too_short += 1;
            }
        }

        (Self { paths }, report)
    }

    pub fn get(&self, path_id: u32) -> Option<&wow_movement::WaypointPath> {
        self.paths.get(&path_id)
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}

pub fn initialize_world_creature_default_waypoint_from_store_like_cpp(
    creature: &mut wow_world::map_manager::WorldCreature,
    waypoint_paths: &WaypointPathStoreLikeCpp,
) -> wow_movement::WaypointMovementAction {
    creature.initialize_default_waypoint_movement_with_path_resolver_like_cpp(|path_id| {
        waypoint_paths.get(path_id).cloned()
    })
}

fn waypoint_move_type_from_db_like_cpp(move_type: u8) -> Option<wow_movement::WaypointMoveType> {
    match move_type {
        0 => Some(wow_movement::WaypointMoveType::Walk),
        1 => Some(wow_movement::WaypointMoveType::Run),
        2 => Some(wow_movement::WaypointMoveType::Land),
        3 => Some(wow_movement::WaypointMoveType::TakeOff),
        _ => None,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureFormationLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_missing_leader: usize,
    pub skipped_missing_member: usize,
    pub duplicate_member_ignored: usize,
    pub removed_missing_leader_self: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolMemberLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_missing_spawn: usize,
    pub skipped_missing_template: usize,
    pub skipped_invalid_chance: usize,
    pub skipped_map_mismatch: usize,
    pub skipped_child_id_overflow: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolMgrLoadReportLikeCpp {
    pub template_rows: usize,
    pub templates_loaded: usize,
    pub creature_members: PoolMemberLoadReportLikeCpp,
    pub gameobject_members: PoolMemberLoadReportLikeCpp,
    pub pool_members: PoolMemberLoadReportLikeCpp,
    pub relation_removals: usize,
    pub map_mismatches: usize,
    pub circular_relations: usize,
    pub empty_pools: usize,
    pub missing_map_after_non_empty: usize,
    pub autospawn_rows: usize,
    pub autospawn_loaded: usize,
    pub autospawn_skipped_empty: usize,
    pub autospawn_skipped_broken: usize,
    pub autospawn_skipped_child: usize,
}

mod game_event_catalog;
mod game_event_loader;
mod game_event_runtime;
mod pool_loader;
mod world_state_catalog;
use game_event_loader::*;
pub use pool_loader::spawn_group_templates_for_spawn_store;
use pool_loader::*;
pub use {game_event_catalog::*, world_state_catalog::*};

#[derive(Debug, Clone, Default)]
pub struct CanonicalSpawnMetadataLikeCpp {
    spawn_store: SpawnStore,
    spawn_group_templates: BTreeMap<u32, SpawnGroupTemplateData>,
    linked_respawns: LinkedRespawnStoreLikeCpp,
    pool_mgr: PoolMgrLikeCpp,
    game_events: GameEventDataStoreLikeCpp,
    game_event_active_set: GameEventActiveSetLikeCpp,
    game_event_pools: GameEventPoolIdsLikeCpp,
    game_event_spawn_guids: GameEventSpawnGuidsLikeCpp,
    game_event_model_equip: GameEventModelEquipLikeCpp,
    game_event_quest_relations: GameEventQuestRelationsLikeCpp,
    game_event_quest_conditions_by_quest: BTreeMap<u32, GameEventQuestConditionRecordLikeCpp>,
    game_event_npc_flags: GameEventNpcFlagsLikeCpp,
    game_event_npc_vendors: GameEventNpcVendorsLikeCpp,
    game_event_active_creature_quest_relations_by_giver:
        BTreeMap<u32, Vec<GameEventQuestRelationRecordLikeCpp>>,
    game_event_active_gameobject_quest_relations_by_giver:
        BTreeMap<u32, Vec<GameEventQuestRelationRecordLikeCpp>>,
    game_event_vendor_cache_by_entry: BTreeMap<u32, Vec<GameEventNpcVendorRecordLikeCpp>>,
    waypoint_paths: WaypointPathStoreLikeCpp,
    creature_runtime_rows: BTreeMap<SpawnId, CreatureSpawnRuntimeRowLikeCpp>,
    gameobject_runtime_rows: BTreeMap<SpawnId, GameObjectSpawnRuntimeRowLikeCpp>,
    area_trigger_runtime_rows: BTreeMap<SpawnId, AreaTriggerSpawnRuntimeRowLikeCpp>,
    creature_formations: BTreeMap<SpawnId, CreatureFormationInfoLikeCpp>,
}

impl CanonicalSpawnMetadataLikeCpp {
    pub fn new(
        spawn_store: SpawnStore,
        spawn_group_templates: BTreeMap<u32, SpawnGroupTemplateData>,
    ) -> Self {
        Self {
            spawn_store,
            spawn_group_templates,
            linked_respawns: LinkedRespawnStoreLikeCpp::new(),
            pool_mgr: PoolMgrLikeCpp::new(),
            game_events: GameEventDataStoreLikeCpp::default(),
            game_event_active_set: GameEventActiveSetLikeCpp::default(),
            game_event_pools: GameEventPoolIdsLikeCpp::default(),
            game_event_spawn_guids: GameEventSpawnGuidsLikeCpp::default(),
            game_event_model_equip: GameEventModelEquipLikeCpp::default(),
            game_event_quest_relations: GameEventQuestRelationsLikeCpp::default(),
            game_event_quest_conditions_by_quest: BTreeMap::new(),
            game_event_npc_flags: GameEventNpcFlagsLikeCpp::default(),
            game_event_npc_vendors: GameEventNpcVendorsLikeCpp::default(),
            game_event_active_creature_quest_relations_by_giver: BTreeMap::new(),
            game_event_active_gameobject_quest_relations_by_giver: BTreeMap::new(),
            game_event_vendor_cache_by_entry: BTreeMap::new(),
            waypoint_paths: WaypointPathStoreLikeCpp::default(),
            creature_runtime_rows: BTreeMap::new(),
            gameobject_runtime_rows: BTreeMap::new(),
            area_trigger_runtime_rows: BTreeMap::new(),
            creature_formations: BTreeMap::new(),
        }
    }

    pub fn spawn_store(&self) -> &SpawnStore {
        &self.spawn_store
    }

    pub fn spawn_group_templates(&self) -> &BTreeMap<u32, SpawnGroupTemplateData> {
        &self.spawn_group_templates
    }

    pub fn with_linked_respawns_like_cpp(
        mut self,
        linked_respawns: LinkedRespawnStoreLikeCpp,
    ) -> Self {
        self.linked_respawns = linked_respawns;
        self
    }

    pub fn with_pool_mgr_like_cpp(mut self, pool_mgr: PoolMgrLikeCpp) -> Self {
        self.pool_mgr = pool_mgr;
        self
    }

    pub fn with_game_events_like_cpp(mut self, game_events: GameEventDataStoreLikeCpp) -> Self {
        self.game_events = game_events;
        self
    }

    pub fn with_game_event_pools_like_cpp(
        mut self,
        game_event_pools: GameEventPoolIdsLikeCpp,
    ) -> Self {
        self.game_event_pools = game_event_pools;
        self
    }

    pub fn with_game_event_spawn_guids_like_cpp(
        mut self,
        game_event_spawn_guids: GameEventSpawnGuidsLikeCpp,
    ) -> Self {
        self.game_event_spawn_guids = game_event_spawn_guids;
        self
    }

    pub fn with_game_event_model_equip_like_cpp(
        mut self,
        game_event_model_equip: GameEventModelEquipLikeCpp,
    ) -> Self {
        self.game_event_model_equip = game_event_model_equip;
        self
    }

    pub fn with_game_event_npc_flags_like_cpp(
        mut self,
        game_event_npc_flags: GameEventNpcFlagsLikeCpp,
    ) -> Self {
        self.game_event_npc_flags = game_event_npc_flags;
        self
    }

    pub fn with_game_event_quest_relations_like_cpp(
        mut self,
        game_event_quest_relations: GameEventQuestRelationsLikeCpp,
    ) -> Self {
        self.game_event_quest_relations = game_event_quest_relations;
        self
    }

    pub fn with_game_event_quest_conditions_like_cpp(
        mut self,
        game_event_quest_conditions_by_quest: BTreeMap<u32, GameEventQuestConditionRecordLikeCpp>,
    ) -> Self {
        self.game_event_quest_conditions_by_quest = game_event_quest_conditions_by_quest;
        self
    }

    pub fn game_event_quest_condition_like_cpp(
        &self,
        quest_id: u32,
    ) -> Option<&GameEventQuestConditionRecordLikeCpp> {
        self.game_event_quest_conditions_by_quest.get(&quest_id)
    }

    pub fn with_game_event_npc_vendors_like_cpp(
        mut self,
        game_event_npc_vendors: GameEventNpcVendorsLikeCpp,
    ) -> Self {
        self.game_event_npc_vendors = game_event_npc_vendors;
        self
    }

    pub fn with_waypoint_paths_like_cpp(
        mut self,
        waypoint_paths: WaypointPathStoreLikeCpp,
    ) -> Self {
        self.waypoint_paths = waypoint_paths;
        self
    }

    pub fn waypoint_paths_like_cpp(&self) -> &WaypointPathStoreLikeCpp {
        &self.waypoint_paths
    }

    pub fn linked_respawns_like_cpp(&self) -> &LinkedRespawnStoreLikeCpp {
        &self.linked_respawns
    }

    pub fn pool_mgr_like_cpp(&self) -> &PoolMgrLikeCpp {
        &self.pool_mgr
    }

    #[allow(dead_code)]
    pub fn game_events_like_cpp(&self) -> &GameEventDataStoreLikeCpp {
        &self.game_events
    }

    #[allow(dead_code)]
    pub fn game_event_active_set_like_cpp(&self) -> &GameEventActiveSetLikeCpp {
        &self.game_event_active_set
    }

    #[allow(dead_code)]
    pub fn game_event_active_set_mut_like_cpp(&mut self) -> &mut GameEventActiveSetLikeCpp {
        &mut self.game_event_active_set
    }

    pub fn creature_runtime_row_like_cpp(
        &self,
        spawn_id: SpawnId,
    ) -> Option<&CreatureSpawnRuntimeRowLikeCpp> {
        self.creature_runtime_rows.get(&spawn_id)
    }
    pub fn creature_formation_info_like_cpp(
        &self,
        spawn_id: SpawnId,
    ) -> Option<&CreatureFormationInfoLikeCpp> {
        self.creature_formations.get(&spawn_id)
    }

    pub fn with_creature_formations_like_cpp(
        mut self,
        formations: BTreeMap<SpawnId, CreatureFormationInfoLikeCpp>,
    ) -> Self {
        self.creature_formations = formations;
        self
    }

    pub fn with_creature_runtime_rows_like_cpp(
        mut self,
        rows: BTreeMap<SpawnId, CreatureSpawnRuntimeRowLikeCpp>,
    ) -> Self {
        self.creature_runtime_rows = rows;
        self
    }

    pub fn gameobject_runtime_row_like_cpp(
        &self,
        spawn_id: SpawnId,
    ) -> Option<&GameObjectSpawnRuntimeRowLikeCpp> {
        self.gameobject_runtime_rows.get(&spawn_id)
    }

    pub fn with_gameobject_runtime_rows_like_cpp(
        mut self,
        rows: BTreeMap<SpawnId, GameObjectSpawnRuntimeRowLikeCpp>,
    ) -> Self {
        self.gameobject_runtime_rows = rows;
        self
    }

    pub fn area_trigger_runtime_row_like_cpp(
        &self,
        spawn_id: SpawnId,
    ) -> Option<&AreaTriggerSpawnRuntimeRowLikeCpp> {
        self.area_trigger_runtime_rows.get(&spawn_id)
    }

    pub fn with_area_trigger_runtime_rows_like_cpp(
        mut self,
        rows: BTreeMap<SpawnId, AreaTriggerSpawnRuntimeRowLikeCpp>,
    ) -> Self {
        self.area_trigger_runtime_rows = rows;
        self
    }

    /// C++ shaped dependency for future `Map::InitSpawnGroupState` wiring.
    ///
    /// Mirrors the read side of
    /// `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp:2455-2468`:
    /// use `GetSpawnGroupsForMap(mapId)` order, then resolve each group through the
    /// `GetSpawnGroupData(groupId)`/map filter shape. Missing maps/templates are runtime-empty,
    /// not panics. This does not evaluate `ConditionMgr` or mutate map-owned runtime toggles.
    pub fn spawn_group_templates_for_map_like_cpp(
        &self,
        map_id: u32,
    ) -> Vec<(u32, &SpawnGroupTemplateData)> {
        self.spawn_store
            .spawn_group_ids_by_map(map_id)
            .into_iter()
            .flat_map(|group_ids| group_ids.iter().copied())
            .filter_map(|group_id| {
                SpawnStore::spawn_group_template_for_map(
                    &self.spawn_group_templates,
                    group_id,
                    map_id,
                )
                .map(|template| (group_id, template))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnDifficultyParseReport {
    pub invalid_tokens_as_none: usize,
    pub unsupported: Vec<Difficulty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSpawnDifficulties {
    pub difficulties: Vec<Difficulty>,
    pub report: SpawnDifficultyParseReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureSpawnRuntimeRowLikeCpp {
    pub spawn_id: SpawnId,
    pub model_id: u32,
    pub equipment_id: i8,
    pub wander_distance: f32,
    pub curhealth: u32,
    pub curmana: u32,
    pub movement_type: u8,
    pub npc_flags: Option<u64>,
    pub unit_flags: Option<u32>,
    pub unit_flags2: Option<u32>,
    pub unit_flags3: Option<u32>,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub string_id: String,
    pub spawn_time_secs: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectSpawnRuntimeRowLikeCpp {
    pub spawn_id: SpawnId,
    pub rotation: [f32; 4],
    pub anim_progress: u8,
    pub state: u8,
    pub string_id: String,
    pub spawn_time_secs: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaTriggerSpawnRuntimeRowLikeCpp {
    pub spawn_id: SpawnId,
    pub create_properties_id: wow_data::AreaTriggerIdLikeCpp,
    pub spell_for_visuals: Option<i32>,
}

fn linked_respawn_row_like_cpp(row: LinkedRespawnDbRow) -> LinkedRespawnRowLikeCpp {
    LinkedRespawnRowLikeCpp {
        guid: row.guid,
        linked_guid: row.linked_guid,
        link_type: row.link_type,
    }
}

pub fn apply_creature_formation_rows_like_cpp(
    rows: impl IntoIterator<Item = CreatureFormationRowLikeCpp>,
    store: &SpawnStore,
    report: &mut CreatureFormationLoadReportLikeCpp,
) -> BTreeMap<SpawnId, CreatureFormationInfoLikeCpp> {
    let mut formations = BTreeMap::new();
    let mut leader_spawn_ids = std::collections::BTreeSet::new();

    for row in rows {
        report.rows += 1;
        if store
            .spawn_data(SpawnObjectType::Creature, row.leader_spawn_id)
            .is_none()
        {
            report.skipped_missing_leader += 1;
            continue;
        }
        if store
            .spawn_data(SpawnObjectType::Creature, row.member_spawn_id)
            .is_none()
        {
            report.skipped_missing_member += 1;
            continue;
        }
        leader_spawn_ids.insert(row.leader_spawn_id);
        if formations.contains_key(&row.member_spawn_id) {
            report.duplicate_member_ignored += 1;
            continue;
        }

        let (follow_dist, follow_angle_radians) = if row.leader_spawn_id == row.member_spawn_id {
            (0.0, 0.0)
        } else {
            (row.dist, row.angle_degrees * std::f32::consts::PI / 180.0)
        };
        formations.insert(
            row.member_spawn_id,
            CreatureFormationInfoLikeCpp {
                leader_spawn_id: row.leader_spawn_id,
                follow_dist,
                follow_angle_radians,
                group_ai: row.group_ai,
                leader_waypoint_ids: [row.point_1, row.point_2],
            },
        );
        report.loaded += 1;
    }

    for leader_spawn_id in leader_spawn_ids {
        if !formations.contains_key(&leader_spawn_id) {
            let before = formations.len();
            formations.retain(|_, info| info.leader_spawn_id != leader_spawn_id);
            report.removed_missing_leader_self += before.saturating_sub(formations.len());
        }
    }
    report.loaded = formations.len();

    formations
}

async fn load_creature_formations_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    store: &SpawnStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<BTreeMap<SpawnId, CreatureFormationInfoLikeCpp>> {
    let rows = match persistence.load_creature_formations_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical creature formation catalog failed: {reason}")
        }
    };

    Ok(apply_creature_formation_rows_like_cpp(
        rows,
        store,
        &mut report.creature_formations,
    ))
}

async fn load_waypoint_paths_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<WaypointPathStoreLikeCpp> {
    let catalog = match persistence.load_waypoint_paths_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(catalog) => catalog,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical waypoint catalog failed: {reason}")
        }
    };

    let (store, load_report) =
        WaypointPathStoreLikeCpp::from_rows_like_cpp(catalog.paths, catalog.nodes);
    report.waypoint_paths = load_report;
    Ok(store)
}

async fn load_game_event_world_prefix_like_cpp(
    persistence: &dyn GameEventWorldCatalogPersistencePortLikeCpp,
) -> Result<GameEventWorldCatalogPrefixLikeCpp> {
    match persistence.load_prefix_like_cpp().await {
        GameEventWorldCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        GameEventWorldCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("GameEvent startup World catalog prefix failed: {reason}")
        }
    }
}

async fn load_game_event_condition_saves_then_world_suffix_like_cpp(
    game_event_persistence: &dyn wow_persistence::GameEventPersistencePortLikeCpp,
    world_catalog: &dyn GameEventWorldCatalogPersistencePortLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<GameEventWorldCatalogSuffixLikeCpp> {
    load_game_event_condition_saves_like_cpp(game_event_persistence, game_events, report).await?;
    match world_catalog.load_suffix_like_cpp().await {
        GameEventWorldCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        GameEventWorldCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("GameEvent startup World catalog suffix failed: {reason}")
        }
    }
}

pub async fn load_canonical_spawn_store_like_cpp(
    spawn_persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    game_event_persistence: &dyn wow_persistence::GameEventPersistencePortLikeCpp,
    game_event_world_catalog: &dyn GameEventWorldCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    spawn_group_store: &wow_data::SpawnGroupTemplateStore,
    creature_equipment_store: &wow_data::CreatureEquipmentStoreLikeCpp,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    mut area_trigger_spell_exists: impl FnMut(u32) -> bool,
    mut script_id_for_name: impl FnMut(&str) -> wow_data::ScriptIdLikeCpp,
) -> Result<(CanonicalSpawnMetadataLikeCpp, CanonicalSpawnStoreLoadReport)> {
    let mut store = SpawnStore::new();
    let mut creature_runtime_rows = BTreeMap::new();
    let mut gameobject_runtime_rows = BTreeMap::new();
    let mut area_trigger_runtime_rows = BTreeMap::new();
    let mut report = CanonicalSpawnStoreLoadReport::default();

    load_creature_spawns_like_cpp(
        spawn_persistence,
        map_store,
        map_difficulty_store,
        creature_equipment_store,
        &mut store,
        &mut creature_runtime_rows,
        &mut report,
    )
    .await?;
    // C++ `World::SetInitialWorldSettings` loads waypoint paths before
    // `FormationMgr::LoadCreatureFormations`; this stores metadata only and does not
    // launch waypoint movement.
    let waypoint_paths = load_waypoint_paths_like_cpp(spawn_persistence, &mut report).await?;
    let creature_formations =
        load_creature_formations_like_cpp(spawn_persistence, &store, &mut report).await?;
    load_gameobject_spawns_like_cpp(
        spawn_persistence,
        map_store,
        map_difficulty_store,
        &mut store,
        &mut gameobject_runtime_rows,
        &mut report,
    )
    .await?;
    load_area_trigger_spawns_like_cpp(
        spawn_persistence,
        map_store,
        map_difficulty_store,
        area_trigger_template_store,
        &mut area_trigger_spell_exists,
        &mut script_id_for_name,
        &mut store,
        &mut area_trigger_runtime_rows,
        &mut report,
    )
    .await?;

    // C++ `ObjectMgr::LoadLinkedRespawn` runs after creature/gameobject data is canonical.
    let linked_respawns =
        load_linked_respawns_like_cpp(spawn_persistence, &store, map_store, &mut report).await?;

    // C++ `PoolMgr::LoadFromDB` uses ObjectMgr creature/gameobject spawn data as
    // existence/map truth. This builds only PoolMgr metadata/plans; no live spawn.
    let pool_mgr = load_pool_mgr_like_cpp(spawn_persistence, &store, &mut report).await?;
    let game_event_prefix = load_game_event_world_prefix_like_cpp(game_event_world_catalog).await?;
    let game_event_sizing =
        GameEventSizingLikeCpp::from_max_event_entry_like_cpp(game_event_prefix.max_event_entry);
    // C++ `GameEventMgr::LoadFromDB` loads master `game_event` metadata into
    // `mGameEvent` before prerequisite and later event-specific lists consume the same sizing.
    // This is read-only startup metadata: no scheduler, active set, DB2 holiday
    // rewrite, persistence, or apply/unapply side effect is performed here.
    let mut game_events =
        load_game_events_like_cpp(game_event_prefix.events, game_event_sizing, &mut report);
    // C++ `GameEventMgr::LoadFromDB` stores prerequisites on the same `mGameEvent`
    // entries before scheduler helpers read them; no second prerequisite store is created.
    load_game_event_prerequisites_like_cpp(
        game_event_prefix.prerequisites,
        &mut game_events,
        &mut report,
    );
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_condition` into
    // `mGameEvent[event].conditions`, then overlays character DB saved `done` values.
    load_game_event_conditions_like_cpp(
        game_event_prefix.conditions,
        &mut game_events,
        &mut report,
    );
    let game_event_suffix = load_game_event_condition_saves_then_world_suffix_like_cpp(
        game_event_persistence,
        game_event_world_catalog,
        &mut game_events,
        &mut report,
    )
    .await?;
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_quest_condition` into
    // `mQuestToEventConditions` with quest-key last-row-wins semantics for later
    // `HandleQuestComplete`; this is metadata/evidence only and does not wire quests live.
    let game_event_quest_conditions = load_game_event_quest_conditions_like_cpp(
        game_event_suffix.quest_conditions,
        &game_events,
        &mut report,
    );
    // C++ `GameEventMgr` loads `game_event_pool` after PoolMgr validation so
    // `CheckPool(entry)` can gate each row; this is metadata only.
    let game_event_pools = load_game_event_pool_ids_like_cpp(
        game_event_suffix.pools,
        game_event_sizing,
        &pool_mgr,
        &mut report,
    );
    // C++ `GameEventMgr` also loads creature/gameobject GUID lists after ObjectMgr
    // spawn metadata exists. This stores only future caller input; no live grid mutation.
    let game_event_spawn_guids = load_game_event_spawn_guids_like_cpp(
        game_event_suffix.creature_guids,
        game_event_suffix.gameobject_guids,
        game_event_sizing,
        &store,
        &mut report,
    );
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_model_equip` startup metadata
    // for later `ChangeEquipOrModel`; this slice stores only validated metadata and
    // does not mutate live maps, CreatureData/ObjectMgr baselines, display ids or equipment.
    let game_event_model_equip = load_game_event_model_equip_like_cpp(
        game_event_suffix.equipment_ids,
        game_event_suffix.model_equips,
        game_event_sizing,
        &mut report,
    );
    // C++ `GameEventMgr::LoadFromDB` loads quest relation metadata from
    // `game_event_creature_quest` and `game_event_gameobject_quest` before later
    // condition/NPC flag/vendor metadata. This is read-only startup metadata for
    // future `UpdateEventQuests`; no ObjectMgr quest maps or sessions are mutated.
    let game_event_quest_relations = load_game_event_quest_relations_like_cpp(
        game_event_suffix.creature_quest_relations,
        game_event_suffix.gameobject_quest_relations,
        game_event_sizing,
        &mut report,
    );
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_npcflag` into
    // `mGameEventNPCFlags` for later `UpdateEventNPCFlags`/`GetNPCFlag`.
    // This slice stores only static metadata and pure read-only helpers.
    let game_event_npc_flags = load_game_event_npc_flags_like_cpp(
        game_event_suffix.npc_flags,
        game_event_sizing,
        &mut report,
    );
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_npc_vendor` after
    // `game_event_npcflag` because vendor validation receives the first matching
    // NPC flag low32 mask. Rust stores metadata only and defers ObjectMgr validation/mutation.
    let game_event_npc_vendors = load_game_event_npc_vendors_like_cpp(
        game_event_suffix.npc_vendors,
        game_event_sizing,
        &store,
        &game_event_npc_flags,
        &mut report,
    );

    let mut templates = spawn_group_templates_for_spawn_store(spawn_group_store);
    let members = load_spawn_group_members_like_cpp(spawn_persistence).await?;
    report.spawn_group_rows = members.len();
    report.spawn_group_apply = store.apply_spawn_groups_like_cpp(&mut templates, members);

    Ok((
        CanonicalSpawnMetadataLikeCpp::new(store, templates)
            .with_linked_respawns_like_cpp(linked_respawns)
            .with_pool_mgr_like_cpp(pool_mgr)
            .with_game_events_like_cpp(game_events)
            .with_game_event_pools_like_cpp(game_event_pools)
            .with_game_event_spawn_guids_like_cpp(game_event_spawn_guids)
            .with_game_event_model_equip_like_cpp(game_event_model_equip)
            .with_game_event_quest_relations_like_cpp(game_event_quest_relations)
            .with_game_event_quest_conditions_like_cpp(game_event_quest_conditions)
            .with_game_event_npc_flags_like_cpp(game_event_npc_flags)
            .with_game_event_npc_vendors_like_cpp(game_event_npc_vendors)
            .with_waypoint_paths_like_cpp(waypoint_paths)
            .with_creature_runtime_rows_like_cpp(creature_runtime_rows)
            .with_gameobject_runtime_rows_like_cpp(gameobject_runtime_rows)
            .with_area_trigger_runtime_rows_like_cpp(area_trigger_runtime_rows)
            .with_creature_formations_like_cpp(creature_formations),
        report,
    ))
}

async fn load_creature_spawns_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    creature_equipment_store: &wow_data::CreatureEquipmentStoreLikeCpp,
    store: &mut SpawnStore,
    creature_runtime_rows: &mut BTreeMap<SpawnId, CreatureSpawnRuntimeRowLikeCpp>,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let rows = match persistence.load_creature_spawns_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical creature spawn catalog failed: {reason}")
        }
    };
    for mut row in rows {
        normalize_creature_spawn_equipment_id_like_cpp(&mut row, creature_equipment_store);
        let runtime_row = creature_row_to_runtime_row_like_cpp(&row);
        report.creature.rows += 1;
        if let Some(spawn) = creature_row_to_spawn_data_like_cpp(
            &row,
            map_store,
            map_difficulty_store,
            &mut report.creature,
        ) {
            if row.event_entry != 0 {
                store.insert_spawn_metadata_like_cpp(&spawn);
                creature_runtime_rows.insert(row.spawn_id, runtime_row.clone());
                report.creature.skipped_event += 1;
            } else {
                store.add_object_spawn(&spawn, is_personal_phase_like_cpp_represented);
                creature_runtime_rows.insert(row.spawn_id, runtime_row.clone());
                report.creature.indexed += 1;
            }
        }
    }

    Ok(())
}

async fn load_gameobject_spawns_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    store: &mut SpawnStore,
    gameobject_runtime_rows: &mut BTreeMap<SpawnId, GameObjectSpawnRuntimeRowLikeCpp>,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let rows = match persistence.load_gameobject_spawns_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical gameobject spawn catalog failed: {reason}")
        }
    };
    for row in rows {
        report.gameobject.rows += 1;
        let runtime_row = gameobject_row_to_runtime_row_like_cpp(&row);
        if let Some(spawn) = gameobject_row_to_spawn_data_like_cpp(
            &row,
            map_store,
            map_difficulty_store,
            &mut report.gameobject,
        ) {
            if row.event_entry != 0 {
                store.insert_spawn_metadata_like_cpp(&spawn);
                gameobject_runtime_rows.insert(row.spawn_id, runtime_row.clone());
                report.gameobject.skipped_event += 1;
            } else {
                store.add_object_spawn(&spawn, is_personal_phase_like_cpp_represented);
                gameobject_runtime_rows.insert(row.spawn_id, runtime_row.clone());
                report.gameobject.indexed += 1;
            }
        }
    }

    Ok(())
}

async fn load_area_trigger_spawns_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    spell_exists: &mut impl FnMut(u32) -> bool,
    script_id_for_name: &mut impl FnMut(&str) -> wow_data::ScriptIdLikeCpp,
    store: &mut SpawnStore,
    area_trigger_runtime_rows: &mut BTreeMap<SpawnId, AreaTriggerSpawnRuntimeRowLikeCpp>,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let rows = match persistence.load_area_trigger_spawns_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical area-trigger spawn catalog failed: {reason}")
        }
    };
    for row in rows {
        report.area_trigger.rows += 1;
        if let Some(spawn) = area_trigger_row_to_spawn_data_like_cpp(
            &row,
            map_store,
            map_difficulty_store,
            area_trigger_template_store,
            spell_exists,
            script_id_for_name,
            area_trigger_runtime_rows,
            &mut report.area_trigger,
        ) {
            store.add_area_trigger_spawn(&spawn);
            report.area_trigger.indexed += 1;
        }
    }

    Ok(())
}

async fn load_linked_respawns_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    store: &SpawnStore,
    map_store: &wow_data::MapStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<LinkedRespawnStoreLikeCpp> {
    let mut linked_store = LinkedRespawnStoreLikeCpp::new();
    let rows = match persistence.load_linked_respawns_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical linked-respawn catalog failed: {reason}")
        }
    };
    for row in rows {
        apply_linked_respawn_row_like_cpp(
            linked_respawn_row_like_cpp(row),
            store,
            map_store,
            &mut linked_store,
            &mut report.linked_respawn,
        );
    }

    Ok(linked_store)
}

fn apply_linked_respawn_row_like_cpp(
    row: LinkedRespawnRowLikeCpp,
    store: &SpawnStore,
    map_store: &wow_data::MapStore,
    linked_store: &mut LinkedRespawnStoreLikeCpp,
    report: &mut LinkedRespawnLoadReportLikeCpp,
) {
    report.rows += 1;
    let Some(link_type) = LinkedRespawnTypeLikeCpp::from_raw(row.link_type) else {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::InvalidType,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: None,
            master_type: None,
            slave_map_id: None,
            master_map_id: None,
        });
        return;
    };

    let slave_type = link_type.slave_type();
    let master_type = link_type.master_type();
    let Some(slave) = store.spawn_data(slave_type, row.guid) else {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::MissingSlave,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: Some(slave_type),
            master_type: Some(master_type),
            slave_map_id: None,
            master_map_id: None,
        });
        return;
    };
    let Some(master) = store.spawn_data(master_type, row.linked_guid) else {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::MissingMaster,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: Some(slave_type),
            master_type: Some(master_type),
            slave_map_id: Some(slave.map_id),
            master_map_id: None,
        });
        return;
    };

    if map_store
        .get(master.map_id)
        .is_none_or(|map| !map_entry_instanceable_like_cpp(*map))
        || master.map_id != slave.map_id
    {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::NotInstanceableOrMapMismatch,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: Some(slave_type),
            master_type: Some(master_type),
            slave_map_id: Some(slave.map_id),
            master_map_id: Some(master.map_id),
        });
        return;
    }

    if !spawn_difficulties_intersect_like_cpp(slave, master) {
        report.push(LinkedRespawnLoadIssueLikeCpp {
            kind: LinkedRespawnLoadIssueKindLikeCpp::DifficultyMismatch,
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
            slave_type: Some(slave_type),
            master_type: Some(master_type),
            slave_map_id: Some(slave.map_id),
            master_map_id: Some(master.map_id),
        });
        return;
    }

    linked_store.insert_like_cpp(
        spawn_data_guid_like_cpp(slave),
        spawn_data_guid_like_cpp(master),
    );
    report.inserted += 1;
}

fn spawn_difficulties_intersect_like_cpp(left: &SpawnData, right: &SpawnData) -> bool {
    left.spawn_difficulties
        .iter()
        .any(|difficulty| right.spawn_difficulties.contains(difficulty))
}

fn spawn_data_guid_like_cpp(spawn: &SpawnData) -> ObjectGuid {
    let high = match spawn.object_type {
        SpawnObjectType::Creature => HighGuid::Creature,
        SpawnObjectType::GameObject => HighGuid::GameObject,
        SpawnObjectType::AreaTrigger => HighGuid::AreaTrigger,
    };
    ObjectGuid::create_world_object(
        high,
        0,
        0,
        spawn.map_id as u16,
        0,
        spawn.id,
        spawn.spawn_id as i64,
    )
}

fn map_entry_instanceable_like_cpp(map: wow_data::MapEntry) -> bool {
    matches!(
        map.instance_type,
        wow_data::map::MAP_INSTANCE
            | wow_data::map::MAP_RAID
            | wow_data::map::MAP_BATTLEGROUND
            | wow_data::map::MAP_ARENA
            | wow_data::map::MAP_SCENARIO
    )
}

async fn load_spawn_group_members_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
) -> Result<Vec<SpawnGroupMemberRow>> {
    match persistence.load_spawn_group_members_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows
            .into_iter()
            .map(
                |row: SpawnGroupMemberPersistenceRowLikeCpp| SpawnGroupMemberRow {
                    group_id: row.group_id,
                    spawn_type: row.spawn_type,
                    spawn_id: row.spawn_id,
                },
            )
            .collect()),
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical spawn-group member catalog failed: {reason}")
        }
    }
}

fn creature_row_to_spawn_data_like_cpp(
    row: &CreatureSpawnRow,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    report: &mut SpawnKindLoadReport,
) -> Option<SpawnData> {
    object_row_to_spawn_data_like_cpp(
        SpawnObjectType::Creature,
        row.spawn_id,
        row.entry,
        row.map_id,
        row.x,
        row.y,
        row.z,
        row.orientation,
        row.spawn_time_secs,
        &row.spawn_difficulties,
        row.pool_id,
        row.phase_use_flags,
        row.phase_id,
        row.phase_group,
        row.terrain_swap_map,
        &row.script_name,
        &row.string_id,
        map_store,
        map_difficulty_store,
        report,
    )
}

fn creature_row_to_runtime_row_like_cpp(row: &CreatureSpawnRow) -> CreatureSpawnRuntimeRowLikeCpp {
    CreatureSpawnRuntimeRowLikeCpp {
        spawn_id: row.spawn_id,
        model_id: row.model_id,
        equipment_id: row.equipment_id,
        wander_distance: row.wander_distance,
        curhealth: row.curhealth,
        curmana: row.curmana,
        movement_type: row.movement_type,
        npc_flags: row.npc_flags,
        unit_flags: row.unit_flags,
        unit_flags2: row.unit_flags2,
        unit_flags3: row.unit_flags3,
        ground_movement_type: row.ground_movement_type,
        swim_allowed: row.swim_allowed,
        flight_movement_type: row.flight_movement_type,
        rooted: row.rooted,
        chase_movement_type: row.chase_movement_type,
        random_movement_type: row.random_movement_type,
        interaction_pause_timer_ms: row.interaction_pause_timer_ms,
        string_id: row.string_id.clone(),
        spawn_time_secs: row.spawn_time_secs,
    }
}

fn normalize_creature_spawn_equipment_id_like_cpp(
    row: &mut CreatureSpawnRow,
    equipment_store: &wow_data::CreatureEquipmentStoreLikeCpp,
) {
    // C++ `ObjectMgr::LoadCreatureData`: `-1` means random equipment, `0` means
    // no equipment, and any non-zero id missing from `creature_equip_template`
    // is normalized back to no equipment before `Creature::LoadFromDB`.
    if row.equipment_id == 0 {
        return;
    }

    let mut equipment_id = row.equipment_id;
    if equipment_store
        .get_equipment_info_like_cpp(row.entry, &mut equipment_id, wow_core::urand_like_cpp)
        .is_some()
    {
        row.equipment_id = equipment_id;
    } else {
        row.equipment_id = 0;
    }
}

fn gameobject_row_to_runtime_row_like_cpp(
    row: &GameObjectSpawnRow,
) -> GameObjectSpawnRuntimeRowLikeCpp {
    GameObjectSpawnRuntimeRowLikeCpp {
        spawn_id: row.spawn_id,
        rotation: row.rotation,
        anim_progress: row.anim_progress,
        state: row.state,
        string_id: row.string_id.clone(),
        spawn_time_secs: row.spawn_time_secs,
    }
}

fn gameobject_row_to_spawn_data_like_cpp(
    row: &GameObjectSpawnRow,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    report: &mut SpawnKindLoadReport,
) -> Option<SpawnData> {
    object_row_to_spawn_data_like_cpp(
        SpawnObjectType::GameObject,
        row.spawn_id,
        row.entry,
        row.map_id,
        row.x,
        row.y,
        row.z,
        row.orientation,
        row.spawn_time_secs,
        &row.spawn_difficulties,
        row.pool_id,
        row.phase_use_flags,
        row.phase_id,
        row.phase_group,
        row.terrain_swap_map,
        &row.script_name,
        &row.string_id,
        map_store,
        map_difficulty_store,
        report,
    )
}

#[allow(clippy::too_many_arguments)]
fn object_row_to_spawn_data_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
    entry: u32,
    map_id: u32,
    x: f32,
    y: f32,
    z: f32,
    orientation: f32,
    spawn_time_secs: i32,
    spawn_difficulties: &str,
    pool_id: u32,
    phase_use_flags: u8,
    phase_id: u32,
    phase_group: u32,
    terrain_swap_map: i32,
    script_name: &str,
    string_id: &str,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    report: &mut SpawnKindLoadReport,
) -> Option<SpawnData> {
    if map_store.get(map_id).is_none() {
        report.skipped_missing_map += 1;
        return None;
    }
    if !is_valid_map_coord_like_cpp(x, y, z, orientation) {
        report.skipped_invalid_position += 1;
        return None;
    }

    let is_transport = is_transport_map_like_cpp_represented(map_id);
    let parsed = parse_spawn_difficulties_like_cpp(
        spawn_difficulties,
        map_id,
        is_transport,
        map_difficulty_store,
    );
    if parsed.difficulties.is_empty() {
        report.skipped_empty_difficulties += 1;
        return None;
    }

    report.validation_skipped += 1;
    if !script_name.is_empty() {
        report.script_id_unresolved += 1;
    }

    Some(SpawnData {
        object_type,
        spawn_id,
        map_id,
        db_data: true,
        spawn_group: default_spawn_group_like_cpp(is_transport),
        id: entry,
        spawn_point: SpawnPosition::new(x, y, z, orientation),
        phase_use_flags,
        phase_id,
        phase_group,
        terrain_swap_map,
        pool_id,
        spawn_time_secs,
        spawn_difficulties: parsed.difficulties,
        script_id: 0,
        string_id: string_id.to_string(),
    })
}

fn area_trigger_row_to_spawn_data_like_cpp(
    row: &AreaTriggerSpawnRow,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    spell_exists: &mut impl FnMut(u32) -> bool,
    script_id_for_name: &mut impl FnMut(&str) -> wow_data::ScriptIdLikeCpp,
    area_trigger_runtime_rows: &mut BTreeMap<SpawnId, AreaTriggerSpawnRuntimeRowLikeCpp>,
    report: &mut SpawnKindLoadReport,
) -> Option<SpawnData> {
    let create_properties_id = wow_data::AreaTriggerIdLikeCpp {
        id: row.create_properties_id,
        is_custom: row.is_custom,
    };
    let Some(create_properties) =
        area_trigger_template_store.get_create_properties_like_cpp(create_properties_id)
    else {
        report.skipped_invalid_create_properties.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    };
    if create_properties.flags
        != wow_data::area_trigger_template::AREATRIGGER_CREATE_PROPERTIES_FLAG_NONE_LIKE_CPP
    {
        report.skipped_nonzero_create_properties_flags.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if create_properties.scale_curve_id != 0
        || create_properties.morph_curve_id != 0
        || create_properties.facing_curve_id != 0
        || create_properties.move_curve_id != 0
    {
        report.skipped_create_properties_curves.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if create_properties.time_to_target != 0
        || create_properties.time_to_target_scale != 0
        || create_properties.facing_curve_id != 0
        || create_properties.move_curve_id != 0
    {
        report.skipped_create_properties_time_to_target.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if create_properties.orbit_info.is_some() {
        report.skipped_create_properties_orbit.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if create_properties.spline_points.len() >= 2 {
        report.skipped_create_properties_splines.push((
            row.spawn_id,
            row.create_properties_id,
            row.is_custom,
        ));
        return None;
    }
    if map_store.get(row.map_id).is_none() {
        report.skipped_missing_map += 1;
        return None;
    }
    if !is_valid_map_coord_like_cpp(row.x, row.y, row.z, row.orientation) {
        report.skipped_invalid_position += 1;
        return None;
    }

    let parsed = parse_spawn_difficulties_like_cpp(
        &row.spawn_difficulties,
        row.map_id,
        is_transport_map_like_cpp_represented(row.map_id),
        map_difficulty_store,
    );
    if parsed.difficulties.is_empty() {
        report.skipped_empty_difficulties += 1;
        return None;
    }

    let spell_for_visuals = match row.spell_for_visuals {
        Some(spell_id) if spell_id >= 0 && spell_exists(spell_id as u32) => Some(spell_id),
        Some(spell_id) => {
            report
                .corrected_invalid_spell_for_visuals
                .push((row.spawn_id, spell_id));
            None
        }
        None => None,
    };
    let script_id = script_id_for_name(&row.script_name).0;

    area_trigger_runtime_rows.insert(
        row.spawn_id,
        AreaTriggerSpawnRuntimeRowLikeCpp {
            spawn_id: row.spawn_id,
            create_properties_id,
            spell_for_visuals,
        },
    );

    Some(SpawnData {
        object_type: SpawnObjectType::AreaTrigger,
        spawn_id: row.spawn_id,
        map_id: row.map_id,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::legacy_group(),
        id: row.create_properties_id,
        spawn_point: SpawnPosition::new(row.x, row.y, row.z, row.orientation),
        phase_use_flags: row.phase_use_flags,
        phase_id: row.phase_id,
        phase_group: row.phase_group,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 0,
        spawn_difficulties: parsed.difficulties,
        script_id,
        string_id: String::new(),
    })
}

fn parse_spawn_difficulties_like_cpp(
    difficulty_string: &str,
    map_id: u32,
    is_transport_map: bool,
    map_difficulty_store: &wow_data::MapDifficultyStore,
) -> ParsedSpawnDifficulties {
    let mut difficulties = Vec::new();
    let mut report = SpawnDifficultyParseReport {
        invalid_tokens_as_none: 0,
        unsupported: Vec::new(),
    };

    for token in difficulty_string
        .split(',')
        .filter(|token| !token.is_empty())
    {
        let difficulty = match token.parse::<Difficulty>() {
            Ok(difficulty) => difficulty,
            Err(_) => {
                report.invalid_tokens_as_none += 1;
                DIFFICULTY_NONE_LIKE_CPP
            }
        };

        if !is_transport_map && map_difficulty_store.get(map_id, difficulty).is_none() {
            report.unsupported.push(difficulty);
            continue;
        }

        difficulties.push(difficulty);
    }

    difficulties.sort_unstable();
    ParsedSpawnDifficulties {
        difficulties,
        report,
    }
}

fn default_spawn_group_like_cpp(is_transport_map: bool) -> SpawnGroupTemplateData {
    if is_transport_map {
        SpawnGroupTemplateData::legacy_group()
    } else {
        SpawnGroupTemplateData::default_group()
    }
}

fn is_valid_map_coord_like_cpp(x: f32, y: f32, z: f32, orientation: f32) -> bool {
    Position::new(x, y, z, orientation).is_valid_map_coord_like_cpp()
}

fn is_personal_phase_like_cpp_represented(phase_id: u32) -> bool {
    // C++ checks `PhaseEntryFlags::Personal` via `PhasingHandler::IsPersonalPhase`.
    // Phase DB2 flag lookup is not available in this metadata-only loader yet, so
    // this keeps the predicate isolated and intentionally conservative.
    phase_id & PERSONAL_PHASE_FLAG_LIKE_CPP != 0
}

fn is_transport_map_like_cpp_represented(map_id: u32) -> bool {
    // C++ `ObjectMgr::_transportMaps` is populated while validating
    // GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT/GARRISON_BUILDING templates. RustyCore
    // has no canonical transport-map store yet; keep the fallback explicit so a
    // later transport-template slice can replace only this predicate.
    TRANSPORT_MAP_IDS_REPRESENTED.contains(&map_id)
}

#[cfg(test)]
#[path = "spawn_store_loader_tests.rs"]
mod tests;
