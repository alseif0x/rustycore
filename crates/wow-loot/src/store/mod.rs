//! Loot store definitions and templates.
//!
//! Separated from the crate root under #642.

use std::collections::{HashMap, HashSet};

use rand::Rng;
use wow_core::ObjectGuid;

mod state_1;
mod state_2;
mod state_3;
mod state_4;

pub use state_1::*;
pub use state_2::*;
pub use state_3::*;
#[allow(unused_imports)]
use state_4::*;
