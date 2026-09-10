//! WoW server `.conf` file parser.
//!
//! Parses configuration files that use the `Key = Value` format found in
//! TrinityCore/RustyCore `.conf.dist` files. Provides a global singleton
//! [`ConfigMgr`] for application-wide configuration access.
//!
//! # Format
//!
//! ```text
//! # This is a comment
//! DataDir = "/home/server/data"
//! WorldServerPort = 8085
//! Rate.XP.Kill = 1.5
//! ```

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

mod store;
mod world_registry;
mod world_validation;

pub use store::*;
pub use world_registry::*;
pub use world_validation::*;

use store::{CONFIG, ConfigStore, parse_config_bool};

#[cfg(test)]
use store::env_key_for_ini_key;

use world_validation::{apply_world_config_validations, non_empty, signed_i32};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
