//! Unit values, visibility and health-revision state operations, part 1 of 3.
//!
//! The inherent `Unit` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;
use super::visibility;

mod presence;
mod visibility_state;
mod health_revisions;
mod combat_control;
