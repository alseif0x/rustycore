//! Collections handler regression scenarios.
//!
//! Shared entries/packets/session fixtures live in
//! [`crate::handlers::test_support`]; the toy bodies the scenarios exercise
//! moved to `wow_world_inventory` and `wow_world_spell` (#1363 slice 4d) and
//! are reached through the session's test entry points.

use crate::handlers::test_support::*;

mod collections;
