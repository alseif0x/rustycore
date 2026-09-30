//! Legacy validation signature delegates to the shared Map spell operation.
use super::*;
use super::super::canonical_runtime::spell::legacy;
use wow_map::map_manager::{SpellAction, SpellCast, SpellOutcome};

pub(in crate::session) fn validate_and_append_creature_spell_cast_like_cpp(
    canonical_map_manager: &SharedCanonicalMapManager,
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    command: &CreatureSpellCastPlanLikeCpp, difficulty_id: u8, turret_ai: bool,
    config: &LegacyCreatureAggroConfigLikeCpp, plan: &mut RuntimePlan,
) -> CreatureSpellCastValidationResultLikeCpp {
    // Clone only the existing immutable plan identity required by this borrowed
    // compatibility signature; never clone an Actor or its runtime.
    let Some(outcome) = legacy::consume_action(canonical_map_manager, legacy_map_manager,
        SpellAction::Cast(SpellCast { command: command.clone(), difficulty_id, turret_ai }), config, plan)
    else { return CreatureSpellCastValidationResultLikeCpp::MissingTarget; };
    result(&outcome)
}
fn result(outcome: &SpellOutcome) -> CreatureSpellCastValidationResultLikeCpp {
    if outcome.spell_hits != 0 { return CreatureSpellCastValidationResultLikeCpp::Ready(CreatureSpellTargetHitResultLikeCpp::Hit); }
    if outcome.spell_misses != 0 { return CreatureSpellCastValidationResultLikeCpp::Ready(CreatureSpellTargetHitResultLikeCpp::Miss); }
    if outcome.spell_range_rejections != 0 { return CreatureSpellCastValidationResultLikeCpp::OutOfRange; }
    if outcome.spell_los_rejections != 0 { return CreatureSpellCastValidationResultLikeCpp::LosRejected; }
    if outcome.canonical_cast_target_rejections != 0 { return CreatureSpellCastValidationResultLikeCpp::TargetRejected; }
    if outcome.canonical_cast_cooldown_rejections != 0 { return CreatureSpellCastValidationResultLikeCpp::CooldownRejected; }
    if outcome.spell_hit_results_unrepresented != 0 { return CreatureSpellCastValidationResultLikeCpp::HitResultUnrepresented; }
    if outcome.caster_incarnation_rejections != 0 { return CreatureSpellCastValidationResultLikeCpp::CasterIncarnationRejected; }
    if outcome.runtime_rng_authority_rejections != 0 { return CreatureSpellCastValidationResultLikeCpp::RuntimeRngAuthorityRejected; }
    CreatureSpellCastValidationResultLikeCpp::MissingTarget
}
