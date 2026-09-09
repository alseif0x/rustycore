use std::collections::HashMap;

use wow_constants::{TypeId, TypeMask};
use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;

use crate::{
    AreaTrigger, Conversation, Corpse, Creature, DynamicObject, GameObject, Item, Pet, Player,
    PlayerInventoryStorage, SceneObject, Transport, WorldObject,
};

mod ops_1;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "object_accessor/tests/mod.rs"]
mod tests;
