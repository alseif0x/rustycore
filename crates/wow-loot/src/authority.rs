// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Compatibility re-exports for entity-owned runtime loot authority.

pub use wow_constants::LOOT_SLOT_TYPE_OWNER_LIKE_CPP;
pub use wow_entities::{
    CreatureLoot, LootClaimCommitError, LootClaimError, LootClaimLease, LootClaimPayload,
    LootClaimPersistenceGuard, LootEntry, LootEntryFlags, LootFullyLootedLifecycleObservation,
    LootInstallOutcome, LootItemClaimKey, LootRoundRobinReleaseOutcome, LootViewerCloseOutcome,
    LootViewerOpenOutcome, NotNormalLootItem, OwnedLootAuthority, OwnedLootAuthorityLifecycle,
    OwnedLootAuthorityStamp, OwnedLootScope, OwnedLootSnapshot,
};
