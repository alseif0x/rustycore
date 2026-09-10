// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Client and Server opcodes for the WoW 3.4.3 protocol.

use num_derive::{FromPrimitive, ToPrimitive};

mod client;
mod server;

pub use client::*;
pub use server::*;
