// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! PlayerCondition.db2 store and C++-like evaluator.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;
use crate::{
    ChrSpecializationStore, WorldStateExpressionContextLikeCpp, WorldStateExpressionStore,
    is_meeting_world_state_expression_like_cpp,
};

mod entry;
mod evaluate;
mod store;

pub use entry::*;
pub use evaluate::*;
pub use store::*;

use entry::{
    read_i32_array4, read_u8_array3, read_u8_array4, read_u16_array, read_u32_array3,
    read_u32_array4, read_u32_array6,
};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
