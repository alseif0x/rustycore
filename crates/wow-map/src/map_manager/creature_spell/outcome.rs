//! Counters and accepted effects; APP owns serialization/publication.
use super::*;
#[derive(Debug, Default)]
pub struct SpellOutcome {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub ai_selection_unrepresented: usize,
    pub missing_spell_metadata: usize,
    pub schedules_initialized: usize,
    pub casts_ready: usize,
    pub noninstant_casts_unrepresented: usize,
    pub spell_runtime_hooks_unrepresented: usize,
    pub spell_casting_requirements_unrepresented: usize,
    pub spell_disable_context_unrepresented: usize,
    pub spells_disabled: usize,
    /// TurretAI attempts whose rejected `CastSpell` still consumed BASE_ATTACK
    /// because the raw combat-range gate had admitted them.
    pub turret_rejected_attempt_swings: usize,
    /// Casts dropped because the live creature was no longer the incarnation the
    /// plan had been captured from.
    pub caster_incarnation_rejections: usize,
    pub spell_effects_unrepresented: usize,
    pub spell_projectiles_unrepresented: usize,
    pub spell_visuals_unrepresented: usize,
    pub unit_state_casting_skips: usize,
    pub spell_range_rejections: usize,
    pub spell_los_rejections: usize,
    pub spell_hit_results_unrepresented: usize,
    pub runtime_rng_authority_rejections: usize,
    pub spell_hits: usize,
    pub spell_misses: usize,
    pub canonical_cast_preconditions_passed: usize,
    pub canonical_cast_missing_target: usize,
    pub canonical_cast_target_rejections: usize,
    pub canonical_cast_cooldown_rejections: usize,
    pub completions: Vec<SpellCompletion>,
}
