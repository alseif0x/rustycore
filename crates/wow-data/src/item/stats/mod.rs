// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item stat modifiers loaded from ItemSparse.db2.
//!
//! ItemSparse.db2 uses the WDC4 offset-map format where string fields
//! (Description, Display3/2/1/Display) are stored inline. This means
//! the field_storage_info offsets can't be used directly — we must read
//! each record sequentially, scanning past inline null-terminated strings.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_constants::ItemFlags;

use crate::wdc4::Wdc4Reader;

mod entries;
mod store;

pub use entries::*;
pub use store::*;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
