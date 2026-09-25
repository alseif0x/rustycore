//! Represented social responsibility, separated from the
//! Session root under #615. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

mod contacts;
mod duel;
mod group;
mod guild;
#[cfg(test)]
pub(crate) mod test_fixtures;
mod trade;

#[cfg(test)]
#[path = "group_tests.rs"]
mod group_tests;
