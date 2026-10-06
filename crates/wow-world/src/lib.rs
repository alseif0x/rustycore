// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World server core: session management, handlers, and world state.

pub(crate) use wow_world_core::battle_pet_account;
pub(crate) mod battle_pet_purchase;
pub mod canonical_player_access;
mod canonical_player_sync;
mod character_administration;
mod finalization;
pub use finalization::{
    FinalizationDisposition, FinalizationMode, FinalizationOutcome, FinalizationReport,
    FinalizationStep,
};
pub mod conditions;
pub mod entity_update_bridge;
pub mod handler_composition;
pub mod handlers;
pub mod loot_persistence;
pub use wow_world_core::map_manager;
#[allow(dead_code)] // Private decision seam introduced by trainer issue #157.
#[cfg(test)]
#[path = "../unit_tests/handler_contract_tests.rs"]
mod handler_contract_tests;
mod player;
mod player_cast;
#[path = "session/directory.rs"]
pub mod player_directory;
#[allow(dead_code)] // Private prerequisite seam consumed by trainer issue #157.
pub(crate) mod profession;
mod quest;
pub mod session;
mod session_commands;
mod session_persistence_capabilities;
mod session_policy;
mod session_rules;
#[allow(dead_code)] // Private prerequisite seam consumed by trainer issue #157.
pub(crate) mod spell_acquisition;
mod spell_cast_adapter;
#[cfg(test)]
#[path = "../unit_tests/teleport_test_fixtures.rs"]
mod teleport_test_fixtures;
#[cfg(test)]
#[path = "../unit_tests/vendor_trade_persistence_test_fixture.rs"]
mod vendor_trade_persistence_test_fixture;

pub use map_manager::{
    ChaseTargetSnapshotLikeCpp, GridCoord, MapManager, SharedMapManager, WorldCreature,
    WorldMMapPathfinderWorkerLikeCpp,
};
pub use session::{MMapRuntimeConfigLikeCpp, SharedCanonicalMapManager, WorldSession};
pub use session_policy::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
    LootDropRatesLikeCpp, PacketSpoofConfigLikeCpp, PlayerRegenerationRatesLikeCpp,
    ReputationRatesLikeCpp,
};

pub use battle_pet_account::{BattlePetAccountAttachmentLikeCpp, BattlePetAccountRegistryLikeCpp};
