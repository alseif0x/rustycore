// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map-object-owned runtime loot and cancellation-safe claims.
//!
//! TrinityCore owns one shared `Loot` plus an optional GUID-keyed personal-loot map on
//! `Creature` and `GameObject`. `GetLootForPlayer` uses shared loot only while the personal
//! map is empty (`Creature.cpp:1377-1386`, `GameObject.cpp:3898-3907`). The C++ world-session
//! scheduler serializes loot handlers globally. Rust sessions run concurrently, so this module
//! preserves the same single-owner result with short synchronous critical sections and async
//! waiters. No authority lock is held across an `.await`.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::watch;
use wow_core::ObjectGuid;

use crate::LOOT_SLOT_TYPE_OWNER_LIKE_CPP;

mod ops_1;
mod ops_2;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "authority/tests/mod.rs"]
mod tests;
