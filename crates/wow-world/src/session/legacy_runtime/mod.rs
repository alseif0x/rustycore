//! Creature and player runtime phases, separated from the Session root.
//!
//! Separated from the Session root under #619. Each submodule owns one
//! complete phase responsibility; since #1263 F6-8E the admitted canonical
//! executor (`admitted_creature_execution`) is their only runner. The module
//! keeps its historical name until the follow-up rename.

use super::*;
mod admitted_creature_execution;
mod creature_aggro_tick;
mod creature_combat_commit;
mod creature_lifecycle_tick;
mod creature_melee_share;
mod creature_melee_split;
mod creature_melee_threat;
mod creature_melee_tick;
mod creature_movement_tick;
mod creature_phase_store;
mod creature_spell_actions;
mod creature_spell_tick;
mod creature_spell_validation;
mod creature_threat;
mod creature_tick;
mod player_melee_victims;
mod player_tick;

// #1263 F6-8D2: the admitted creature execution seam. #1263 F6-8D3b-1: the
// world-server canonical map loop calls its locked-manager entries. The other
// re-exports look unused inside this module: they are consumed through the
// Session root glob, so removing them breaks the callers.
pub use admitted_creature_execution::{
    AdmittedCreatureCombatPhaseInputsLikeCpp, AdmittedCreatureCombatPhasesOutcomeLikeCpp,
    AdmittedCreatureExecutionLikeCpp, AdmittedCreatureExecutionMapLikeCpp,
    AdmittedCreatureExecutionObjectLikeCpp, CreatureExecutionAdmissionLikeCpp,
    CreatureExecutionLeaseLikeCpp, CreatureExecutionOwnerLikeCpp,
    CreatureExecutionTransitionLikeCpp, IsolatedCreatureExecutionOutcomeLikeCpp,
    SharedCreatureExecutionLeaseLikeCpp,
    capture_admitted_creature_execution_like_cpp,
    capture_admitted_creature_execution_on_manager_like_cpp,
    run_admitted_creature_combat_phases_isolated_like_cpp,
    run_admitted_creature_combat_phases_on_locked_manager_like_cpp,
    run_admitted_creature_execution_isolated_like_cpp,
};
// The single-phase entries the executor composes; the unit tests drive one
// phase at a time through them.
#[allow(unused_imports)]
pub(in crate::session) use admitted_creature_execution::{
    run_creature_aggro_phase_on_manager_like_cpp, run_creature_lifecycle_phase_on_manager_like_cpp,
    run_creature_melee_phase_on_manager_like_cpp, run_creature_movement_phase_on_manager_like_cpp,
    run_creature_spell_phase_on_manager_like_cpp, run_player_melee_phase_on_manager_like_cpp,
};
#[allow(unused_imports)]
pub(in crate::session) use creature_aggro_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_lifecycle_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_melee_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_movement_tick::*;
pub(in crate::session) use creature_phase_store::*;
use creature_spell_actions::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_spell_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_spell_validation::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_threat::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use player_melee_victims::*;
#[allow(unused_imports)]
pub(in crate::session) use player_tick::*;
pub use creature_combat_commit::*;
