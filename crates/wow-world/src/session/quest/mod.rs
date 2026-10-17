//! Represented quest responsibility, separated from the
//! Session root under #605. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;

mod host;
mod objective_drain;
pub(crate) use objective_drain::QuestObjectiveOperationLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use objective_drain::quest_reward_fixture_refs_like_cpp;
mod objectives;
mod persistence;
mod reset;
mod rewards;
pub(in crate::session) mod state;
