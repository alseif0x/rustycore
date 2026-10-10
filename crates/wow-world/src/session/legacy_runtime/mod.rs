//! Legacy creature and player runtime tick, separated from the Session root.
//!
//! Separated from the Session root under #619. Each submodule owns one
//! complete tick responsibility.

use super::*;
mod admitted_creature_execution;
mod canonical_incarnation_ownership;
mod creature_aggro_tick;
mod creature_combat_commit;
mod creature_lifecycle_tick;
mod creature_melee_share;
mod creature_melee_split;
mod creature_melee_sync;
mod creature_melee_threat;
mod creature_melee_tick;
mod creature_movement_publication;
mod creature_movement_tick;
mod creature_phase_store;
mod creature_spell_actions;
mod creature_spell_tick;
mod creature_spell_validation;
mod creature_threat;
mod creature_tick;
mod player_melee_victims;
mod player_tick;

// These re-exports look unused inside this module: they are consumed through
// the Session root glob, so removing them breaks the callers.
#[allow(unused_imports)]
pub(in crate::session) use canonical_incarnation_ownership::*;
// #1263 F6-8D2: the admitted creature execution seam, exported the same way as
// the seven legacy tick entries. #1263 F6-8D3b-1: the world-server
// `RuntimeTickOwner::CanonicalMap` owner calls its locked-manager entries.
pub use admitted_creature_execution::{
    AdmittedCreatureCombatPhaseInputsLikeCpp, AdmittedCreatureCombatPhasesOutcomeLikeCpp,
    AdmittedCreatureExecutionLikeCpp, AdmittedCreatureExecutionMapLikeCpp,
    AdmittedCreatureExecutionObjectLikeCpp, CreatureExecutionAdmissionLikeCpp,
    CreatureExecutionLeaseLikeCpp, CreatureExecutionOwnerLikeCpp,
    CreatureExecutionTransitionLikeCpp, IsolatedCreatureExecutionCompositionOutcomeLikeCpp,
    IsolatedCreatureExecutionOutcomeLikeCpp, IsolatedLegacyArmOutcomeLikeCpp,
    IsolatedSessionArmOutcomeLikeCpp, SharedCreatureExecutionLeaseLikeCpp,
    capture_admitted_creature_execution_like_cpp,
    capture_admitted_creature_execution_on_manager_like_cpp,
    run_admitted_creature_combat_phases_isolated_like_cpp,
    run_admitted_creature_combat_phases_on_locked_manager_like_cpp,
    run_admitted_creature_execution_isolated_like_cpp,
    run_isolated_admitted_creature_execution_composition_like_cpp,
};
#[allow(unused_imports)]
pub(in crate::session) use creature_aggro_tick::*;
#[allow(unused_imports)]
pub(in crate::session) use creature_lifecycle_tick::*;
pub(in crate::session) use creature_melee_sync::{
    CreatureMeleeApplyResultLikeCpp, CreatureMeleeVictimSyncIdentityLikeCpp,
    CreatureMeleeVictimSyncStateLikeCpp,
};
pub(in crate::session) use creature_melee_threat::CreatureDamageThreatOutcomeLikeCpp;
#[allow(unused_imports)]
pub(in crate::session) use creature_melee_tick::*;
pub(in crate::session) use creature_movement_publication::*;
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

// The tick entry points keep their original external path,
// `wow_world::session::run_legacy_*`, so world-server keeps calling them
// unchanged. Only these seven are public; everything else stays crate-internal.
pub use creature_aggro_tick::run_legacy_creature_aggro_tick_once_like_cpp;
pub use creature_aggro_tick::run_legacy_creature_aggro_tick_once_with_config_and_canonical_like_cpp;
pub use creature_aggro_tick::run_legacy_creature_aggro_tick_once_with_config_like_cpp;
pub use creature_combat_commit::*;
pub use creature_lifecycle_tick::run_legacy_creature_lifecycle_tick_once_like_cpp;
pub use creature_melee_tick::run_legacy_creature_melee_tick_once_like_cpp;
pub use creature_movement_tick::run_legacy_creature_movement_tick_once_like_cpp;
pub use creature_spell_tick::run_legacy_creature_spell_tick_once_like_cpp;
pub use player_tick::run_legacy_player_melee_tick_once_like_cpp;
