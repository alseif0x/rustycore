// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature and player runtime phases, separated from the Session root.
//!
//! Separated from the Session root under #619. Each submodule owns one
//! complete phase responsibility; since #1263 F6-8E the admitted canonical
//! executor (`admitted_creature_execution`) is their only runner. The module
//! keeps its historical name until the follow-up rename.
//!
//! #1363 slice 3: moved out of `wow-world` (`session/creature_runtime`) into this
//! crate. The bodies are unchanged; `wow-world` re-exports the entries its
//! Session code and `world-server` call.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use wow_ai::{
    CreatureAiCanAttackInputLikeCpp, CreatureAiKindLikeCpp, CreatureAiSelectionInputLikeCpp,
    CreatureAttackDistanceInputLikeCpp, creature_ai_can_attack_like_cpp,
    creature_ai_uses_base_move_in_line_of_sight_like_cpp, creature_attack_distance_like_cpp,
    max_level_for_expansion_like_cpp, select_creature_ai_like_cpp,
};
use wow_constants::UnitState;
use wow_constants::creature::{CreatureFlagsExtra, CreatureType};
use wow_constants::movement::MovementFlag;
use wow_constants::unit::{NPCFlags1, UnitFlags, UnitFlags2, WeaponAttackType};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_data::CreatureSpellDisableDecisionLikeCpp;
use wow_entities::Unit;
use wow_map::coords::SIZE_OF_GRID_CELL;
use wow_world_core::entity_update_bridge::unit_values_update_to_update_object;
use wow_world_core::runtime_plan::{RecipientRule, RuntimeEvent, RuntimePlan};
use wow_world_core::session::connection_identity::unix_now;
use wow_world_core::session::{
    LIQUID_MAP_IN_WATER_LIKE_CPP, LIQUID_MAP_UNDER_WATER_LIKE_CPP, MMapRuntimeConfigLikeCpp,
    SharedCanonicalMapManager,
};
use wow_world_entities::creature_aggro_contracts::{
    CreatureAggroCandidateLikeCpp, CreatureAggroConfigLikeCpp, CreatureAggroOwnerSnapshotLikeCpp,
    CreatureAggroTickOutcomeLikeCpp, CreatureAggroVisibilityDecisionLikeCpp,
    CreatureAiCanAttackDecisionLikeCpp, CreatureAiSelectionDecisionLikeCpp,
    CreatureCanAttackLeashDecisionLikeCpp, CreatureLifecycleTickOutcomeLikeCpp,
    CreatureMeleeTickOutcomeLikeCpp, CreatureMovementTickOutcomeLikeCpp,
    CreatureSpellTargetHitResultLikeCpp, CreatureSpellTickOutcomeLikeCpp,
    CreatureThreatUpdateLikeCpp, check_no_gray_aggro_config_like_cpp,
    creature_ai_spell_disable_decision_like_cpp,
};
use wow_world_entities::creature_movement_adapter::{
    creature_path_request_like_cpp, resolve_creature_detour_path_like_cpp,
    trace_monster_move_packet_like_cpp,
};
use wow_world_entities::creature_spell_admission::{
    CreatureSpellCastValidationResultLikeCpp, TurretRejectedCastAttemptLikeCpp,
    creature_ai_effective_spell_range_like_cpp,
    creature_spell_target_is_valid_attack_target_like_cpp,
};
use wow_world_entities::creature_spell_metadata::{
    CreatureAiSpellConditionLikeCpp, creature_ai_effective_spell_info_like_cpp,
    creature_ai_has_temporally_unrepresented_noninstant_spell_like_cpp,
    creature_ai_spell_condition_like_cpp, creature_ai_spell_cooldown_profile_like_cpp,
    creature_ai_spell_has_represented_cooldown_semantics_like_cpp,
    creature_ai_spell_has_unrepresented_target_restrictions_like_cpp,
    creature_ai_spell_initial_cooldown_like_cpp, creature_ai_spell_is_combat_forbidden_like_cpp,
    creature_ai_spell_repeat_cooldown_like_cpp, creature_ai_spell_target_like_cpp,
};
use wow_world_entities::creature_spell_planning::{
    CreatureAiSpellRepresentationRejectionLikeCpp, CreatureSpellCastPlanLikeCpp,
    creature_ai_spell_plan_like_cpp, creature_ai_spell_requires_projectile_payload_like_cpp,
    creature_ai_spell_single_unit_topology_like_cpp,
    creature_ai_successful_untriggered_spell_resets_combat_timers_like_cpp,
    creature_ai_zero_power_rows_have_unrepresented_implicit_cost_like_cpp,
};
use wow_world_entities::creature_spell_publication::{
    append_committed_creature_spell_packets_like_cpp, creature_spell_cast_log_data_like_cpp,
    represented_creature_spell_hit_profile_like_cpp, resolve_creature_spell_hit_profile_like_cpp,
};
use wow_world_spell::spell_cast_adapter::represented_spell_cast_guid_for_map_like_cpp;

const ATTACK_DISPLAY_DELAY_LIKE_CPP_MS: u32 = 200;
const MIN_MELEE_REACH_LIKE_CPP: f32 = 2.0;
const NOMINAL_MELEE_RANGE_LIKE_CPP: f32 = 5.0;

mod player_melee_application;
use player_melee_application::PLAYER_MELEE_COMBAT_REF_REVALIDATE_INTERVAL_MS;
use player_melee_application::PendingPlayerSwingLikeCpp;
pub use player_melee_application::PlayerMeleeAttackerSnapshotLikeCpp;
use player_melee_application::PlayerMeleeCreatureHitLikeCpp;
pub use player_melee_application::PlayerMeleePhaseStateLikeCpp;
pub use player_melee_application::PlayerMeleeTickOutcomeLikeCpp;
use player_melee_application::apply_player_melee_to_canonical_player_like_cpp;
pub use player_melee_application::is_unit_facing_target_for_melee_like_cpp;
use player_melee_application::is_within_melee_range_like_cpp;
use player_melee_application::is_within_target_boundary_radius_like_cpp;
pub use player_melee_application::take_canonical_player_attack_swings_like_cpp;
pub use player_melee_application::{
    RepresentedArmorMitigationLikeCpp, RepresentedMeleeDamageBonusLikeCpp,
    RepresentedMeleeSwingLikeCpp, legacy_attack_power_multiplier_like_cpp,
};
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
mod native_player_death;
mod player_melee_victims;
mod player_tick;
pub use native_player_death::{
    CommittedMapCreatureMelee, MapPlayerCombatCall, MapPlayerCombatCallback,
    NativePlayerDeathFailure, PendingMapPlayerDeath,
};

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
    SharedCreatureExecutionLeaseLikeCpp, capture_admitted_creature_execution_like_cpp,
    capture_admitted_creature_execution_on_manager_like_cpp,
    run_admitted_creature_combat_phases_isolated_like_cpp,
    run_admitted_creature_combat_phases_on_locked_manager_like_cpp,
    run_admitted_creature_combat_phases_with_native_death,
    run_admitted_creature_execution_isolated_like_cpp,
};
// The single-phase entries the executor composes; the unit tests drive one
// phase at a time through them.
#[allow(unused_imports)]
pub(crate) use admitted_creature_execution::run_creature_aggro_phase_on_manager_like_cpp;
#[allow(unused_imports)]
pub(crate) use creature_aggro_tick::*;
pub use creature_combat_commit::*;
#[allow(unused_imports)]
pub(crate) use creature_lifecycle_tick::*;
#[allow(unused_imports)]
pub(crate) use creature_melee_tick::*;
#[allow(unused_imports)]
pub(crate) use creature_movement_tick::*;
#[allow(unused_imports)]
pub(crate) use creature_phase_store::*;
use creature_spell_actions::*;
#[allow(unused_imports)]
pub(crate) use creature_spell_tick::*;
#[allow(unused_imports)]
pub(crate) use creature_spell_validation::*;
#[allow(unused_imports)]
pub(crate) use creature_threat::*;
#[allow(unused_imports)]
pub(crate) use creature_tick::*;
#[allow(unused_imports)]
pub(crate) use player_melee_victims::*;
#[allow(unused_imports)]
pub(crate) use player_tick::*;
// #1363 slice 3: entries only the `wow-world` scenario tests drive directly.
#[cfg(any(test, feature = "test-fixtures"))]
pub use admitted_creature_execution::{
    run_creature_lifecycle_phase_on_manager_like_cpp, run_creature_melee_phase_on_manager_like_cpp,
    run_creature_movement_phase_on_manager_like_cpp, run_creature_spell_phase_on_manager_like_cpp,
    run_player_melee_phase_on_manager_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use creature_aggro_tick::run_creature_aggro_phase_on_store_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use creature_movement_tick::{
    creature_melee_spell_miss_threshold_3_3_5_like_cpp, is_creature_melee_los_clear_like_cpp,
    step_creature_movement_on_view_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use creature_phase_store::AdmittedCanonicalCreatureStoreLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use creature_tick::creature_ai_can_attack_decision_like_cpp;
// #1363 slice 3b: the `wow-world-core` modules the moved drivers and scenarios
// name at the crate root, as they did in `wow-world`.
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::{creature_movement, pathfinder, terrain};
#[cfg(test)]
use wow_world_core::{creature_projection, creature_respawn};
// #1363 slice 3b: the canonical single-phase drivers that the runtime's own
// scenario tests and the `wow-world` session tests share.
#[cfg(any(test, feature = "test-fixtures"))]
#[path = "../unit_tests/canonical_creatures.rs"]
pub mod canonical_creatures;
#[cfg(test)]
#[path = "../unit_tests/scenarios.rs"]
mod scenarios;
