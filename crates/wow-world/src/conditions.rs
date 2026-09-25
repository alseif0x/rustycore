// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Compatibility facade for the standalone C++ `ConditionMgr` application boundary.
//!
//! The implementation and its focused scenarios live in `wow-conditions`; this
//! re-export preserves the existing `wow_world::conditions::*` paths for world
//! handlers and downstream callers.

pub use wow_conditions::*;
