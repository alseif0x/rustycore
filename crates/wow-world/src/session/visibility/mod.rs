//! Represented visibility and phase state at the Session boundary.
//!
//! Separated from the Session root under #632.

use super::*;

mod operations;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) mod test_fixtures;
