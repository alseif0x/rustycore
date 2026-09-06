//! Map respawn/corpse and game-event durability capabilities; no runtime ownership.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp};

/// Raw Character DB respawn row. Object and map identifiers deliberately stay
/// database-shaped here so the adapter can report malformed values without
/// depending on either map runtime's enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespawnPersistenceRowLikeCpp {
    pub object_type_raw: u16,
    pub spawn_id: u64,
    pub respawn_time: i64,
    pub map_id: u32,
    pub instance_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RespawnPersistenceKeyLikeCpp {
    pub object_type_raw: u16,
    pub spawn_id: u64,
    pub map_id: u16,
    pub instance_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnPersistenceMutationLikeCpp {
    Save {
        key: RespawnPersistenceKeyLikeCpp,
        respawn_time: i64,
    },
    Delete {
        key: RespawnPersistenceKeyLikeCpp,
    },
}

impl RespawnPersistenceMutationLikeCpp {
    pub const fn key(self) -> RespawnPersistenceKeyLikeCpp {
        match self {
            Self::Save { key, .. } | Self::Delete { key } => key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespawnPersistenceLoadOutcomeLikeCpp {
    Loaded(Vec<RespawnPersistenceRowLikeCpp>),
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespawnPersistenceMutationOutcomeLikeCpp {
    Applied { affected_rows: u64 },
    Failed { reason: String },
}

/// SQLx-free capability for C++ `Map` respawn durability. The map owners build
/// typed mutations; the MariaDB adapter alone selects statements, binds values,
/// decodes rows and executes them.
pub trait RespawnPersistencePortLikeCpp: Send + Sync {
    fn load_for_map_like_cpp<'a>(
        &'a self,
        map_id: u16,
        instance_id: u32,
    ) -> PersistenceFutureLikeCpp<'a, RespawnPersistenceLoadOutcomeLikeCpp>;

    fn load_all_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, RespawnPersistenceLoadOutcomeLikeCpp>;

    fn execute_mutation_like_cpp<'a>(
        &'a self,
        mutation: RespawnPersistenceMutationLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, RespawnPersistenceMutationOutcomeLikeCpp>;
}

/// Raw `game_event_condition_save` row. Validation against the canonical
/// event/condition stores remains with the game-event owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventConditionSavePersistenceRowLikeCpp {
    pub event_id: u8,
    pub condition_id: u32,
    pub done: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameEventPersistenceMutationLikeCpp {
    ReplaceConditionSave {
        event_id: u8,
        condition_id: u32,
        done: f32,
    },
    SaveWorldEventState {
        event_id: u8,
        state: u8,
        next_start: i64,
    },
    DeleteWorldEventState {
        event_id: u8,
        delete_condition_saves: bool,
        delete_world_event_state: bool,
    },
    ResetSeasonalQuests {
        event_id: u16,
        event_start_time: i64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameEventConditionSaveLoadOutcomeLikeCpp {
    Loaded(Vec<GameEventConditionSavePersistenceRowLikeCpp>),
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEventPersistenceMutationOutcomeLikeCpp {
    Applied,
    Failed { reason: String },
}

pub trait GameEventPersistencePortLikeCpp: Send + Sync {
    fn load_condition_saves_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, GameEventConditionSaveLoadOutcomeLikeCpp>;

    fn execute_mutation_like_cpp<'a>(
        &'a self,
        mutation: GameEventPersistenceMutationLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GameEventPersistenceMutationOutcomeLikeCpp>;
}

/// One canonical-map corpse hydration request.
///
/// C++ `Map::LoadCorpseData` owns the state transition. This request keeps the
/// database identity out of the map/application layer while preserving the
/// exact `(mapId, instanceId)` scope shared by all three reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCorpseLoadRequestLikeCpp {
    pub map_id: u32,
    pub instance_id: u32,
}

impl MapCorpseLoadRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapCorpseLoadRowLikeCpp {
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub orientation: f32,
    pub map_id: u16,
    pub display_id: u32,
    pub item_cache: String,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub flags: u8,
    pub dynamic_flags: u8,
    pub ghost_time: u32,
    pub corpse_type: u8,
    pub instance_id: u32,
    pub owner_guid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCorpsePhaseLoadRowLikeCpp {
    pub owner_guid: u64,
    pub phase_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCorpseCustomizationLoadRowLikeCpp {
    pub owner_guid: u64,
    pub option_id: u32,
    pub choice_id: u32,
}

/// Each auxiliary read may fail independently after the base corpse rows have
/// loaded. C++ continues without that auxiliary data in either case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapCorpseAuxiliaryLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MapCorpseLoadOutcomeLikeCpp {
    Loaded {
        corpses: Vec<MapCorpseLoadRowLikeCpp>,
        phases: MapCorpseAuxiliaryLoadOutcomeLikeCpp<MapCorpsePhaseLoadRowLikeCpp>,
        customizations: MapCorpseAuxiliaryLoadOutcomeLikeCpp<MapCorpseCustomizationLoadRowLikeCpp>,
    },
    Failed {
        reason: String,
    },
}

/// SQLx-free persistence boundary for C++ `Map::LoadCorpseData`.
pub trait MapCorpsePersistencePortLikeCpp: Send + Sync {
    fn load_map_corpses_like_cpp<'a>(
        &'a self,
        request: MapCorpseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, MapCorpseLoadOutcomeLikeCpp>;
}
