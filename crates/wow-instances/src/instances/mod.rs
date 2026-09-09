//! Instance lifecycle state.
//!
//! Separated from the crate root under #658.

use super::*;

mod ops_1;
mod state_1;
mod state_2;

pub use ops_1::*;
pub use state_1::*;
pub use state_2::*;
