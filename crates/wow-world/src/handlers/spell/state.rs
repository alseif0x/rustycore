// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot-template rolling rules: re-exported from `wow-world-lifecycle` after the
//! #1263 F5 move.

pub(super) use wow_world_lifecycle::loot_template_rules::*;
pub(super) use wow_world_lifecycle::{
    LootTemplateRow, LootTemplateTable, WrappedGiftLoad, WrappedGiftRow,
};
