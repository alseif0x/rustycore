// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_data::PlayerSpellBonusInputLikeCpp;
use wow_world_core::session::state::hub_support::{
    RepresentedPlayerGearStatsLikeCpp, SPELL_SCHOOL_MASK_ALL_LIKE_CPP,
    SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP, SPELL_SCHOOL_MASK_SPELL_LIKE_CPP,
};
use wow_world_core::session::{PlayerStatCatalogs, PlayerStatFacts};

use super::equipment;

pub(super) fn represented_spell_bonus(
    facts: &PlayerStatFacts<'_>,
    catalogs: &PlayerStatCatalogs<'_>,
    gear: &RepresentedPlayerGearStatsLikeCpp,
) -> PlayerSpellBonusInputLikeCpp {
    let aura = facts.aura_view();
    let mut damage_done_flat = [0i32; 7];
    let mut damage_done_neg = [0i32; 7];
    for (misc_value, amount) in
        aura.effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE)
    {
        for (school, flat) in damage_done_flat.iter_mut().enumerate().skip(1) {
            if misc_value & (1_i32 << school) == 0 {
                continue;
            }
            *flat = flat.saturating_add(amount);
            if amount < 0 {
                damage_done_neg[school] = damage_done_neg[school].saturating_add(amount);
            }
        }
    }

    let mut damage_of_stat_percent = [[0i32; 5]; 7];
    for (school_mask, stat_index, amount) in aura.effects_with_misc_values_by_aura_type(
        wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT,
    ) {
        let Ok(stat) = usize::try_from(stat_index) else {
            continue;
        };
        for (school, per_school) in damage_of_stat_percent.iter_mut().enumerate().skip(1) {
            if school_mask & (1_i32 << school) == 0 {
                continue;
            }
            if let Some(value) = per_school.get_mut(stat) {
                *value = value.saturating_add(amount);
            }
        }
    }

    let healing_done_flat = aura
        .effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE)
        .into_iter()
        .filter(|(misc_value, _)| {
            *misc_value == 0 || (*misc_value & SPELL_SCHOOL_MASK_ALL_LIKE_CPP) != 0
        })
        .map(|(_, amount)| amount)
        .sum::<i32>();

    let mut healing_of_stat_percent = [0i32; 5];
    for (stat_index, _, amount) in aura.effects_with_misc_values_by_aura_type(
        wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT,
    ) {
        if let Ok(stat) = usize::try_from(stat_index)
            && let Some(value) = healing_of_stat_percent.get_mut(stat)
        {
            *value = value.saturating_add(amount);
        }
    }

    // C++ `AuraEffect::HandleModDamagePercentDone` (`SpellAuraEffects.cpp:4525-4548`)
    // sets each selected school to the product of all matching effects.
    let damage_percent_effects =
        aura.effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE);
    let mut damage_done_percent = [1.0_f32; 7];
    for (school, percent) in damage_done_percent.iter_mut().enumerate() {
        let mask = 1_i32 << school;
        if !damage_percent_effects
            .iter()
            .any(|(misc_value, _)| misc_value & mask != 0)
        {
            continue;
        }
        *percent = damage_percent_effects
            .iter()
            .filter(|(misc_value, _)| misc_value & mask != 0)
            .fold(1.0_f32, |acc, (_, amount)| {
                acc * (1.0 + *amount as f32 / 100.0)
            });
    }

    let healing_done_percent = aura
        .effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE_PERCENT)
        .into_iter()
        .fold(1.0_f32, |acc, (_, amount)| {
            acc * (1.0 + amount as f32 / 100.0)
        })
        .max(0.0);

    let versatility_bonus_aura = aura
        .effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_VERSATILITY)
        .into_iter()
        .map(|(_, amount)| amount)
        .sum::<i32>();

    let mut target_resistance_aura = 0i32;
    let mut target_physical_resistance_aura = 0i32;
    for (misc_value, amount) in
        aura.effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE)
    {
        if misc_value & SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP != 0 {
            target_physical_resistance_aura =
                target_physical_resistance_aura.saturating_add(amount);
        }
        if misc_value & SPELL_SCHOOL_MASK_SPELL_LIKE_CPP == SPELL_SCHOOL_MASK_SPELL_LIKE_CPP {
            target_resistance_aura = target_resistance_aura.saturating_add(amount);
        }
    }

    let override_effects = aura.effects_by_aura_type(
        wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT,
    );
    // C++ `Player::ApplySpellPowerBonus` returns early while the override aura
    // is present, so the item accumulator never reaches `m_baseSpellPower`.
    let base_spell_power = if override_effects.is_empty() {
        gear.spell_power
    } else {
        0
    };
    let override_spell_power_by_ap_pct = override_effects
        .into_iter()
        .map(|(_, amount)| amount as f32)
        .sum::<f32>();

    PlayerSpellBonusInputLikeCpp {
        base_spell_power,
        damage_done_flat,
        damage_done_neg,
        damage_of_stat_percent,
        healing_done_flat,
        healing_of_stat_percent,
        override_spell_power_by_ap_pct,
        damage_done_percent,
        healing_done_percent,
        versatility_bonus_aura,
        target_resistance_aura,
        item_spell_penetration: gear.spell_penetration_bonus,
        target_physical_resistance_aura,
        weapon_damage_pct: equipment::weapon_damage_pct(facts, catalogs),
        weapon_damage_flat: equipment::weapon_damage_flat(facts, catalogs),
    }
}
