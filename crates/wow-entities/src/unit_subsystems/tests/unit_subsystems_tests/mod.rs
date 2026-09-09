//! Unit-subsystem regressions.
//!
//! Separated from the tests.rs root under #648.

use super::*;

use super::*;

fn guid(low: i64) -> ObjectGuid {
    ObjectGuid::new(0, low)
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
