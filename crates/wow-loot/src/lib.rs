// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++-shaped loot store primitives.
//!
//! This module mirrors the small, reusable parts of TrinityCore's
//! `LootStoreItem`, `LootTemplate`, and private `LootGroup` model. Runtime
//! condition evaluation and `Loot::FillLoot` orchestration are intentionally
//! layered above this crate.

use std::collections::{HashMap, HashSet};

use rand::Rng;
use wow_core::ObjectGuid;

mod authority;
mod store;
pub use store::*;

pub use authority::{
    CreatureLoot, LootClaimCommitError, LootClaimError, LootClaimLease, LootClaimPayload,
    LootClaimPersistenceGuard, LootEntry, LootEntryFlags, LootInstallOutcome, LootItemClaimKey,
    LootRoundRobinReleaseOutcome, LootViewerCloseOutcome, LootViewerOpenOutcome, NotNormalLootItem,
    OwnedLootAuthority, OwnedLootAuthorityLifecycle, OwnedLootAuthorityStamp, OwnedLootScope,
    OwnedLootSnapshot,
};

#[cfg(test)]
#[path = "store/tests/mod.rs"]
mod tests;
