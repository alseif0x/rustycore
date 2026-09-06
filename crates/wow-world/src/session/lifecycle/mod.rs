// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session login, logout and cleanup lifecycle.
//!
//! The exact current behaviour, moved out of `session/mod.rs` unchanged. The
//! ordering each part must preserve is documented beside the code that owns
//! it rather than in a separate note that can drift.

mod cleanup;
mod login;
mod logout;
mod map_entry;
mod persistence;
mod pet_load;

pub(super) use pet_load::PetLoadQueryHolderRowsLikeCpp;
