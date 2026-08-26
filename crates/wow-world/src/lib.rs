// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World server core: session management, handlers, and world state.

pub(crate) mod battle_pet_account;
pub(crate) mod battle_pet_purchase;
pub mod canonical_player_access;
mod canonical_player_sync;
pub mod conditions;
pub mod entity_update_bridge;
pub mod handlers;
pub mod loot_persistence;
pub mod map_manager;
pub mod phasing;
#[path = "session/directory.rs"]
pub mod player_directory;
mod player_directory_canonical_queries;
#[allow(dead_code)] // Private prerequisite seam consumed by trainer issue #157.
pub(crate) mod profession;
pub mod reputation;
pub mod session;
mod session_commands;
mod session_policy;
#[allow(dead_code)] // Private prerequisite seam consumed by trainer issue #157.
pub(crate) mod spell_acquisition;
#[allow(dead_code)] // Private decision seam introduced by trainer issue #157.
pub(crate) mod trainer_offer;

#[cfg(test)]
mod handler_contract_tests;
#[cfg(any(test, feature = "test-fixtures"))]
mod player_directory_test_fixtures;
#[cfg(test)]
mod player_lifecycle_contract;

pub use map_manager::{
    ChaseTargetSnapshotLikeCpp, GridCoord, MapManager, SharedMapManager, WorldCreature,
    WorldMMapPathfinderWorkerLikeCpp,
};
pub use session::{
    MMapRuntimeConfigLikeCpp, SharedCanonicalMapManager, SharedObjectAccessor, WorldSession,
    new_shared_object_accessor,
};
pub use session_policy::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
    LootDropRatesLikeCpp, PacketSpoofConfigLikeCpp, ReputationRatesLikeCpp,
};

pub use battle_pet_account::{
    BattlePetAccountAttachmentLikeCpp, BattlePetAccountRegistryLikeCpp,
    LoginBattlePetPersistenceLikeCpp,
};
