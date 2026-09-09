use std::collections::{HashMap, HashSet};

use wow_constants::{TypeId, TypeMask};
use wow_core::{ObjectGuid, Position};
use wow_loot::{
    CreatureLoot, LootInstallOutcome, OwnedLootAuthority, OwnedLootAuthorityLifecycle,
    OwnedLootAuthorityStamp, OwnedLootSnapshot,
};

use crate::{
    CreateObjectFlags, MapBindingError, ObjectChangedFields, ObjectDataUpdate, UpdateMask,
    WorldObject,
    update_fields::{GAME_OBJECT_DATA_BITS, TYPEID_GAME_OBJECT},
};

mod ops_1;
mod ops_2;
mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "game_object/tests/mod.rs"]
mod tests;
