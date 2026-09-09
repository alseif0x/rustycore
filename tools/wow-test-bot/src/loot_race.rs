//! Multi-client live smokes for atomic shared world-loot and group claims.
//!
//! These are deliberately mutating QA-only workflows. The two-session loot
//! race uses a wrapper-installed, shared Tattered Chest GameObject, while the
//! strict one-session capture continues to use stationary Doctor Maleficus.
//! The group-capacity race consumes the fifth slot of a preloaded party. Each
//! fixture requires explicit restoration and a world restart before reuse.

use super::*;
use mysql::params;
use mysql::prelude::Queryable;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Barrier, Mutex};
use tokio_util::sync::CancellationToken;

mod fixtures_1;
mod fixtures_2;
mod gameobject;
mod misc_1;
mod misc_2;
mod misc_3;
mod misc_4;
mod misc_5;
mod sql;
mod wire;
mod workflow;
#[allow(unused_imports)]
pub(crate) use fixtures_1::*;
pub(crate) use fixtures_2::*;
pub(crate) use gameobject::*;
pub(crate) use misc_1::*;
pub(crate) use misc_2::*;
pub(crate) use misc_3::*;
pub(crate) use misc_4::*;
pub(crate) use misc_5::*;
pub(crate) use sql::*;
pub(crate) use wire::*;
pub(crate) use workflow::*;

#[cfg(test)]
#[path = "loot_race/tests/mod.rs"]
mod tests;
