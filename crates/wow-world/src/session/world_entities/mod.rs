//! Represented world-entity (creature and gameobject) responsibility, separated from the
//! Session root under #599. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod aggro;
mod creature_kill;
mod creature_publication;
mod creature_query;
pub(crate) use wow_world_entities::{
    CanonicalCreatureInsertOutcomeLikeCpp, insert_canonical_creature_map_object_on_map_like_cpp,
};
mod gameobject;
mod gameobject_query;
mod gameobject_state;
mod spell_click;

#[cfg(test)]
/// The #1241 F3 entry points of the creature state, the gameobject overrides
/// and the spawn helpers, mounted here instead of from the deleted
/// `creature.rs`, `gameobject_overrides.rs` and `spawn.rs` (#1363 slice 10b).
#[path = "../../../unit_tests/session/world_entities/creature/f3_shims.rs"]
mod creature_f3_shims;
#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/gameobject_overrides/f3_shims.rs"]
mod gameobject_overrides_f3_shims;
#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/spawn/f3_shims.rs"]
mod spawn_f3_shims;
