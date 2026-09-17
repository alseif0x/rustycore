// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3

//! C++ `ObjectMgr::LoadPlayerInfo` player stat data.
//!
//! Primary stats come from `player_classlevelstats` plus the signed
//! `player_racestats` modifiers. Base mana comes from the client
//! `gt/BaseMp.txt` GameTable through `ObjectMgr::GetPlayerClassLevelInfo`.
//! C++ does not consume the legacy C# `player_levelstats.basehp/basemana`
//! projection.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::BaseMpGameTableLikeCpp;
use anyhow::{Context, Result, bail};
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerRaceStatsRowLikeCpp {
    pub race: u8,
    pub stat_modifiers: [i16; 5],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerClassLevelStatsRowLikeCpp {
    pub class: u8,
    pub level: u8,
    pub primary_stats: [u16; 5],
}

pub struct PlayerRaceStatsRowsLikeCpp(Vec<PlayerRaceStatsRowLikeCpp>);

impl PlayerRaceStatsRowsLikeCpp {
    pub fn try_from_rows_like_cpp(
        rows: impl IntoIterator<Item = PlayerRaceStatsRowLikeCpp>,
    ) -> Result<Self> {
        let rows: Vec<_> = rows.into_iter().collect();
        if rows.is_empty() {
            bail!("Loaded 0 race stats definitions: player_racestats is empty");
        }
        Ok(Self(rows))
    }
}

pub struct PlayerClassLevelStatsRowsLikeCpp(Vec<PlayerClassLevelStatsRowLikeCpp>);

impl PlayerClassLevelStatsRowsLikeCpp {
    pub fn try_from_rows_like_cpp(
        rows: impl IntoIterator<Item = PlayerClassLevelStatsRowLikeCpp>,
    ) -> Result<Self> {
        let rows: Vec<_> = rows.into_iter().collect();
        if rows.is_empty() {
            bail!("Loaded 0 level stats definitions: player_classlevelstats is empty");
        }
        Ok(Self(rows))
    }
}

/// C++ `PlayerLevelInfo` plus the class/level `GtBaseMP` value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerLevelStats {
    pub strength: u16,
    pub agility: u16,
    pub stamina: u16,
    pub intellect: u16,
    pub spirit: u16,
    pub base_mana: u32,
}

impl PlayerLevelStats {
    pub const fn primary_stats_like_cpp(&self) -> [u16; 5] {
        [
            self.strength,
            self.agility,
            self.stamina,
            self.intellect,
            self.spirit,
        ]
    }
}

/// C++ `Player::UpdateSpellDamageAndHealingBonus` inputs
/// (`StatSystem.cpp:171-197`) built from `Unit::SpellBaseDamageBonusDone` and
/// `Unit::SpellBaseHealingBonusDone` (`Unit.cpp:6860-6890`, `7282-7315`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerSpellBonusInputLikeCpp {
    /// C++ `Player::GetBaseSpellPowerBonus()`: the item/enchant spell power
    /// `Player::ApplySpellPowerBonus` (`StatSystem.cpp:153-168`) accumulates.
    pub base_spell_power: i32,
    /// `GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE, 1 << school)`
    /// per school; index 0 is never published by C++.
    pub damage_done_flat: [i32; 7],
    /// Negative `SPELL_AURA_MOD_DAMAGE_DONE` sums per school
    /// (`ActivePlayerData::ModDamageDoneNeg`).
    pub damage_done_neg: [i32; 7],
    /// `SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT` sums per school and stat
    /// (the effect `MiscValue` is the school mask, `MiscValueB` the stat).
    pub damage_of_stat_percent: [[i32; 5]; 7],
    /// `GetTotalAuraModifier(SPELL_AURA_MOD_HEALING_DONE, SPELL_SCHOOL_MASK_ALL)`.
    pub healing_done_flat: i32,
    /// `SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT` sums per stat index (the
    /// effect `MiscValue` is the stat).
    pub healing_of_stat_percent: [i32; 5],
    /// `ActivePlayerData::OverrideSpellPowerByAPPercent`; `> 0` replaces both
    /// bonuses with `CalculatePct(GetTotalAttackPowerValue(BASE_ATTACK), pct)`.
    pub override_spell_power_by_ap_pct: f32,
    /// `ActivePlayerData::ModDamageDonePercent[7]` from
    /// `AuraEffect::HandleModDamagePercentDone`
    /// (`SpellAuraEffects.cpp:4525-4548`): the product of
    /// `1 + amount/100` over the active `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE`
    /// (79) effects intersecting each school, `1.0` when none does.
    pub damage_done_percent: [f32; 7],
    /// `ActivePlayerData::ModHealingDonePercent` from
    /// `Player::UpdateHealingDonePercentMod` (`StatSystem.cpp:588-599`): the
    /// product of `1 + amount/100` over the active
    /// `SPELL_AURA_MOD_HEALING_DONE_PERCENT` (136) effects, `1.0` when none.
    pub healing_done_percent: f32,
}

impl Default for PlayerSpellBonusInputLikeCpp {
    fn default() -> Self {
        Self {
            base_spell_power: 0,
            damage_done_flat: [0; 7],
            damage_done_neg: [0; 7],
            damage_of_stat_percent: [[0; 5]; 7],
            healing_done_flat: 0,
            healing_of_stat_percent: [0; 5],
            override_spell_power_by_ap_pct: 0.0,
            damage_done_percent: [1.0; 7],
            healing_done_percent: 1.0,
        }
    }
}

/// Inputs currently represented by Rust for C++ `Player::UpdateAllStats`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerStatSystemInputLikeCpp {
    pub base: PlayerLevelStats,
    pub class: u8,
    pub level: u8,
    pub attack_power_per_strength: u8,
    pub attack_power_per_agility: u8,
    pub ranged_attack_power_per_agility: u8,
    /// C++ UnitMods `TOTAL_PCT` after represented
    /// `SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE` effects, one factor per stat.
    pub stat_total_multipliers: [f32; 5],
    /// C++ `Unit::UpdateStatBuffMod` factor for the client-visible positive
    /// and negative flat-stat buff fields.
    pub stat_buff_total_multipliers: [f32; 5],
    pub gear_stats: [i32; 5],
    pub gear_health: i32,
    pub gear_mana: i32,
    pub gear_armor: i32,
    /// C++ `GetPctModifierValue(UNIT_MOD_ARMOR, BASE_PCT)` from
    /// `SPELL_AURA_MOD_BASE_RESISTANCE_PCT` effects carrying the normal school
    /// mask.
    pub armor_base_pct: f32,
    /// C++ `GetFlatModifierValue(UNIT_MOD_ARMOR, TOTAL_VALUE)` from
    /// `SPELL_AURA_MOD_RESISTANCE`/`SPELL_AURA_MOD_BASE_RESISTANCE` effects
    /// carrying the normal school mask.
    pub armor_flat_aura: i32,
    /// `SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT` amounts carrying the normal
    /// school mask, aggregated per `MiscValueB` stat index (`CalculatePct`).
    pub armor_of_stat_percent: [i32; 5],
    /// C++ `GetPctModifierValue(UNIT_MOD_ARMOR, TOTAL_PCT)` from
    /// `SPELL_AURA_MOD_RESISTANCE_PCT` effects carrying the normal school mask.
    pub armor_total_pct: f32,
    /// C++ `GetTotalAuraMultiplier(SPELL_AURA_MOD_BONUS_ARMOR_PCT)`.
    pub armor_bonus_pct: f32,
    /// C++ `GetTotalAuraModifier(SPELL_AURA_MOD_DODGE_PERCENT)`.
    pub spell_dodge_pct: f32,
    /// C++ `GetTotalAuraModifier(SPELL_AURA_MOD_PARRY_PERCENT)`.
    pub spell_parry_pct: f32,
    /// C++ `GetTotalAuraModifier(SPELL_AURA_MOD_BLOCK_PERCENT)`.
    pub spell_block_pct: f32,
    /// C++ `GetBaseModValue(CRIT_PERCENTAGE, FLAT_MOD)` from
    /// `UpdateWeaponDependentCritAuras(BASE_ATTACK)`.
    pub crit_mainhand_aura_pct: f32,
    /// C++ `GetBaseModValue(OFFHAND_CRIT_PERCENTAGE, FLAT_MOD)`.
    pub crit_offhand_aura_pct: f32,
    /// C++ `GetBaseModValue(RANGED_CRIT_PERCENTAGE, FLAT_MOD)`.
    pub crit_ranged_aura_pct: f32,
    /// C++ `GetTotalAuraModifier(SPELL_AURA_MOD_SPELL_CRIT_CHANCE)` plus
    /// `GetTotalAuraModifier(SPELL_AURA_MOD_CRIT_PCT)`.
    pub spell_crit_aura_pct: f32,
    pub gear_attack_power: i32,
    pub gear_ranged_attack_power: i32,
    /// C++ `GetFlatModifierValue(UNIT_MOD_ATTACK_POWER, TOTAL_VALUE)` from
    /// `SPELL_AURA_MOD_ATTACK_POWER` (99).
    pub attack_power_flat_aura: i32,
    /// C++ `GetPctModifierValue(UNIT_MOD_ATTACK_POWER, TOTAL_PCT)` from
    /// `SPELL_AURA_MOD_ATTACK_POWER_PCT` (166).
    pub attack_power_total_pct: f32,
    /// C++ `GetFlatModifierValue(UNIT_MOD_ATTACK_POWER_RANGED, TOTAL_VALUE)`
    /// from `SPELL_AURA_MOD_RANGED_ATTACK_POWER` (124).
    pub ranged_attack_power_flat_aura: i32,
    /// C++ `GetPctModifierValue(UNIT_MOD_ATTACK_POWER_RANGED, TOTAL_PCT)` from
    /// `SPELL_AURA_MOD_RANGED_ATTACK_POWER_PCT` (167).
    pub ranged_attack_power_total_pct: f32,
    /// C++ `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:341-379`):
    /// `Some(ActivePlayerData::OverrideAPBySpellPowerPercent)` while
    /// `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` is active. Both attack mods
    /// then read `CalculatePct(min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX]),
    /// percent)`; `None` keeps the strength/agility/level base.
    pub attack_power_override_by_spell_power_pct: Option<f32>,
    /// C++ `Player::UpdateSpellDamageAndHealingBonus` producers.
    pub spell_bonus: PlayerSpellBonusInputLikeCpp,
    pub rating_bonuses: [f32; 32],
    pub can_parry: bool,
    pub can_block: bool,
}

/// C++-shaped result of the represented `Player::UpdateAllStats` inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerStatSystemProjectionLikeCpp {
    pub stats: [i32; 5],
    pub stat_pos_buff: [i32; 5],
    pub stat_neg_buff: [i32; 5],
    pub create_health: i32,
    pub base_mana: i32,
    pub max_health: i64,
    pub max_mana: i64,
    pub armor: i32,
    pub attack_power: i32,
    pub attack_power_mod_pos: i32,
    /// C++ `UnitData::AttackPowerMultiplier` (`TOTAL_PCT - 1.0`).
    pub attack_power_multiplier: f32,
    pub ranged_attack_power: i32,
    pub ranged_attack_power_mod_pos: i32,
    /// C++ `UnitData::RangedAttackPowerMultiplier` (`TOTAL_PCT - 1.0`).
    pub ranged_attack_power_multiplier: f32,
    pub total_attack_power: i32,
    pub total_ranged_attack_power: i32,
    pub block_pct: f32,
    pub dodge_pct: f32,
    pub dodge_from_attr: f32,
    pub parry_pct: f32,
    pub parry_from_attr: f32,
    pub crit_pct: f32,
    pub ranged_crit_pct: f32,
    pub offhand_crit_pct: f32,
    pub spell_crit_pct: [f32; 7],
    /// C++ `ActivePlayerData::ModDamageDonePos[7]`; index 0 stays unwritten and
    /// each magic school is `max(SpellBaseDamageBonusDone(1 << school) - Neg, 0)`.
    pub mod_damage_done_pos: [i32; 7],
    /// C++ `ActivePlayerData::ModDamageDoneNeg[7]`.
    pub mod_damage_done_neg: [i32; 7],
    /// C++ `ActivePlayerData::ModHealingDonePos`.
    pub mod_healing_done_pos: i32,
    /// C++ `ActivePlayerData::ModDamageDonePercent[7]`
    /// (`SpellAuraEffects.cpp:4525-4548`).
    pub mod_damage_done_percent: [f32; 7],
    /// C++ `ActivePlayerData::ModHealingDonePercent`
    /// (`StatSystem.cpp:588-599`).
    pub mod_healing_done_percent: f32,
}

/// C++ `Unit::CalculateMinMaxDamage` for the represented player weapon
/// ranges. Item ranges replace the base MINDAMAGE/MAXDAMAGE values and the
/// attack-power term uses the equipped delay; an empty range retains the
/// unarmed 1/2 values and the two-second C++ default multiplier.
pub fn effective_weapon_damage_ranges_like_cpp(
    projection: PlayerStatSystemProjectionLikeCpp,
    weapon_damage: [[f32; 2]; 3],
    base_attack_time: [u32; 3],
) -> [[f32; 2]; 3] {
    let total_ap = projection.total_attack_power.max(0) as f32;
    let total_ranged_ap = projection.total_ranged_attack_power.max(0) as f32;
    std::array::from_fn(|index| {
        let attack =
            <wow_constants::WeaponAttackType as num_traits::FromPrimitive>::from_usize(index)
                .unwrap_or(wow_constants::WeaponAttackType::BaseAttack);
        let has_item_range = weapon_damage[index][0] > 0.0 && weapon_damage[index][1] > 0.0;
        if attack == wow_constants::WeaponAttackType::RangedAttack
            && !has_item_range
            && total_ranged_ap == 0.0
        {
            return [0.0, 0.0];
        }
        let attack_power = if attack == wow_constants::WeaponAttackType::RangedAttack {
            total_ranged_ap
        } else {
            total_ap
        };
        let attack_power_multiplier = if base_attack_time[index] > 0 {
            // C++ clamps `GetAPMultiplier` to 0.25 in
            // `Player::CalculateMinMaxDamage`, even when a malformed/custom
            // weapon delay is shorter than 250 ms.
            (base_attack_time[index] as f32 / 1000.0).max(0.25)
        } else {
            2.0
        };
        let [weapon_min, weapon_max] = if has_item_range {
            weapon_damage[index]
        } else {
            [1.0, 2.0]
        };
        let ap_component = attack_power / 14.0 * attack_power_multiplier;
        [
            (weapon_min + ap_component).max(1.0),
            (weapon_max + ap_component).max(1.0),
        ]
    })
}

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
pub fn calculate_player_stat_system_like_cpp(
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
    }
}

/// In-memory C++ player level information keyed by `(race, class, level)`.
pub struct PlayerStatsStore {
    stats: HashMap<(u8, u8, u8), PlayerLevelStats>,
}

impl PlayerStatsStore {
    /// Compose the exact C++ sources used by `ObjectMgr::LoadPlayerInfo`.
    pub fn load_from_validated_rows_like_cpp(
        data_dir: impl AsRef<Path>,
        max_player_level: u8,
        valid_race_classes: &[(u8, u8)],
        race_rows: PlayerRaceStatsRowsLikeCpp,
        class_rows: PlayerClassLevelStatsRowsLikeCpp,
    ) -> Result<Self> {
        let race_rows: Vec<_> = race_rows
            .0
            .into_iter()
            .map(|row| (row.race, row.stat_modifiers))
            .collect();

        let class_rows: Vec<_> = class_rows
            .0
            .into_iter()
            .map(|row| (row.class, row.level, row.primary_stats))
            .collect();

        let base_mp = BaseMpGameTableLikeCpp::load(data_dir)
            .context("Failed to load gt/BaseMp.txt for player class-level stats")?;
        let store = Self::from_cpp_sources(
            valid_race_classes.iter().copied(),
            race_rows,
            class_rows,
            &base_mp,
            max_player_level,
        )?;
        info!(
            "Loaded {} C++ player race/class/level stat entries",
            store.len()
        );
        Ok(store)
    }

    /// Build the same combined rows as C++ `ObjectMgr::LoadPlayerInfo`.
    ///
    /// Missing class-level rows after level 1 inherit the previous level.
    /// A class represented in the input without level-1 data is rejected,
    /// matching C++'s fatal integrity check for playable combinations.
    pub fn from_cpp_sources(
        valid_race_classes: impl IntoIterator<Item = (u8, u8)>,
        race_rows: impl IntoIterator<Item = (u8, [i16; 5])>,
        class_rows: impl IntoIterator<Item = (u8, u8, [u16; 5])>,
        base_mp: &BaseMpGameTableLikeCpp,
        max_player_level: u8,
    ) -> Result<Self> {
        if max_player_level == 0 {
            bail!("CONFIG_MAX_PLAYER_LEVEL must be at least 1");
        }

        let valid_race_classes: HashSet<(u8, u8)> = valid_race_classes.into_iter().collect();
        if valid_race_classes.is_empty() {
            bail!("playercreateinfo has no valid race/class combinations");
        }

        let race_modifiers: HashMap<u8, [i16; 5]> = race_rows.into_iter().collect();
        if race_modifiers.is_empty() {
            bail!("player_racestats is empty");
        }

        let mut class_level_stats = HashMap::new();
        for (class, level, primary_stats) in class_rows {
            if level == 0 || level > max_player_level {
                continue;
            }
            class_level_stats.insert((class, level), primary_stats);
        }
        if class_level_stats.is_empty() {
            bail!("player_classlevelstats has no rows within the configured level range");
        }

        let required_classes: HashSet<u8> =
            valid_race_classes.iter().map(|&(_, class)| class).collect();
        for &class in &required_classes {
            let Some(mut previous) = class_level_stats.get(&(class, 1)).copied() else {
                bail!("Class {class} level 1 does not have stats data");
            };
            for level in 2..=max_player_level {
                match class_level_stats.get(&(class, level)).copied() {
                    Some(stats) if stats[0] != 0 => previous = stats,
                    _ => {
                        class_level_stats.insert((class, level), previous);
                    }
                }
            }
        }

        let mut stats = HashMap::new();
        for (race, class) in valid_race_classes {
            let race_modifiers = race_modifiers.get(&race).copied().unwrap_or([0; 5]);
            for level in 1..=max_player_level {
                let Some(class_stats) = class_level_stats.get(&(class, level)) else {
                    continue;
                };
                let combined: [u16; 5] = std::array::from_fn(|index| {
                    // C++ assigns the promoted `uint16 + int16` result back
                    // to `uint16`. Valid world rows remain non-negative.
                    (i32::from(class_stats[index]) + i32::from(race_modifiers[index])) as u16
                });
                if level == 1 && combined[0] == 0 {
                    bail!("Race {race} Class {class} Level 1 does not have stats data");
                }
                stats.insert(
                    (race, class, level),
                    PlayerLevelStats {
                        strength: combined[0],
                        agility: combined[1],
                        stamina: combined[2],
                        intellect: combined[3],
                        spirit: combined[4],
                        base_mana: base_mp.base_mana_like_cpp(class, level).unwrap_or(0),
                    },
                );
            }
        }

        Ok(Self { stats })
    }

    pub fn get(&self, race: u8, class: u8, level: u8) -> Option<&PlayerLevelStats> {
        self.stats.get(&(race, class, level))
    }

    /// Test/fixture constructor for already-combined C++ rows.
    pub fn from_entries(
        entries: impl IntoIterator<Item = ((u8, u8, u8), PlayerLevelStats)>,
    ) -> Self {
        Self {
            stats: entries.into_iter().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.stats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stats.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BaseMpEntryLikeCpp;

    fn base_mp_fixture() -> BaseMpGameTableLikeCpp {
        BaseMpGameTableLikeCpp::from_rows([
            BaseMpEntryLikeCpp::from_columns([
                0.0, 31.0, 0.0, 155.0, 31.0, 155.0, 31.0, 155.0, 0.0, 0.0, 155.0, 0.0,
            ]),
            BaseMpEntryLikeCpp::from_columns([
                0.0, 34.0, 0.0, 170.0, 34.0, 170.0, 34.0, 170.0, 0.0, 0.0, 170.0, 0.0,
            ]),
        ])
    }

    #[test]
    fn combines_class_stats_race_modifiers_and_base_mp_like_cpp() {
        let store = PlayerStatsStore::from_cpp_sources(
            [(1, 5), (2, 5)],
            [(1, [3, -1, 2, 4, 0]), (2, [-2, 1, 0, -3, 5])],
            [(5, 1, [10, 11, 12, 13, 14])],
            &base_mp_fixture(),
            1,
        )
        .expect("valid C++ sources");

        assert_eq!(
            store.get(1, 5, 1),
            Some(&PlayerLevelStats {
                strength: 13,
                agility: 10,
                stamina: 14,
                intellect: 17,
                spirit: 14,
                base_mana: 155,
            })
        );
        assert_eq!(
            store.get(2, 5, 1).unwrap().primary_stats_like_cpp(),
            [8, 12, 12, 10, 19]
        );
    }

    #[test]
    fn fills_missing_class_level_from_previous_level_like_cpp() {
        let store = PlayerStatsStore::from_cpp_sources(
            [(1, 11)],
            [(1, [0; 5])],
            [(11, 1, [1, 2, 3, 4, 5]), (11, 3, [10, 20, 30, 40, 50])],
            &base_mp_fixture(),
            3,
        )
        .expect("valid gapped C++ sources");

        assert_eq!(
            store.get(1, 11, 2).unwrap().primary_stats_like_cpp(),
            [1, 2, 3, 4, 5]
        );
        assert_eq!(
            store.get(1, 11, 3).unwrap().primary_stats_like_cpp(),
            [10, 20, 30, 40, 50]
        );
        assert_eq!(store.get(1, 11, 2).unwrap().base_mana, 34);
        assert_eq!(store.get(1, 11, 3).unwrap().base_mana, 0);
    }

    #[test]
    fn rejects_class_without_level_one_like_cpp() {
        let error = PlayerStatsStore::from_cpp_sources(
            [(1, 5)],
            [(1, [0; 5])],
            [(5, 2, [10; 5])],
            &base_mp_fixture(),
            2,
        )
        .err()
        .expect("missing level 1 must fail");

        assert!(error.to_string().contains("level 1"));
    }

    #[test]
    fn only_builds_and_validates_playercreateinfo_combinations_like_cpp() {
        let store = PlayerStatsStore::from_cpp_sources(
            [(1, 5)],
            [(2, [50; 5])],
            [
                (5, 1, [10, 11, 12, 13, 14]),
                // This unused class has no level-1 row. C++ never allocates
                // levelInfo for it without a matching playercreateinfo row.
                (8, 2, [20; 5]),
            ],
            &base_mp_fixture(),
            2,
        )
        .expect("unused race/class combinations must not affect integrity");

        assert_eq!(
            store.get(1, 5, 1).unwrap().primary_stats_like_cpp(),
            [10, 11, 12, 13, 14],
            "a race missing from player_racestats uses C++ zero modifiers"
        );
        assert!(store.get(2, 5, 1).is_none());
        assert!(store.get(1, 8, 2).is_none());
    }

    #[test]
    fn stat_system_uses_create_health_zero_base_mp_and_chrclasses_ap_coefficients() {
        let projection = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 10,
                agility: 12,
                stamina: 30,
                intellect: 40,
                spirit: 20,
                base_mana: 155,
            },
            class: 5,
            level: 80,
            attack_power_per_strength: 0,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.0; 5],
            stat_buff_total_multipliers: [1.0; 5],
            gear_stats: [0, 0, 5, 3, 0],
            gear_health: 100,
            gear_mana: 50,
            gear_armor: 25,
            armor_base_pct: 1.0,
            armor_flat_aura: 0,
            armor_of_stat_percent: [0; 5],
            armor_total_pct: 1.0,
            armor_bonus_pct: 1.0,
            spell_dodge_pct: 0.0,
            spell_parry_pct: 0.0,
            spell_block_pct: 0.0,
            crit_mainhand_aura_pct: 0.0,
            crit_offhand_aura_pct: 0.0,
            crit_ranged_aura_pct: 0.0,
            spell_crit_aura_pct: 0.0,
            gear_attack_power: 17,
            gear_ranged_attack_power: 4,
            attack_power_flat_aura: 0,
            attack_power_total_pct: 1.0,
            ranged_attack_power_flat_aura: 0,
            ranged_attack_power_total_pct: 1.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
            rating_bonuses: [0.0; 32],
            can_parry: false,
            can_block: false,
        });

        assert_eq!(projection.create_health, 0);
        assert_eq!(projection.max_health, 100 + 20 + 15 * 10);
        assert_eq!(projection.base_mana, 155);
        assert_eq!(projection.max_mana, 155 + 50 + 20 + 23 * 15);
        assert_eq!(projection.armor, 12 * 2 + 25);
        assert_eq!(projection.attack_power, -20);
        assert_eq!(projection.attack_power_mod_pos, 17);
        // C++ `Unit::GetTotalAttackPowerValue` clamps the base plus modifier at
        // zero before the multiplier, so -20 + 17 yields zero.
        assert_eq!(projection.total_attack_power, 0);
        assert_eq!(projection.attack_power_multiplier, 0.0);
        assert_eq!(projection.ranged_attack_power, -10);
        assert_eq!(projection.ranged_attack_power_mod_pos, 21);
        assert_eq!(projection.total_ranged_attack_power, 11);
        assert_eq!(projection.ranged_attack_power_multiplier, 0.0);
    }

    #[test]
    fn stat_system_applies_cpp_attack_power_aura_producers_like_cpp() {
        // C++ `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:333-403`)
        // and `Unit::GetTotalAttackPowerValue`: gear plus the
        // `MOD_ATTACK_POWER`/`MOD_RANGED_ATTACK_POWER` flats form the modifier,
        // the `..._PCT` auras form `TOTAL_PCT - 1.0`, and the total clamps the
        // base plus modifier at zero before multiplying.
        let input = PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 10,
                agility: 10,
                stamina: 10,
                intellect: 40,
                spirit: 30,
                base_mana: 0,
            },
            class: 1,
            level: 80,
            attack_power_per_strength: 2,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.0; 5],
            stat_buff_total_multipliers: [1.0; 5],
            gear_stats: [0; 5],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 0,
            armor_base_pct: 1.0,
            armor_flat_aura: 0,
            armor_of_stat_percent: [0; 5],
            armor_total_pct: 1.0,
            armor_bonus_pct: 1.0,
            spell_dodge_pct: 0.0,
            spell_parry_pct: 0.0,
            spell_block_pct: 0.0,
            crit_mainhand_aura_pct: 0.0,
            crit_offhand_aura_pct: 0.0,
            crit_ranged_aura_pct: 0.0,
            spell_crit_aura_pct: 0.0,
            gear_attack_power: 50,
            gear_ranged_attack_power: 20,
            attack_power_flat_aura: 100,
            attack_power_total_pct: 1.5,
            ranged_attack_power_flat_aura: 40,
            ranged_attack_power_total_pct: 2.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
            rating_bonuses: [0.0; 32],
            can_parry: false,
            can_block: false,
        };
        let projection = calculate_player_stat_system_like_cpp(input);

        // Class 1 (warrior): 10 Strength * 2 + (80 * 3 - 20) = 240 base AP,
        // and (80 + 10) * 0 - 10 = -10 ranged.
        assert_eq!(projection.attack_power, 240);
        assert_eq!(projection.attack_power_mod_pos, 150);
        assert_eq!(projection.attack_power_multiplier, 0.5);
        assert_eq!(projection.total_attack_power, 585);
        assert_eq!(projection.ranged_attack_power, -10);
        assert_eq!(projection.ranged_attack_power_mod_pos, 110);
        assert_eq!(projection.ranged_attack_power_multiplier, 1.0);
        assert_eq!(projection.total_ranged_attack_power, 200);
    }

    #[test]
    fn stat_system_applies_cpp_armor_aura_producers_order_like_cpp() {
        // C++ `Player::UpdateArmor` (`StatSystem.cpp:251-276`): item BASE_VALUE
        // scaled by BASE_PCT, plus agility, plus the aura flat TOTAL_VALUE and
        // `MOD_RESISTANCE_OF_STAT_PERCENT` terms, then TOTAL_PCT and
        // `MOD_BONUS_ARMOR_PCT`, truncated by `int32(value)`.
        let projection = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 10,
                agility: 12,
                stamina: 30,
                intellect: 40,
                spirit: 20,
                base_mana: 155,
            },
            class: 5,
            level: 80,
            attack_power_per_strength: 0,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.0; 5],
            stat_buff_total_multipliers: [1.0; 5],
            gear_stats: [0; 5],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 100,
            armor_base_pct: 1.5,
            armor_flat_aura: 40,
            // 50% of the final Agility (12) is added before the multipliers.
            armor_of_stat_percent: [0, 50, 0, 0, 0],
            armor_total_pct: 1.25,
            armor_bonus_pct: 1.1,
            spell_dodge_pct: 0.0,
            spell_parry_pct: 0.0,
            spell_block_pct: 0.0,
            crit_mainhand_aura_pct: 0.0,
            crit_offhand_aura_pct: 0.0,
            crit_ranged_aura_pct: 0.0,
            spell_crit_aura_pct: 0.0,
            gear_attack_power: 0,
            gear_ranged_attack_power: 0,
            attack_power_flat_aura: 0,
            attack_power_total_pct: 1.0,
            ranged_attack_power_flat_aura: 0,
            ranged_attack_power_total_pct: 1.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
            rating_bonuses: [0.0; 32],
            can_parry: false,
            can_block: false,
        });

        // ((100 * 1.5) + 12 * 2 + 40 + 12 * 50 / 100) * 1.25 * 1.1 = 302.5
        assert_eq!(projection.armor, 302);
    }

    #[test]
    fn stat_system_applies_cpp_avoidance_aura_percentages_like_cpp() {
        // C++ `Player::UpdateBlockPercentage`/`UpdateParryPercentage`/
        // `UpdateDodgePercentage` (`StatSystem.cpp:483-499`, `659-679`,
        // `700-717`): the aura `GetTotalAuraModifier` terms are flat
        // percentages added to the non-diminishing side.
        let warrior = PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 10,
                agility: 12,
                stamina: 30,
                intellect: 40,
                spirit: 20,
                base_mana: 0,
            },
            class: 1,
            level: 80,
            attack_power_per_strength: 2,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.0; 5],
            stat_buff_total_multipliers: [1.0; 5],
            gear_stats: [0; 5],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 0,
            armor_base_pct: 1.0,
            armor_flat_aura: 0,
            armor_of_stat_percent: [0; 5],
            armor_total_pct: 1.0,
            armor_bonus_pct: 1.0,
            spell_dodge_pct: 10.0,
            spell_parry_pct: 3.0,
            spell_block_pct: 7.0,
            crit_mainhand_aura_pct: 0.0,
            crit_offhand_aura_pct: 0.0,
            crit_ranged_aura_pct: 0.0,
            spell_crit_aura_pct: 0.0,
            gear_attack_power: 0,
            gear_ranged_attack_power: 0,
            attack_power_flat_aura: 0,
            attack_power_total_pct: 1.0,
            ranged_attack_power_flat_aura: 0,
            ranged_attack_power_total_pct: 1.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
            rating_bonuses: [0.0; 32],
            can_parry: true,
            can_block: true,
        };
        let projection = calculate_player_stat_system_like_cpp(warrior);

        // With no rating bonus the diminishing term is zero, so the result is
        // exactly the non-diminishing side.
        assert_eq!(projection.dodge_pct, 10.0);
        assert_eq!(projection.parry_pct, 8.0);
        assert_eq!(projection.block_pct, 12.0);

        // A class whose parry cap is zero keeps parry at zero even with the
        // aura and `can_parry` set, matching `UpdateParryPercentage`.
        let priest = PlayerStatSystemInputLikeCpp {
            class: 5,
            can_block: false,
            ..warrior
        };
        let projection = calculate_player_stat_system_like_cpp(priest);
        assert_eq!(projection.parry_pct, 0.0);
    }

    #[test]
    fn stat_system_applies_cpp_crit_aura_percentages_like_cpp() {
        // C++ `Player::UpdateAllCritPercentages`/`UpdateCritPercentage`
        // (`StatSystem.cpp:502-538`) and `UpdateSpellCritChance` (`718-731`):
        // every group starts at 5% and adds its own aura flat sum.
        let input = PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 10,
                agility: 12,
                stamina: 30,
                intellect: 40,
                spirit: 20,
                base_mana: 0,
            },
            class: 1,
            level: 80,
            attack_power_per_strength: 2,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.0; 5],
            stat_buff_total_multipliers: [1.0; 5],
            gear_stats: [0; 5],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 0,
            armor_base_pct: 1.0,
            armor_flat_aura: 0,
            armor_of_stat_percent: [0; 5],
            armor_total_pct: 1.0,
            armor_bonus_pct: 1.0,
            spell_dodge_pct: 0.0,
            spell_parry_pct: 0.0,
            spell_block_pct: 0.0,
            crit_mainhand_aura_pct: 2.0,
            crit_offhand_aura_pct: 3.0,
            crit_ranged_aura_pct: 4.0,
            spell_crit_aura_pct: 5.0,
            gear_attack_power: 0,
            gear_ranged_attack_power: 0,
            attack_power_flat_aura: 0,
            attack_power_total_pct: 1.0,
            ranged_attack_power_flat_aura: 0,
            ranged_attack_power_total_pct: 1.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
            rating_bonuses: [0.0; 32],
            can_parry: false,
            can_block: false,
        };
        let projection = calculate_player_stat_system_like_cpp(input);

        assert_eq!(projection.crit_pct, 7.0);
        assert_eq!(projection.offhand_crit_pct, 8.0);
        assert_eq!(projection.ranged_crit_pct, 9.0);
        assert_eq!(projection.spell_crit_pct, [10.0; 7]);
    }

    #[test]
    fn stat_system_uses_cpp_rating_and_diminishing_return_branches() {
        let mut rating_bonuses = [0.0; 32];
        rating_bonuses[2] = 10.0;
        rating_bonuses[3] = 10.0;
        rating_bonuses[4] = 2.0;
        rating_bonuses[8] = 3.0;
        rating_bonuses[9] = 4.0;
        rating_bonuses[10] = 5.0;

        let projection = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats::default(),
            class: 1,
            level: 80,
            attack_power_per_strength: 2,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.0; 5],
            stat_buff_total_multipliers: [1.0; 5],
            gear_stats: [0; 5],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 0,
            armor_base_pct: 1.0,
            armor_flat_aura: 0,
            armor_of_stat_percent: [0; 5],
            armor_total_pct: 1.0,
            armor_bonus_pct: 1.0,
            spell_dodge_pct: 0.0,
            spell_parry_pct: 0.0,
            spell_block_pct: 0.0,
            crit_mainhand_aura_pct: 0.0,
            crit_offhand_aura_pct: 0.0,
            crit_ranged_aura_pct: 0.0,
            spell_crit_aura_pct: 0.0,
            gear_attack_power: 0,
            gear_ranged_attack_power: 0,
            attack_power_flat_aura: 0,
            attack_power_total_pct: 1.0,
            ranged_attack_power_flat_aura: 0,
            ranged_attack_power_total_pct: 1.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
            rating_bonuses,
            can_parry: true,
            can_block: true,
        });

        let expected_dodge = 65.631440 * 10.0 / (10.0 + 65.631440 * 0.9560);
        let expected_parry = expected_dodge + 5.0;
        assert!((projection.dodge_pct - expected_dodge).abs() < 0.00001);
        assert!((projection.parry_pct - expected_parry).abs() < 0.00001);
        assert_eq!(projection.block_pct, 7.0);
        assert_eq!(projection.crit_pct, 8.0);
        assert_eq!(projection.ranged_crit_pct, 9.0);
        assert_eq!(projection.spell_crit_pct, [10.0; 7]);
        assert_eq!(projection.dodge_from_attr, 0.0);
        assert_eq!(projection.parry_from_attr, 0.0);
    }

    #[test]
    fn stat_system_applies_total_stat_percentage_before_dependent_stats_like_cpp() {
        let projection = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 173,
                agility: 90,
                stamina: 160,
                intellect: 98,
                spirit: 108,
                base_mana: 4_880,
            },
            class: 2,
            level: 80,
            attack_power_per_strength: 2,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.03; 5],
            stat_buff_total_multipliers: [1.03; 5],
            gear_stats: [0; 5],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 0,
            armor_base_pct: 1.0,
            armor_flat_aura: 0,
            armor_of_stat_percent: [0; 5],
            armor_total_pct: 1.0,
            armor_bonus_pct: 1.0,
            spell_dodge_pct: 0.0,
            spell_parry_pct: 0.0,
            spell_block_pct: 0.0,
            crit_mainhand_aura_pct: 0.0,
            crit_offhand_aura_pct: 0.0,
            crit_ranged_aura_pct: 0.0,
            spell_crit_aura_pct: 0.0,
            gear_attack_power: 0,
            gear_ranged_attack_power: 0,
            attack_power_flat_aura: 0,
            attack_power_total_pct: 1.0,
            ranged_attack_power_flat_aura: 0,
            ranged_attack_power_total_pct: 1.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
            rating_bonuses: [0.0; 32],
            can_parry: false,
            can_block: false,
        });

        assert_eq!(projection.stats, [178, 92, 164, 100, 111]);
        assert_eq!(projection.max_health, 1_460);
        assert_eq!(projection.max_mana, 6_100);
        assert_eq!(projection.armor, 184);
        assert_eq!(projection.attack_power, 576);
    }

    #[test]
    fn stat_system_scales_positive_and_negative_client_buffs_like_cpp() {
        let projection = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 10,
                agility: 200,
                stamina: 10,
                intellect: 10,
                spirit: 10,
                base_mana: 0,
            },
            class: 1,
            level: 1,
            attack_power_per_strength: 2,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.03; 5],
            stat_buff_total_multipliers: [1.03; 5],
            gear_stats: [100, -100, 0, 0, 0],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 0,
            armor_base_pct: 1.0,
            armor_flat_aura: 0,
            armor_of_stat_percent: [0; 5],
            armor_total_pct: 1.0,
            armor_bonus_pct: 1.0,
            spell_dodge_pct: 0.0,
            spell_parry_pct: 0.0,
            spell_block_pct: 0.0,
            crit_mainhand_aura_pct: 0.0,
            crit_offhand_aura_pct: 0.0,
            crit_ranged_aura_pct: 0.0,
            spell_crit_aura_pct: 0.0,
            gear_attack_power: 0,
            gear_ranged_attack_power: 0,
            attack_power_flat_aura: 0,
            attack_power_total_pct: 1.0,
            ranged_attack_power_flat_aura: 0,
            ranged_attack_power_total_pct: 1.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
            rating_bonuses: [0.0; 32],
            can_parry: false,
            can_block: false,
        });

        assert_eq!(projection.stats[..2], [113, 103]);
        assert_eq!(projection.stat_pos_buff[..2], [103, 0]);
        assert_eq!(projection.stat_neg_buff[..2], [0, -103]);
    }

    #[test]
    fn stat_system_overrides_attack_power_by_spell_power_like_cpp() {
        let input = PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 10,
                agility: 10,
                stamina: 10,
                intellect: 10,
                spirit: 10,
                base_mana: 0,
            },
            class: 1,
            level: 80,
            attack_power_per_strength: 0,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.0; 5],
            stat_buff_total_multipliers: [1.0; 5],
            gear_stats: [0; 5],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 0,
            armor_base_pct: 1.0,
            armor_flat_aura: 0,
            armor_of_stat_percent: [0; 5],
            armor_total_pct: 1.0,
            armor_bonus_pct: 1.0,
            spell_dodge_pct: 0.0,
            spell_parry_pct: 0.0,
            spell_block_pct: 0.0,
            crit_mainhand_aura_pct: 0.0,
            crit_offhand_aura_pct: 0.0,
            crit_ranged_aura_pct: 0.0,
            spell_crit_aura_pct: 0.0,
            gear_attack_power: 0,
            gear_ranged_attack_power: 0,
            attack_power_flat_aura: 0,
            attack_power_total_pct: 1.0,
            ranged_attack_power_flat_aura: 0,
            ranged_attack_power_total_pct: 1.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
            rating_bonuses: [0.0; 32],
            can_parry: false,
            can_block: false,
        };
        let baseline = calculate_player_stat_system_like_cpp(input);
        assert_eq!(baseline.attack_power, 220);
        assert_eq!(baseline.ranged_attack_power, -10);

        // C++ `CalculatePct(1234.0f, 12.5f)` = 154.25, truncated by the
        // `int32(base_attPower)` store; both attack mods share the base.
        let overridden = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            attack_power_override_by_spell_power_pct: Some(12.5),
            spell_bonus: PlayerSpellBonusInputLikeCpp {
                base_spell_power: 1_234,
                ..Default::default()
            },
            ..input
        });
        assert_eq!(overridden.attack_power, 154);
        assert_eq!(overridden.ranged_attack_power, 154);

        // `HasAuraType` presence alone overrides: an active effect whose summed
        // percent is zero yields `CalculatePct(spellPower, 0.0) == 0`.
        let zeroed = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            attack_power_override_by_spell_power_pct: Some(0.0),
            spell_bonus: PlayerSpellBonusInputLikeCpp {
                base_spell_power: 1_234,
                ..Default::default()
            },
            ..input
        });
        assert_eq!(zeroed.attack_power, 0);
        assert_eq!(zeroed.ranged_attack_power, 0);
    }

    #[test]
    fn stat_system_publishes_spell_damage_and_healing_bonuses_like_cpp() {
        let input = PlayerStatSystemInputLikeCpp {
            base: PlayerLevelStats {
                strength: 10,
                agility: 10,
                stamina: 10,
                intellect: 40,
                spirit: 30,
                base_mana: 1_000,
            },
            class: 5,
            level: 80,
            attack_power_per_strength: 0,
            attack_power_per_agility: 0,
            ranged_attack_power_per_agility: 0,
            stat_total_multipliers: [1.0; 5],
            stat_buff_total_multipliers: [1.0; 5],
            gear_stats: [0; 5],
            gear_health: 0,
            gear_mana: 0,
            gear_armor: 0,
            armor_base_pct: 1.0,
            armor_flat_aura: 0,
            armor_of_stat_percent: [0; 5],
            armor_total_pct: 1.0,
            armor_bonus_pct: 1.0,
            spell_dodge_pct: 0.0,
            spell_parry_pct: 0.0,
            spell_block_pct: 0.0,
            crit_mainhand_aura_pct: 0.0,
            crit_offhand_aura_pct: 0.0,
            crit_ranged_aura_pct: 0.0,
            spell_crit_aura_pct: 0.0,
            gear_attack_power: 0,
            gear_ranged_attack_power: 0,
            attack_power_flat_aura: 0,
            attack_power_total_pct: 1.0,
            ranged_attack_power_flat_aura: 0,
            ranged_attack_power_total_pct: 1.0,
            attack_power_override_by_spell_power_pct: None,
            spell_bonus: PlayerSpellBonusInputLikeCpp {
                base_spell_power: 100,
                // School 1 (holy) has a +30 aura; school 2 (fire) has a +20
                // aura and a -50 aura, so the C++ net sum is -30 while the
                // negative field is -50.
                damage_done_flat: [0, 30, -30, 0, 0, 0, 0],
                damage_done_neg: [0, 0, -50, 0, 0, 0, 0],
                damage_of_stat_percent: [[0; 5]; 7],
                healing_done_flat: 40,
                healing_of_stat_percent: [0; 5],
                override_spell_power_by_ap_pct: 0.0,
                // School 1 has (1 + 0.5) * (1 + 1.0) = 3.0, school 2 1.25.
                damage_done_percent: [1.0, 3.0, 1.25, 1.0, 1.0, 1.0, 1.0],
                // `UpdateHealingDonePercentMod` starts from 1.0.
                healing_done_percent: 2.0,
            },
            rating_bonuses: [0.0; 32],
            can_parry: false,
            can_block: false,
        };
        let projection = calculate_player_stat_system_like_cpp(input);
        assert_eq!(projection.mod_damage_done_pos[0], 0);
        // Holy: 100 base + 30 aura.
        assert_eq!(projection.mod_damage_done_pos[1], 130);
        // Fire: (100 - 30) - (-50) leaves the +20 aura.
        assert_eq!(projection.mod_damage_done_pos[2], 120);
        assert_eq!(projection.mod_damage_done_neg[2], -50);
        // Healing: 100 base + 40 aura + max(0, intellect 40).
        assert_eq!(projection.mod_healing_done_pos, 180);
        assert_eq!(
            projection.mod_damage_done_percent,
            [1.0, 3.0, 1.25, 1.0, 1.0, 1.0, 1.0]
        );
        assert_eq!(projection.mod_healing_done_percent, 2.0);

        // `SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` replaces both bonuses with
        // `int32(CalculatePct(GetTotalAttackPowerValue(BASE_ATTACK), pct) + 0.5)`:
        // melee AP is `max(0, -20 + 500) = 480`, so 50% rounds to 240.
        let overridden = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
            attack_power_flat_aura: 500,
            spell_bonus: PlayerSpellBonusInputLikeCpp {
                override_spell_power_by_ap_pct: 50.0,
                ..input.spell_bonus
            },
            ..input
        });
        assert_eq!(overridden.mod_damage_done_pos[1], 240);
        assert_eq!(overridden.mod_damage_done_pos[2], 290);
        assert_eq!(overridden.mod_healing_done_pos, 240);
    }
}
