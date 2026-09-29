//! Behaviour tests for [`super`].
//!
//! Extracted from `map_manager.rs`, which was 12,935 lines of which
//! 6,328 — 49% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use crate::map_manager::terrain::terrain_grid_bitset_index_like_cpp;

#[path = "map_manager_tests/fixtures.rs"]
mod fixtures;

pub use self::fixtures::*;

#[path = "map_manager_tests/combat.rs"]
mod combat;
#[path = "map_manager_tests/creature_1.rs"]
mod creature_1;
#[path = "map_manager_tests/creature_2.rs"]
mod creature_2;
#[path = "map_manager_tests/creature_3.rs"]
mod creature_3;
#[path = "map_manager_tests/creature_4.rs"]
mod creature_4;
#[path = "map_manager_tests/creature_5.rs"]
mod creature_5;
#[path = "map_manager_tests/gameobject.rs"]
mod gameobject;
#[path = "map_manager_tests/instance.rs"]
mod instance;
#[path = "map_manager_tests/misc.rs"]
mod misc;
#[path = "map_manager_tests/movement.rs"]
mod movement;
#[path = "map_manager_tests/persistence.rs"]
mod persistence;
#[path = "map_manager_tests/spawn.rs"]
mod spawn;
#[path = "map_manager_tests/spell.rs"]
mod spell;
#[path = "map_manager_tests/visibility.rs"]
mod visibility;
