//! Represented character-progression responsibility, separated from the
//! Session root under #611. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

#[cfg(any(test, feature = "test-fixtures"))]
pub(super) use wow_world_core::session::PlayerSkillTestFixtureLikeCpp;

mod reputation;
mod skills;
mod talents;
