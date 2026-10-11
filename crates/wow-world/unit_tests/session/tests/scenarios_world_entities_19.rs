//! Session scenarios for represented world-entity and runtime responsibilities.
//!
//! Focused child modules retain the parent session-test fixtures while keeping
//! world-entity, creature runtime and melee cases cohesive.

use super::*;

// #1263 C1: the creature mutation root capture.
#[path = "scenarios_world_entities_19/c1_runtime_capture_replay.rs"]
mod c1_runtime_capture_replay;
#[path = "scenarios_world_entities_19/creature_movement_tick.rs"]
mod creature_movement_tick;
// #1263 F6-8D1: the ported phase operations run on canonical ownership.
#[path = "scenarios_world_entities_19/f6_8d1_canonical_execution.rs"]
mod f6_8d1_canonical_execution;
// #1263 F6-8D2: the admitted tick drives the canonical engine exactly once.
#[path = "scenarios_world_entities_19/f6_8d2_admitted_execution.rs"]
mod f6_8d2_admitted_execution;
// #1263 F6-8E: the recorded legacy-bridge expectations of the F6-8D3a tests.
#[path = "scenarios_world_entities_19/f6_8d3a_recorded_expectations.rs"]
mod f6_8d3a_recorded_expectations;
// #1263 F6-8D3a-1: the executor's combat phases against recorded expectations.
#[path = "scenarios_world_entities_19/f6_8d3a1_combat_parity.rs"]
mod f6_8d3a1_combat_parity;
// #1263 F6-8D3a-1b: the executor's spell phase against recorded expectations.
#[path = "scenarios_world_entities_19/f6_8d3a1b_spell_parity.rs"]
mod f6_8d3a1b_spell_parity;
// #1263 F6-8D3a-2: the executor's movement phase against recorded expectations.
#[path = "scenarios_world_entities_19/f6_8d3a2_movement_lifecycle_parity.rs"]
mod f6_8d3a2_movement_lifecycle_parity;
// #1263 F6-8D3a-3: the executor's lifecycle phase against recorded expectations.
#[path = "scenarios_world_entities_19/f6_8d3a3_lifecycle_parity.rs"]
mod f6_8d3a3_lifecycle_parity;
// #1263 F6-8D3b-2: the canonical-first root and the canonical R7b-1 deferral.
#[path = "scenarios_world_entities_19/f6_8d3b2_canonical_first.rs"]
mod f6_8d3b2_canonical_first;
// #1263 F6-8D3b-3: the root always returns the lent incarnation and relocates it.
#[path = "scenarios_world_entities_19/f6_8d3b3_root_hardening.rs"]
mod f6_8d3b3_root_hardening;
#[path = "scenarios_world_entities_19/gameobject_use.rs"]
mod gameobject_use;
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
#[path = "scenarios_world_entities_19/shared_creature_authority.rs"]
mod shared_creature_authority;
