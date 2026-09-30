//! GameObject template, loot and runtime state state definitions, part 1 of 2.
//!
//! Separated from the game_object.rs root under #636. Behaviour is preserved.

use super::*;

pub const DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS: u32 = 300;

pub const GAMEOBJECT_LOOT_MODE_DEFAULT: u16 = 0x1;

pub const GO_DYNFLAG_LO_ACTIVATE: u32 = 0x0004;

pub const GO_DYNFLAG_LO_DEPLETED: u32 = 0x0010;

pub const GO_DYNFLAG_LO_SPARKLE: u32 = 0x0020;

pub const GO_DYNFLAG_LO_NO_INTERACT: u32 = 0x0080;

pub const GO_DYNFLAG_LO_HIGHLIGHT: u32 = 0x0200;

// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h:2892
pub const GO_FLAG_IN_USE: u32 = 0x0000_0001;

pub const GO_FLAG_INTERACT_COND: u32 = 0x0000_0004;

// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h:2902
pub const GO_FLAG_NODESPAWN: u32 = 0x0000_0020;

// C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Miscellaneous/SharedDefines.h:2914
pub const GO_FLAG_MAP_OBJECT: u32 = 0x0010_0000;

pub const GO_FLAG_IN_MULTI_USE: u32 = 0x0020_0000;

pub const GAME_OBJECT_DATA_PARENT_BIT: usize = 0;

pub const GAME_OBJECT_DATA_DISPLAY_ID_BIT: usize = 4;

pub const GAME_OBJECT_DATA_CREATED_BY_BIT: usize = 9;

pub const GAME_OBJECT_DATA_FLAGS_BIT: usize = 11;

pub const GAME_OBJECT_DATA_FACTION_TEMPLATE_BIT: usize = 13;

pub const GAME_OBJECT_DATA_LEVEL_BIT: usize = 14;

pub const GAME_OBJECT_DATA_STATE_BIT: usize = 15;

pub const GAME_OBJECT_DATA_TYPE_ID_BIT: usize = 16;

pub const GAME_OBJECT_DATA_PERCENT_HEALTH_BIT: usize = 17;

pub const GAME_OBJECT_DATA_ART_KIT_BIT: usize = 18;

pub const GAME_OBJECT_DATA_CUSTOM_PARAM_BIT: usize = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum GoState {
    Active = 0,
    Ready = 1,
    Destroyed = 2,
    TransportActive = 24,
    TransportStopped = 25,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LootState {
    NotReady = 0,
    Ready = 1,
    Activated = 2,
    JustDeactivated = 3,
}

/// Represented status for the `m_despawnDelay` branch of TrinityCore
/// `GameObject::Update(diff)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectUpdateStatusLikeCpp {
    Updated,
    DespawnRequested,
}

/// Evidence for the bounded Rust representation of TrinityCore
/// `GameObject::Update(diff)`.
///
/// C++ anchors:
/// - `GameObject.cpp:1215-1233`: `WorldObject::Update(diff)`, AI lookup/
///   initialization branch, `m_despawnDelay` countdown, and immediate
///   `DespawnOrUnsummon(0ms, m_despawnRespawnTime)` when the delay expires.
/// - `GameObject.cpp:1235-1274` and `1276+`: go-type implementation,
///   per-player state/visibility packets and loot-state machine remain explicit
///   gaps; booleans here mark non-represented branches, not execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectUpdateOutcomeLikeCpp {
    pub diff_ms: u32,
    pub status: GameObjectUpdateStatusLikeCpp,
    pub despawn_delay_before_ms: u32,
    pub despawn_delay_after_ms: u32,
    pub despawn_respawn_time_secs: u32,
    pub world_update_would_run: bool,
    pub ai_update_not_represented: bool,
    pub go_type_impl_update_not_represented: bool,
    pub despawn_or_unsummon_requested: bool,
}

/// Evidence for the bounded Rust representation of TrinityCore
/// `GameObject::EnableCollision(bool)`.
///
/// C++ anchor: `GameObject.cpp:3856-3864` returns early when `!m_model`; otherwise it only
/// forwards the requested value to `m_model->enableCollision(enable)`. The commented map insert
/// remains intentionally unrepresented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectCollisionOutcomeLikeCpp {
    pub requested_enable: bool,
    pub represented_model_present: bool,
    pub previous_collision_enabled: Option<bool>,
    pub new_collision_enabled: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GameObjectOwnedLoot {
    pub(super) gold: u32,
    pub(super) unlooted_count: u32,
}

impl GameObjectOwnedLoot {
    pub const fn new(gold: u32, unlooted_count: u32) -> Self {
        Self {
            gold,
            unlooted_count,
        }
    }

    pub const fn gold(&self) -> u32 {
        self.gold
    }

    pub const fn unlooted_count(&self) -> u32 {
        self.unlooted_count
    }

    pub const fn is_looted_like_cpp(&self) -> bool {
        self.gold == 0 && self.unlooted_count == 0
    }
}

pub(super) fn game_object_owned_loot_from_snapshot(
    snapshot: &OwnedLootSnapshot,
) -> GameObjectOwnedLoot {
    GameObjectOwnedLoot::new(snapshot.loot.coins, u32::from(snapshot.loot.unlooted_count))
}

/// Resolved, testable input for TrinityCore `GameObject::Create`.
///
/// Transport type 11 remains a resolved intrinsic shape only here: transport GUID/server-time,
/// implementation data, `startOpen`, active state transitions, pathing and passenger runtime are
/// owned by future transport/map wiring, not by this entity lifecycle record.
#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectCreateLifecycleRecord {
    pub guid: ObjectGuid,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub rotation: [f32; 4],
    pub anim_progress: u8,
    pub go_state: GoState,
    pub art_kit: u32,
    pub dynamic: bool,
    pub spawn_id: u64,
    pub template: GameObjectTemplateLifecycleRecord,
}

/// Represented subset of TrinityCore `GameObjectData` consumed by `GameObject::LoadFromDB`.
#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectLoadFromDbLifecycleRecord {
    pub create: GameObjectCreateLifecycleRecord,
    pub spawntimesecs: i32,
    /// Effective map-owned respawn time after caller-owned `Map`/time processing.
    ///
    /// C++ `LoadFromDB` clears due timers (`respawnTime <= now`) and calls `RemoveRespawnTime`
    /// before applying entity state. `wow-entities` does not own Map time or DB timer removal, so
    /// callers must pre-normalize due timers to `0` before building this record.
    pub effective_map_respawn_time: i64,
    pub despawn_possible: bool,
    pub despawn_at_action: bool,
    pub respawn_compatibility_mode: bool,
    /// C++ stores `GameObjectData::StringId` in `m_stringIds[1]`; Rust stores the represented
    /// lifecycle handoff value as entity metadata only, with DB/phasing/AddToMap still external.
    pub string_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameObjectLifecycleError {
    InvalidGameObjectType { entry: u32, go_type: u32 },
    InvalidMapObjectTransportType { entry: u32 },
    InvalidPosition { entry: u32, position: Position },
    MapBinding { entry: u32, source: MapBindingError },
}
