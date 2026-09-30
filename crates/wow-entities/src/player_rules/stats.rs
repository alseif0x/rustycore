//! Pure derived Player stats from resolved immutable inputs.

use wow_data_model::player_stats::{
    PlayerStatSystemInputLikeCpp, PlayerStatSystemProjectionLikeCpp,
};

const DIMINISHING_K_LIKE_CPP: [f32; 14] = [
    0.9560, 0.9560, 0.9880, 0.9880, 0.9830, 0.9560, 0.9880, 0.9830, 0.9830, 0.9830, 0.9720, 0.9830,
    0.9880, 1.0,
];
const PARRY_CAP_LIKE_CPP: [f32; 14] = [
    65.631440, 65.631440, 145.560408, 145.560408, 0.0, 65.631440, 145.560408, 0.0, 0.0, 90.6425,
    0.0, 65.631440, 0.0, 0.0,
];
const DODGE_CAP_LIKE_CPP: [f32; 14] = [
    65.631440, 65.631440, 145.560408, 145.560408, 150.375940, 65.631440, 145.560408, 150.375940,
    150.375940, 145.560408, 116.890707, 145.560408, 145.560408, 0.0,
];

fn diminishing_returns_like_cpp(
    cap: &[f32; 14],
    class: u8,
    non_diminishing: f32,
    diminishing: f32,
) -> f32 {
    let Some(index) = class.checked_sub(1).map(usize::from) else {
        return non_diminishing;
    };
    let Some((&cap, &k)) = cap.get(index).zip(DIMINISHING_K_LIKE_CPP.get(index)) else {
        return non_diminishing;
    };
    if cap == 0.0 {
        return non_diminishing;
    }
    cap * diminishing / (diminishing + cap * k) + non_diminishing
}

fn health_bonus_from_stamina_like_cpp(stamina: i32) -> i64 {
    let stamina = i64::from(stamina);
    stamina.min(20) + (stamina - 20).max(0) * 10
}

fn mana_bonus_from_intellect_like_cpp(intellect: i32) -> i64 {
    let intellect = i64::from(intellect);
    intellect.min(20) + (intellect - 20).max(0) * 15
}

/// Represent the C++ `Player::UpdateAllStats` branches backed by the inputs
/// available in this runtime.
///
/// Item flat modifiers, combat ratings and the represented total-stat
/// percentage multipliers are included. The remaining aura modifiers stay
/// owned by the wider represented aura runtime.
pub fn calculate_derived_stats(
    input: PlayerStatSystemInputLikeCpp,
) -> PlayerStatSystemProjectionLikeCpp {
    let base_stats = input.base.primary_stats_like_cpp().map(i32::from);
    let stats = std::array::from_fn(|index| {
        let value = base_stats[index].saturating_add(input.gear_stats[index]);
        // C++ `Player::UpdateStats` truncates `GetTotalStatValue()` to int32.
        (value as f32 * input.stat_total_multipliers[index].max(0.0)) as i32
    });
    let stat_pos_buff = std::array::from_fn(|index| {
        (input.gear_stats[index].max(0) as f32 * input.stat_buff_total_multipliers[index].max(0.0))
            as i32
    });
    let stat_neg_buff = std::array::from_fn(|index| {
        (input.gear_stats[index].min(0) as f32 * input.stat_buff_total_multipliers[index].max(0.0))
            as i32
    });

    let max_health =
        i64::from(input.gear_health).saturating_add(health_bonus_from_stamina_like_cpp(stats[2]));
    let base_mana = i32::try_from(input.base.base_mana).unwrap_or(i32::MAX);
    let mana_bonus = if base_mana > 0 {
        mana_bonus_from_intellect_like_cpp(stats[3])
    } else {
        0
    };
    let max_mana = i64::from(base_mana)
        .saturating_add(i64::from(input.gear_mana))
        .saturating_add(mana_bonus);

    let class_specific_attack_power = match input.class {
        1 | 2 | 6 => f32::from(input.level) * 3.0 - 20.0,
        3 | 4 | 7 | 11 => f32::from(input.level) * 2.0 - 20.0,
        _ => -20.0,
    };
    // C++ `Player::UpdateAllStats` (`StatSystem.cpp:199-222`) runs
    // `UpdateAttackPowerAndDamage` before `UpdateSpellDamageAndHealingBonus`, so
    // the attack-power override reads the spell fields of the previous pass.
    // `Unit::SpellBaseDamageBonusDone` (`Unit.cpp:6860-6890`) and
    // `SpellBaseHealingBonusDone` (`7282-7315`) add the base spell power, the
    // `SPELL_AURA_MOD_DAMAGE_DONE`/`MOD_HEALING_DONE` flat sums and the
    // stat-percent auras; `SetUpdateFieldStatValue` clamps the published fields
    // at zero.
    let stat_percent = |amount: i32, stat_index: i32| -> i32 {
        usize::try_from(stat_index)
            .ok()
            .and_then(|index| stats.get(index).copied())
            .map(|stat| (stat as f32 * amount as f32 / 100.0) as i32)
            .unwrap_or(0)
    };
    let damage_bonus = |school: usize| -> i32 {
        let mut benefit = input.spell_bonus.damage_done_flat[school]
            .saturating_add(input.spell_bonus.base_spell_power);
        for (stat_index, amount) in input.spell_bonus.damage_of_stat_percent[school]
            .iter()
            .enumerate()
        {
            benefit = benefit.saturating_add(stat_percent(*amount, stat_index as i32));
        }
        benefit
    };
    let healing_bonus = || -> i32 {
        let mut benefit = input
            .spell_bonus
            .healing_done_flat
            .saturating_add(input.spell_bonus.base_spell_power);
        if base_mana > 0 {
            // C++ `GetPowerIndex(POWER_MANA) != MAX_POWERS` adds the intellect
            // term; the class base-mana row represents that mana slot.
            benefit = benefit.saturating_add(stats[3].max(0));
        }
        for (stat_index, amount) in input.spell_bonus.healing_of_stat_percent.iter().enumerate() {
            benefit = benefit.saturating_add(stat_percent(*amount, stat_index as i32));
        }
        benefit
    };
    let mod_damage_done_neg = input.spell_bonus.damage_done_neg;
    let mut mod_damage_done_pos = [0i32; 7];
    for (school, positive) in mod_damage_done_pos.iter_mut().enumerate().skip(1) {
        *positive = (damage_bonus(school) - mod_damage_done_neg[school]).max(0);
    }
    let mut mod_healing_done_pos = healing_bonus().max(0);

    // C++ `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:341-379`):
    // while `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` is active, both the
    // melee and the ranged unit mod replace the strength/agility/level base
    // with `CalculatePct(float(minSpellPower), percent)` truncated by the
    // `int32(base_attPower)` store.
    let (attack_power, ranged_attack_power) = match input.attack_power_override_by_spell_power_pct {
        Some(percent) => {
            let min_spell_power = mod_damage_done_pos
                .iter()
                .skip(1)
                .fold(mod_healing_done_pos, |min, value| min.min(*value));
            let overridden = (min_spell_power as f32 * percent / 100.0) as i32;
            (overridden, overridden)
        }
        None => (
            ((stats[0] as f32 * f32::from(input.attack_power_per_strength)).max(0.0)
                + (stats[1] as f32 * f32::from(input.attack_power_per_agility)).max(0.0)
                + class_specific_attack_power) as i32,
            ((f32::from(input.level) + (stats[1] as f32).max(0.0))
                * f32::from(input.ranged_attack_power_per_agility)
                - 10.0) as i32,
        ),
    };

    // C++ `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:333-403`):
    // `SetAttackPower(BASE_VALUE)`, `SetAttackPowerModPos(TOTAL_VALUE)` with
    // gear plus the `MOD_ATTACK_POWER` auras, and
    // `SetAttackPowerMultiplier(TOTAL_PCT - 1.0)` from `MOD_ATTACK_POWER_PCT`.
    // `Unit::GetTotalAttackPowerValue` then clamps the base plus modifier at
    // zero before the multiplier.
    let attack_power_mod_pos = input
        .gear_attack_power
        .saturating_add(input.attack_power_flat_aura);
    let attack_power_multiplier = input.attack_power_total_pct - 1.0;
    let total_attack_power = (attack_power.saturating_add(attack_power_mod_pos)).max(0) as f32
        * input.attack_power_total_pct;
    let ranged_attack_power_mod_pos = input
        .gear_attack_power
        .saturating_add(input.gear_ranged_attack_power)
        .saturating_add(input.ranged_attack_power_flat_aura);
    let ranged_attack_power_multiplier = input.ranged_attack_power_total_pct - 1.0;
    let total_ranged_attack_power =
        (ranged_attack_power.saturating_add(ranged_attack_power_mod_pos)).max(0) as f32
            * input.ranged_attack_power_total_pct;

    // C++ `Unit::SpellBaseDamageBonusDone`/`SpellBaseHealingBonusDone` short
    // circuit to `int32(CalculatePct(GetTotalAttackPowerValue(BASE_ATTACK),
    // percent) + 0.5f)` while `SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` is
    // active (`StatSystem.cpp:154-168`, `417-418` re-runs this pass after the
    // attack-power update above).
    if input.spell_bonus.override_spell_power_by_ap_pct > 0.0 {
        let overridden = (total_attack_power * input.spell_bonus.override_spell_power_by_ap_pct
            / 100.0
            + 0.5) as i32;
        for (school, positive) in mod_damage_done_pos.iter_mut().enumerate().skip(1) {
            *positive = (overridden - mod_damage_done_neg[school]).max(0);
        }
        mod_healing_done_pos = overridden.max(0);
    }

    let rating = |index: usize| input.rating_bonuses.get(index).copied().unwrap_or(0.0);
    // C++ `Player::UpdateAllCritPercentages`/`UpdateCritPercentage`
    // (`StatSystem.cpp:502-538`) seeds each group with 5%, adds the
    // weapon-dependent `FLAT_MOD` aura sum and the melee/ranged rating bonus;
    // `UpdateSpellCritChance` (`718-731`) applies the same shape to every
    // school with the spell rating.
    let crit_pct = 5.0 + input.crit_mainhand_aura_pct + rating(8);
    let offhand_crit_pct = 5.0 + input.crit_offhand_aura_pct + rating(8);
    let ranged_crit_pct = 5.0 + input.crit_ranged_aura_pct + rating(9);
    let spell_crit = 5.0 + input.spell_crit_aura_pct + rating(10);
    let dodge_pct = diminishing_returns_like_cpp(
        &DODGE_CAP_LIKE_CPP,
        input.class,
        input.spell_dodge_pct,
        rating(2),
    );
    let parry_pct = if input.can_parry
        && PARRY_CAP_LIKE_CPP
            .get(usize::from(input.class.saturating_sub(1)))
            .is_some_and(|cap| *cap > 0.0)
    {
        diminishing_returns_like_cpp(
            &PARRY_CAP_LIKE_CPP,
            input.class,
            5.0 + input.spell_parry_pct,
            rating(3),
        )
    } else {
        0.0
    };
    let block_pct = if input.can_block {
        5.0 + input.spell_block_pct + rating(4)
    } else {
        0.0
    };

    // C++ `Player::UpdateArmor` (`StatSystem.cpp:251-276`): the item
    // `BASE_VALUE` is scaled by the base-resistance percentage, the agility
    // term and the aura `TOTAL_VALUE`/`MOD_RESISTANCE_OF_STAT_PERCENT` terms
    // follow, and the `TOTAL_PCT`/`MOD_BONUS_ARMOR_PCT` multipliers apply last.
    // `SetArmor(int32(value), ...)` truncates toward zero.
    let mut armor = input.gear_armor as f32 * input.armor_base_pct;
    armor += stats[1] as f32 * 2.0;
    armor += input.armor_flat_aura as f32;
    for (stat_index, amount) in input.armor_of_stat_percent.iter().enumerate() {
        if *amount != 0 {
            armor += stats[stat_index] as f32 * *amount as f32 / 100.0;
        }
    }
    armor = armor * input.armor_total_pct * input.armor_bonus_pct;
    let armor = armor as i32;

    PlayerStatSystemProjectionLikeCpp {
        stats,
        stat_pos_buff,
        stat_neg_buff,
        create_health: 0,
        base_mana,
        max_health,
        max_mana,
        armor,
        attack_power,
        attack_power_mod_pos,
        attack_power_multiplier,
        ranged_attack_power,
        ranged_attack_power_mod_pos,
        ranged_attack_power_multiplier,
        total_attack_power: total_attack_power as i32,
        total_ranged_attack_power: total_ranged_attack_power as i32,
        block_pct,
        dodge_pct,
        dodge_from_attr: 0.0,
        parry_pct,
        parry_from_attr: 0.0,
        crit_pct,
        ranged_crit_pct,
        offhand_crit_pct,
        spell_crit_pct: [spell_crit; 7],
        mod_damage_done_pos,
        mod_damage_done_neg,
        mod_healing_done_pos,
        mod_damage_done_percent: input.spell_bonus.damage_done_percent,
        mod_healing_done_percent: input.spell_bonus.healing_done_percent,
        weapon_damage_pct: input.spell_bonus.weapon_damage_pct,
        weapon_damage_flat: input.spell_bonus.weapon_damage_flat,
        mod_target_resistance: input
            .spell_bonus
            .target_resistance_aura
            .saturating_sub(input.spell_bonus.item_spell_penetration),
        mod_target_physical_resistance: input.spell_bonus.target_physical_resistance_aura,
        versatility_bonus: (input.spell_bonus.versatility_bonus_aura as f32).max(0.0),
        override_spell_power_by_ap_percent: input.spell_bonus.override_spell_power_by_ap_pct,
        override_ap_by_spell_power_percent: input
            .attack_power_override_by_spell_power_pct
            .unwrap_or(0.0),
    }
}

#[cfg(test)]
mod tests;
