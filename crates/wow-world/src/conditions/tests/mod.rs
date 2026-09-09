//! Condition snapshot borrows regression scenarios.
//!
//! Separated from the conditions.rs root under #656.

use super::*;
use wow_constants::{ConditionType, PhaseFlags, TypeId, TypeMask};
use wow_core::Position;
use wow_data::{PlayerConditionContextLikeCpp, PlayerConditionEntry, PlayerConditionStore};
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
