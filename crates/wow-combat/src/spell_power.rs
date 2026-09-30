// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell-power and attack-power coefficient arithmetic over resolved values.

/// C++ SpellBaseDamageBonusDone and SpellBaseHealingBonusDone AP override.
pub fn spell_power_override_from_ap_like_cpp(
    attack_power: i32,
    attack_power_mod_pos: i32,
    attack_power_multiplier: f32,
    override_percent: f32,
) -> i32 {
    let total_attack_power = attack_power
        .saturating_add(attack_power_mod_pos)
        .max(0) as f32
        * (1.0 + attack_power_multiplier);
    (total_attack_power * override_percent / 100.0 + 0.5) as i32
}

/// C++ table BonusCoefficientFromAP contribution after the world owner resolves
/// canonical BASE_ATTACK total attack power.
pub fn spell_bonus_coefficient_from_ap_like_cpp(
    coefficient_from_ap: f32,
    attack_power: f32,
) -> i32 {
    if !(coefficient_from_ap > 0.0) {
        return 0;
    }
    (coefficient_from_ap * attack_power) as i32
}

/// C++ default BonusCoefficient term, evaluated before the AP getter when both
/// table coefficients are present.
pub fn spell_advertised_coefficient_benefit_like_cpp(
    advertised_benefit: i32,
    coefficient: f32,
) -> i32 {
    (advertised_benefit as f32 * coefficient) as i32
}

/// Add the already-computed BonusCoefficientFromAP contribution in the
/// represented damage/healing done order.
pub fn spell_done_flat_benefit_add_ap_like_cpp(
    coefficient_benefit: i32,
    coefficient_from_ap: i32,
) -> i32 {
    coefficient_benefit + coefficient_from_ap
}

/// C++ SpellBaseDamageBonusDone fallback after the AP override branch.
///
/// Aura rows are caller-resolved in their original iteration order as
/// (misc_value, amount) and (aura_mask, stat_index, amount).
pub fn spell_base_damage_bonus_fallback_like_cpp(
    school_mask: u8,
    spell_power: i32,
    stats: &[i32; 5],
    damage_done_effects: &[(i32, i32)],
    damage_stat_effects: &[(i32, i32, i32)],
) -> i32 {
    let mask = i32::from(school_mask);
    let mut benefit = damage_done_effects
        .iter()
        .filter(|(misc_value, _)| misc_value & mask != 0)
        .map(|(_, amount)| *amount)
        .sum::<i32>()
        .saturating_add(spell_power);
    for &(aura_mask, stat_index, amount) in damage_stat_effects {
        if aura_mask & mask == 0 {
            continue;
        }
        if let Some(stat) = usize::try_from(stat_index)
            .ok()
            .and_then(|index| stats.get(index))
        {
            benefit =
                benefit.saturating_add((*stat as f32 * amount as f32 / 100.0) as i32);
        }
    }
    benefit
}

/// C++ SpellBaseHealingBonusDone fallback after the AP override branch.
///
/// Healing aura rows are (misc_value, amount); stat rows are
/// (stat_index, misc_value_b, amount), with misc_value_b intentionally unused
/// by this represented calculation.
pub fn spell_base_healing_bonus_fallback_like_cpp(
    school_mask: u8,
    spell_power: i32,
    base_mana: i32,
    stats: &[i32; 5],
    healing_done_effects: &[(i32, i32)],
    healing_stat_effects: &[(i32, i32, i32)],
) -> i32 {
    let mask = i32::from(school_mask);
    let mut benefit = healing_done_effects
        .iter()
        .filter(|(misc_value, _)| *misc_value == 0 || misc_value & mask != 0)
        .map(|(_, amount)| *amount)
        .sum::<i32>()
        .saturating_add(spell_power);
    if base_mana > 0 {
        benefit = benefit.saturating_add(stats[3].max(0));
    }
    for &(stat_index, _, amount) in healing_stat_effects {
        if let Some(stat) = usize::try_from(stat_index)
            .ok()
            .and_then(|index| stats.get(index))
        {
            benefit =
                benefit.saturating_add((*stat as f32 * amount as f32 / 100.0) as i32);
        }
    }
    benefit
}
