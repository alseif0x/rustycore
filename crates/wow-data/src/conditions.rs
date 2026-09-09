// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ConditionMgr` data rows.

use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use anyhow::Result;
use num_traits::FromPrimitive;
use wow_constants::{
    ComparisonType, ConditionInstanceInfo, ConditionSourceType, ConditionType, Gender,
    RelationType, Team, TypeId, TypeMask, UnitStandStateType,
};

mod state_1;
mod state_2;
mod state_3;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;

#[cfg(test)]
#[path = "conditions/tests/mod.rs"]
mod tests;
