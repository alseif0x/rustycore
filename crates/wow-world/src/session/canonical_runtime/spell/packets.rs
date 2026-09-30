//! Counter projection and unchanged APP wire construction outside map guards.
use super::*;
pub(in crate::session) fn add_outcome(target: &mut LegacyCreatureSpellTickOutcomeLikeCpp, outcome: SpellOutcome) {
    target.skipped_owner_not_global |= outcome.skipped_owner_not_global;
    target.maps_seen += outcome.maps_seen;
    target.creatures_seen += outcome.creatures_seen;
    target.ai_selection_unrepresented += outcome.ai_selection_unrepresented;
    target.missing_spell_metadata += outcome.missing_spell_metadata;
    target.schedules_initialized += outcome.schedules_initialized;
    target.casts_ready += outcome.casts_ready;
    target.noninstant_casts_unrepresented += outcome.noninstant_casts_unrepresented;
    target.spell_runtime_hooks_unrepresented += outcome.spell_runtime_hooks_unrepresented;
    target.spell_casting_requirements_unrepresented += outcome.spell_casting_requirements_unrepresented;
    target.spell_disable_context_unrepresented += outcome.spell_disable_context_unrepresented;
    target.spells_disabled += outcome.spells_disabled;
    target.turret_rejected_attempt_swings += outcome.turret_rejected_attempt_swings;
    target.caster_incarnation_rejections += outcome.caster_incarnation_rejections;
    target.spell_effects_unrepresented += outcome.spell_effects_unrepresented;
    target.spell_projectiles_unrepresented += outcome.spell_projectiles_unrepresented;
    target.spell_visuals_unrepresented += outcome.spell_visuals_unrepresented;
    target.unit_state_casting_skips += outcome.unit_state_casting_skips;
    target.spell_range_rejections += outcome.spell_range_rejections;
    target.spell_los_rejections += outcome.spell_los_rejections;
    target.spell_hit_results_unrepresented += outcome.spell_hit_results_unrepresented;
    target.runtime_rng_authority_rejections += outcome.runtime_rng_authority_rejections;
    target.spell_hits += outcome.spell_hits;
    target.spell_misses += outcome.spell_misses;
    target.canonical_cast_preconditions_passed += outcome.canonical_cast_preconditions_passed;
    target.canonical_cast_missing_target += outcome.canonical_cast_missing_target;
    target.canonical_cast_target_rejections += outcome.canonical_cast_target_rejections;
    target.canonical_cast_cooldown_rejections += outcome.canonical_cast_cooldown_rejections;
    for completion in outcome.completions {
        creature_spell_publication::append_completion(&mut target.plan, completion);
    }
}
pub(in crate::session) fn replace_counters(target: &mut LegacyCreatureSpellTickOutcomeLikeCpp, outcome: &SpellOutcome) {
    target.skipped_owner_not_global = outcome.skipped_owner_not_global;
    target.maps_seen = outcome.maps_seen;
    target.creatures_seen = outcome.creatures_seen;
    target.ai_selection_unrepresented = outcome.ai_selection_unrepresented;
    target.missing_spell_metadata = outcome.missing_spell_metadata;
    target.schedules_initialized = outcome.schedules_initialized;
    target.casts_ready = outcome.casts_ready;
    target.noninstant_casts_unrepresented = outcome.noninstant_casts_unrepresented;
    target.spell_runtime_hooks_unrepresented = outcome.spell_runtime_hooks_unrepresented;
    target.spell_casting_requirements_unrepresented = outcome.spell_casting_requirements_unrepresented;
    target.spell_disable_context_unrepresented = outcome.spell_disable_context_unrepresented;
    target.spells_disabled = outcome.spells_disabled;
    target.turret_rejected_attempt_swings = outcome.turret_rejected_attempt_swings;
    target.caster_incarnation_rejections = outcome.caster_incarnation_rejections;
    target.spell_effects_unrepresented = outcome.spell_effects_unrepresented;
    target.spell_projectiles_unrepresented = outcome.spell_projectiles_unrepresented;
    target.spell_visuals_unrepresented = outcome.spell_visuals_unrepresented;
    target.unit_state_casting_skips = outcome.unit_state_casting_skips;
    target.spell_range_rejections = outcome.spell_range_rejections;
    target.spell_los_rejections = outcome.spell_los_rejections;
    target.spell_hit_results_unrepresented = outcome.spell_hit_results_unrepresented;
    target.runtime_rng_authority_rejections = outcome.runtime_rng_authority_rejections;
    target.spell_hits = outcome.spell_hits;
    target.spell_misses = outcome.spell_misses;
    target.canonical_cast_preconditions_passed = outcome.canonical_cast_preconditions_passed;
    target.canonical_cast_missing_target = outcome.canonical_cast_missing_target;
    target.canonical_cast_target_rejections = outcome.canonical_cast_target_rejections;
    target.canonical_cast_cooldown_rejections = outcome.canonical_cast_cooldown_rejections;
}
pub(in crate::session) fn replace_outcome(target: &mut LegacyCreatureSpellTickOutcomeLikeCpp, outcome: SpellOutcome) {
    replace_counters(target, &outcome);
    for completion in outcome.completions {
        creature_spell_publication::append_completion(&mut target.plan, completion);
    }
}
