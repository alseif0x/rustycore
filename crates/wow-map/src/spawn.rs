//! Spawn metadata and cell indexes used by grid loading.
//!
//! C++ references:
//! - `game/Maps/SpawnData.h`
//! - `game/Globals/ObjectMgr.cpp` (`AddSpawnDataToGrid`)
//! - `game/Globals/AreaTriggerDataStore.cpp` (`LoadAreaTriggerSpawns`)

use std::collections::{BTreeMap, BTreeSet};

use crate::coords::compute_cell_coord;
use wow_core::ObjectGuid;

mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "spawn/tests/mod.rs"]
mod tests;
