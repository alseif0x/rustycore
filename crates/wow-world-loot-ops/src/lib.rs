// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot operations shared by World adapters (#1363 A7: moved from
//! `wow-world-application`).
//!
//! The loot handlers and the loot-release operations sit above
//! `wow-world-quest` (loot release reads quest eligibility, conditions and
//! the registry sync); `wow-world-loot` stays the loot state owner.

mod loot_handlers;
mod loot_release;
pub use loot_handlers::{
    LootHandlerCxLikeCpp, LootHandlerHostLikeCpp, register_loot_handlers_like_cpp,
};
pub use loot_release::{
    LootReleaseCxLikeCpp, direct_item_count_after_loot_release_like_cpp,
    durable_loot_item_fanout_viewers_like_cpp,
    queue_chest_gameobject_state_refresh_for_same_map_like_cpp,
    represented_gameobject_can_autostore_loot_item_like_cpp,
};
