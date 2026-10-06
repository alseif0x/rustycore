// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loaded item restoration: re-exported from `wow-world-inventory` after the
//! #1263 F5 move.

pub(in crate::handlers::character) use wow_world_inventory::loaded_item_support::*;
