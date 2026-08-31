//! SQLx-free staged source contract for GameEvent startup World catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEventDataPersistenceRowLikeCpp {
    pub event_id: u16,
    pub start: u64,
    pub end: u64,
    pub occurence: u32,
    pub length: u32,
    pub holiday_id: u32,
    pub holiday_stage: u8,
    pub description: String,
    pub state_raw: u8,
    pub announce: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventPrerequisitePersistenceRowLikeCpp {
    pub event_id: u16,
    pub prerequisite_event: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventConditionPersistenceRowLikeCpp {
    pub event_id: u16,
    pub condition_id: u32,
    pub req_num: f32,
    pub max_world_state: u16,
    pub done_world_state: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameEventWorldCatalogPrefixLikeCpp {
    pub max_event_entry: Option<u32>,
    pub events: Vec<GameEventDataPersistenceRowLikeCpp>,
    pub prerequisites: Vec<GameEventPrerequisitePersistenceRowLikeCpp>,
    pub conditions: Vec<GameEventConditionPersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventQuestConditionPersistenceRowLikeCpp {
    pub quest_id: u32,
    pub event_id: u16,
    pub condition_id: u32,
    pub num: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventPoolPersistenceRowLikeCpp {
    pub pool_entry: u32,
    pub event_id: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameEventObjectGuidPersistenceRowLikeCpp {
    pub guid: u64,
    pub event_id: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureEquipmentIdPersistenceRowLikeCpp {
    pub creature_id: u32,
    pub equipment_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventModelEquipPersistenceRowLikeCpp {
    pub spawn_id: u64,
    pub entry: u32,
    pub event_id: u16,
    pub model_id: u32,
    pub equipment_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventQuestRelationPersistenceRowLikeCpp {
    pub event_id: u8,
    pub giver_id: u32,
    pub quest_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEventNpcFlagPersistenceRowLikeCpp {
    pub spawn_id: u64,
    pub event_id: u16,
    pub npcflag: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEventNpcVendorPersistenceRowLikeCpp {
    pub event_id: u8,
    pub spawn_id: u64,
    pub item: u32,
    pub maxcount: u32,
    pub incrtime: u32,
    pub extended_cost: u32,
    pub vendor_type: u8,
    pub bonus_list_ids: String,
    pub player_condition_id: u32,
    pub ignore_filtering: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameEventWorldCatalogSuffixLikeCpp {
    pub quest_conditions: Vec<GameEventQuestConditionPersistenceRowLikeCpp>,
    pub pools: Vec<GameEventPoolPersistenceRowLikeCpp>,
    pub creature_guids: Vec<GameEventObjectGuidPersistenceRowLikeCpp>,
    pub gameobject_guids: Vec<GameEventObjectGuidPersistenceRowLikeCpp>,
    pub equipment_ids: Vec<CreatureEquipmentIdPersistenceRowLikeCpp>,
    pub model_equips: Vec<GameEventModelEquipPersistenceRowLikeCpp>,
    pub creature_quest_relations: Vec<GameEventQuestRelationPersistenceRowLikeCpp>,
    pub gameobject_quest_relations: Vec<GameEventQuestRelationPersistenceRowLikeCpp>,
    pub npc_flags: Vec<GameEventNpcFlagPersistenceRowLikeCpp>,
    pub npc_vendors: Vec<GameEventNpcVendorPersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameEventWorldCatalogLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

/// Two World stages separated by the existing Character condition-save load.
pub trait GameEventWorldCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_prefix_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameEventWorldCatalogLoadOutcomeLikeCpp<GameEventWorldCatalogPrefixLikeCpp>,
    >;

    fn load_suffix_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameEventWorldCatalogLoadOutcomeLikeCpp<GameEventWorldCatalogSuffixLikeCpp>,
    >;
}
