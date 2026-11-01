// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Mount for the inventory packet handler scenarios. The handler bodies moved
//! to `wow_world_inventory::item_session` and their test entry points to
//! `handlers/item_test_shims.rs` (#1363 slice 8c).

#[cfg(test)]
use super::*;

#[cfg(test)]
#[path = "../../../../unit_tests/handlers/character/items/handlers/test_only.rs"]
mod test_only;
