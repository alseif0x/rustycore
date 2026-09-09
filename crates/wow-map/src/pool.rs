//! Pure C++-shaped pool data helpers.
//!
//! Source of truth: TrinityCore `PoolMgr.h` / `PoolMgr.cpp` pool data and
//! `PoolGroup<T>` helpers. This module intentionally does not implement live
//! `PoolMgr` runtime, RNG, DB loading, entity creation, or live side effects.
//! It does implement deterministic C++-shaped `SpawnPool`/`DespawnPool` plans
//! over the caller-provided map-owned `SpawnedPoolDataLikeCpp`; plans record
//! future live side effects without performing DB writes, AddToMap/RemoveFromMap,
//! packet fanout, or entity creation/destruction.

use crate::map::SpawnedPoolDataLikeCpp;
use crate::spawn::{SpawnId, SpawnObjectType};
use std::collections::{HashMap, HashSet};

mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "pool/tests/mod.rs"]
mod tests;
