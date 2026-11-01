// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot-template rolling rules: re-exported from `wow-world-lifecycle` after the
//! #1263 F5 move. Test-only since #1363 slice 4c moved the item-container
//! bodies that used them to `wow_world_loot_ops`; the spell unit tests name them.

#![cfg(test)]

pub(super) use wow_world_lifecycle::LootTemplateRow;
pub(super) use wow_world_lifecycle::loot_template_rules::*;
