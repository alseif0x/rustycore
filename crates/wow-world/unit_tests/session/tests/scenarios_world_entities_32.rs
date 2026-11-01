//! Session scenarios covering creature attacks against player victims.
//!
//! Split from `scenarios_world_entities_28.rs` under #29 when that file
//! reached 1,977 of its 2,000-line test-file budget. Focused private children
//! retain access to the shared session-test fixtures.

use super::*;

#[path = "scenarios_world_entities_32/melee_absorption.rs"]
mod melee_absorption;
#[path = "scenarios_world_entities_32/melee_damage_modifiers.rs"]
mod melee_damage_modifiers;
#[path = "scenarios_world_entities_32/melee_hit_and_armor.rs"]
mod melee_hit_and_armor;
