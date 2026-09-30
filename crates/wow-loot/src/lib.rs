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

mod active_views;
mod authority;
mod disenchant_generation;
mod distribution;
mod item_access;
mod pool_construction;
mod response_view;
mod random_enchantment;
pub use random_enchantment::select_random_enchantment;
mod source_consumption;
pub use source_consumption::remaining_source_item_count;
mod rolls;
mod store;
pub use active_views::LootViews;
pub use authority::*;
pub use disenchant_generation::DisenchantLootBuilder;
pub use distribution::*;
pub use item_access::{
    DirectItemRejection, MasterItemRejection, direct_item_rejection, find_unlooted_item,
    master_award_recipient_allowed, roll_award_batch_allowed, select_master_item,
    loot_store_item_matches,
};
pub use response_view::{LootItemView, loot_response_item_views};
pub use pool_construction::{
    loot_entry_from_generated, materialize_loot_pools, shared_loot_entry_from_generated,
};
pub use rolls::{
    ROLL_ALL_TYPE_MASK_LIKE_CPP, ROLL_FLAG_TYPE_DISENCHANT_LIKE_CPP, ROLL_FLAG_TYPE_NEED_LIKE_CPP,
    ROLL_VOTE_DISENCHANT_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP, ROLL_VOTE_NEED_LIKE_CPP,
    ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP, ROLL_VOTE_PASS_LIKE_CPP,
    RepresentedLootRollVote, RollBallots, represented_loot_roll_current_winner_like_cpp,
    represented_loot_roll_finish_winner_like_cpp, represented_loot_roll_valid_rolls_like_cpp,
};
pub use store::*;

#[cfg(test)]
#[path = "store/tests/mod.rs"]
mod tests;
