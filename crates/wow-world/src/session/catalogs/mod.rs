//! Catalog, config and policy accessors used across the Session.
//!
//! Separated from the Session root under #632.

use super::*;

mod operations;
pub use operations::{install_condition_presence_catalog, install_serverside_spell_catalog};
