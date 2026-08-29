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
use wow_database::{CharStatements, CharacterDatabase, SqlResult, WorldDatabase, WorldStatements};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaypointPathRowLikeCpp {
    pub path_id: u32,
    pub move_type: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaypointPathNodeRowLikeCpp {
    pub path_id: u32,
    pub node_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: Option<f32>,
    pub delay: u32,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventDataLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_reserved_zero: usize,
    pub skipped_out_of_range: usize,
    pub invalid_normal_zero_length: usize,
    pub holiday_validation_deferred: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventPrerequisiteLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range_event: usize,
    pub skipped_non_world_event: usize,
    pub skipped_out_of_range_prerequisite: usize,
    pub duplicate_ignored: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventConditionLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventConditionSaveLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range_event: usize,
    pub skipped_missing_condition: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestConditionLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range_event: usize,
    pub overwrites: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventPoolLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
    pub skipped_broken_pool: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventObjectGuidLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_missing_spawn_metadata: usize,
    pub skipped_out_of_range: usize,
    pub pooled_still_loaded: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventSpawnGuidLoadReportLikeCpp {
    pub creature: GameEventObjectGuidLoadReportLikeCpp,
    pub gameobject: GameEventObjectGuidLoadReportLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventModelEquipLoadReportLikeCpp {
    pub equipment_rows: usize,
    pub equipment_ids_loaded: usize,
    pub rows: usize,
    pub loaded: usize,
    pub invalid_event_id: usize,
    pub missing_equipment_template: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestRelationFamilyLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
    pub events_touched: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestRelationsLoadReportLikeCpp {
    pub creature: GameEventQuestRelationFamilyLoadReportLikeCpp,
    pub gameobject: GameEventQuestRelationFamilyLoadReportLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcFlagLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
    pub events_touched: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcVendorLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
    pub skipped_missing_creature_spawn_metadata: usize,
    pub validation_deferred: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcVendorCacheUpdateSummaryLikeCpp {
    pub event_id: u16,
    pub activate: bool,
    pub missing_event_bucket: bool,
    pub records_seen: usize,
    pub items_added: usize,
    pub items_removed: usize,
    pub remove_misses: usize,
    pub no_match: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct GameEventSizingLikeCpp {
    game_event_size: i32,
    slot_count: usize,
}

impl GameEventSizingLikeCpp {
    fn from_max_event_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        let max_event_id = max_event_entry.unwrap_or(0).saturating_add(1);
        let slot_count = max_event_id.saturating_mul(2).saturating_sub(1) as usize;
        let game_event_size = i32::try_from(max_event_id).unwrap_or(i32::MAX);
        Self {
            game_event_size,
            slot_count,
        }
    }

    fn master_slot_count_like_cpp(self) -> usize {
        usize::try_from(self.game_event_size).unwrap_or(usize::MAX)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameEventStateLikeCpp {
    Normal = 0,
    WorldInactive = 1,
    WorldConditions = 2,
    WorldNextPhase = 3,
    WorldFinished = 4,
    Internal = 5,
}

#[allow(dead_code)]
impl GameEventStateLikeCpp {
    pub fn from_raw_like_cpp(state_raw: u8) -> Option<Self> {
        match state_raw {
            0 => Some(Self::Normal),
            1 => Some(Self::WorldInactive),
            2 => Some(Self::WorldConditions),
            3 => Some(Self::WorldNextPhase),
            4 => Some(Self::WorldFinished),
            5 => Some(Self::Internal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventCheckOutcomeLikeCpp {
    Active(bool),
    MissingEvent { event_id: u16 },
    MissingPrerequisite { event_id: u16 },
    InvalidTimingZeroOccurrence { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventPrerequisiteInsertOutcomeLikeCpp {
    Loaded,
    Duplicate,
    OutOfRangeEvent,
    NonWorldEvent,
    OutOfRangePrerequisite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventNextCheckOutcomeLikeCpp {
    DelaySecs(u64),
    MissingEvent { event_id: u16 },
    InvalidTimingZeroOccurrence { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventHolidayActiveOutcomeLikeCpp {
    Active(bool),
    MissingActiveEvent { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventStartOutcomeLikeCpp {
    Started(GameEventStartSummaryLikeCpp),
    MissingEvent { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventStartSummaryLikeCpp {
    pub event_id: u16,
    pub state_before_raw: u8,
    pub state_after_raw: u8,
    pub active_added: bool,
    pub active_was_present: bool,
    pub apply_new_event_requested: bool,
    pub save_world_event_state_requested: bool,
    pub force_game_event_update_requested: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventStopOutcomeLikeCpp {
    Stopped(GameEventStopSummaryLikeCpp),
    MissingEvent { event_id: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventStopSummaryLikeCpp {
    pub event_id: u16,
    pub state_before_raw: u8,
    pub state_after_raw: u8,
    pub active_removed: bool,
    pub active_was_present: bool,
    pub unapply_event_requested: bool,
    pub serverwide: bool,
    pub condition_reset_requested: bool,
    pub delete_world_event_state_requested: bool,
    pub delete_condition_saves_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventWorldStateSaveEvidenceLikeCpp {
    pub event_id: u16,
    pub state_after_raw: u8,
    pub next_start_after: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventWorldNextPhaseFinishedLikeCpp {
    pub event_id: u16,
    pub was_active_before_queue: bool,
    pub state_before_raw: u8,
    pub state_after_raw: u8,
    pub next_start_before: u64,
    pub next_start_after: u64,
    pub save_state_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEventUpdateOutcomeLikeCpp {
    pub current_time_secs: u64,
    pub scanned_event_ids: Vec<u16>,
    pub check_outcomes: Vec<(u16, GameEventCheckOutcomeLikeCpp)>,
    pub next_check_outcomes: Vec<(u16, GameEventNextCheckOutcomeLikeCpp)>,
    pub queued_activation_event_ids: Vec<u16>,
    pub queued_deactivation_event_ids: Vec<u16>,
    pub start_outcomes: Vec<GameEventStartOutcomeLikeCpp>,
    pub stop_outcomes: Vec<GameEventStopOutcomeLikeCpp>,
    pub negative_spawn_event_ids: Vec<i16>,
    pub world_nextphase_finished: Vec<GameEventWorldNextPhaseFinishedLikeCpp>,
    pub world_conditions_save_requested: Vec<GameEventWorldStateSaveEvidenceLikeCpp>,
    pub invalid_check_outcomes: Vec<GameEventCheckOutcomeLikeCpp>,
    pub invalid_next_check_outcomes: Vec<GameEventNextCheckOutcomeLikeCpp>,
    pub next_event_delay_secs_before_padding: u64,
    pub next_update_delay_millis: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventConditionLikeCpp {
    pub req_num: f32,
    pub done: f32,
    pub max_world_state: u16,
    pub done_world_state: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventWorldStateUpdateSourceLikeCpp {
    Done,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventWorldStateValueSkipReasonLikeCpp {
    NonFinite,
    Negative,
    OutOfI32Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventWorldStateUpdateEvidenceLikeCpp {
    pub event_id: u16,
    pub condition_id: u32,
    pub variable_id: u32,
    pub value: i32,
    pub source: GameEventWorldStateUpdateSourceLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventWorldStateUpdateSkipLikeCpp {
    pub event_id: u16,
    pub condition_id: u32,
    pub variable_id: u32,
    pub source: GameEventWorldStateUpdateSourceLikeCpp,
    pub reason: GameEventWorldStateValueSkipReasonLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEventWorldStateUpdateOutcomeLikeCpp {
    Updates {
        event_id: u16,
        updates: Vec<GameEventWorldStateUpdateEvidenceLikeCpp>,
        skipped: Vec<GameEventWorldStateUpdateSkipLikeCpp>,
    },
    MissingEvent {
        event_id: u16,
    },
}

/// C++-shaped subset of `WorldStateTemplate` for represented `WorldStateMgr` startup state and realm-wide `SetValue`.
///
/// This intentionally does not close `FillInitialWorldStates`, real player-area login packet filtering,
/// map-local `Map::SetWorldStateValue`, persistence, or real script dispatch. `script_hook_represented`
/// and `global_message_represented` in outcomes are evidence flags only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStateTemplateLikeCpp {
    pub id: i32,
    pub default_value: i32,
    pub map_ids: BTreeSet<i32>,
    pub area_ids: BTreeSet<u32>,
    pub script_name: String,
}

impl WorldStateTemplateLikeCpp {
    pub fn realm_wide(id: i32, default_value: i32) -> Self {
        Self {
            id,
            default_value,
            map_ids: BTreeSet::new(),
            area_ids: BTreeSet::new(),
            script_name: String::new(),
        }
    }

    pub fn map_specific(
        id: i32,
        default_value: i32,
        map_ids: impl IntoIterator<Item = i32>,
    ) -> Self {
        Self {
            id,
            default_value,
            map_ids: map_ids.into_iter().collect(),
            area_ids: BTreeSet::new(),
            script_name: String::new(),
        }
    }

    pub fn with_area_ids(mut self, area_ids: impl IntoIterator<Item = u32>) -> Self {
        self.area_ids = area_ids.into_iter().collect();
        self
    }

    pub fn with_script_name(mut self, script_name: impl Into<String>) -> Self {
        self.script_name = script_name.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldStateSetValueOutcomeLikeCpp {
    RealmInsertedOrChanged {
        world_state_id: i32,
        old_value: i32,
        new_value: i32,
        hidden: bool,
        script_hook_represented: bool,
        global_message_represented: bool,
    },
    RealmUnchanged {
        world_state_id: i32,
        value: i32,
    },
    MapSpecificNoMapUnsupported {
        world_state_id: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStateDbTemplateRowLikeCpp {
    pub id: i32,
    pub default_value: i32,
    pub map_ids_csv: String,
    pub area_ids_csv: String,
    pub script_name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldStateMgrLoadReportLikeCpp {
    pub template_rows: u32,
    pub templates_loaded: u32,
    pub skipped_invalid_map_list: u32,
    pub skipped_invalid_area_list: u32,
    pub realm_area_requirements_ignored: u32,
    pub saved_rows: u32,
    pub saved_applied: u32,
    pub saved_skipped_unknown: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldStateMgrLikeCpp {
    world_state_templates: BTreeMap<i32, WorldStateTemplateLikeCpp>,
    realm_world_state_values: BTreeMap<i32, i32>,
    world_states_by_map: BTreeMap<i32, BTreeMap<i32, i32>>,
}

impl WorldStateMgrLikeCpp {
    /// Builds represented state in the same high-level order as C++ LoadFromDB:
    /// `world_state` templates/defaults first, then `world_state_value` saved overlay.
    pub fn from_templates_and_saved_values(
        templates: impl IntoIterator<Item = WorldStateTemplateLikeCpp>,
        saved_values: impl IntoIterator<Item = (i32, i32)>,
    ) -> Self {
        let mut mgr = Self::default();
        for template in templates {
            if template.map_ids.is_empty() {
                mgr.realm_world_state_values
                    .insert(template.id, template.default_value);
            } else {
                for &map_id in &template.map_ids {
                    mgr.world_states_by_map
                        .entry(map_id)
                        .or_default()
                        .insert(template.id, template.default_value);
                }
            }
            mgr.world_state_templates.insert(template.id, template);
        }
        for (world_state_id, value) in saved_values {
            if let Some(template) = mgr.world_state_templates.get(&world_state_id) {
                if template.map_ids.is_empty() {
                    mgr.realm_world_state_values.insert(world_state_id, value);
                } else {
                    for &map_id in &template.map_ids {
                        mgr.world_states_by_map
                            .entry(map_id)
                            .or_default()
                            .insert(world_state_id, value);
                    }
                }
            }
        }
        mgr
    }

    pub fn from_db_rows_like_cpp(
        template_rows: impl IntoIterator<Item = WorldStateDbTemplateRowLikeCpp>,
        saved_values: impl IntoIterator<Item = (i32, i32)>,
        map_exists: impl Fn(i32) -> bool,
        area_continent_id: impl Fn(u32) -> Option<u16>,
    ) -> (Self, WorldStateMgrLoadReportLikeCpp) {
        let mut mgr = Self::default();
        let mut report = WorldStateMgrLoadReportLikeCpp::default();

        for row in template_rows {
            report.template_rows += 1;
            let map_ids = parse_world_state_map_ids_like_cpp(row.id, &row.map_ids_csv, &map_exists);
            if !row.map_ids_csv.is_empty() && map_ids.is_empty() {
                report.skipped_invalid_map_list += 1;
                continue;
            }

            let mut area_ids = BTreeSet::new();
            if !map_ids.is_empty() {
                area_ids = parse_world_state_area_ids_like_cpp(
                    row.id,
                    &row.area_ids_csv,
                    &map_ids,
                    &area_continent_id,
                );
                if !row.area_ids_csv.is_empty() && area_ids.is_empty() {
                    report.skipped_invalid_area_list += 1;
                    continue;
                }
            } else if !row.area_ids_csv.is_empty() {
                report.realm_area_requirements_ignored += 1;
            }

            let template = WorldStateTemplateLikeCpp {
                id: row.id,
                default_value: row.default_value,
                map_ids,
                area_ids,
                script_name: row.script_name,
            };
            if template.map_ids.is_empty() {
                mgr.realm_world_state_values
                    .insert(template.id, template.default_value);
            } else {
                for &map_id in &template.map_ids {
                    mgr.world_states_by_map
                        .entry(map_id)
                        .or_default()
                        .insert(template.id, template.default_value);
                }
            }
            mgr.world_state_templates.insert(template.id, template);
            report.templates_loaded += 1;
        }

        for (world_state_id, value) in saved_values {
            report.saved_rows += 1;
            let Some(template) = mgr.world_state_templates.get(&world_state_id) else {
                report.saved_skipped_unknown += 1;
                continue;
            };
            if template.map_ids.is_empty() {
                mgr.realm_world_state_values.insert(world_state_id, value);
            } else {
                for &map_id in &template.map_ids {
                    mgr.world_states_by_map
                        .entry(map_id)
                        .or_default()
                        .insert(world_state_id, value);
                }
            }
            report.saved_applied += 1;
        }

        (mgr, report)
    }

    pub fn template_like_cpp(&self, world_state_id: i32) -> Option<&WorldStateTemplateLikeCpp> {
        self.world_state_templates.get(&world_state_id)
    }

    pub fn realm_value_like_cpp(&self, world_state_id: i32) -> i32 {
        self.realm_world_state_values
            .get(&world_state_id)
            .copied()
            .unwrap_or(0)
    }

    pub fn map_value_like_cpp(&self, map_id: i32, world_state_id: i32) -> i32 {
        self.world_states_by_map
            .get(&map_id)
            .and_then(|values| values.get(&world_state_id))
            .copied()
            .unwrap_or(0)
    }

    pub fn initial_world_states_for_map_like_cpp(&self, map_id: i32) -> BTreeMap<i32, i32> {
        let mut values = BTreeMap::new();
        if let Some(any_map_values) = self.world_states_by_map.get(&WORLDSTATE_ANY_MAP_LIKE_CPP) {
            values.extend(any_map_values.iter().map(|(&id, &value)| (id, value)));
        }
        if let Some(map_values) = self.world_states_by_map.get(&map_id) {
            values.extend(map_values.iter().map(|(&id, &value)| (id, value)));
        }
        values
    }

    pub fn set_value_realm_or_map_null_like_cpp(
        &mut self,
        world_state_id: i32,
        value: i32,
        hidden: bool,
    ) -> WorldStateSetValueOutcomeLikeCpp {
        let template = self.world_state_templates.get(&world_state_id);
        if template.is_some_and(|template| !template.map_ids.is_empty()) {
            return WorldStateSetValueOutcomeLikeCpp::MapSpecificNoMapUnsupported {
                world_state_id,
            };
        }

        let inserted = !self.realm_world_state_values.contains_key(&world_state_id);
        let old_value = self
            .realm_world_state_values
            .get(&world_state_id)
            .copied()
            .unwrap_or(0);
        if old_value == value && !inserted {
            return WorldStateSetValueOutcomeLikeCpp::RealmUnchanged {
                world_state_id,
                value,
            };
        }

        self.realm_world_state_values.insert(world_state_id, value);
        WorldStateSetValueOutcomeLikeCpp::RealmInsertedOrChanged {
            world_state_id,
            old_value,
            new_value: value,
            hidden,
            script_hook_represented: template.is_some(),
            global_message_represented: true,
        }
    }
}

const WORLDSTATE_ANY_MAP_LIKE_CPP: i32 = -1;

fn parse_world_state_map_ids_like_cpp(
    _world_state_id: i32,
    map_ids_csv: &str,
    map_exists: &impl Fn(i32) -> bool,
) -> BTreeSet<i32> {
    let mut map_ids = BTreeSet::new();
    for token in map_ids_csv.split(',').filter(|token| !token.is_empty()) {
        let Ok(map_id) = token.trim().parse::<i32>() else {
            continue;
        };
        if map_id != WORLDSTATE_ANY_MAP_LIKE_CPP && !map_exists(map_id) {
            continue;
        }
        map_ids.insert(map_id);
    }
    map_ids
}

fn parse_world_state_area_ids_like_cpp(
    _world_state_id: i32,
    area_ids_csv: &str,
    map_ids: &BTreeSet<i32>,
    area_continent_id: &impl Fn(u32) -> Option<u16>,
) -> BTreeSet<u32> {
    let mut area_ids = BTreeSet::new();
    for token in area_ids_csv.split(',').filter(|token| !token.is_empty()) {
        let Ok(area_id) = token.trim().parse::<u32>() else {
            continue;
        };
        let Some(continent_id) = area_continent_id(area_id) else {
            continue;
        };
        if !map_ids.contains(&i32::from(continent_id)) {
            continue;
        }
        area_ids.insert(area_id);
    }
    area_ids
}

pub async fn load_world_state_mgr_like_cpp(
    world_db: &WorldDatabase,
    character_db: &CharacterDatabase,
    map_store: &wow_data::MapStore,
    area_table_store: &wow_data::AreaTableStore,
) -> Result<(WorldStateMgrLikeCpp, WorldStateMgrLoadReportLikeCpp)> {
    let mut template_rows = Vec::new();
    let stmt = world_db.prepare(WorldStatements::SEL_WORLD_STATES);
    let mut result = world_db.query(&stmt).await?;
    if !result.is_empty() {
        loop {
            template_rows.push(WorldStateDbTemplateRowLikeCpp {
                id: result.read(0),
                default_value: result.read(1),
                map_ids_csv: result.try_read(2).unwrap_or_default(),
                area_ids_csv: result.try_read(3).unwrap_or_default(),
                script_name: result.try_read(4).unwrap_or_default(),
            });
            if !result.next_row() {
                break;
            }
        }
    }

    let mut saved_values = Vec::new();
    let stmt = character_db.prepare(CharStatements::SEL_WORLD_STATE_VALUES);
    let mut result = character_db.query(&stmt).await?;
    if !result.is_empty() {
        loop {
            saved_values.push((result.read(0), result.read(1)));
            if !result.next_row() {
                break;
            }
        }
    }

    Ok(WorldStateMgrLikeCpp::from_db_rows_like_cpp(
        template_rows,
        saved_values,
        |map_id| {
            u32::try_from(map_id)
                .ok()
                .is_some_and(|map_id| map_store.get(map_id).is_some())
        },
        |area_id| area_table_store.get(area_id).map(|area| area.continent_id),
    ))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventConditionApplyOutcomeLikeCpp {
    Loaded,
    OutOfRangeEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventConditionSaveApplyOutcomeLikeCpp {
    Loaded,
    OutOfRangeEvent,
    MissingCondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventConditionCheckOutcomeLikeCpp {
    Completed(GameEventConditionCheckSummaryLikeCpp),
    NotCompleted {
        event_id: u16,
        blocking_condition_id: u32,
    },
    MissingEvent {
        event_id: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventConditionCheckSummaryLikeCpp {
    pub event_id: u16,
    pub condition_count: usize,
    pub state_before_raw: u8,
    pub state_after_raw: u8,
    pub next_start_before: u64,
    pub next_start_after: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventQuestConditionRecordLikeCpp {
    pub quest_id: u32,
    pub event_id: u16,
    pub condition_id: u32,
    pub num: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameEventQuestCompleteOutcomeLikeCpp {
    MissingQuestMapping { quest_id: u32 },
    Progress(GameEventConditionProgressOutcomeLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameEventConditionProgressOutcomeLikeCpp {
    Progressed(GameEventConditionProgressSummaryLikeCpp),
    MissingEvent {
        event_id: u16,
    },
    InactiveEvent {
        event_id: u16,
    },
    NotWorldConditions {
        event_id: u16,
        state_raw: u8,
    },
    MissingCondition {
        event_id: u16,
        condition_id: u32,
    },
    AlreadyComplete {
        event_id: u16,
        condition_id: u32,
        done: f32,
        req_num: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventConditionProgressSummaryLikeCpp {
    pub event_id: u16,
    pub condition_id: u32,
    pub done_before: f32,
    pub done_after: f32,
    pub req_num: f32,
    pub persistence_event_id: u8,
    pub completed_event: bool,
    pub check_outcome: GameEventConditionCheckOutcomeLikeCpp,
    pub save_world_event_state_requested: bool,
    pub force_game_event_update_requested: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameEventDataLikeCpp {
    pub event_id: u16,
    pub start: u64,
    pub end: u64,
    pub next_start: u64,
    pub occurence: u32,
    pub length: u32,
    pub holiday_id: u32,
    pub holiday_stage: u8,
    pub state_raw: u8,
    pub prerequisite_events: BTreeSet<u16>,
    pub conditions: BTreeMap<u32, GameEventConditionLikeCpp>,
    pub description: String,
    pub announce: u8,
}

impl Default for GameEventDataLikeCpp {
    fn default() -> Self {
        Self {
            event_id: 0,
            start: 1,
            end: 0,
            next_start: 0,
            occurence: 0,
            length: 0,
            holiday_id: 0,
            holiday_stage: 0,
            state_raw: GameEventStateLikeCpp::Normal as u8,
            prerequisite_events: BTreeSet::new(),
            conditions: BTreeMap::new(),
            description: String::new(),
            announce: 0,
        }
    }
}

#[allow(dead_code)]
impl GameEventDataLikeCpp {
    pub fn state_like_cpp(&self) -> Option<GameEventStateLikeCpp> {
        GameEventStateLikeCpp::from_raw_like_cpp(self.state_raw)
    }

    pub fn is_valid_like_cpp(&self) -> bool {
        self.length > 0 || self.state_raw > GameEventStateLikeCpp::Normal as u8
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GameEventDataStoreLikeCpp {
    events: Vec<GameEventDataLikeCpp>,
}

#[allow(dead_code)]
impl GameEventDataStoreLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    fn from_game_event_sizing_like_cpp(sizing: GameEventSizingLikeCpp) -> Self {
        let mut events = vec![GameEventDataLikeCpp::default(); sizing.master_slot_count_like_cpp()];
        for (event_id, event) in events.iter_mut().enumerate() {
            event.event_id = u16::try_from(event_id).unwrap_or(u16::MAX);
        }
        Self { events }
    }

    pub fn len_like_cpp(&self) -> usize {
        self.events.len()
    }

    pub fn event_like_cpp(&self, event_id: u16) -> Option<&GameEventDataLikeCpp> {
        self.events.get(usize::from(event_id))
    }

    pub fn prerequisite_events_like_cpp(&self, event_id: u16) -> Option<&BTreeSet<u16>> {
        self.event_like_cpp(event_id)
            .map(|event| &event.prerequisite_events)
    }

    pub fn insert_prerequisite_event_like_cpp(
        &mut self,
        event_id: u16,
        prerequisite_event: u32,
    ) -> GameEventPrerequisiteInsertOutcomeLikeCpp {
        let event_index = usize::from(event_id);
        if event_index >= self.events.len() {
            return GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangeEvent;
        }

        let state_raw = self.events[event_index].state_raw;
        if state_raw == GameEventStateLikeCpp::Normal as u8
            || state_raw == GameEventStateLikeCpp::Internal as u8
        {
            return GameEventPrerequisiteInsertOutcomeLikeCpp::NonWorldEvent;
        }

        let Ok(prerequisite_event_id) = u16::try_from(prerequisite_event) else {
            return GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangePrerequisite;
        };
        if usize::from(prerequisite_event_id) >= self.events.len() {
            return GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangePrerequisite;
        }

        if self.events[event_index]
            .prerequisite_events
            .insert(prerequisite_event_id)
        {
            GameEventPrerequisiteInsertOutcomeLikeCpp::Loaded
        } else {
            GameEventPrerequisiteInsertOutcomeLikeCpp::Duplicate
        }
    }

    pub fn check_one_game_event_like_cpp(
        &self,
        event_id: u16,
        current_time_secs: u64,
    ) -> GameEventCheckOutcomeLikeCpp {
        let Some(event) = self.event_like_cpp(event_id) else {
            return GameEventCheckOutcomeLikeCpp::MissingEvent { event_id };
        };

        match event.state_like_cpp() {
            Some(
                GameEventStateLikeCpp::WorldConditions | GameEventStateLikeCpp::WorldNextPhase,
            ) => GameEventCheckOutcomeLikeCpp::Active(true),
            Some(GameEventStateLikeCpp::WorldFinished | GameEventStateLikeCpp::Internal) => {
                GameEventCheckOutcomeLikeCpp::Active(false)
            }
            Some(GameEventStateLikeCpp::WorldInactive) => {
                if event.prerequisite_events.is_empty() {
                    return GameEventCheckOutcomeLikeCpp::Active(false);
                }

                for &prerequisite_event_id in &event.prerequisite_events {
                    let Some(prerequisite_event) = self.event_like_cpp(prerequisite_event_id)
                    else {
                        return GameEventCheckOutcomeLikeCpp::MissingPrerequisite {
                            event_id: prerequisite_event_id,
                        };
                    };
                    let prerequisite_state = prerequisite_event.state_like_cpp();
                    let prerequisite_done = matches!(
                        prerequisite_state,
                        Some(
                            GameEventStateLikeCpp::WorldNextPhase
                                | GameEventStateLikeCpp::WorldFinished
                        )
                    );
                    if !prerequisite_done || prerequisite_event.next_start > current_time_secs {
                        return GameEventCheckOutcomeLikeCpp::Active(false);
                    }
                }

                GameEventCheckOutcomeLikeCpp::Active(true)
            }
            Some(GameEventStateLikeCpp::Normal) | None => {
                Self::check_periodic_window_like_cpp(event, current_time_secs)
            }
        }
    }

    pub fn last_start_time_like_cpp(&self, event_id: u16, current_time_secs: u64) -> u64 {
        let Some(event) = self.event_like_cpp(event_id) else {
            return 0;
        };
        if event.state_like_cpp() != Some(GameEventStateLikeCpp::Normal) {
            return 0;
        }
        let Some(period_secs) = periodic_occurence_secs_like_cpp(event.occurence) else {
            return 0;
        };
        current_time_secs
            .saturating_sub(current_time_secs.saturating_sub(event.start) % period_secs)
    }

    pub fn next_check_like_cpp(
        &self,
        event_id: u16,
        current_time_secs: u64,
    ) -> GameEventNextCheckOutcomeLikeCpp {
        let Some(event) = self.event_like_cpp(event_id) else {
            return GameEventNextCheckOutcomeLikeCpp::MissingEvent { event_id };
        };

        if matches!(
            event.state_like_cpp(),
            Some(GameEventStateLikeCpp::WorldNextPhase | GameEventStateLikeCpp::WorldFinished)
        ) && event.next_start >= current_time_secs
        {
            return GameEventNextCheckOutcomeLikeCpp::DelaySecs(
                event.next_start.saturating_sub(current_time_secs),
            );
        }

        if event.state_like_cpp() == Some(GameEventStateLikeCpp::WorldConditions) {
            return if event.length != 0 {
                GameEventNextCheckOutcomeLikeCpp::DelaySecs(
                    u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
                )
            } else {
                GameEventNextCheckOutcomeLikeCpp::DelaySecs(
                    MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP,
                )
            };
        }

        if current_time_secs > event.end {
            return GameEventNextCheckOutcomeLikeCpp::DelaySecs(
                MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP,
            );
        }

        if event.start > current_time_secs {
            return GameEventNextCheckOutcomeLikeCpp::DelaySecs(event.start - current_time_secs);
        }

        let Some(period_secs) = periodic_occurence_secs_like_cpp(event.occurence) else {
            return GameEventNextCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { event_id };
        };
        let length_secs = u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP);
        let elapsed_in_period = current_time_secs.saturating_sub(event.start) % period_secs;
        let delay = if elapsed_in_period < length_secs {
            length_secs.saturating_sub(elapsed_in_period)
        } else {
            period_secs.saturating_sub(elapsed_in_period)
        };
        let end_delay = event.end.saturating_sub(current_time_secs);
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(
            if event.end < current_time_secs.saturating_add(delay) {
                end_delay
            } else {
                delay
            },
        )
    }

    pub fn apply_game_event_condition_row_like_cpp(
        &mut self,
        event_id: u16,
        condition_id: u32,
        req_num: f32,
        max_world_state: u16,
        done_world_state: u16,
    ) -> GameEventConditionApplyOutcomeLikeCpp {
        let Some(event) = self.event_mut_like_cpp(event_id) else {
            return GameEventConditionApplyOutcomeLikeCpp::OutOfRangeEvent;
        };

        event.conditions.insert(
            condition_id,
            GameEventConditionLikeCpp {
                req_num,
                done: 0.0,
                max_world_state,
                done_world_state,
            },
        );
        GameEventConditionApplyOutcomeLikeCpp::Loaded
    }

    pub fn apply_game_event_condition_save_row_like_cpp(
        &mut self,
        event_id: u16,
        condition_id: u32,
        done: f32,
    ) -> GameEventConditionSaveApplyOutcomeLikeCpp {
        let Some(event) = self.event_mut_like_cpp(event_id) else {
            return GameEventConditionSaveApplyOutcomeLikeCpp::OutOfRangeEvent;
        };
        let Some(condition) = event.conditions.get_mut(&condition_id) else {
            return GameEventConditionSaveApplyOutcomeLikeCpp::MissingCondition;
        };

        condition.done = done;
        GameEventConditionSaveApplyOutcomeLikeCpp::Loaded
    }

    pub fn send_world_state_update_evidence_like_cpp(
        &self,
        event_id: u16,
    ) -> GameEventWorldStateUpdateOutcomeLikeCpp {
        let Some(event) = self.event_like_cpp(event_id) else {
            return GameEventWorldStateUpdateOutcomeLikeCpp::MissingEvent { event_id };
        };

        let mut updates = Vec::new();
        let mut skipped = Vec::new();
        for (&condition_id, condition) in &event.conditions {
            if condition.done_world_state != 0 {
                push_game_event_world_state_update_like_cpp(
                    event_id,
                    condition_id,
                    u32::from(condition.done_world_state),
                    condition.done,
                    GameEventWorldStateUpdateSourceLikeCpp::Done,
                    &mut updates,
                    &mut skipped,
                );
            }
            if condition.max_world_state != 0 {
                push_game_event_world_state_update_like_cpp(
                    event_id,
                    condition_id,
                    u32::from(condition.max_world_state),
                    condition.req_num,
                    GameEventWorldStateUpdateSourceLikeCpp::Max,
                    &mut updates,
                    &mut skipped,
                );
            }
        }

        GameEventWorldStateUpdateOutcomeLikeCpp::Updates {
            event_id,
            updates,
            skipped,
        }
    }

    pub fn check_one_game_event_conditions_like_cpp(
        &mut self,
        event_id: u16,
        current_time_secs: u64,
    ) -> GameEventConditionCheckOutcomeLikeCpp {
        let Some(event) = self.event_mut_like_cpp(event_id) else {
            return GameEventConditionCheckOutcomeLikeCpp::MissingEvent { event_id };
        };

        for (&condition_id, condition) in &event.conditions {
            if condition.done < condition.req_num {
                return GameEventConditionCheckOutcomeLikeCpp::NotCompleted {
                    event_id,
                    blocking_condition_id: condition_id,
                };
            }
        }

        let state_before_raw = event.state_raw;
        let next_start_before = event.next_start;
        event.state_raw = GameEventStateLikeCpp::WorldNextPhase as u8;
        if event.next_start == 0 {
            event.next_start = current_time_secs.saturating_add(
                u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
            );
        }

        GameEventConditionCheckOutcomeLikeCpp::Completed(GameEventConditionCheckSummaryLikeCpp {
            event_id,
            condition_count: event.conditions.len(),
            state_before_raw,
            state_after_raw: event.state_raw,
            next_start_before,
            next_start_after: event.next_start,
        })
    }

    fn check_periodic_window_like_cpp(
        event: &GameEventDataLikeCpp,
        current_time_secs: u64,
    ) -> GameEventCheckOutcomeLikeCpp {
        if !(event.start < current_time_secs && current_time_secs < event.end) {
            return GameEventCheckOutcomeLikeCpp::Active(false);
        }
        let Some(period_secs) = periodic_occurence_secs_like_cpp(event.occurence) else {
            return GameEventCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence {
                event_id: event.event_id,
            };
        };
        let length_secs = u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP);
        let elapsed_in_period = current_time_secs.saturating_sub(event.start) % period_secs;
        GameEventCheckOutcomeLikeCpp::Active(elapsed_in_period < length_secs)
    }

    pub fn iter_like_cpp(&self) -> impl Iterator<Item = &GameEventDataLikeCpp> {
        self.events.iter()
    }

    fn event_mut_like_cpp(&mut self, event_id: u16) -> Option<&mut GameEventDataLikeCpp> {
        self.events.get_mut(usize::from(event_id))
    }

    #[cfg(test)]
    pub(crate) fn with_event_like_cpp(mut self, event: GameEventDataLikeCpp) -> Self {
        if let Some(slot) = self.event_mut_like_cpp(event.event_id) {
            *slot = event;
        }
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventActiveSetLikeCpp {
    active_events: BTreeSet<u16>,
}

#[allow(dead_code)]
impl GameEventActiveSetLikeCpp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_active_event_like_cpp(&mut self, event_id: u16) -> bool {
        self.active_events.insert(event_id)
    }

    pub fn remove_active_event_like_cpp(&mut self, event_id: u16) -> bool {
        self.active_events.remove(&event_id)
    }

    pub fn clear_active_events_like_cpp(&mut self) {
        self.active_events.clear();
    }

    pub fn is_active_event_like_cpp(&self, event_id: u16) -> bool {
        self.active_events.contains(&event_id)
    }

    pub fn active_event_ids_like_cpp(&self) -> impl Iterator<Item = u16> + '_ {
        self.active_events.iter().copied()
    }

    pub fn is_holiday_active_like_cpp(
        &self,
        events: &GameEventDataStoreLikeCpp,
        holiday_id: u32,
    ) -> GameEventHolidayActiveOutcomeLikeCpp {
        if holiday_id == 0 {
            return GameEventHolidayActiveOutcomeLikeCpp::Active(false);
        }

        for event_id in self.active_event_ids_like_cpp() {
            let Some(event) = events.event_like_cpp(event_id) else {
                return GameEventHolidayActiveOutcomeLikeCpp::MissingActiveEvent { event_id };
            };

            if event.holiday_id == holiday_id {
                return GameEventHolidayActiveOutcomeLikeCpp::Active(true);
            }
        }

        GameEventHolidayActiveOutcomeLikeCpp::Active(false)
    }
}

fn periodic_occurence_secs_like_cpp(occurence_minutes: u32) -> Option<u64> {
    (occurence_minutes != 0)
        .then(|| u64::from(occurence_minutes).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP))
}

fn push_game_event_world_state_update_like_cpp(
    event_id: u16,
    condition_id: u32,
    variable_id: u32,
    raw_value: f32,
    source: GameEventWorldStateUpdateSourceLikeCpp,
    updates: &mut Vec<GameEventWorldStateUpdateEvidenceLikeCpp>,
    skipped: &mut Vec<GameEventWorldStateUpdateSkipLikeCpp>,
) {
    match world_state_value_i32_like_cpp(raw_value) {
        Ok(value) => updates.push(GameEventWorldStateUpdateEvidenceLikeCpp {
            event_id,
            condition_id,
            variable_id,
            value,
            source,
        }),
        Err(reason) => skipped.push(GameEventWorldStateUpdateSkipLikeCpp {
            event_id,
            condition_id,
            variable_id,
            source,
            reason,
        }),
    }
}

fn world_state_value_i32_like_cpp(
    raw_value: f32,
) -> Result<i32, GameEventWorldStateValueSkipReasonLikeCpp> {
    if !raw_value.is_finite() {
        return Err(GameEventWorldStateValueSkipReasonLikeCpp::NonFinite);
    }
    if raw_value < 0.0 {
        return Err(GameEventWorldStateValueSkipReasonLikeCpp::Negative);
    }
    let truncated = raw_value.trunc();
    if f64::from(truncated) > f64::from(i32::MAX) {
        return Err(GameEventWorldStateValueSkipReasonLikeCpp::OutOfI32Range);
    }
    Ok(truncated as i32)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GameEventDataRowLikeCpp {
    event_id: u16,
    start: u64,
    end: u64,
    occurence: u32,
    length: u32,
    holiday_id: u32,
    holiday_stage: u8,
    description: String,
    state_raw: u8,
    announce: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GameEventPrerequisiteRowLikeCpp {
    event_id: u16,
    prerequisite_event: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GameEventConditionRowLikeCpp {
    event_id: u16,
    condition_id: u32,
    req_num: f32,
    max_world_state: u16,
    done_world_state: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GameEventConditionSaveRowLikeCpp {
    event_id: u16,
    condition_id: u32,
    done: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GameEventQuestConditionRowLikeCpp {
    quest_id: u32,
    event_id: u16,
    condition_id: u32,
    num: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventPoolIdsLikeCpp {
    game_event_size: i32,
    pool_ids_by_internal_event_id: Vec<Vec<u32>>,
}

impl GameEventPoolIdsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    fn from_game_event_sizing_like_cpp(sizing: GameEventSizingLikeCpp) -> Self {
        Self {
            game_event_size: sizing.game_event_size,
            pool_ids_by_internal_event_id: vec![Vec::new(); sizing.slot_count],
        }
    }

    pub fn game_event_size_like_cpp(&self) -> i32 {
        self.game_event_size
    }

    pub fn internal_event_id_like_cpp(&self, event_id: i16) -> Option<usize> {
        let internal_event_id = self.game_event_size + i32::from(event_id) - 1;
        let index = usize::try_from(internal_event_id).ok()?;
        (index < self.pool_ids_by_internal_event_id.len()).then_some(index)
    }

    pub fn pool_ids_like_cpp(&self, event_id: i16) -> Option<&[u32]> {
        self.internal_event_id_like_cpp(event_id)
            .and_then(|index| self.pool_ids_by_internal_event_id.get(index))
            .map(Vec::as_slice)
    }

    #[cfg(test)]
    pub fn with_pool_ids_for_event_like_cpp(
        mut self,
        event_id: i16,
        pool_ids: impl IntoIterator<Item = u32>,
    ) -> Self {
        if let Some(index) = self.internal_event_id_like_cpp(event_id) {
            self.pool_ids_by_internal_event_id[index].extend(pool_ids);
        }
        self
    }

    fn push_pool_id_like_cpp(&mut self, event_id: i16, pool_id: u32) -> bool {
        let Some(index) = self.internal_event_id_like_cpp(event_id) else {
            return false;
        };
        self.pool_ids_by_internal_event_id[index].push(pool_id);
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventSpawnGuidsLikeCpp {
    game_event_size: i32,
    creature_guids_by_internal_event_id: Vec<Vec<SpawnId>>,
    gameobject_guids_by_internal_event_id: Vec<Vec<SpawnId>>,
}

impl GameEventSpawnGuidsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    fn from_game_event_sizing_like_cpp(sizing: GameEventSizingLikeCpp) -> Self {
        Self {
            game_event_size: sizing.game_event_size,
            creature_guids_by_internal_event_id: vec![Vec::new(); sizing.slot_count],
            gameobject_guids_by_internal_event_id: vec![Vec::new(); sizing.slot_count],
        }
    }

    pub fn game_event_size_like_cpp(&self) -> i32 {
        self.game_event_size
    }

    pub fn internal_event_id_like_cpp(&self, event_id: i16) -> Option<usize> {
        let internal_event_id = self.game_event_size + i32::from(event_id) - 1;
        let index = usize::try_from(internal_event_id).ok()?;
        (index < self.creature_guids_by_internal_event_id.len()).then_some(index)
    }

    pub fn creature_guids_like_cpp(&self, event_id: i16) -> Option<&[SpawnId]> {
        self.internal_event_id_like_cpp(event_id)
            .and_then(|index| self.creature_guids_by_internal_event_id.get(index))
            .map(Vec::as_slice)
    }

    pub fn gameobject_guids_like_cpp(&self, event_id: i16) -> Option<&[SpawnId]> {
        self.internal_event_id_like_cpp(event_id)
            .and_then(|index| self.gameobject_guids_by_internal_event_id.get(index))
            .map(Vec::as_slice)
    }

    pub(crate) fn push_guid_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        event_id: i16,
        guid: SpawnId,
    ) -> bool {
        let Some(index) = self.internal_event_id_like_cpp(event_id) else {
            return false;
        };
        match object_type {
            SpawnObjectType::Creature => self.creature_guids_by_internal_event_id[index].push(guid),
            SpawnObjectType::GameObject => {
                self.gameobject_guids_by_internal_event_id[index].push(guid);
            }
            SpawnObjectType::AreaTrigger => return false,
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn truncate_gameobject_guid_buckets_for_test_like_cpp(
        mut self,
        bucket_count: usize,
    ) -> Self {
        self.gameobject_guids_by_internal_event_id
            .truncate(bucket_count);
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameEventModelEquipRecordLikeCpp {
    pub spawn_id: SpawnId,
    pub model_id: u32,
    pub model_id_prev: u32,
    pub equipment_id: u8,
    /// C++ member is spelled `equipement_id_prev`; Rust keeps the corrected field name.
    pub equipment_id_prev: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventModelEquipLikeCpp {
    records_by_event_id: Vec<Vec<GameEventModelEquipRecordLikeCpp>>,
}

impl GameEventModelEquipLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    fn from_game_event_sizing_like_cpp(sizing: GameEventSizingLikeCpp) -> Self {
        Self {
            records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
        }
    }

    pub fn records_like_cpp(&self, event_id: u16) -> Option<&[GameEventModelEquipRecordLikeCpp]> {
        self.records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub fn records_mut_like_cpp(
        &mut self,
        event_id: u16,
    ) -> Option<&mut [GameEventModelEquipRecordLikeCpp]> {
        self.records_by_event_id
            .get_mut(usize::from(event_id))
            .map(Vec::as_mut_slice)
    }

    fn push_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventModelEquipRecordLikeCpp,
    ) -> bool {
        let Some(records) = self.records_by_event_id.get_mut(usize::from(event_id)) else {
            return false;
        };
        records.push(record);
        true
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameEventNpcFlagRecordLikeCpp {
    pub spawn_id: SpawnId,
    pub npcflag: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcFlagsLikeCpp {
    records_by_event_id: Vec<Vec<GameEventNpcFlagRecordLikeCpp>>,
}

#[allow(dead_code)]
impl GameEventNpcFlagsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    fn from_game_event_sizing_like_cpp(sizing: GameEventSizingLikeCpp) -> Self {
        Self {
            records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
        }
    }

    pub fn records_like_cpp(&self, event_id: u16) -> Option<&[GameEventNpcFlagRecordLikeCpp]> {
        self.records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub fn push_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventNpcFlagRecordLikeCpp,
    ) -> bool {
        let Some(records) = self.records_by_event_id.get_mut(usize::from(event_id)) else {
            return false;
        };
        records.push(record);
        true
    }

    pub fn game_event_npc_flag_mask_like_cpp(
        &self,
        spawn_id: SpawnId,
        active_event_ids: &[u16],
    ) -> u64 {
        let mut mask = 0_u64;
        for event_id in active_event_ids {
            let Some(records) = self.records_like_cpp(*event_id) else {
                continue;
            };
            for record in records {
                if record.spawn_id == spawn_id {
                    mask |= record.npcflag;
                }
            }
        }
        mask
    }
}

/// C++ `GameEventMgr.h` `QuestRelation(id, quest)` metadata for GameEvent quest givers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventQuestRelationRecordLikeCpp {
    pub giver_id: u32,
    pub quest_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestRelationsLikeCpp {
    creature_records_by_event_id: Vec<Vec<GameEventQuestRelationRecordLikeCpp>>,
    gameobject_records_by_event_id: Vec<Vec<GameEventQuestRelationRecordLikeCpp>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestRelationCacheUpdateSummaryLikeCpp {
    pub event_id: u16,
    pub activate: bool,
    pub creature_records_seen: usize,
    pub gameobject_records_seen: usize,
    pub creature_inserted: usize,
    pub gameobject_inserted: usize,
    pub creature_removed: usize,
    pub gameobject_removed: usize,
    pub creature_remove_misses: usize,
    pub gameobject_remove_misses: usize,
    pub creature_no_match: usize,
    pub gameobject_no_match: usize,
    pub creature_missing_event_bucket: bool,
    pub gameobject_missing_event_bucket: bool,
    pub creature_skipped_active_other_event: usize,
    pub gameobject_skipped_active_other_event: usize,
}

#[allow(dead_code)]
impl GameEventQuestRelationsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    fn from_game_event_sizing_like_cpp(sizing: GameEventSizingLikeCpp) -> Self {
        Self {
            creature_records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
            gameobject_records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
        }
    }

    pub fn creature_records_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventQuestRelationRecordLikeCpp]> {
        self.creature_records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub fn gameobject_records_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventQuestRelationRecordLikeCpp]> {
        self.gameobject_records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub(crate) fn push_creature_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventQuestRelationRecordLikeCpp,
    ) -> bool {
        let Some(records) = self
            .creature_records_by_event_id
            .get_mut(usize::from(event_id))
        else {
            return false;
        };
        records.push(record);
        true
    }

    pub(crate) fn push_gameobject_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventQuestRelationRecordLikeCpp,
    ) -> bool {
        let Some(records) = self
            .gameobject_records_by_event_id
            .get_mut(usize::from(event_id))
        else {
            return false;
        };
        records.push(record);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GameEventQuestRelationRowLikeCpp {
    event_id: u8,
    giver_id: u32,
    quest_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEventNpcVendorRecordLikeCpp {
    pub spawn_id: SpawnId,
    pub guid: SpawnId,
    pub entry: u32,
    pub item: u32,
    pub maxcount: u32,
    pub incrtime: u32,
    pub extended_cost: u32,
    pub vendor_type: u8,
    pub item_type: u8,
    pub bonus_list_ids: Vec<i32>,
    pub player_condition_id: u32,
    pub ignore_filtering: bool,
    pub event_npc_flag_low32: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcVendorsLikeCpp {
    records_by_event_id: Vec<Vec<GameEventNpcVendorRecordLikeCpp>>,
}

#[allow(dead_code)]
impl GameEventNpcVendorsLikeCpp {
    pub fn from_game_event_max_entry_like_cpp(max_event_entry: Option<u32>) -> Self {
        Self::from_game_event_sizing_like_cpp(
            GameEventSizingLikeCpp::from_max_event_entry_like_cpp(max_event_entry),
        )
    }

    fn from_game_event_sizing_like_cpp(sizing: GameEventSizingLikeCpp) -> Self {
        Self {
            records_by_event_id: vec![Vec::new(); sizing.master_slot_count_like_cpp()],
        }
    }

    pub fn records_like_cpp(&self, event_id: u16) -> Option<&[GameEventNpcVendorRecordLikeCpp]> {
        self.records_by_event_id
            .get(usize::from(event_id))
            .map(Vec::as_slice)
    }

    pub fn records_for_entry_like_cpp(
        &self,
        event_id: u16,
        entry: u32,
    ) -> Option<Vec<&GameEventNpcVendorRecordLikeCpp>> {
        self.records_like_cpp(event_id).map(|records| {
            records
                .iter()
                .filter(|record| record.entry == entry)
                .collect()
        })
    }

    pub(crate) fn push_record_like_cpp(
        &mut self,
        event_id: u16,
        record: GameEventNpcVendorRecordLikeCpp,
    ) -> bool {
        let Some(records) = self.records_by_event_id.get_mut(usize::from(event_id)) else {
            return false;
        };
        records.push(record);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GameEventNpcVendorRowLikeCpp {
    event_id: u8,
    spawn_id: SpawnId,
    item: u32,
    maxcount: u32,
    incrtime: u32,
    extended_cost: u32,
    vendor_type: u8,
    bonus_list_ids: String,
    player_condition_id: u32,
    ignore_filtering: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GameEventNpcFlagRowLikeCpp {
    spawn_id: SpawnId,
    event_id: u16,
    npcflag: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GameEventModelEquipRowLikeCpp {
    spawn_id: SpawnId,
    entry: u32,
    event_id: u16,
    model_id: u32,
    equipment_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEventModelEquipBaselineRecordOutcomeLikeCpp {
    Applied {
        spawn_id: SpawnId,
        model_id_prev: u32,
        equipment_id_prev: u8,
        model_id_after: u32,
        equipment_id_after: u8,
    },
    MissingSpawnMetadata {
        spawn_id: SpawnId,
    },
    MissingCreatureRuntimeRow {
        spawn_id: SpawnId,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventModelEquipBaselineChangeSummaryLikeCpp {
    pub event_id: u16,
    pub activate: bool,
    pub records_seen: usize,
    pub records_applied: usize,
    pub missing_event_bucket: bool,
    pub missing_spawn_metadata: usize,
    pub missing_creature_runtime_rows: usize,
    pub record_outcomes: Vec<GameEventModelEquipBaselineRecordOutcomeLikeCpp>,
}

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

    pub fn clear_active_game_events_like_cpp(&mut self) {
        self.game_event_active_set.clear_active_events_like_cpp();
    }

    pub fn represented_handle_game_event_quest_complete_like_cpp(
        &mut self,
        quest_id: u32,
        current_time_secs: u64,
    ) -> GameEventQuestCompleteOutcomeLikeCpp {
        let Some(record) = self
            .game_event_quest_conditions_by_quest
            .get(&quest_id)
            .copied()
        else {
            return GameEventQuestCompleteOutcomeLikeCpp::MissingQuestMapping { quest_id };
        };

        GameEventQuestCompleteOutcomeLikeCpp::Progress(
            self.represented_update_game_event_condition_progress_like_cpp(
                record.event_id,
                record.condition_id,
                record.num,
                current_time_secs,
            ),
        )
    }

    pub fn represented_update_game_event_condition_progress_like_cpp(
        &mut self,
        event_id: u16,
        condition_id: u32,
        num: f32,
        current_time_secs: u64,
    ) -> GameEventConditionProgressOutcomeLikeCpp {
        let Some(event) = self.game_events.event_like_cpp(event_id) else {
            return GameEventConditionProgressOutcomeLikeCpp::MissingEvent { event_id };
        };
        if !self
            .game_event_active_set
            .is_active_event_like_cpp(event_id)
        {
            return GameEventConditionProgressOutcomeLikeCpp::InactiveEvent { event_id };
        }
        if event.state_raw != GameEventStateLikeCpp::WorldConditions as u8 {
            return GameEventConditionProgressOutcomeLikeCpp::NotWorldConditions {
                event_id,
                state_raw: event.state_raw,
            };
        }
        let Some(condition) = event.conditions.get(&condition_id).copied() else {
            return GameEventConditionProgressOutcomeLikeCpp::MissingCondition {
                event_id,
                condition_id,
            };
        };
        if condition.done >= condition.req_num {
            return GameEventConditionProgressOutcomeLikeCpp::AlreadyComplete {
                event_id,
                condition_id,
                done: condition.done,
                req_num: condition.req_num,
            };
        }

        let done_before = condition.done;
        let done_after = (condition.done + num).min(condition.req_num);
        if let Some(event) = self.game_events.event_mut_like_cpp(event_id) {
            if let Some(condition) = event.conditions.get_mut(&condition_id) {
                condition.done = done_after;
            }
        }

        let check_outcome = self
            .game_events
            .check_one_game_event_conditions_like_cpp(event_id, current_time_secs);
        let completed_event = matches!(
            check_outcome,
            GameEventConditionCheckOutcomeLikeCpp::Completed(_)
        );
        let event_id_param = u8::try_from(event_id & 0x00ff).unwrap_or(0);

        GameEventConditionProgressOutcomeLikeCpp::Progressed(
            GameEventConditionProgressSummaryLikeCpp {
                event_id,
                condition_id,
                done_before,
                done_after,
                req_num: condition.req_num,
                persistence_event_id: event_id_param,
                completed_event,
                check_outcome,
                save_world_event_state_requested: completed_event,
                force_game_event_update_requested: completed_event,
            },
        )
    }

    pub fn start_game_event_like_cpp(
        &mut self,
        event_id: u16,
        overwrite: bool,
        current_time_secs: u64,
        world_conditions_met: bool,
    ) -> GameEventStartOutcomeLikeCpp {
        let Some(event) = self.game_events.event_mut_like_cpp(event_id) else {
            return GameEventStartOutcomeLikeCpp::MissingEvent { event_id };
        };

        let state_before_raw = event.state_raw;
        let normal_or_internal = state_before_raw == GameEventStateLikeCpp::Normal as u8
            || state_before_raw == GameEventStateLikeCpp::Internal as u8;

        if normal_or_internal {
            let active_added = self
                .game_event_active_set
                .add_active_event_like_cpp(event_id);
            if overwrite {
                event.start = current_time_secs;
                if event.end <= event.start {
                    event.end = event.start.saturating_add(u64::from(event.length));
                }
            }
            return GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
                event_id,
                state_before_raw,
                state_after_raw: event.state_raw,
                active_added,
                active_was_present: !active_added,
                apply_new_event_requested: true,
                save_world_event_state_requested: false,
                force_game_event_update_requested: false,
                completed: false,
            });
        }

        if event.state_raw == GameEventStateLikeCpp::WorldInactive as u8 {
            event.state_raw = GameEventStateLikeCpp::WorldConditions as u8;
        }

        let active_added = self
            .game_event_active_set
            .add_active_event_like_cpp(event_id);
        if world_conditions_met {
            event.state_raw = GameEventStateLikeCpp::WorldNextPhase as u8;
            if event.next_start == 0 {
                event.next_start = current_time_secs.saturating_add(
                    u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
                );
            }
        }

        GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
            event_id,
            state_before_raw,
            state_after_raw: event.state_raw,
            active_added,
            active_was_present: !active_added,
            apply_new_event_requested: true,
            save_world_event_state_requested: true,
            force_game_event_update_requested: overwrite && world_conditions_met,
            completed: world_conditions_met,
        })
    }

    pub fn stop_game_event_like_cpp(
        &mut self,
        event_id: u16,
        overwrite: bool,
        current_time_secs: u64,
    ) -> GameEventStopOutcomeLikeCpp {
        let Some(event) = self.game_events.event_mut_like_cpp(event_id) else {
            return GameEventStopOutcomeLikeCpp::MissingEvent { event_id };
        };

        let state_before_raw = event.state_raw;
        let serverwide = state_before_raw != GameEventStateLikeCpp::Normal as u8
            && state_before_raw != GameEventStateLikeCpp::Internal as u8;
        let active_removed = self
            .game_event_active_set
            .remove_active_event_like_cpp(event_id);
        let mut condition_reset_requested = false;
        let mut delete_world_event_state_requested = false;
        let mut delete_condition_saves_requested = false;

        if overwrite && !serverwide {
            event.start = current_time_secs.saturating_sub(
                u64::from(event.length).saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
            );
            if event.end <= event.start {
                event.end = event.start.saturating_add(u64::from(event.length));
            }
        } else if serverwide
            && (overwrite || state_before_raw != GameEventStateLikeCpp::WorldFinished as u8)
        {
            event.next_start = 0;
            event.state_raw = GameEventStateLikeCpp::WorldInactive as u8;
            condition_reset_requested = true;
            delete_world_event_state_requested = true;
            delete_condition_saves_requested = true;
        }

        GameEventStopOutcomeLikeCpp::Stopped(GameEventStopSummaryLikeCpp {
            event_id,
            state_before_raw,
            state_after_raw: event.state_raw,
            active_removed,
            active_was_present: active_removed,
            unapply_event_requested: true,
            serverwide,
            condition_reset_requested,
            delete_world_event_state_requested,
            delete_condition_saves_requested,
        })
    }

    pub fn update_game_events_like_cpp<F>(
        &mut self,
        current_time_secs: u64,
        is_system_init: bool,
        mut world_conditions_met: F,
    ) -> GameEventUpdateOutcomeLikeCpp
    where
        F: FnMut(u16) -> bool,
    {
        let mut scanned_event_ids = Vec::new();
        let mut check_outcomes = Vec::new();
        let mut next_check_outcomes = Vec::new();
        let mut activate = BTreeSet::new();
        let mut deactivate = BTreeSet::new();
        let mut negative_spawn_event_ids = Vec::new();
        let mut world_nextphase_finished = Vec::new();
        let mut world_conditions_save_requested = Vec::new();
        let mut invalid_check_outcomes = Vec::new();
        let mut invalid_next_check_outcomes = Vec::new();
        let mut start_conditions_met = BTreeMap::new();
        let mut next_event_delay_secs = MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP;

        for event_index in 1..self.game_events.len_like_cpp() {
            let Ok(event_id) = u16::try_from(event_index) else {
                continue;
            };
            scanned_event_ids.push(event_id);

            let check_outcome = self
                .game_events
                .check_one_game_event_like_cpp(event_id, current_time_secs);
            check_outcomes.push((event_id, check_outcome));

            match check_outcome {
                GameEventCheckOutcomeLikeCpp::Active(true) => {
                    let active_before_queue = self
                        .game_event_active_set
                        .is_active_event_like_cpp(event_id);

                    let mut nextphase_finished = false;
                    if let Some(event) = self.game_events.event_mut_like_cpp(event_id) {
                        if event.state_raw == GameEventStateLikeCpp::WorldNextPhase as u8
                            && event.next_start <= current_time_secs
                        {
                            let state_before_raw = event.state_raw;
                            let next_start_before = event.next_start;
                            event.state_raw = GameEventStateLikeCpp::WorldFinished as u8;
                            event.next_start = 0;
                            world_nextphase_finished.push(GameEventWorldNextPhaseFinishedLikeCpp {
                                event_id,
                                was_active_before_queue: active_before_queue,
                                state_before_raw,
                                state_after_raw: event.state_raw,
                                next_start_before,
                                next_start_after: event.next_start,
                                save_state_requested: true,
                            });
                            if active_before_queue {
                                deactivate.insert(event_id);
                            }
                            nextphase_finished = true;
                        }
                    }
                    if nextphase_finished {
                        continue;
                    }

                    let mut condition_met_for_start = false;
                    let mut condition_checked_during_scan = false;
                    if let Some(event) = self.game_events.event_mut_like_cpp(event_id) {
                        if event.state_raw == GameEventStateLikeCpp::WorldConditions as u8 {
                            condition_checked_during_scan = true;
                            if world_conditions_met(event_id) {
                                event.state_raw = GameEventStateLikeCpp::WorldNextPhase as u8;
                                if event.next_start == 0 {
                                    event.next_start = current_time_secs.saturating_add(
                                        u64::from(event.length)
                                            .saturating_mul(GAME_EVENT_MINUTE_SECS_LIKE_CPP),
                                    );
                                }
                                world_conditions_save_requested.push(
                                    GameEventWorldStateSaveEvidenceLikeCpp {
                                        event_id,
                                        state_after_raw: event.state_raw,
                                        next_start_after: event.next_start,
                                    },
                                );
                                condition_met_for_start = true;
                            }
                        }
                    }
                    if condition_checked_during_scan {
                        start_conditions_met.insert(event_id, condition_met_for_start);
                    }

                    if !active_before_queue {
                        activate.insert(event_id);
                    }
                }
                GameEventCheckOutcomeLikeCpp::Active(false) => {
                    if self
                        .game_event_active_set
                        .is_active_event_like_cpp(event_id)
                    {
                        deactivate.insert(event_id);
                    } else if !is_system_init {
                        negative_spawn_event_ids.push(-i16::try_from(event_id).unwrap_or(i16::MAX));
                    }
                }
                invalid @ (GameEventCheckOutcomeLikeCpp::MissingEvent { .. }
                | GameEventCheckOutcomeLikeCpp::MissingPrerequisite { .. }
                | GameEventCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence { .. }) => {
                    invalid_check_outcomes.push(invalid);
                    continue;
                }
            }

            let next_check_outcome = self
                .game_events
                .next_check_like_cpp(event_id, current_time_secs);
            next_check_outcomes.push((event_id, next_check_outcome));
            match next_check_outcome {
                GameEventNextCheckOutcomeLikeCpp::DelaySecs(delay_secs) => {
                    next_event_delay_secs = next_event_delay_secs.min(delay_secs);
                }
                invalid @ (GameEventNextCheckOutcomeLikeCpp::MissingEvent { .. }
                | GameEventNextCheckOutcomeLikeCpp::InvalidTimingZeroOccurrence {
                    ..
                }) => {
                    invalid_next_check_outcomes.push(invalid);
                }
            }
        }

        let queued_activation_event_ids = activate.iter().copied().collect::<Vec<_>>();
        let queued_deactivation_event_ids = deactivate.iter().copied().collect::<Vec<_>>();

        let mut start_outcomes = Vec::new();
        for event_id in queued_activation_event_ids.iter().copied() {
            let start_outcome = self.start_game_event_like_cpp(
                event_id,
                false,
                current_time_secs,
                start_conditions_met
                    .get(&event_id)
                    .copied()
                    .unwrap_or_else(|| world_conditions_met(event_id)),
            );
            if matches!(
                start_outcome,
                GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
                    completed: true,
                    ..
                })
            ) {
                next_event_delay_secs = 0;
            }
            start_outcomes.push(start_outcome);
        }

        let mut stop_outcomes = Vec::new();
        for event_id in queued_deactivation_event_ids.iter().copied() {
            stop_outcomes.push(self.stop_game_event_like_cpp(event_id, false, current_time_secs));
        }

        GameEventUpdateOutcomeLikeCpp {
            current_time_secs,
            scanned_event_ids,
            check_outcomes,
            next_check_outcomes,
            queued_activation_event_ids,
            queued_deactivation_event_ids,
            start_outcomes,
            stop_outcomes,
            negative_spawn_event_ids,
            world_nextphase_finished,
            world_conditions_save_requested,
            invalid_check_outcomes,
            invalid_next_check_outcomes,
            next_event_delay_secs_before_padding: next_event_delay_secs,
            next_update_delay_millis: next_event_delay_secs
                .saturating_add(1)
                .saturating_mul(1_000),
        }
    }

    #[allow(dead_code)]
    pub fn game_event_like_cpp(&self, event_id: u16) -> Option<&GameEventDataLikeCpp> {
        self.game_events.event_like_cpp(event_id)
    }

    pub fn game_event_last_start_time_like_cpp(
        &self,
        event_id: u16,
        current_time_secs: u64,
    ) -> u64 {
        self.game_events
            .last_start_time_like_cpp(event_id, current_time_secs)
    }

    pub fn game_event_pool_ids_like_cpp(&self, event_id: i16) -> Option<&[u32]> {
        self.game_event_pools.pool_ids_like_cpp(event_id)
    }

    pub fn game_event_creature_guids_like_cpp(&self, event_id: i16) -> Option<&[SpawnId]> {
        self.game_event_spawn_guids
            .creature_guids_like_cpp(event_id)
    }

    pub fn game_event_gameobject_guids_like_cpp(&self, event_id: i16) -> Option<&[SpawnId]> {
        self.game_event_spawn_guids
            .gameobject_guids_like_cpp(event_id)
    }

    pub fn game_event_model_equip_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventModelEquipRecordLikeCpp]> {
        self.game_event_model_equip.records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_npc_flags_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventNpcFlagRecordLikeCpp]> {
        self.game_event_npc_flags.records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_creature_quests_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventQuestRelationRecordLikeCpp]> {
        self.game_event_quest_relations
            .creature_records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_gameobject_quests_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventQuestRelationRecordLikeCpp]> {
        self.game_event_quest_relations
            .gameobject_records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_npc_vendors_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<&[GameEventNpcVendorRecordLikeCpp]> {
        self.game_event_npc_vendors.records_like_cpp(event_id)
    }

    #[allow(dead_code)]
    pub fn game_event_npc_vendor_records_for_entry_like_cpp(
        &self,
        event_id: u16,
        entry: u32,
    ) -> Option<Vec<&GameEventNpcVendorRecordLikeCpp>> {
        self.game_event_npc_vendors
            .records_for_entry_like_cpp(event_id, entry)
    }

    pub fn game_event_active_npc_vendor_items_like_cpp(
        &self,
        entry: u32,
    ) -> &[GameEventNpcVendorRecordLikeCpp] {
        self.game_event_vendor_cache_by_entry
            .get(&entry)
            .map_or(&[], Vec::as_slice)
    }

    pub fn game_event_active_creature_quest_relations_like_cpp(
        &self,
        giver_id: u32,
    ) -> &[GameEventQuestRelationRecordLikeCpp] {
        self.game_event_active_creature_quest_relations_by_giver
            .get(&giver_id)
            .map_or(&[], Vec::as_slice)
    }

    pub fn game_event_active_gameobject_quest_relations_like_cpp(
        &self,
        giver_id: u32,
    ) -> &[GameEventQuestRelationRecordLikeCpp] {
        self.game_event_active_gameobject_quest_relations_by_giver
            .get(&giver_id)
            .map_or(&[], Vec::as_slice)
    }

    fn has_creature_quest_active_event_except_like_cpp(
        &self,
        quest_id: u32,
        event_id: u16,
    ) -> bool {
        self.game_event_active_set
            .active_event_ids_like_cpp()
            .filter(|active_event_id| *active_event_id != event_id)
            .any(|active_event_id| {
                self.game_event_quest_relations
                    .creature_records_like_cpp(active_event_id)
                    .is_some_and(|records| records.iter().any(|record| record.quest_id == quest_id))
            })
    }

    fn has_gameobject_quest_active_event_except_like_cpp(
        &self,
        quest_id: u32,
        event_id: u16,
    ) -> bool {
        self.game_event_active_set
            .active_event_ids_like_cpp()
            .filter(|active_event_id| *active_event_id != event_id)
            .any(|active_event_id| {
                self.game_event_quest_relations
                    .gameobject_records_like_cpp(active_event_id)
                    .is_some_and(|records| records.iter().any(|record| record.quest_id == quest_id))
            })
    }

    pub fn update_game_event_quest_relation_cache_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
    ) -> GameEventQuestRelationCacheUpdateSummaryLikeCpp {
        let mut summary = GameEventQuestRelationCacheUpdateSummaryLikeCpp {
            event_id,
            activate,
            ..GameEventQuestRelationCacheUpdateSummaryLikeCpp::default()
        };

        match self
            .game_event_quest_relations
            .creature_records_like_cpp(event_id)
            .map(<[_]>::to_vec)
        {
            Some(records) => self.update_game_event_creature_quest_relation_cache_records_like_cpp(
                event_id,
                activate,
                &records,
                &mut summary,
            ),
            None => summary.creature_missing_event_bucket = true,
        }

        match self
            .game_event_quest_relations
            .gameobject_records_like_cpp(event_id)
            .map(<[_]>::to_vec)
        {
            Some(records) => self
                .update_game_event_gameobject_quest_relation_cache_records_like_cpp(
                    event_id,
                    activate,
                    &records,
                    &mut summary,
                ),
            None => summary.gameobject_missing_event_bucket = true,
        }

        summary
    }

    fn update_game_event_creature_quest_relation_cache_records_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
        records: &[GameEventQuestRelationRecordLikeCpp],
        summary: &mut GameEventQuestRelationCacheUpdateSummaryLikeCpp,
    ) {
        summary.creature_records_seen = records.len();
        if activate {
            for record in records {
                self.game_event_active_creature_quest_relations_by_giver
                    .entry(record.giver_id)
                    .or_default()
                    .push(*record);
                summary.creature_inserted += 1;
            }
            return;
        }

        for record in records {
            if self.has_creature_quest_active_event_except_like_cpp(record.quest_id, event_id) {
                summary.creature_skipped_active_other_event += 1;
                continue;
            }
            let Some(active_records) = self
                .game_event_active_creature_quest_relations_by_giver
                .get_mut(&record.giver_id)
            else {
                summary.creature_remove_misses += 1;
                continue;
            };
            let Some(index) = active_records.iter().position(|active_record| {
                active_record.giver_id == record.giver_id
                    && active_record.quest_id == record.quest_id
            }) else {
                summary.creature_no_match += 1;
                continue;
            };
            active_records.remove(index);
            summary.creature_removed += 1;
            if active_records.is_empty() {
                self.game_event_active_creature_quest_relations_by_giver
                    .remove(&record.giver_id);
            }
        }
    }

    fn update_game_event_gameobject_quest_relation_cache_records_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
        records: &[GameEventQuestRelationRecordLikeCpp],
        summary: &mut GameEventQuestRelationCacheUpdateSummaryLikeCpp,
    ) {
        summary.gameobject_records_seen = records.len();
        if activate {
            for record in records {
                self.game_event_active_gameobject_quest_relations_by_giver
                    .entry(record.giver_id)
                    .or_default()
                    .push(*record);
                summary.gameobject_inserted += 1;
            }
            return;
        }

        for record in records {
            if self.has_gameobject_quest_active_event_except_like_cpp(record.quest_id, event_id) {
                summary.gameobject_skipped_active_other_event += 1;
                continue;
            }
            let Some(active_records) = self
                .game_event_active_gameobject_quest_relations_by_giver
                .get_mut(&record.giver_id)
            else {
                summary.gameobject_remove_misses += 1;
                continue;
            };
            let Some(index) = active_records.iter().position(|active_record| {
                active_record.giver_id == record.giver_id
                    && active_record.quest_id == record.quest_id
            }) else {
                summary.gameobject_no_match += 1;
                continue;
            };
            active_records.remove(index);
            summary.gameobject_removed += 1;
            if active_records.is_empty() {
                self.game_event_active_gameobject_quest_relations_by_giver
                    .remove(&record.giver_id);
            }
        }
    }

    pub fn update_game_event_npc_vendor_cache_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
    ) -> GameEventNpcVendorCacheUpdateSummaryLikeCpp {
        let mut summary = GameEventNpcVendorCacheUpdateSummaryLikeCpp {
            event_id,
            activate,
            ..GameEventNpcVendorCacheUpdateSummaryLikeCpp::default()
        };
        let Some(records) = self.game_event_npc_vendors.records_like_cpp(event_id) else {
            summary.missing_event_bucket = true;
            return summary;
        };
        summary.records_seen = records.len();

        if activate {
            for record in records {
                self.game_event_vendor_cache_by_entry
                    .entry(record.entry)
                    .or_default()
                    .push(record.clone());
                summary.items_added += 1;
            }
            return summary;
        }

        for record in records {
            let Some(cached_records) = self.game_event_vendor_cache_by_entry.get_mut(&record.entry)
            else {
                summary.remove_misses += 1;
                continue;
            };
            let before = cached_records.len();
            cached_records.retain(|cached| {
                cached.item != record.item || cached.vendor_type != record.vendor_type
            });
            let removed = before.saturating_sub(cached_records.len());
            if removed == 0 {
                summary.no_match += 1;
            } else {
                summary.items_removed += removed;
            }
            if cached_records.is_empty() {
                self.game_event_vendor_cache_by_entry.remove(&record.entry);
            }
        }
        summary
    }

    #[allow(dead_code)]
    pub fn game_event_npc_flag_mask_like_cpp(
        &self,
        spawn_id: SpawnId,
        active_event_ids: &[u16],
    ) -> u64 {
        self.game_event_npc_flags
            .game_event_npc_flag_mask_like_cpp(spawn_id, active_event_ids)
    }

    pub fn change_game_event_model_equip_baseline_like_cpp(
        &mut self,
        event_id: u16,
        activate: bool,
    ) -> GameEventModelEquipBaselineChangeSummaryLikeCpp {
        let mut summary = GameEventModelEquipBaselineChangeSummaryLikeCpp {
            event_id,
            activate,
            ..GameEventModelEquipBaselineChangeSummaryLikeCpp::default()
        };

        let Some(records) = self.game_event_model_equip.records_mut_like_cpp(event_id) else {
            summary.missing_event_bucket = true;
            return summary;
        };
        summary.records_seen = records.len();

        for record in records {
            if self
                .spawn_store
                .spawn_data(SpawnObjectType::Creature, record.spawn_id)
                .is_none()
            {
                summary.missing_spawn_metadata += 1;
                summary.record_outcomes.push(
                    GameEventModelEquipBaselineRecordOutcomeLikeCpp::MissingSpawnMetadata {
                        spawn_id: record.spawn_id,
                    },
                );
                continue;
            }

            let Some(row) = self.creature_runtime_rows.get_mut(&record.spawn_id) else {
                summary.missing_creature_runtime_rows += 1;
                summary.record_outcomes.push(
                    GameEventModelEquipBaselineRecordOutcomeLikeCpp::MissingCreatureRuntimeRow {
                        spawn_id: record.spawn_id,
                    },
                );
                continue;
            };

            if activate {
                record.model_id_prev = row.model_id;
                record.equipment_id_prev = u8::try_from(row.equipment_id).unwrap_or(0);
                row.model_id = record.model_id;
                row.equipment_id = i8::try_from(record.equipment_id).unwrap_or(i8::MAX);
            } else {
                row.model_id = record.model_id_prev;
                row.equipment_id = i8::try_from(record.equipment_id_prev).unwrap_or(i8::MAX);
            }

            summary.records_applied += 1;
            summary.record_outcomes.push(
                GameEventModelEquipBaselineRecordOutcomeLikeCpp::Applied {
                    spawn_id: record.spawn_id,
                    model_id_prev: record.model_id_prev,
                    equipment_id_prev: record.equipment_id_prev,
                    model_id_after: row.model_id,
                    equipment_id_after: u8::try_from(row.equipment_id).unwrap_or(0),
                },
            );
        }

        summary
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

#[derive(Debug, Clone)]
struct CreatureSpawnRow {
    spawn_id: SpawnId,
    entry: u32,
    map_id: u32,
    x: f32,
    y: f32,
    z: f32,
    orientation: f32,
    model_id: u32,
    equipment_id: i8,
    spawn_time_secs: i32,
    wander_distance: f32,
    curhealth: u32,
    curmana: u32,
    movement_type: u8,
    npc_flags: Option<u64>,
    unit_flags: Option<u32>,
    unit_flags2: Option<u32>,
    unit_flags3: Option<u32>,
    ground_movement_type: u8,
    swim_allowed: bool,
    flight_movement_type: u8,
    rooted: bool,
    chase_movement_type: u8,
    random_movement_type: u8,
    interaction_pause_timer_ms: u32,
    spawn_difficulties: String,
    event_entry: i16,
    pool_id: u32,
    phase_use_flags: u8,
    phase_id: u32,
    phase_group: u32,
    terrain_swap_map: i32,
    script_name: String,
    string_id: String,
}

#[derive(Debug, Clone)]
struct GameObjectSpawnRow {
    spawn_id: SpawnId,
    entry: u32,
    map_id: u32,
    x: f32,
    y: f32,
    z: f32,
    orientation: f32,
    rotation: [f32; 4],
    spawn_time_secs: i32,
    anim_progress: u8,
    state: u8,
    spawn_difficulties: String,
    event_entry: i16,
    pool_id: u32,
    phase_use_flags: u8,
    phase_id: u32,
    phase_group: u32,
    terrain_swap_map: i32,
    script_name: String,
    string_id: String,
}

#[derive(Debug, Clone)]
struct AreaTriggerSpawnRow {
    spawn_id: SpawnId,
    create_properties_id: u32,
    is_custom: bool,
    map_id: u32,
    spawn_difficulties: String,
    x: f32,
    y: f32,
    z: f32,
    orientation: f32,
    phase_use_flags: u8,
    phase_id: u32,
    phase_group: u32,
    spell_for_visuals: Option<i32>,
    script_name: String,
}

#[derive(Debug, Clone, Copy)]
struct LinkedRespawnDbRow {
    guid: SpawnId,
    linked_guid: SpawnId,
    link_type: u8,
}

#[derive(Debug, Clone, Copy)]
struct PoolTemplateRowLikeCpp {
    entry: u32,
    max_limit: u32,
}

#[derive(Debug, Clone, Copy)]
struct PoolMemberRowLikeCpp {
    spawn_id: u64,
    pool_spawn_id: u32,
    chance: f32,
}

#[derive(Debug, Clone, Copy)]
struct PoolAutospawnCandidateRowLikeCpp {
    pool_entry: u32,
    child_pool_id: u64,
    mother_pool_id: u32,
}

#[derive(Debug, Clone, Copy)]
struct GameEventPoolRowLikeCpp {
    pool_entry: u32,
    event_id: i16,
}

#[derive(Debug, Clone, Copy)]
struct GameEventObjectGuidRowLikeCpp {
    guid: SpawnId,
    event_id: i16,
}

impl From<LinkedRespawnDbRow> for LinkedRespawnRowLikeCpp {
    fn from(row: LinkedRespawnDbRow) -> Self {
        Self {
            guid: row.guid,
            linked_guid: row.linked_guid,
            link_type: row.link_type,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureFormationRowLikeCpp {
    pub leader_spawn_id: SpawnId,
    pub member_spawn_id: SpawnId,
    pub dist: f32,
    pub angle_degrees: f32,
    pub group_ai: u32,
    pub point_1: u32,
    pub point_2: u32,
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
    db: &WorldDatabase,
    store: &SpawnStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<BTreeMap<SpawnId, CreatureFormationInfoLikeCpp>> {
    let stmt = db.prepare(WorldStatements::SEL_CREATURE_FORMATIONS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut rows = Vec::new();
    loop {
        rows.push(CreatureFormationRowLikeCpp {
            leader_spawn_id: read_unsigned_db_u64_like_cpp(
                &result,
                0,
                "creature_formations.leaderGUID",
            )?,
            member_spawn_id: read_unsigned_db_u64_like_cpp(
                &result,
                1,
                "creature_formations.memberGUID",
            )?,
            dist: result.read(2),
            angle_degrees: result.read(3),
            group_ai: read_unsigned_db_u32_like_cpp(&result, 4, "creature_formations.groupAI")?,
            point_1: u32::from(read_unsigned_db_u16_like_cpp(
                &result,
                5,
                "creature_formations.point_1",
            )?),
            point_2: u32::from(read_unsigned_db_u16_like_cpp(
                &result,
                6,
                "creature_formations.point_2",
            )?),
        });
        if !result.next_row() {
            break;
        }
    }

    Ok(apply_creature_formation_rows_like_cpp(
        rows,
        store,
        &mut report.creature_formations,
    ))
}

async fn load_waypoint_paths_like_cpp(
    db: &WorldDatabase,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<WaypointPathStoreLikeCpp> {
    let path_stmt = db.prepare(WorldStatements::SEL_WAYPOINT_PATHS);
    let mut path_result = db.query(&path_stmt).await?;
    let mut path_rows = Vec::new();
    if !path_result.is_empty() {
        loop {
            path_rows.push(WaypointPathRowLikeCpp {
                path_id: read_unsigned_db_u32_like_cpp(&path_result, 0, "waypoint_path.PathId")?,
                move_type: path_result.try_read(1).unwrap_or(0),
                flags: path_result.try_read(2).unwrap_or(0),
            });
            if !path_result.next_row() {
                break;
            }
        }
    }

    let node_stmt = db.prepare(WorldStatements::SEL_WAYPOINT_PATH_NODES);
    let mut node_result = db.query(&node_stmt).await?;
    let mut node_rows = Vec::new();
    if !node_result.is_empty() {
        loop {
            node_rows.push(WaypointPathNodeRowLikeCpp {
                path_id: read_unsigned_db_u32_like_cpp(
                    &node_result,
                    0,
                    "waypoint_path_node.PathId",
                )?,
                node_id: read_unsigned_db_u32_like_cpp(
                    &node_result,
                    1,
                    "waypoint_path_node.NodeId",
                )?,
                x: node_result.read(2),
                y: node_result.read(3),
                z: node_result.read(4),
                orientation: node_result.try_read::<Option<f32>>(5).unwrap_or(None),
                delay: read_unsigned_db_u32_like_cpp(&node_result, 6, "waypoint_path_node.Delay")?,
            });
            if !node_result.next_row() {
                break;
            }
        }
    }

    let (store, load_report) = WaypointPathStoreLikeCpp::from_rows_like_cpp(path_rows, node_rows);
    report.waypoint_paths = load_report;
    Ok(store)
}

pub async fn load_canonical_spawn_store_like_cpp(
    db: &WorldDatabase,
    game_event_persistence: &dyn wow_persistence::GameEventPersistencePortLikeCpp,
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
        db,
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
    let waypoint_paths = load_waypoint_paths_like_cpp(db, &mut report).await?;
    let creature_formations = load_creature_formations_like_cpp(db, &store, &mut report).await?;
    load_gameobject_spawns_like_cpp(
        db,
        map_store,
        map_difficulty_store,
        &mut store,
        &mut gameobject_runtime_rows,
        &mut report,
    )
    .await?;
    load_area_trigger_spawns_like_cpp(
        db,
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
    let linked_respawns = load_linked_respawns_like_cpp(db, &store, map_store, &mut report).await?;

    // C++ `PoolMgr::LoadFromDB` uses ObjectMgr creature/gameobject spawn data as
    // existence/map truth. This builds only PoolMgr metadata/plans; no live spawn.
    let pool_mgr = load_pool_mgr_like_cpp(db, &store, &mut report).await?;
    let game_event_sizing = GameEventSizingLikeCpp::from_max_event_entry_like_cpp(
        load_max_game_event_entry_like_cpp(db).await?,
    );
    // C++ `GameEventMgr::LoadFromDB` loads master `game_event` metadata into
    // `mGameEvent` before prerequisite and later event-specific lists consume the same sizing.
    // This is read-only startup metadata: no scheduler, active set, DB2 holiday
    // rewrite, persistence, or apply/unapply side effect is performed here.
    let mut game_events = load_game_events_like_cpp(db, game_event_sizing, &mut report).await?;
    // C++ `GameEventMgr::LoadFromDB` stores prerequisites on the same `mGameEvent`
    // entries before scheduler helpers read them; no second prerequisite store is created.
    load_game_event_prerequisites_like_cpp(db, &mut game_events, &mut report).await?;
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_condition` into
    // `mGameEvent[event].conditions`, then overlays character DB saved `done` values.
    load_game_event_conditions_like_cpp(db, &mut game_events, &mut report).await?;
    load_game_event_condition_saves_like_cpp(game_event_persistence, &mut game_events, &mut report)
        .await?;
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_quest_condition` into
    // `mQuestToEventConditions` with quest-key last-row-wins semantics for later
    // `HandleQuestComplete`; this is metadata/evidence only and does not wire quests live.
    let game_event_quest_conditions =
        load_game_event_quest_conditions_like_cpp(db, &game_events, &mut report).await?;
    // C++ `GameEventMgr` loads `game_event_pool` after PoolMgr validation so
    // `CheckPool(entry)` can gate each row; this is metadata only.
    let game_event_pools =
        load_game_event_pool_ids_like_cpp(db, game_event_sizing, &pool_mgr, &mut report).await?;
    // C++ `GameEventMgr` also loads creature/gameobject GUID lists after ObjectMgr
    // spawn metadata exists. This stores only future caller input; no live grid mutation.
    let game_event_spawn_guids =
        load_game_event_spawn_guids_like_cpp(db, game_event_sizing, &store, &mut report).await?;
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_model_equip` startup metadata
    // for later `ChangeEquipOrModel`; this slice stores only validated metadata and
    // does not mutate live maps, CreatureData/ObjectMgr baselines, display ids or equipment.
    let game_event_model_equip =
        load_game_event_model_equip_like_cpp(db, game_event_sizing, &mut report).await?;
    // C++ `GameEventMgr::LoadFromDB` loads quest relation metadata from
    // `game_event_creature_quest` and `game_event_gameobject_quest` before later
    // condition/NPC flag/vendor metadata. This is read-only startup metadata for
    // future `UpdateEventQuests`; no ObjectMgr quest maps or sessions are mutated.
    let game_event_quest_relations =
        load_game_event_quest_relations_like_cpp(db, game_event_sizing, &mut report).await?;
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_npcflag` into
    // `mGameEventNPCFlags` for later `UpdateEventNPCFlags`/`GetNPCFlag`.
    // This slice stores only static metadata and pure read-only helpers.
    let game_event_npc_flags =
        load_game_event_npc_flags_like_cpp(db, game_event_sizing, &mut report).await?;
    // C++ `GameEventMgr::LoadFromDB` loads `game_event_npc_vendor` after
    // `game_event_npcflag` because vendor validation receives the first matching
    // NPC flag low32 mask. Rust stores metadata only and defers ObjectMgr validation/mutation.
    let game_event_npc_vendors = load_game_event_npc_vendors_like_cpp(
        db,
        game_event_sizing,
        &store,
        &game_event_npc_flags,
        &mut report,
    )
    .await?;

    let mut templates = spawn_group_templates_for_spawn_store(spawn_group_store);
    let members = load_spawn_group_members_like_cpp(db).await?;
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

async fn load_pool_mgr_like_cpp(
    db: &WorldDatabase,
    store: &SpawnStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<PoolMgrLikeCpp> {
    let mut mgr = PoolMgrLikeCpp::new();

    let stmt = db.prepare(WorldStatements::SEL_POOL_TEMPLATES);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(mgr);
    }
    loop {
        apply_pool_template_row_like_cpp(
            PoolTemplateRowLikeCpp {
                entry: result.read(0),
                max_limit: result.read(1),
            },
            &mut mgr,
            &mut report.pool_mgr,
        );
        if !result.next_row() {
            break;
        }
    }

    load_pool_member_rows_like_cpp(db, store, PoolMemberKindLikeCpp::Creature, &mut mgr, report)
        .await?;
    load_pool_member_rows_like_cpp(
        db,
        store,
        PoolMemberKindLikeCpp::GameObject,
        &mut mgr,
        report,
    )
    .await?;
    load_pool_member_rows_like_cpp(db, store, PoolMemberKindLikeCpp::Pool, &mut mgr, report)
        .await?;

    apply_pool_map_propagation_like_cpp(&mut mgr, &mut report.pool_mgr);
    apply_pool_final_validation_like_cpp(&mgr, &mut report.pool_mgr);
    load_pool_autospawn_candidates_like_cpp(db, &mut mgr, report).await?;

    Ok(mgr)
}

async fn load_pool_member_rows_like_cpp(
    db: &WorldDatabase,
    store: &SpawnStore,
    kind: PoolMemberKindLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let mut stmt = db.prepare(WorldStatements::SEL_POOL_MEMBERS_BY_TYPE);
    stmt.set_u8(0, kind as u8);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        let row = PoolMemberRowLikeCpp {
            spawn_id: result.read(0),
            pool_spawn_id: result.read(1),
            chance: result.read(2),
        };
        match kind {
            PoolMemberKindLikeCpp::Creature | PoolMemberKindLikeCpp::GameObject => {
                apply_pool_spawn_member_row_like_cpp(row, store, kind, mgr, &mut report.pool_mgr);
            }
            PoolMemberKindLikeCpp::Pool => {
                apply_pool_pool_member_row_like_cpp(row, mgr, &mut report.pool_mgr);
            }
        }
        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

async fn load_pool_autospawn_candidates_like_cpp(
    db: &WorldDatabase,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let stmt = db.prepare(WorldStatements::SEL_POOL_AUTOSPAWN_CANDIDATES);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        apply_pool_autospawn_candidate_row_like_cpp(
            PoolAutospawnCandidateRowLikeCpp {
                pool_entry: result.read(0),
                child_pool_id: result.try_read(1).unwrap_or(0),
                mother_pool_id: result.try_read(2).unwrap_or(0),
            },
            mgr,
            &mut report.pool_mgr,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

async fn load_game_event_pool_ids_like_cpp(
    db: &WorldDatabase,
    game_event_sizing: GameEventSizingLikeCpp,
    mgr: &PoolMgrLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<GameEventPoolIdsLikeCpp> {
    let mut game_event_pools =
        GameEventPoolIdsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENT_POOLS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(game_event_pools);
    }

    loop {
        apply_game_event_pool_row_like_cpp(
            GameEventPoolRowLikeCpp {
                pool_entry: read_unsigned_db_u32_like_cpp(
                    &result,
                    0,
                    "game_event_pool.pool_entry",
                )?,
                event_id: i16::from(read_signed_db_i8_like_cpp(
                    &result,
                    1,
                    "game_event_pool.eventEntry",
                )?),
            },
            mgr,
            &mut game_event_pools,
            &mut report.game_event_pools,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(game_event_pools)
}

async fn load_max_game_event_entry_like_cpp(db: &WorldDatabase) -> Result<Option<u32>> {
    let stmt = db.prepare(WorldStatements::SEL_MAX_GAME_EVENT_ENTRY);
    let result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(None);
    }

    Ok(result.try_read(0))
}

async fn load_game_events_like_cpp(
    db: &WorldDatabase,
    game_event_sizing: GameEventSizingLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<GameEventDataStoreLikeCpp> {
    let mut game_events =
        GameEventDataStoreLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);
    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENTS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(GameEventDataStoreLikeCpp::default());
    }

    loop {
        apply_game_event_data_row_like_cpp(
            GameEventDataRowLikeCpp {
                event_id: u16::from(read_unsigned_db_u8_like_cpp(
                    &result,
                    0,
                    "game_event.eventEntry",
                )?),
                start: read_unsigned_db_u64_like_cpp(&result, 1, "game_event.start_time")?,
                end: read_unsigned_db_u64_like_cpp(&result, 2, "game_event.end_time")?,
                occurence: read_unsigned_db_u32_like_cpp(&result, 3, "game_event.occurence")?,
                length: read_unsigned_db_u32_like_cpp(&result, 4, "game_event.length")?,
                holiday_id: read_unsigned_db_u32_like_cpp(&result, 5, "game_event.holiday")?,
                holiday_stage: read_unsigned_db_u8_like_cpp(&result, 6, "game_event.holidayStage")?,
                description: result.read(7),
                state_raw: read_unsigned_db_u8_like_cpp(&result, 8, "game_event.world_event")?,
                announce: read_unsigned_db_u8_like_cpp(&result, 9, "game_event.announce")?,
            },
            &mut game_events,
            &mut report.game_events,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(game_events)
}

async fn load_game_event_prerequisites_like_cpp(
    db: &WorldDatabase,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENT_PREREQUISITES);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        apply_game_event_prerequisite_row_like_cpp(
            GameEventPrerequisiteRowLikeCpp {
                event_id: u16::from(read_unsigned_db_u8_like_cpp(
                    &result,
                    0,
                    "game_event_prerequisite.eventEntry",
                )?),
                prerequisite_event: read_unsigned_db_u32_like_cpp(
                    &result,
                    1,
                    "game_event_prerequisite.prerequisite_event",
                )?,
            },
            game_events,
            &mut report.game_event_prerequisites,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

fn apply_game_event_prerequisite_row_like_cpp(
    row: GameEventPrerequisiteRowLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut GameEventPrerequisiteLoadReportLikeCpp,
) {
    report.rows += 1;
    match game_events.insert_prerequisite_event_like_cpp(row.event_id, row.prerequisite_event) {
        GameEventPrerequisiteInsertOutcomeLikeCpp::Loaded => report.loaded += 1,
        GameEventPrerequisiteInsertOutcomeLikeCpp::Duplicate => report.duplicate_ignored += 1,
        GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangeEvent => {
            report.skipped_out_of_range_event += 1;
        }
        GameEventPrerequisiteInsertOutcomeLikeCpp::NonWorldEvent => {
            report.skipped_non_world_event += 1;
        }
        GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangePrerequisite => {
            report.skipped_out_of_range_prerequisite += 1;
        }
    }
}

async fn load_game_event_conditions_like_cpp(
    db: &WorldDatabase,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENT_CONDITIONS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        let event_id = read_unsigned_db_u8_like_cpp(&result, 0, "game_event_condition.eventEntry")?;
        apply_game_event_condition_row_like_cpp(
            GameEventConditionRowLikeCpp {
                event_id: u16::from(event_id),
                condition_id: read_unsigned_db_u32_like_cpp(
                    &result,
                    1,
                    "game_event_condition.condition_id",
                )?,
                req_num: result.read(2),
                max_world_state: read_unsigned_db_u16_like_cpp(
                    &result,
                    3,
                    "game_event_condition.max_world_state_field",
                )?,
                done_world_state: read_unsigned_db_u16_like_cpp(
                    &result,
                    4,
                    "game_event_condition.done_world_state_field",
                )?,
            },
            game_events,
            &mut report.game_event_conditions,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

fn apply_game_event_condition_row_like_cpp(
    row: GameEventConditionRowLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut GameEventConditionLoadReportLikeCpp,
) {
    report.rows += 1;
    match game_events.apply_game_event_condition_row_like_cpp(
        row.event_id,
        row.condition_id,
        row.req_num,
        row.max_world_state,
        row.done_world_state,
    ) {
        GameEventConditionApplyOutcomeLikeCpp::Loaded => report.loaded += 1,
        GameEventConditionApplyOutcomeLikeCpp::OutOfRangeEvent => {
            report.skipped_out_of_range += 1;
        }
    }
}

async fn load_game_event_condition_saves_like_cpp(
    persistence: &dyn wow_persistence::GameEventPersistencePortLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let rows = match persistence.load_condition_saves_like_cpp().await {
        wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Loaded(rows) => rows,
        wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Failed { reason } => {
            anyhow::bail!("game-event condition-save persistence load failed: {reason}")
        }
    };
    for row in rows {
        apply_game_event_condition_save_row_like_cpp(
            GameEventConditionSaveRowLikeCpp {
                event_id: u16::from(row.event_id),
                condition_id: row.condition_id,
                done: row.done,
            },
            game_events,
            &mut report.game_event_condition_saves,
        );
    }

    Ok(())
}

fn apply_game_event_condition_save_row_like_cpp(
    row: GameEventConditionSaveRowLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut GameEventConditionSaveLoadReportLikeCpp,
) {
    report.rows += 1;
    match game_events.apply_game_event_condition_save_row_like_cpp(
        row.event_id,
        row.condition_id,
        row.done,
    ) {
        GameEventConditionSaveApplyOutcomeLikeCpp::Loaded => report.loaded += 1,
        GameEventConditionSaveApplyOutcomeLikeCpp::OutOfRangeEvent => {
            report.skipped_out_of_range_event += 1;
        }
        GameEventConditionSaveApplyOutcomeLikeCpp::MissingCondition => {
            report.skipped_missing_condition += 1;
        }
    }
}

async fn load_game_event_quest_conditions_like_cpp(
    db: &WorldDatabase,
    game_events: &GameEventDataStoreLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<BTreeMap<u32, GameEventQuestConditionRecordLikeCpp>> {
    let mut quest_conditions = BTreeMap::new();
    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENT_QUEST_CONDITIONS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(quest_conditions);
    }

    loop {
        let event_id =
            read_unsigned_db_u8_like_cpp(&result, 1, "game_event_quest_condition.eventEntry")?;
        apply_game_event_quest_condition_row_like_cpp(
            GameEventQuestConditionRowLikeCpp {
                quest_id: read_unsigned_db_u32_like_cpp(
                    &result,
                    0,
                    "game_event_quest_condition.quest",
                )?,
                event_id: u16::from(event_id),
                condition_id: read_unsigned_db_u32_like_cpp(
                    &result,
                    2,
                    "game_event_quest_condition.condition_id",
                )?,
                num: result.read(3),
            },
            game_events,
            &mut quest_conditions,
            &mut report.game_event_quest_conditions,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(quest_conditions)
}

fn apply_game_event_quest_condition_row_like_cpp(
    row: GameEventQuestConditionRowLikeCpp,
    game_events: &GameEventDataStoreLikeCpp,
    quest_conditions: &mut BTreeMap<u32, GameEventQuestConditionRecordLikeCpp>,
    report: &mut GameEventQuestConditionLoadReportLikeCpp,
) {
    report.rows += 1;
    if game_events.event_like_cpp(row.event_id).is_none() {
        report.skipped_out_of_range_event += 1;
        return;
    }

    let previous = quest_conditions.insert(
        row.quest_id,
        GameEventQuestConditionRecordLikeCpp {
            quest_id: row.quest_id,
            event_id: row.event_id,
            condition_id: row.condition_id,
            num: row.num,
        },
    );
    report.loaded += 1;
    if previous.is_some() {
        report.overwrites += 1;
    }
}

fn apply_game_event_data_row_like_cpp(
    row: GameEventDataRowLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut GameEventDataLoadReportLikeCpp,
) {
    report.rows += 1;
    if row.event_id == 0 {
        report.skipped_reserved_zero += 1;
        return;
    }

    let Some(event) = game_events.event_mut_like_cpp(row.event_id) else {
        report.skipped_out_of_range += 1;
        return;
    };

    event.event_id = row.event_id;
    event.start = row.start;
    event.end = row.end;
    event.next_start = 0;
    event.occurence = row.occurence;
    event.length = row.length;
    event.holiday_id = row.holiday_id;
    event.holiday_stage = row.holiday_stage;
    event.description = row.description;
    event.state_raw = row.state_raw;
    event.announce = row.announce;
    report.loaded += 1;

    if !event.is_valid_like_cpp() {
        report.invalid_normal_zero_length += 1;
    }
    if event.holiday_id != 0 {
        report.holiday_validation_deferred += 1;
    }
}

fn apply_game_event_pool_row_like_cpp(
    row: GameEventPoolRowLikeCpp,
    mgr: &PoolMgrLikeCpp,
    game_event_pools: &mut GameEventPoolIdsLikeCpp,
    report: &mut GameEventPoolLoadReportLikeCpp,
) {
    report.rows += 1;
    if game_event_pools
        .internal_event_id_like_cpp(row.event_id)
        .is_none()
    {
        report.skipped_out_of_range += 1;
        return;
    }
    if !mgr.templates.contains_key(&row.pool_entry) || !mgr.check_pool_like_cpp(row.pool_entry) {
        report.skipped_broken_pool += 1;
        return;
    }
    if game_event_pools.push_pool_id_like_cpp(row.event_id, row.pool_entry) {
        report.loaded += 1;
    }
}

async fn load_game_event_spawn_guids_like_cpp(
    db: &WorldDatabase,
    game_event_sizing: GameEventSizingLikeCpp,
    store: &SpawnStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<GameEventSpawnGuidsLikeCpp> {
    let mut game_event_spawn_guids =
        GameEventSpawnGuidsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    load_game_event_object_guids_like_cpp(
        db,
        WorldStatements::SEL_GAME_EVENT_CREATURES,
        SpawnObjectType::Creature,
        store,
        &mut game_event_spawn_guids,
        &mut report.game_event_spawn_guids.creature,
    )
    .await?;
    load_game_event_object_guids_like_cpp(
        db,
        WorldStatements::SEL_GAME_EVENT_GAMEOBJECTS,
        SpawnObjectType::GameObject,
        store,
        &mut game_event_spawn_guids,
        &mut report.game_event_spawn_guids.gameobject,
    )
    .await?;

    Ok(game_event_spawn_guids)
}

async fn load_game_event_object_guids_like_cpp(
    db: &WorldDatabase,
    statement: WorldStatements,
    object_type: SpawnObjectType,
    store: &SpawnStore,
    game_event_spawn_guids: &mut GameEventSpawnGuidsLikeCpp,
    report: &mut GameEventObjectGuidLoadReportLikeCpp,
) -> Result<()> {
    let stmt = db.prepare(statement);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        apply_game_event_object_guid_row_like_cpp(
            GameEventObjectGuidRowLikeCpp {
                guid: read_unsigned_db_u64_like_cpp(&result, 0, "game_event_object.guid")?,
                event_id: i16::from(read_signed_db_i8_like_cpp(
                    &result,
                    1,
                    "game_event_object.eventEntry",
                )?),
            },
            object_type,
            store,
            game_event_spawn_guids,
            report,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

fn apply_game_event_object_guid_row_like_cpp(
    row: GameEventObjectGuidRowLikeCpp,
    object_type: SpawnObjectType,
    store: &SpawnStore,
    game_event_spawn_guids: &mut GameEventSpawnGuidsLikeCpp,
    report: &mut GameEventObjectGuidLoadReportLikeCpp,
) {
    report.rows += 1;
    let Some(spawn_data) = store.spawn_data(object_type, row.guid) else {
        report.skipped_missing_spawn_metadata += 1;
        return;
    };
    if game_event_spawn_guids
        .internal_event_id_like_cpp(row.event_id)
        .is_none()
    {
        report.skipped_out_of_range += 1;
        return;
    }
    if spawn_data.pool_id != 0 {
        report.pooled_still_loaded += 1;
    }
    if game_event_spawn_guids.push_guid_like_cpp(object_type, row.event_id, row.guid) {
        report.loaded += 1;
    }
}

async fn load_creature_equip_template_ids_like_cpp(
    db: &WorldDatabase,
    report: &mut GameEventModelEquipLoadReportLikeCpp,
) -> Result<BTreeSet<(u32, u8)>> {
    let stmt = db.prepare(WorldStatements::SEL_CREATURE_EQUIP_TEMPLATE_IDS);
    let mut result = db.query(&stmt).await?;
    let mut equipment_ids = BTreeSet::new();
    if result.is_empty() {
        return Ok(equipment_ids);
    }

    loop {
        report.equipment_rows += 1;
        let creature_id: u32 = result.read(0);
        let equipment_id: u8 = result.read(1);
        // C++ game_event_model_equip validation calls GetEquipmentInfo only for > 0 ids;
        // id 0 is not a valid template key for that positive-id validation path.
        if equipment_id > 0 && equipment_ids.insert((creature_id, equipment_id)) {
            report.equipment_ids_loaded += 1;
        }
        if !result.next_row() {
            break;
        }
    }

    Ok(equipment_ids)
}

async fn load_game_event_model_equip_like_cpp(
    db: &WorldDatabase,
    game_event_sizing: GameEventSizingLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<GameEventModelEquipLikeCpp> {
    let equipment_ids =
        load_creature_equip_template_ids_like_cpp(db, &mut report.game_event_model_equip).await?;
    let mut model_equip =
        GameEventModelEquipLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENT_MODEL_EQUIP);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(model_equip);
    }

    loop {
        apply_game_event_model_equip_row_like_cpp(
            GameEventModelEquipRowLikeCpp {
                spawn_id: read_unsigned_db_u64_like_cpp(&result, 0, "game_event_model_equip.guid")?,
                entry: read_unsigned_db_u32_like_cpp(
                    &result,
                    1,
                    "game_event_model_equip.creature.id",
                )?,
                event_id: u16::from(read_unsigned_db_u8_like_cpp(
                    &result,
                    2,
                    "game_event_model_equip.eventEntry",
                )?),
                model_id: read_unsigned_db_u32_like_cpp(
                    &result,
                    3,
                    "game_event_model_equip.modelid",
                )?,
                equipment_id: read_unsigned_db_u8_like_cpp(
                    &result,
                    4,
                    "game_event_model_equip.equipment_id",
                )?,
            },
            &equipment_ids,
            &mut model_equip,
            &mut report.game_event_model_equip,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(model_equip)
}

fn apply_game_event_model_equip_row_like_cpp(
    row: GameEventModelEquipRowLikeCpp,
    equipment_ids: &BTreeSet<(u32, u8)>,
    model_equip: &mut GameEventModelEquipLikeCpp,
    report: &mut GameEventModelEquipLoadReportLikeCpp,
) {
    report.rows += 1;
    if model_equip.records_like_cpp(row.event_id).is_none() {
        report.invalid_event_id += 1;
        return;
    }
    if row.equipment_id > 0 && !equipment_ids.contains(&(row.entry, row.equipment_id)) {
        report.missing_equipment_template += 1;
        return;
    }

    if model_equip.push_record_like_cpp(
        row.event_id,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: row.spawn_id,
            model_id: row.model_id,
            model_id_prev: 0,
            equipment_id: row.equipment_id,
            equipment_id_prev: 0,
        },
    ) {
        report.loaded += 1;
    }
}

async fn load_game_event_quest_relations_like_cpp(
    db: &WorldDatabase,
    game_event_sizing: GameEventSizingLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<GameEventQuestRelationsLikeCpp> {
    let mut quest_relations =
        GameEventQuestRelationsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    load_game_event_creature_quest_relations_like_cpp(db, &mut quest_relations, report).await?;
    load_game_event_gameobject_quest_relations_like_cpp(db, &mut quest_relations, report).await?;

    report.game_event_quest_relations.creature.events_touched = quest_relations
        .creature_records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();
    report.game_event_quest_relations.gameobject.events_touched = quest_relations
        .gameobject_records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();

    Ok(quest_relations)
}

async fn load_game_event_creature_quest_relations_like_cpp(
    db: &WorldDatabase,
    quest_relations: &mut GameEventQuestRelationsLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENT_CREATURE_QUESTS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        let event_id =
            read_unsigned_db_u8_like_cpp(&result, 2, "game_event_creature_quest.eventEntry")?;
        apply_game_event_creature_quest_relation_row_like_cpp(
            GameEventQuestRelationRowLikeCpp {
                giver_id: read_unsigned_db_u32_like_cpp(
                    &result,
                    0,
                    "game_event_creature_quest.id",
                )?,
                quest_id: read_unsigned_db_u32_like_cpp(
                    &result,
                    1,
                    "game_event_creature_quest.quest",
                )?,
                event_id,
            },
            quest_relations,
            &mut report.game_event_quest_relations.creature,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

async fn load_game_event_gameobject_quest_relations_like_cpp(
    db: &WorldDatabase,
    quest_relations: &mut GameEventQuestRelationsLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENT_GAMEOBJECT_QUESTS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        let event_id =
            read_unsigned_db_u8_like_cpp(&result, 2, "game_event_gameobject_quest.eventEntry")?;
        apply_game_event_gameobject_quest_relation_row_like_cpp(
            GameEventQuestRelationRowLikeCpp {
                giver_id: read_unsigned_db_u32_like_cpp(
                    &result,
                    0,
                    "game_event_gameobject_quest.id",
                )?,
                quest_id: read_unsigned_db_u32_like_cpp(
                    &result,
                    1,
                    "game_event_gameobject_quest.quest",
                )?,
                event_id,
            },
            quest_relations,
            &mut report.game_event_quest_relations.gameobject,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

fn apply_game_event_creature_quest_relation_row_like_cpp(
    row: GameEventQuestRelationRowLikeCpp,
    quest_relations: &mut GameEventQuestRelationsLikeCpp,
    report: &mut GameEventQuestRelationFamilyLoadReportLikeCpp,
) {
    report.rows += 1;
    let event_id = u16::from(row.event_id);
    if quest_relations
        .creature_records_like_cpp(event_id)
        .is_none()
    {
        report.skipped_out_of_range += 1;
        return;
    }

    if quest_relations.push_creature_record_like_cpp(
        event_id,
        GameEventQuestRelationRecordLikeCpp {
            giver_id: row.giver_id,
            quest_id: row.quest_id,
        },
    ) {
        report.loaded += 1;
    }
}

fn apply_game_event_gameobject_quest_relation_row_like_cpp(
    row: GameEventQuestRelationRowLikeCpp,
    quest_relations: &mut GameEventQuestRelationsLikeCpp,
    report: &mut GameEventQuestRelationFamilyLoadReportLikeCpp,
) {
    report.rows += 1;
    let event_id = u16::from(row.event_id);
    if quest_relations
        .gameobject_records_like_cpp(event_id)
        .is_none()
    {
        report.skipped_out_of_range += 1;
        return;
    }

    if quest_relations.push_gameobject_record_like_cpp(
        event_id,
        GameEventQuestRelationRecordLikeCpp {
            giver_id: row.giver_id,
            quest_id: row.quest_id,
        },
    ) {
        report.loaded += 1;
    }
}

async fn load_game_event_npc_flags_like_cpp(
    db: &WorldDatabase,
    game_event_sizing: GameEventSizingLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<GameEventNpcFlagsLikeCpp> {
    let mut npc_flags =
        GameEventNpcFlagsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENT_NPC_FLAGS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(npc_flags);
    }

    loop {
        apply_game_event_npc_flag_row_like_cpp(
            GameEventNpcFlagRowLikeCpp {
                spawn_id: read_unsigned_db_u64_like_cpp(&result, 0, "game_event_npcflag.guid")?,
                event_id: u16::from(read_unsigned_db_u8_like_cpp(
                    &result,
                    1,
                    "game_event_npcflag.eventEntry",
                )?),
                npcflag: read_unsigned_db_u64_like_cpp(&result, 2, "game_event_npcflag.npcflag")?,
            },
            &mut npc_flags,
            &mut report.game_event_npc_flags,
        );
        if !result.next_row() {
            break;
        }
    }

    report.game_event_npc_flags.events_touched = npc_flags
        .records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();

    Ok(npc_flags)
}

fn apply_game_event_npc_flag_row_like_cpp(
    row: GameEventNpcFlagRowLikeCpp,
    npc_flags: &mut GameEventNpcFlagsLikeCpp,
    report: &mut GameEventNpcFlagLoadReportLikeCpp,
) {
    report.rows += 1;
    if npc_flags.records_like_cpp(row.event_id).is_none() {
        report.skipped_out_of_range += 1;
        return;
    }

    if npc_flags.push_record_like_cpp(
        row.event_id,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: row.spawn_id,
            npcflag: row.npcflag,
        },
    ) {
        report.loaded += 1;
    }
}

async fn load_game_event_npc_vendors_like_cpp(
    db: &WorldDatabase,
    game_event_sizing: GameEventSizingLikeCpp,
    store: &SpawnStore,
    npc_flags: &GameEventNpcFlagsLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<GameEventNpcVendorsLikeCpp> {
    let mut npc_vendors =
        GameEventNpcVendorsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    let stmt = db.prepare(WorldStatements::SEL_GAME_EVENT_NPC_VENDOR);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(npc_vendors);
    }

    loop {
        let event_id =
            read_unsigned_db_u8_like_cpp(&result, 0, "game_event_npc_vendor.eventEntry")?;
        let ignore_filtering_raw =
            read_unsigned_db_u8_like_cpp(&result, 9, "game_event_npc_vendor.IgnoreFiltering")?;
        apply_game_event_npc_vendor_row_like_cpp(
            GameEventNpcVendorRowLikeCpp {
                event_id,
                spawn_id: read_unsigned_db_u64_like_cpp(&result, 1, "game_event_npc_vendor.guid")?,
                item: read_unsigned_db_u32_like_cpp(&result, 2, "game_event_npc_vendor.item")?,
                maxcount: read_unsigned_db_u32_like_cpp(
                    &result,
                    3,
                    "game_event_npc_vendor.maxcount",
                )?,
                incrtime: read_unsigned_db_u32_like_cpp(
                    &result,
                    4,
                    "game_event_npc_vendor.incrtime",
                )?,
                extended_cost: read_unsigned_db_u32_like_cpp(
                    &result,
                    5,
                    "game_event_npc_vendor.ExtendedCost",
                )?,
                vendor_type: read_unsigned_db_u8_like_cpp(
                    &result,
                    6,
                    "game_event_npc_vendor.type",
                )?,
                bonus_list_ids: result.read_string(7),
                player_condition_id: read_unsigned_db_u32_like_cpp(
                    &result,
                    8,
                    "game_event_npc_vendor.PlayerConditionId",
                )?,
                ignore_filtering: ignore_filtering_raw != 0,
            },
            store,
            npc_flags,
            &mut npc_vendors,
            &mut report.game_event_npc_vendors,
        );
        if !result.next_row() {
            break;
        }
    }

    Ok(npc_vendors)
}

fn apply_game_event_npc_vendor_row_like_cpp(
    row: GameEventNpcVendorRowLikeCpp,
    store: &SpawnStore,
    npc_flags: &GameEventNpcFlagsLikeCpp,
    npc_vendors: &mut GameEventNpcVendorsLikeCpp,
    report: &mut GameEventNpcVendorLoadReportLikeCpp,
) {
    report.rows += 1;
    let event_id = u16::from(row.event_id);
    if npc_vendors.records_like_cpp(event_id).is_none() {
        report.skipped_out_of_range += 1;
        return;
    }

    let Some(spawn_data) = store.spawn_data(SpawnObjectType::Creature, row.spawn_id) else {
        report.skipped_missing_creature_spawn_metadata += 1;
        return;
    };

    let event_npc_flag_low32 = npc_flags
        .records_like_cpp(event_id)
        .and_then(|records| {
            records
                .iter()
                .find(|record| record.spawn_id == row.spawn_id)
                .map(|record| record.npcflag as u32)
        })
        .unwrap_or(0);

    if npc_vendors.push_record_like_cpp(
        event_id,
        GameEventNpcVendorRecordLikeCpp {
            spawn_id: row.spawn_id,
            guid: row.spawn_id,
            entry: spawn_data.id,
            item: row.item,
            maxcount: row.maxcount,
            incrtime: row.incrtime,
            extended_cost: row.extended_cost,
            vendor_type: row.vendor_type,
            item_type: row.vendor_type,
            bonus_list_ids: parse_game_event_npc_vendor_bonus_list_ids_like_cpp(
                &row.bonus_list_ids,
            ),
            player_condition_id: row.player_condition_id,
            ignore_filtering: row.ignore_filtering,
            event_npc_flag_low32,
        },
    ) {
        report.loaded += 1;
        report.validation_deferred += 1;
    }
}

fn parse_game_event_npc_vendor_bonus_list_ids_like_cpp(raw: &str) -> Vec<i32> {
    raw.split_whitespace()
        .filter_map(|token| token.parse::<i32>().ok())
        .collect()
}

fn apply_pool_template_row_like_cpp(
    row: PoolTemplateRowLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    report.template_rows += 1;
    mgr.insert_template_like_cpp(row.entry, PoolTemplateDataLikeCpp::new(row.max_limit, -1));
    report.templates_loaded += 1;
}

fn apply_pool_spawn_member_row_like_cpp(
    row: PoolMemberRowLikeCpp,
    store: &SpawnStore,
    kind: PoolMemberKindLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    let member_report = match kind {
        PoolMemberKindLikeCpp::Creature => &mut report.creature_members,
        PoolMemberKindLikeCpp::GameObject => &mut report.gameobject_members,
        PoolMemberKindLikeCpp::Pool => {
            unreachable!("pool rows use apply_pool_pool_member_row_like_cpp")
        }
    };
    member_report.rows += 1;

    let spawn_type = match kind {
        PoolMemberKindLikeCpp::Creature => SpawnObjectType::Creature,
        PoolMemberKindLikeCpp::GameObject => SpawnObjectType::GameObject,
        PoolMemberKindLikeCpp::Pool => {
            unreachable!("pool rows use apply_pool_pool_member_row_like_cpp")
        }
    };
    let Some(spawn_data) = store.spawn_data(spawn_type, row.spawn_id) else {
        member_report.skipped_missing_spawn += 1;
        return;
    };
    let Some(template) = mgr.templates.get_mut(&row.pool_spawn_id) else {
        member_report.skipped_missing_template += 1;
        return;
    };
    if !(0.0..=100.0).contains(&row.chance) {
        member_report.skipped_invalid_chance += 1;
        return;
    }

    let map_id = match i32::try_from(spawn_data.map_id) {
        Ok(map_id) => map_id,
        Err(_) => {
            member_report.skipped_map_mismatch += 1;
            return;
        }
    };
    if template.map_id == -1 {
        template.map_id = map_id;
    }
    if template.map_id != map_id {
        member_report.skipped_map_mismatch += 1;
        return;
    }

    let max_limit = template.max_limit;
    let group_map = match kind {
        PoolMemberKindLikeCpp::Creature => &mut mgr.creature_groups,
        PoolMemberKindLikeCpp::GameObject => &mut mgr.gameobject_groups,
        PoolMemberKindLikeCpp::Pool => {
            unreachable!("pool rows use apply_pool_pool_member_row_like_cpp")
        }
    };
    let group = group_map
        .entry(row.pool_spawn_id)
        .or_insert_with(|| PoolGroupLikeCpp::with_pool_id(kind, row.pool_spawn_id));
    group.set_pool_id_like_cpp(row.pool_spawn_id);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(row.spawn_id, row.chance), max_limit);
    let spawn_id = row.spawn_id;
    let _ = mgr.register_spawn_pool_relation_like_cpp(kind, spawn_id, row.pool_spawn_id);
    member_report.loaded += 1;
}

fn apply_pool_pool_member_row_like_cpp(
    row: PoolMemberRowLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    report.pool_members.rows += 1;
    let Ok(child_pool_id) = u32::try_from(row.spawn_id) else {
        report.pool_members.skipped_child_id_overflow += 1;
        return;
    };
    if !mgr.templates.contains_key(&row.pool_spawn_id) {
        report.pool_members.skipped_missing_template += 1;
        return;
    }
    if !mgr.templates.contains_key(&child_pool_id) {
        report.pool_members.skipped_missing_spawn += 1;
        return;
    }
    if row.pool_spawn_id == child_pool_id {
        report.circular_relations += 1;
        report.pool_members.skipped_missing_spawn += 1;
        return;
    }
    if !(0.0..=100.0).contains(&row.chance) {
        report.pool_members.skipped_invalid_chance += 1;
        return;
    }

    let max_limit = mgr
        .templates
        .get(&row.pool_spawn_id)
        .map(|template| template.max_limit)
        .unwrap_or(0);
    let group = mgr.pool_groups.entry(row.pool_spawn_id).or_insert_with(|| {
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, row.pool_spawn_id)
    });
    group.set_pool_id_like_cpp(row.pool_spawn_id);
    group.add_entry_like_cpp(
        PoolObjectLikeCpp::new(u64::from(child_pool_id), row.chance),
        max_limit,
    );
    let _ = mgr.register_child_pool_relation_like_cpp(u64::from(child_pool_id), row.pool_spawn_id);
    report.pool_members.loaded += 1;
}

fn apply_pool_map_propagation_like_cpp(
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    let pool_ids = mgr.templates.keys().copied().collect::<Vec<_>>();
    for pool_id in pool_ids {
        let mut checked = std::collections::HashSet::new();
        let mut current = pool_id;
        while let Some(parent) = mgr.child_pool_to_parent.get(&current).copied() {
            let child_map_id = mgr
                .templates
                .get(&current)
                .map_or(-1, |template| template.map_id);
            if child_map_id != -1 {
                if let Some(parent_template) = mgr.templates.get_mut(&parent) {
                    if parent_template.map_id == -1 {
                        parent_template.map_id = child_map_id;
                    }
                    if parent_template.map_id != child_map_id {
                        mgr.remove_child_pool_relation_like_cpp(current, parent);
                        report.map_mismatches += 1;
                        report.relation_removals += 1;
                        report.pool_members.loaded = report.pool_members.loaded.saturating_sub(1);
                        break;
                    }
                }
            }

            checked.insert(current);
            if checked.contains(&parent) {
                mgr.remove_child_pool_relation_like_cpp(current, parent);
                report.circular_relations += 1;
                report.relation_removals += 1;
                report.pool_members.loaded = report.pool_members.loaded.saturating_sub(1);
                break;
            }
            current = parent;
        }
    }
}

fn apply_pool_final_validation_like_cpp(
    mgr: &PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    for (&pool_id, template) in &mgr.templates {
        if mgr.is_empty_like_cpp(pool_id) {
            report.empty_pools += 1;
        } else if template.map_id == -1 {
            report.missing_map_after_non_empty += 1;
        }
    }
}

fn apply_pool_autospawn_candidate_row_like_cpp(
    row: PoolAutospawnCandidateRowLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    report.autospawn_rows += 1;
    if mgr.is_empty_like_cpp(row.pool_entry) {
        report.autospawn_skipped_empty += 1;
        return;
    }
    if !mgr.check_pool_like_cpp(row.pool_entry) {
        report.autospawn_skipped_broken += 1;
        return;
    }
    if row.child_pool_id != 0 {
        let _mother_pool_id = row.mother_pool_id;
        report.autospawn_skipped_child += 1;
        return;
    }
    if let Some(template) = mgr.templates.get(&row.pool_entry) {
        mgr.add_auto_spawn_pool_like_cpp(template.map_id, row.pool_entry);
        report.autospawn_loaded += 1;
    }
}

pub fn spawn_group_templates_for_spawn_store(
    store: &wow_data::SpawnGroupTemplateStore,
) -> BTreeMap<u32, SpawnGroupTemplateData> {
    let mut templates = BTreeMap::new();
    for template in store.iter() {
        let map_id = match template.group_id {
            0 | 1 => 0,
            _ => SPAWNGROUP_MAP_UNSET,
        };
        templates.insert(
            template.group_id,
            SpawnGroupTemplateData {
                group_id: template.group_id,
                name: template.name.clone(),
                map_id,
                flags: SpawnGroupFlags(template.flags),
            },
        );
    }

    templates
        .entry(0)
        .or_insert_with(SpawnGroupTemplateData::default_group);
    templates
        .entry(1)
        .or_insert_with(SpawnGroupTemplateData::legacy_group);
    templates
}

async fn load_creature_spawns_like_cpp(
    db: &WorldDatabase,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    creature_equipment_store: &wow_data::CreatureEquipmentStoreLikeCpp,
    store: &mut SpawnStore,
    creature_runtime_rows: &mut BTreeMap<SpawnId, CreatureSpawnRuntimeRowLikeCpp>,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let stmt = db.prepare(WorldStatements::SEL_CREATURE_SPAWNS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        let mut row = CreatureSpawnRow {
            spawn_id: result.read(0),
            entry: result.read(1),
            map_id: result.read(2),
            x: result.read(3),
            y: result.read(4),
            z: result.read(5),
            orientation: result.read(6),
            model_id: result.try_read(7).unwrap_or(0),
            equipment_id: result.try_read(8).unwrap_or(0),
            spawn_time_secs: creature_spawntimesecs_to_i32_like_cpp(result.read(9))?,
            wander_distance: result.try_read(10).unwrap_or(0.0),
            curhealth: result.try_read(12).unwrap_or(0),
            curmana: result.try_read(13).unwrap_or(0),
            movement_type: result.try_read(14).unwrap_or(0),
            npc_flags: result.try_read::<Option<u64>>(18).unwrap_or(None),
            unit_flags: result.try_read::<Option<u32>>(19).unwrap_or(None),
            unit_flags2: result.try_read::<Option<u32>>(20).unwrap_or(None),
            unit_flags3: result.try_read::<Option<u32>>(21).unwrap_or(None),
            spawn_difficulties: result.read(15),
            event_entry: result.try_read(16).unwrap_or(0),
            pool_id: result.try_read(17).unwrap_or(0),
            phase_use_flags: result.read(22),
            phase_id: read_unsigned_db_u32_like_cpp(&result, 23, "creature.phaseid")?,
            phase_group: read_unsigned_db_u32_like_cpp(&result, 24, "creature.phasegroup")?,
            terrain_swap_map: result.read(25),
            script_name: result.try_read(26).unwrap_or_default(),
            string_id: result.try_read(27).unwrap_or_default(),
            ground_movement_type: result
                .try_read::<Option<u8>>(28)
                .flatten()
                .unwrap_or(wow_constants::CreatureGroundMovementType::Run as u8),
            swim_allowed: result.try_read::<Option<u8>>(29).flatten().unwrap_or(1) != 0,
            flight_movement_type: result.try_read::<Option<u8>>(30).flatten().unwrap_or(0),
            rooted: result.try_read::<Option<u8>>(31).flatten().unwrap_or(0) != 0,
            chase_movement_type: result.try_read::<Option<u8>>(32).flatten().unwrap_or(0),
            random_movement_type: result.try_read::<Option<u8>>(33).flatten().unwrap_or(0),
            interaction_pause_timer_ms: result
                .try_read::<Option<u32>>(34)
                .flatten()
                .unwrap_or(wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP),
        };
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

        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

async fn load_gameobject_spawns_like_cpp(
    db: &WorldDatabase,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    store: &mut SpawnStore,
    gameobject_runtime_rows: &mut BTreeMap<SpawnId, GameObjectSpawnRuntimeRowLikeCpp>,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let stmt = db.prepare(WorldStatements::SEL_GAMEOBJECT_SPAWNS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        let row = GameObjectSpawnRow {
            spawn_id: result.read(0),
            entry: result.read(1),
            map_id: result.read(2),
            x: result.read(3),
            y: result.read(4),
            z: result.read(5),
            orientation: result.read(6),
            rotation: [
                result.read(7),
                result.read(8),
                result.read(9),
                result.read(10),
            ],
            spawn_time_secs: result.read(11),
            anim_progress: result.read(12),
            state: result.read(13),
            spawn_difficulties: result.read(14),
            event_entry: result.try_read(15).unwrap_or(0),
            pool_id: result.try_read(16).unwrap_or(0),
            phase_use_flags: result.read(17),
            phase_id: read_unsigned_db_u32_like_cpp(&result, 18, "gameobject.phaseid")?,
            phase_group: read_unsigned_db_u32_like_cpp(&result, 19, "gameobject.phasegroup")?,
            terrain_swap_map: result.read(20),
            script_name: result.try_read(21).unwrap_or_default(),
            string_id: result.try_read(22).unwrap_or_default(),
        };
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

        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

async fn load_area_trigger_spawns_like_cpp(
    db: &WorldDatabase,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    spell_exists: &mut impl FnMut(u32) -> bool,
    script_id_for_name: &mut impl FnMut(&str) -> wow_data::ScriptIdLikeCpp,
    store: &mut SpawnStore,
    area_trigger_runtime_rows: &mut BTreeMap<SpawnId, AreaTriggerSpawnRuntimeRowLikeCpp>,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let stmt = db.prepare(WorldStatements::SEL_AREATRIGGER_SPAWNS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(());
    }

    loop {
        let row = AreaTriggerSpawnRow {
            spawn_id: result.read(0),
            create_properties_id: result.read(1),
            is_custom: result.read(2),
            map_id: result.read(3),
            spawn_difficulties: result.read(4),
            x: result.read(5),
            y: result.read(6),
            z: result.read(7),
            orientation: result.read(8),
            phase_use_flags: result.read(9),
            phase_id: read_unsigned_db_u32_like_cpp(&result, 10, "areatrigger.phaseid")?,
            phase_group: read_unsigned_db_u32_like_cpp(&result, 11, "areatrigger.phasegroup")?,
            spell_for_visuals: result.try_read(12).unwrap_or(None),
            script_name: result.try_read(13).unwrap_or_default(),
        };
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

        if !result.next_row() {
            break;
        }
    }

    Ok(())
}

async fn load_linked_respawns_like_cpp(
    db: &WorldDatabase,
    store: &SpawnStore,
    map_store: &wow_data::MapStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<LinkedRespawnStoreLikeCpp> {
    let stmt = db.prepare(WorldStatements::SEL_LINKED_RESPAWNS);
    let mut result = db.query(&stmt).await?;
    let mut linked_store = LinkedRespawnStoreLikeCpp::new();
    if result.is_empty() {
        return Ok(linked_store);
    }

    loop {
        let row = LinkedRespawnDbRow {
            guid: read_unsigned_db_u64_like_cpp(&result, 0, "linked_respawn.guid")?,
            linked_guid: read_unsigned_db_u64_like_cpp(&result, 1, "linked_respawn.linkedGuid")?,
            link_type: read_unsigned_db_u8_like_cpp(&result, 2, "linked_respawn.linkType")?,
        };
        apply_linked_respawn_row_like_cpp(
            row.into(),
            store,
            map_store,
            &mut linked_store,
            &mut report.linked_respawn,
        );

        if !result.next_row() {
            break;
        }
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

async fn load_spawn_group_members_like_cpp(db: &WorldDatabase) -> Result<Vec<SpawnGroupMemberRow>> {
    let stmt = db.prepare(WorldStatements::SEL_SPAWN_GROUP_MEMBERS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(Vec::new());
    }

    let mut rows = Vec::new();
    loop {
        rows.push(SpawnGroupMemberRow {
            group_id: result.read(0),
            spawn_type: result.read(1),
            spawn_id: result.read(2),
        });
        if !result.next_row() {
            break;
        }
    }

    Ok(rows)
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

fn creature_spawntimesecs_to_i32_like_cpp(value: u32) -> Result<i32> {
    // C++ `ObjectMgr::LoadCreatures` reads `creature.spawntimesecs` with
    // `Field::GetUInt32()` and stores it in `SpawnData::spawntimesecs` (int32).
    // Creature DB rows are unsigned; reject impossible values instead of
    // silently wrapping them into negative respawn delays.
    if value > i32::MAX as u32 {
        bail!(
            "creature.spawntimesecs value {value} exceeds the represented int32 SpawnData domain"
        );
    }

    Ok(value as i32)
}

fn read_unsigned_db_u32_like_cpp(
    result: &SqlResult,
    column: usize,
    field_name: &str,
) -> Result<u32> {
    if result.is_null(column) {
        return Ok(0);
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(value);
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return normalize_u64_db_u32_like_cpp(value, field_name);
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return Ok(u32::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return Ok(u32::from(value));
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u32_like_cpp(i64::from(value), field_name);
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return normalize_signed_db_u32_like_cpp(value, field_name);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u32_like_cpp(i64::from(value), field_name);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_db_u32_like_cpp(i64::from(value), field_name);
    }

    bail!("could not decode {field_name} at column {column} as a C++ unsigned DB field")
}

fn read_unsigned_db_u64_like_cpp(
    result: &SqlResult,
    column: usize,
    field_name: &str,
) -> Result<u64> {
    if result.is_null(column) {
        return Ok(0);
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return Ok(value);
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(u64::from(value));
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return Ok(u64::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return Ok(u64::from(value));
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return normalize_signed_db_u64_like_cpp(value, field_name);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u64_like_cpp(i64::from(value), field_name);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u64_like_cpp(i64::from(value), field_name);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_db_u64_like_cpp(i64::from(value), field_name);
    }

    bail!("could not decode {field_name} at column {column} as a C++ unsigned 64-bit DB field")
}

fn read_unsigned_db_u8_like_cpp(result: &SqlResult, column: usize, field_name: &str) -> Result<u8> {
    let value = read_unsigned_db_u32_like_cpp(result, column, field_name)?;
    if value > u32::from(u8::MAX) {
        bail!("{field_name} value {value} exceeds the represented u8 domain");
    }

    Ok(value as u8)
}

fn read_unsigned_db_u16_like_cpp(
    result: &SqlResult,
    column: usize,
    field_name: &str,
) -> Result<u16> {
    let value = read_unsigned_db_u32_like_cpp(result, column, field_name)?;
    if value > u32::from(u16::MAX) {
        bail!("{field_name} value {value} exceeds the represented u16 domain");
    }

    Ok(value as u16)
}

fn read_signed_db_i8_like_cpp(result: &SqlResult, column: usize, field_name: &str) -> Result<i8> {
    if result.is_null(column) {
        return Ok(0);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return Ok(value);
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return Ok(value as i8);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_i8_like_cpp(i64::from(value), field_name);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_i8_like_cpp(i64::from(value), field_name);
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return normalize_signed_db_i8_like_cpp(value, field_name);
    }

    bail!("could not decode {field_name} at column {column} as a C++ signed 8-bit DB field")
}

fn normalize_u64_db_u32_like_cpp(value: u64, field_name: &str) -> Result<u32> {
    if value > u64::from(u32::MAX) {
        bail!("{field_name} value {value} exceeds the represented u32 domain");
    }

    Ok(value as u32)
}

fn normalize_signed_db_u64_like_cpp(value: i64, field_name: &str) -> Result<u64> {
    if value < 0 {
        bail!("{field_name} value {value} is negative but C++ reads this field as unsigned");
    }

    Ok(value as u64)
}

fn normalize_signed_db_i8_like_cpp(value: i64, field_name: &str) -> Result<i8> {
    if value < i64::from(i8::MIN) || value > i64::from(i8::MAX) {
        bail!("{field_name} value {value} exceeds the represented i8 domain");
    }

    Ok(value as i8)
}

fn normalize_signed_db_u32_like_cpp(value: i64, field_name: &str) -> Result<u32> {
    if value < 0 {
        bail!("{field_name} value {value} is negative but C++ reads this field as unsigned");
    }

    normalize_u64_db_u32_like_cpp(value as u64, field_name)
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
