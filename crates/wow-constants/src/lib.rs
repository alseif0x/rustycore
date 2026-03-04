// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! WoW constants, enums, opcodes, and bitflags.
//!
//! This crate contains all the World of Warcraft protocol enums,
//! opcodes, and flag types translated from the C# RustyCore source.

pub mod creature;
pub mod item;
pub mod movement;
pub mod object;
pub mod opcodes;
pub mod shared;
pub mod spell;
pub mod unit;
pub mod update;

// Re-export key types for convenience
pub use creature::*;
pub use item::*;
pub use movement::*;
pub use object::*;
pub use opcodes::*;
pub use shared::*;
pub use spell::*;
pub use unit::*;
pub use update::*;
