//! Compatibility entry points use the one shared Aggro rule/motor family.
use super::*;
use super::super::canonical_runtime::aggro::{conversion, catalogs};
use wow_map::map_manager as core;

pub(in crate::session) fn legacy_creature_snapshot_is_hostile_to_creature_like_cpp(actor: &crate::map_manager::WorldCreature,
    target: &LegacyCreatureAggroOwnerSnapshotLikeCpp, config: &LegacyCreatureAggroConfigLikeCpp) -> Option<bool> {
    catalogs::with_policies(config, |policies| core::snapshot_hostile(actor, &conversion::owner(target), policies))
}
pub(in crate::session) fn legacy_creature_ai_selection_decision_like_cpp(actor: &crate::map_manager::WorldCreature,
    config: &LegacyCreatureAggroConfigLikeCpp) -> LegacyCreatureAiSelectionDecisionLikeCpp {
    catalogs::select_kind(actor.aggro_ai_facts(), config)
}
pub(in crate::session) fn legacy_creature_ai_can_attack_decision_like_cpp(kind: &CreatureAiKindLikeCpp,
    actor: &crate::map_manager::WorldCreature, candidate: &LegacyCreatureAggroCandidateLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp) -> LegacyCreatureAiCanAttackDecisionLikeCpp {
    // All non-Turret stock kinds use the original default CanAIAttack input.
    if !matches!(kind, CreatureAiKindLikeCpp::TurretAI) {
        return if creature_ai_can_attack_like_cpp(kind, &CreatureAiCanAttackInputLikeCpp::default()) {
            LegacyCreatureAiCanAttackDecisionLikeCpp::Allowed
        } else { LegacyCreatureAiCanAttackDecisionLikeCpp::Rejected };
    }
    let result = catalogs::turret_decision(core::AggroTurretFacts {
        kind: core::AggroAiKind::Turret, first_spell_id: actor.creature.spells()[0],
        difficulty: candidate.map_difficulty_id, position: actor.position(),
        combat_reach: actor.creature.unit().world().combat_reach(), target_position: candidate.position,
        target_combat_reach: candidate.player_combat_reach,
    }, config);
    match result {
        core::AggroAttackDecision::Allowed => LegacyCreatureAiCanAttackDecisionLikeCpp::Allowed,
        core::AggroAttackDecision::Rejected => LegacyCreatureAiCanAttackDecisionLikeCpp::Rejected,
        core::AggroAttackDecision::Unrepresented => LegacyCreatureAiCanAttackDecisionLikeCpp::Unrepresented,
    }
}
fn leash(result: core::AggroLeash) -> LegacyCreatureCanAttackLeashDecisionLikeCpp {
    match result {
        core::AggroLeash::Allowed => LegacyCreatureCanAttackLeashDecisionLikeCpp::Allowed,
        core::AggroLeash::HomeRangeRejected => LegacyCreatureCanAttackLeashDecisionLikeCpp::HomeRangeRejected,
        core::AggroLeash::OwnerPositionUnrepresented => LegacyCreatureCanAttackLeashDecisionLikeCpp::OwnerPositionUnrepresented,
    }
}
pub(in crate::session) fn legacy_creature_can_attack_leash_decision_like_cpp(actor: &crate::map_manager::WorldCreature,
    candidate: &LegacyCreatureAggroCandidateLikeCpp, config: &LegacyCreatureAggroConfigLikeCpp,
    owners: &HashMap<ObjectGuid, LegacyCreatureAggroOwnerSnapshotLikeCpp>) -> LegacyCreatureCanAttackLeashDecisionLikeCpp {
    leash(core::candidate_leash(actor, &conversion::owned_candidate(candidate.clone()),
        &conversion::settings(config, candidate.map_id), &conversion::owners(owners)))
}
pub(in crate::session) fn legacy_creature_can_attack_snapshot_leash_decision_like_cpp(actor: &crate::map_manager::WorldCreature,
    target: &LegacyCreatureAggroOwnerSnapshotLikeCpp, config: &LegacyCreatureAggroConfigLikeCpp,
    owners: &HashMap<ObjectGuid, LegacyCreatureAggroOwnerSnapshotLikeCpp>) -> LegacyCreatureCanAttackLeashDecisionLikeCpp {
    leash(core::snapshot_leash(actor, &conversion::owner(target),
        &conversion::settings(config, target.map_id), &conversion::owners(owners)))
}
pub(in crate::session) fn legacy_creature_try_trigger_alert_like_cpp(actor: &mut crate::map_manager::WorldCreature,
    candidate: &LegacyCreatureAggroCandidateLikeCpp, config: &LegacyCreatureAggroConfigLikeCpp) -> Option<Vec<u8>> {
    let triggered = catalogs::with_policies(config, |policies| core::trigger_alert(actor,
        &conversion::owned_candidate(candidate.clone()), policies));
    if !triggered { return None; }
    use wow_packet::ServerPacket;
    Some(wow_packet::packets::combat::AIReaction {
        unit_guid: actor.guid(), reaction: wow_constants::creature::AiReaction::Alert,
    }.to_bytes())
}
