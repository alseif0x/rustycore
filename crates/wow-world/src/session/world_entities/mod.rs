//! Represented world-entity (creature and gameobject) responsibility, separated from the
//! Session root under #599. Each submodule owns one complete operation
//! group; the canonical owners keep authority over the state they touch.

use super::*;
mod aggro;
mod creature;
mod creature_interaction;
mod creature_kill;
mod creature_publication;
mod creature_query;
mod creature_registry;
pub(crate) use wow_world_entities::{
    CanonicalCreatureInsertOutcomeLikeCpp, insert_canonical_creature_map_object_on_map_like_cpp,
};
mod gameobject;
mod gameobject_overrides;
mod gameobject_query;
mod gameobject_state;
#[cfg(test)]
mod gameobject_use_scripted;
#[cfg(test)]
mod gameobject_use_types;
mod spawn;
mod spell_click;
