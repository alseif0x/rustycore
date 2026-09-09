//! Narrow semantic packet comparators for fields that are intentionally
//! runtime-allocated, intrinsically unordered in C++, or for a specifically
//! proven accumulated update-mask artifact, and therefore cannot be compared
//! byte-for-byte.
//!
//! Keep this module deliberately small. A semantic comparator is allowed to
//! omit only a field whose value cannot be made stable across equivalent C++
//! and Rust runs, canonicalize only proven unordered collection order, or omit
//! one exact empty mask fragment whose cadence was reproduced independently;
//! every other decoded bit remains part of the comparison.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{Capture, Direction};

mod state_1;
mod state_2;
mod state_3;
mod state_4;
mod state_5;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
#[allow(unused_imports)]
pub use state_3::*;
#[allow(unused_imports)]
pub use state_4::*;
#[allow(unused_imports)]
pub use state_5::*;
