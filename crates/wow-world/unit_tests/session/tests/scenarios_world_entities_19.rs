//! Session scenarios for represented world-entity and runtime responsibilities.
//!
//! Focused child modules retain the parent session-test fixtures while keeping
//! world-entity, tick-owner, and melee cases cohesive.

use super::*;

// #1263 C1: the injected stale/ABA replay capture harness.
#[path = "scenarios_world_entities_19/c1_runtime_capture_replay.rs"]
mod c1_runtime_capture_replay;
#[path = "scenarios_world_entities_19/creature_movement_tick.rs"]
mod creature_movement_tick;
#[path = "scenarios_world_entities_19/creature_tick_owner.rs"]
mod creature_tick_owner;
// #1263 F6-8C: the canonical designated owner decides the combat selections.
#[path = "scenarios_world_entities_19/f6_8c_canonical_ownership.rs"]
mod f6_8c_canonical_ownership;
// #1263 F6-8D1: the ported phase operations run on canonical ownership.
#[path = "scenarios_world_entities_19/f6_8d1_canonical_execution.rs"]
mod f6_8d1_canonical_execution;
// #1263 F6-8D2: the admitted tick drives the canonical engine exactly once.
#[path = "scenarios_world_entities_19/f6_8d2_admitted_execution.rs"]
mod f6_8d2_admitted_execution;
// #1263 F6-8D3a-1: the admitted canonical executor reaches combat-phase parity.
#[path = "scenarios_world_entities_19/f6_8d3a1_combat_parity.rs"]
mod f6_8d3a1_combat_parity;
// #1263 F6-8D3a-1b: the executor runs the creature spell phase with parity.
#[path = "scenarios_world_entities_19/f6_8d3a1b_spell_parity.rs"]
mod f6_8d3a1b_spell_parity;
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
#[path = "scenarios_world_entities_19/r7b2a_canonical_mutation.rs"]
mod r7b2a_canonical_mutation;
#[path = "scenarios_world_entities_19/r7b2a_guarded_loot_release.rs"]
mod r7b2a_guarded_loot_release;
#[path = "scenarios_world_entities_19/r7b2b_mirror_admission.rs"]
mod r7b2b_mirror_admission;
#[path = "scenarios_world_entities_19/shared_creature_authority.rs"]
mod shared_creature_authority;
