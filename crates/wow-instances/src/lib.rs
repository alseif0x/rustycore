// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `game/Instances` foundation.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock, Weak},
};

use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{DungeonEncounterEntry, DungeonEncounterStore};
use wow_persistence::{InstanceLockPersistenceMutationLikeCpp, InstanceLockPersistencePlanLikeCpp};

pub use wow_persistence::{
    CharacterInstanceLockPersistenceRowLikeCpp as CharacterInstanceLockRow,
    SharedInstanceLockPersistenceRowLikeCpp as SharedInstanceLockRow,
};

mod instances;
#[cfg(test)]
#[path = "instances/tests/mod.rs"]
mod tests;
pub use instances::*;
