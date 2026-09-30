//! Legacy entry points delegate the complete Aggro family to the shared Map motor.
use super::*;
use super::super::canonical_runtime::aggro::{conversion, catalogs, packets};
use wow_map::map_manager as core;

pub fn run_legacy_creature_aggro_tick_once_like_cpp(manager: &SharedMapManager,
    candidates: &[LegacyCreatureAggroCandidateLikeCpp]) -> LegacyCreatureAggroTickOutcomeLikeCpp {
    run_legacy_creature_aggro_tick_once_with_config_like_cpp(manager, candidates, LegacyCreatureAggroConfigLikeCpp::default())
}

pub fn run_legacy_creature_aggro_tick_once_with_config_like_cpp(manager: &SharedMapManager,
    candidates: &[LegacyCreatureAggroCandidateLikeCpp], config: LegacyCreatureAggroConfigLikeCpp)
    -> LegacyCreatureAggroTickOutcomeLikeCpp {
    let mut outcome = LegacyCreatureAggroTickOutcomeLikeCpp::default();
    let mut manager = manager.write().unwrap_or_else(|poisoned| poisoned.into_inner());
    let terrain = manager.terrain();
    if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
        outcome.skipped_owner_not_global = true;
        return outcome;
    }
    let keys = manager.active_map_keys();
    let mut results = Vec::new();
    for (map_id, instance_id) in keys {
        let candidates = candidates.iter().cloned().map(conversion::owned_candidate).collect();
        let result = catalogs::with_policies(&config, |policies| manager.run_aggro_map(
            map_id, instance_id, candidates, conversion::settings(&config, map_id), terrain.as_deref(), policies));
        results.push(result);
    }
    drop(manager);
    for result in results { packets::append_outcome(&mut outcome, result); }
    outcome
}

pub(in crate::session) fn legacy_creature_aggro_candidate_is_targetable_for_attack_like_cpp(candidate: &LegacyCreatureAggroCandidateLikeCpp) -> bool {
    core::candidate_targetable(&conversion::owned_candidate(candidate.clone()))
}
pub(in crate::session) fn legacy_creature_aggro_candidate_visibility_decision_like_cpp(actor: &crate::map_manager::WorldCreature,
    map_id: u16, instance_id: u32, candidate: &LegacyCreatureAggroCandidateLikeCpp, check_alert: bool) -> LegacyCreatureAggroVisibilityDecisionLikeCpp {
    match core::candidate_visibility(actor, map_id, instance_id, &conversion::owned_candidate(candidate.clone()), check_alert) {
        core::AggroVisibility::Allowed => LegacyCreatureAggroVisibilityDecisionLikeCpp::Allowed,
        core::AggroVisibility::Rejected => LegacyCreatureAggroVisibilityDecisionLikeCpp::Rejected,
        core::AggroVisibility::Unrepresented => LegacyCreatureAggroVisibilityDecisionLikeCpp::Unrepresented,
    }
}
pub(in crate::session) fn legacy_creature_aggro_candidate_has_stealth_aura_like_cpp(candidate: &LegacyCreatureAggroCandidateLikeCpp) -> bool {
    core::candidate_has_stealth(&conversion::owned_candidate(candidate.clone()))
}
pub(in crate::session) fn legacy_creature_aggro_candidate_is_hostile_to_creature_like_cpp(actor: &crate::map_manager::WorldCreature,
    candidate: &LegacyCreatureAggroCandidateLikeCpp, config: &LegacyCreatureAggroConfigLikeCpp) -> Option<bool> {
    catalogs::with_policies(config, |policies| core::candidate_hostile(actor, &conversion::owned_candidate(candidate.clone()), policies))
}
pub(in crate::session) fn legacy_creature_aggro_candidate_is_accessible_for_creature_like_cpp(actor: &crate::map_manager::WorldCreature,
    candidate: &LegacyCreatureAggroCandidateLikeCpp) -> bool {
    core::candidate_accessible(actor, &conversion::owned_candidate(candidate.clone()))
}
