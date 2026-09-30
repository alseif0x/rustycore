// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory move planning and execution for character item handlers.
//!
//! This private module preserves the existing `WorldSession` ownership while
//! isolating the swap/equip/stack transition family from packet adapters.

use super::*;

mod item_mutations;
mod real_swap;

mod validation;
mod child_redirect;
mod child_equipment;
mod swap_execution;
mod publication;
