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
mod gameobject;
mod gameobject_overrides;
mod gameobject_query;
mod gameobject_state;
mod gameobject_use;
mod gameobject_use_pvp;
mod gameobject_use_scripted;
mod gameobject_use_types;
mod spawn;
mod spell_click;
