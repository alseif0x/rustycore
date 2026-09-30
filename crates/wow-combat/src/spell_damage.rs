// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell damage percentage and final bonus arithmetic over caller-resolved data.

/// Ordered represented inputs to C++ Unit::SpellDamagePctDone.
#[derive(Debug, Clone, Copy)]
pub struct SpellDamagePctDoneInputsLikeCpp<'a> {
    pub school_mask: u8,
    pub school_percentages: &'a [f32; 7],
    pub creature_type_mask: u32,
    pub damage_done_versus: &'a [(i32, i32)],
    pub target_aura_state_mask: u32,
    pub damage_done_versus_aura_state: &'a [(i32, i32)],
    pub target_mechanic_mask: u64,
    pub damage_percent_done_by_target_aura_mechanic: &'a [(i32, i32)],
    pub spell_mechanic: Option<i32>,
    pub damage_done_for_mechanic: Option<&'a [(i32, i32)]>,
    pub scripted_factor: f32,
}

/// C++ Unit::SpellDamagePctDone represented player branch. Caller-owned reads
/// remain ordered: school, creature type, aura state, target mechanic, spell
/// mechanic effects, and finally the branch-specific scripted factor.
pub fn spell_damage_pct_done_like_cpp(inputs: SpellDamagePctDoneInputsLikeCpp<'_>) -> f32 {
    let mask = u32::from(inputs.school_mask);
    let mut modifier = 0.0_f32;
    for (school, percent) in inputs.school_percentages.iter().enumerate() {
        if mask & (1_u32 << school) != 0 {
            modifier = modifier.max(*percent);
        }
    }
    if inputs.creature_type_mask != 0 {
        for &(misc_value, amount) in inputs.damage_done_versus {
            if misc_value & inputs.creature_type_mask as i32 != 0 {
                modifier *= 1.0 + amount as f32 / 100.0;
            }
        }
    }
    if inputs.target_aura_state_mask != 0 {
        for &(misc_value, amount) in inputs.damage_done_versus_aura_state {
            if represented_aura_state_bit_like_cpp(misc_value)
                .is_some_and(|bit| inputs.target_aura_state_mask & bit != 0)
            {
                modifier *= 1.0 + amount as f32 / 100.0;
            }
        }
    }
    if inputs.target_mechanic_mask != 0 {
        for &(misc_value, amount) in inputs.damage_percent_done_by_target_aura_mechanic {
            if represented_mechanic_bit_like_cpp(misc_value)
                .is_some_and(|bit| inputs.target_mechanic_mask & bit != 0)
            {
                modifier *= 1.0 + amount as f32 / 100.0;
            }
        }
    }
    if let Some(mechanic) = inputs.spell_mechanic {
        let percent = inputs
            .damage_done_for_mechanic
            .unwrap_or_default()
            .iter()
            .filter(|(misc_value, _)| *misc_value == mechanic)
            .map(|(_, amount)| *amount)
            .sum::<i32>();
        if percent != 0 {
            modifier *= 1.0 + percent as f32 / 100.0;
        }
    }
    modifier * inputs.scripted_factor
}

/// C++ Unit::SpellDamageBonusDone final float application and nonnegative
/// uint32 clamp for the represented calculation.
pub fn spell_damage_bonus_done_like_cpp(
    base_damage: u32,
    done_flat_benefit: i32,
    done_total_modifier: f32,
) -> u32 {
    let damage = (base_damage as f32 + done_flat_benefit as f32) * done_total_modifier;
    u32::try_from(damage.max(0.0).min(u32::MAX as f32) as u32).unwrap_or(u32::MAX)
}

fn represented_mechanic_bit_like_cpp(mechanic: i32) -> Option<u64> {
    (1..64).contains(&mechanic).then(|| 1_u64 << mechanic)
}

pub(super) fn represented_aura_state_bit_like_cpp(aura_state: i32) -> Option<u32> {
    u32::try_from(aura_state)
        .ok()
        .and_then(|state| state.checked_sub(1))
        .and_then(|bit| 1_u32.checked_shl(bit))
}
