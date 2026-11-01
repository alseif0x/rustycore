//! Condition snapshot borrows regression scenarios.
//!
//! These tests exercise the standalone `wow-conditions` application boundary.

use super::*;
use std::sync::Arc;
use wow_constants::{
    ComparisonType, ConditionInstanceInfo, ConditionSourceType, ConditionType, PhaseFlags,
    RelationType, TypeId, TypeMask, UnitStandStateType,
};
use wow_core::Position;
use wow_data::{
    Condition, ConditionEntriesByTypeStore, NpcSpellClickStoreLikeCpp,
    PlayerConditionContextLikeCpp, PlayerConditionEntry, PlayerConditionStore,
    SPELL_CLICK_USER_FRIEND_LIKE_CPP, SPELL_CLICK_USER_PARTY_LIKE_CPP,
    SPELL_CLICK_USER_RAID_LIKE_CPP, UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
};
use wow_entities::WorldObject;
use wow_loot::{LootStoreItem, LootStoreItemContext, LootStoreKind};

fn world_object(map_id: u32, instance_id: u32) -> WorldObject {
    let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
    object.set_map(map_id, instance_id).unwrap();
    object
}

fn player_object(map_id: u32, instance_id: u32) -> WorldObject {
    let mut object = WorldObject::new(false, TypeId::Player, TypeMask::PLAYER | TypeMask::UNIT);
    object.set_map(map_id, instance_id).unwrap();
    object
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
