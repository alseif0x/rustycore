// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item-related enums and flags: quality, classes, subclasses, inventory types, etc.

use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};

mod classification;
mod flags;
mod results;
mod stats;

pub use classification::*;
pub use flags::*;
pub use results::*;
pub use stats::*;
