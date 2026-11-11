//! Pure domain cases with recording effects, not a real Player executor.
mod activation;
mod admission;
mod deactivation;
mod defaults;
mod effects;
mod fixtures;
#[path = "../order.rs"]
mod order;
mod rewards;
use super::*;
use crate::SkillUpdateState;
use effects::*;
use fixtures::*;
