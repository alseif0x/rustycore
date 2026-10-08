//! Session scenarios for represented world-entity and runtime responsibilities.
//!
//! Focused child modules retain the parent session-test fixtures while keeping
//! world-entity, tick-owner, and melee cases cohesive.

use super::*;

#[path = "scenarios_world_entities_19/creature_movement_tick.rs"]
mod creature_movement_tick;
#[path = "scenarios_world_entities_19/creature_tick_owner.rs"]
mod creature_tick_owner;
#[path = "scenarios_world_entities_19/gameobject_use.rs"]
mod gameobject_use;
#[path = "scenarios_world_entities_19/legacy_creature_tick_noop.rs"]
mod legacy_creature_tick_noop;
#[path = "scenarios_world_entities_19/player_melee_modifiers.rs"]
mod player_melee_modifiers;
#[path = "scenarios_world_entities_19/player_melee_outcomes.rs"]
mod player_melee_outcomes;
#[path = "scenarios_world_entities_19/r1b_incarnation_boundaries.rs"]
mod r1b_incarnation_boundaries;
#[path = "scenarios_world_entities_19/r1b_incarnation_lifecycle.rs"]
mod r1b_incarnation_lifecycle;
#[path = "scenarios_world_entities_19/r7a_canonical_mutation.rs"]
mod r7a_canonical_mutation;
#[path = "scenarios_world_entities_19/r7a_lethal_lifecycle.rs"]
mod r7a_lethal_lifecycle;
#[path = "scenarios_world_entities_19/shared_creature_authority.rs"]
mod shared_creature_authority;
