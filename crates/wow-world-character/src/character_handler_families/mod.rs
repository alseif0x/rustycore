// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private sibling of the character handler owner module.
//!
//! C++ source of truth: `CharacterHandler.cpp` (`HandlePlayerLoginOpcode` and
//! the other packet bodies) and `WorldSocket.cpp` (`HandleConnectToFailed`).
//! The packet bodies live here, one private module per family, while
//! `crate::character_handlers` keeps the facades, the host trait, the thunks
//! and the registrar. The owner module therefore declares no descendant module
//! and the handler-registration ownership policy (`crate::character_handlers`,
//! `allow_descendants: false`) stays satisfied (#1263 F5).

mod barber;
mod char_delete;
mod cinematic;
mod connect_to_failed;
mod creation;
mod customize;
mod declined_names;
mod enumeration;
mod login;
mod rename;
mod undelete;
