// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell healing percentage, taken, and final bonus arithmetic.

/// Ordered represented inputs to C++ Unit::SpellHealingPctDone.
#[derive(Debug, Clone, Copy)]
pub struct SpellHealingPctDoneInputsLikeCpp<'a> {
    pub gated: bool,
    pub healing_done_percent: f32,
    pub target_aura_state_mask: u32,
    pub damage_done_versus_aura_state: &'a [(i32, i32)],
    pub healing_done_pct_versus_target_health: &'a [i32],
    pub target_health_pct: Option<f32>,
}

/// C++ Unit::SpellHealingPctDone represented player branch. World resolves the
/// attribute gates, aura rows, and target health at their original read sites.
pub fn spell_healing_pct_done_like_cpp(inputs: SpellHealingPctDoneInputsLikeCpp<'_>) -> f32 {
    if inputs.gated {
        return 1.0;
    }
    let mut modifier = inputs.healing_done_percent;
    if inputs.target_aura_state_mask != 0 {
        for &(misc_value, amount) in inputs.damage_done_versus_aura_state {
            if crate::spell_damage::represented_aura_state_bit_like_cpp(misc_value)
                .is_some_and(|bit| inputs.target_aura_state_mask & bit != 0)
            {
                modifier *= 1.0 + amount as f32 / 100.0;
            }
        }
    }
    if !inputs.healing_done_pct_versus_target_health.is_empty()
        && let Some(health_pct) = inputs.target_health_pct
    {
        let health_pct_diff = (100.0 - health_pct).max(0.0);
        for &amount in inputs.healing_done_pct_versus_target_health {
            modifier *= 1.0 + (amount as f32 * health_pct_diff / 100.0) / 100.0;
        }
    }
    modifier
}

/// Add represented victim SPELL_AURA_MOD_HEALING amounts that intersect the
/// spell school mask. Unlike SpellBaseHealingBonusDone, misc value zero does
/// not act as a wildcard for this victim-side term.
pub fn spell_healing_bonus_from_victim_aura_effects_like_cpp(
    advertised_benefit: i32,
    school_mask: u8,
    victim_effects: &[(i32, i32)],
) -> i32 {
    let mask = i32::from(school_mask);
    advertised_benefit.saturating_add(
        victim_effects
            .iter()
            .filter(|(misc_value, _)| misc_value & mask != 0)
            .map(|(_, amount)| *amount)
            .sum::<i32>(),
    )
}

/// C++ Unit::SpellHealingBonusTaken's represented min-negative then
/// max-positive SPELL_AURA_MOD_HEALING_PCT multipliers.
pub fn spell_healing_bonus_taken_like_cpp(heal_amount: u32, aura_amounts: &[i32]) -> u32 {
    let mut taken_total_modifier = 1.0_f32;
    let mut min_negative = 0_i32;
    let mut max_positive = 0_i32;
    for &amount in aura_amounts {
        min_negative = min_negative.min(amount);
        max_positive = max_positive.max(amount);
    }
    if min_negative != 0 {
        taken_total_modifier *= 1.0 + min_negative as f32 / 100.0;
    }
    if max_positive != 0 {
        taken_total_modifier *= 1.0 + max_positive as f32 / 100.0;
    }
    let heal = heal_amount as f32 * taken_total_modifier;
    u32::try_from(heal.max(0.0).min(u32::MAX as f32) as u32).unwrap_or(u32::MAX)
}

/// C++ Unit::SpellHealingBonusDone final float application and nonnegative
/// uint32 clamp for the represented calculation.
pub fn spell_healing_bonus_done_like_cpp(
    base_heal: u32,
    done_flat_benefit: i32,
    done_total_modifier: f32,
) -> u32 {
    let heal = (base_heal as f32 + done_flat_benefit as f32) * done_total_modifier;
    u32::try_from(heal.max(0.0).min(u32::MAX as f32) as u32).unwrap_or(u32::MAX)
}
