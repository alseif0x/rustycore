// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World server core: session management, handlers, and world state.

pub mod conditions;
pub mod entity_update_bridge;
pub mod handlers;
pub mod map_manager;
pub mod phasing;
#[allow(dead_code)] // Private prerequisite seam consumed by trainer issue #157.
pub(crate) mod profession;
pub mod reputation;
pub mod session;
#[allow(dead_code)] // Private prerequisite seam consumed by trainer issue #157.
pub(crate) mod spell_acquisition;

#[cfg(test)]
mod handler_contract_tests;

pub use map_manager::{
    ChaseTargetSnapshotLikeCpp, GridCoord, MapManager, SharedMapManager, WorldCreature,
    WorldMMapPathfinderWorkerLikeCpp,
};
pub use session::{
    MMapRuntimeConfigLikeCpp, SharedCanonicalMapManager, SharedObjectAccessor, WorldSession,
    new_shared_object_accessor,
};
