// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell DB2 stores and their loaders.

use super::catalog::{SpellHitEffectMechanicRowLikeCpp, SpellInterruptRowLikeCpp};

use super::*;

mod state_1;
mod state_2;
mod state_3;
mod state_4_ops_1;
mod state_4_ops_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;
