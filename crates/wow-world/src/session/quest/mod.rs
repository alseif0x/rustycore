//! Represented quest responsibility, separated from the
//! Session root under #605. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

mod giver;
mod objectives;
mod persistence;
mod publication;
mod reset;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixture_observations;
mod rewards;
pub(in crate::session) mod state;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) mod test_fixtures;
