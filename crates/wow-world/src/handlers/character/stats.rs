// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character equipment aggregation and effective stat publication.
//!
//! This module owns the application-side projection from equipped item data and
//! Player-owned item modifiers into the pure `wow-data` stat calculation. The
//! resulting snapshot is published to the canonical Player; packet formatting
//! remains in the character handler adapters.

use super::*;

/// C++ `SPELL_SCHOOL_MASK_NORMAL` (`SharedDefines.h:329`).
const SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP: i32 = 1;
/// C++ `SPELL_SCHOOL_MASK_ALL` (`SharedDefines.h:335`): the seven school bits
/// `SpellBaseHealingBonusDone` uses for `ModHealingDonePos`.
const SPELL_SCHOOL_MASK_ALL_LIKE_CPP: i32 = 0x7F;

/// C++ `CLASSMASK_WAND_USERS` (`SharedDefines.h:190`): the priest, mage and
/// warlock classes ignore the ranged attack power aura producers
/// (`HandleAuraModRangedAttackPower`).
fn class_uses_wands_like_cpp(class: u8) -> bool {
    matches!(class, 5 | 8 | 9)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RepresentedPlayerGearStatsLikeCpp {
    pub(super) stats: [i32; 5],
    pub(super) attack_power: i32,
    pub(super) ranged_attack_power: i32,
    pub(super) health: i32,
    pub(super) mana: i32,
    pub(super) combat_ratings: [i32; 32],
    pub(super) spell_power: i32,
    pub(super) armor: i32,
    pub(super) resistances: [i32; 7],
    pub(super) mana_regen_bonus: i32,
    pub(super) health_regen_bonus: i32,
    pub(super) spell_penetration_bonus: i32,
    pub(super) shield_block_base_mod: i32,
    pub(super) shield_block_value: u32,
    /// C++ `UnitData` weapon ranges installed by `_ApplyWeaponDamage`.
    pub(super) weapon_damage: [[f32; 2]; 3],
    pub(super) base_attack_time: [u32; 3],
}

impl WorldSession {
    pub(super) fn mana_regen_from_stats_like_cpp(
        &self,
        level: u8,
        class: u8,
        stats: [i32; 5],
    ) -> f32 {
        // C++ `Player::OCTRegenMPPerSpirit` returns Spirit multiplied by the
        // level/class row from `RegenMPPerSpt.txt`; `UpdateManaRegen` then
        // multiplies that value by sqrt(Intellect). Aura percentages are a
        // separate producer and are intentionally not folded into this
        // equipment projection.
        (stats[3].max(0) as f32).sqrt()
            * stats[4] as f32
            * self.mana_regen_ratio_like_cpp(level, class)
    }

    pub(super) fn mana_regen_aura_multiplier_like_cpp(&self) -> f32 {
        let mana = PowerType::Mana as i32;
        self.resolved_total_aura_multiplier_by_spell_aura_type_and_misc_value_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN_PERCENT,
            mana,
        )
        .unwrap_or(1.0)
            * self
                .resolved_total_aura_multiplier_by_spell_aura_type_and_misc_value_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_PCT,
                    mana,
                )
                .unwrap_or(1.0)
    }

    pub(super) fn mana_regen_mp5_from_auras_like_cpp(&self, stats: [i32; 5]) -> f32 {
        let mana = PowerType::Mana as i32;
        let flat = self
            .resolved_total_aura_modifier_by_spell_aura_type_and_misc_value_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN,
                mana,
            )
            .unwrap_or(0) as f32
            / 5.0;
        let from_stat = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_FROM_STAT,
            )
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(stat_index, amount)| {
                usize::try_from(stat_index)
                    .ok()
                    .and_then(|index| stats.get(index).copied())
                    .map(|stat| stat as f32 * amount as f32 / 500.0)
            })
            .sum::<f32>();
        flat + from_stat
    }

    pub(super) fn mana_regen_interrupt_modifier_like_cpp(&self) -> f32 {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_INTERRUPT,
        )
        .unwrap_or_default()
        .into_iter()
        .map(|(_, amount)| amount)
        .sum::<i32>()
        .min(100) as f32
            / 100.0
    }

    /// C++ `GetTotalAuraMultiplierByMiscMask(aura_type, school_mask)` as used
    /// by `Player::UpdateArmor` (`StatSystem.cpp:251-276`) and
    /// `Unit::UpdateResistances` (`Unit.cpp:9148-9163`): the product of
    /// `1 + amount/100` over the active effects whose school mask intersects
    /// `school_mask`.
    fn represented_resistance_aura_multiplier_like_cpp(
        &self,
        aura_type: i32,
        school_mask: i32,
    ) -> f32 {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .unwrap_or_default()
            .into_iter()
            .filter(|(misc_value, _)| *misc_value & school_mask != 0)
            .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
    }

    /// C++ `GetTotalAuraMultiplier(aura_type)` without a school mask.
    fn represented_total_aura_multiplier_like_cpp(&self, aura_type: i32) -> f32 {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .unwrap_or_default()
            .into_iter()
            .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
    }

    /// C++ `ActivePlayerData::OverrideAPBySpellPowerPercent` from
    /// `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT`
    /// (`SpellAuraDefines.h:499`, `SpellAuraEffects.cpp:3785-3796`): every
    /// active effect amount is summed through `ApplyModUpdateFieldValue`.
    ///
    /// `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:341`) tests
    /// `HasAuraType`, so an active effect whose summed amount is zero still
    /// replaces the base.
    fn represented_override_attack_power_by_spell_power_pct_like_cpp(&self) -> Option<f32> {
        let effects = self.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
        )?;
        if effects.is_empty() {
            return None;
        }
        Some(
            effects
                .into_iter()
                .map(|(_, amount)| amount as f32)
                .sum::<f32>(),
        )
    }

    /// C++ `Player::UpdateSpellDamageAndHealingBonus` producers
    /// (`StatSystem.cpp:171-197`) from `Unit::SpellBaseDamageBonusDone`
    /// (`Unit.cpp:6860-6890`) and `Unit::SpellBaseHealingBonusDone`
    /// (`Unit.cpp:7282-7315`).
    ///
    /// `GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE, mask)` keeps
    /// the full per-school sum (negative amounts included) while
    /// `ModDamageDoneNeg` accumulates only the negative part, so the pure
    /// stat system can reproduce C++'s `Pos = bonus - Neg`.
    fn represented_spell_bonus_like_cpp(
        &self,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> PlayerSpellBonusInputLikeCpp {
        let mut damage_done_flat = [0i32; 7];
        let mut damage_done_neg = [0i32; 7];
        for (misc_value, amount) in self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
            )
            .unwrap_or_default()
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
        for (school_mask, stat_index, amount) in self
            .resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT,
            )
            .unwrap_or_default()
        {
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

        let healing_done_flat = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE,
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|(misc_value, _)| {
                *misc_value == 0 || (*misc_value & SPELL_SCHOOL_MASK_ALL_LIKE_CPP) != 0
            })
            .map(|(_, amount)| amount)
            .sum::<i32>();

        let mut healing_of_stat_percent = [0i32; 5];
        for (stat_index, _, amount) in self
            .resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT,
            )
            .unwrap_or_default()
        {
            if let Ok(stat) = usize::try_from(stat_index)
                && let Some(value) = healing_of_stat_percent.get_mut(stat)
            {
                *value = value.saturating_add(amount);
            }
        }

        // C++ `AuraEffect::HandleModDamagePercentDone`
        // (`SpellAuraEffects.cpp:4525-4548`) sets `ModDamageDonePercent[i]` to
        // `GetTotalAuraMultiplierByMiscMask(SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
        // 1 << i)` for every school the handling effect intersects; a school
        // with no such effect keeps the 1.0 create value.
        let damage_percent_effects = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            )
            .unwrap_or_default();
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

        // C++ `Player::UpdateHealingDonePercentMod` (`StatSystem.cpp:588-599`)
        // multiplies `1 + amount/100` over every active
        // `SPELL_AURA_MOD_HEALING_DONE_PERCENT` (136) effect and clamps the
        // published `ModHealingDonePercent` at zero.
        let healing_done_percent = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE_PERCENT,
            )
            .unwrap_or_default()
            .into_iter()
            .fold(1.0_f32, |acc, (_, amount)| {
                acc * (1.0 + amount as f32 / 100.0)
            })
            .max(0.0);

        let override_effects = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT,
            )
            .unwrap_or_default();
        // C++ `Player::ApplySpellPowerBonus` returns early while the override
        // aura is present, so the item accumulator never reaches
        // `m_baseSpellPower`.
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
        }
    }

    /// C++ `GetFlatModifierValue(UNIT_MOD_ATTACK_POWER, TOTAL_VALUE)` from the
    /// `SPELL_AURA_MOD_ATTACK_POWER` producers
    /// (`HandleAuraModAttackPower`).
    fn represented_attack_power_flat_aura_like_cpp(&self) -> i32 {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACK_POWER,
        )
        .unwrap_or_default()
        .into_iter()
        .map(|(_, amount)| amount)
        .sum()
    }

    /// C++ `GetFlatModifierValue(UNIT_MOD_ATTACK_POWER_RANGED, TOTAL_VALUE)`
    /// from `SPELL_AURA_MOD_RANGED_ATTACK_POWER`, skipped for
    /// `CLASSMASK_WAND_USERS` (`HandleAuraModRangedAttackPower`).
    fn represented_ranged_attack_power_flat_aura_like_cpp(&self, class: u8) -> i32 {
        if class_uses_wands_like_cpp(class) {
            return 0;
        }
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_RANGED_ATTACK_POWER,
        )
        .unwrap_or_default()
        .into_iter()
        .map(|(_, amount)| amount)
        .sum()
    }

    /// C++ `GetPctModifierValue(UNIT_MOD_ATTACK_POWER_RANGED, TOTAL_PCT)` from
    /// `SPELL_AURA_MOD_RANGED_ATTACK_POWER_PCT`, skipped for wand users.
    fn represented_ranged_attack_power_total_pct_like_cpp(&self, class: u8) -> f32 {
        if class_uses_wands_like_cpp(class) {
            return 1.0;
        }
        self.represented_total_aura_multiplier_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_RANGED_ATTACK_POWER_PCT,
        )
    }

    /// C++ `GetFlatModifierValue(UNIT_MOD_RESISTANCE_START + school, TOTAL_VALUE)`
    /// from `SPELL_AURA_MOD_RESISTANCE`/`SPELL_AURA_MOD_BASE_RESISTANCE`
    /// effects carrying `school_mask`.
    fn represented_resistance_aura_flat_like_cpp(&self, school_mask: i32) -> f32 {
        [
            wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE,
            wow_data::spell::aura_types::SPELL_AURA_MOD_BASE_RESISTANCE,
        ]
        .into_iter()
        .filter_map(|aura_type| self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type))
        .flatten()
        .filter(|(misc_value, _)| *misc_value & school_mask != 0)
        .map(|(_, amount)| amount as f32)
        .sum()
    }

    /// C++ `Unit::UpdateResistances` (`Unit.cpp:9148-9163`) for the six magic
    /// schools, indexed as `UnitData::Resistances[1..7]` (holy, fire, nature,
    /// frost, shadow, arcane): the item `BASE_VALUE` scaled by the
    /// `MOD_BASE_RESISTANCE_PCT` `BASE_PCT`, plus the flat
    /// `MOD_RESISTANCE`/`MOD_BASE_RESISTANCE` `TOTAL_VALUE`, then the
    /// `MOD_RESISTANCE_PCT` `TOTAL_PCT`, truncated by `int32(value)`. The
    /// physical school (index 0) is `Player::UpdateArmor` and stays on
    /// `base_armor`.
    pub(super) fn represented_school_resistances_like_cpp(
        &self,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> [i32; 6] {
        std::array::from_fn(|index| {
            let school = index + 1;
            let mask = 1_i32 << school;
            let mut value = gear.resistances[school] as f32
                * self.represented_resistance_aura_multiplier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_BASE_RESISTANCE_PCT,
                    mask,
                );
            value += self.represented_resistance_aura_flat_like_cpp(mask);
            value *= self.represented_resistance_aura_multiplier_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE_PCT,
                mask,
            );
            value as i32
        })
    }

    /// C++ `Player::UpdateArmor`'s `SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT`
    /// loop, aggregated per `MiscValueB` stat index so the stat system can apply
    /// `CalculatePct` against the final stat values.
    fn represented_armor_of_stat_percent_like_cpp(&self) -> [i32; 5] {
        let mut per_stat = [0i32; 5];
        for (school_mask, stat_index, amount) in self
            .resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT,
            )
            .unwrap_or_default()
        {
            if school_mask & SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP == 0 {
                continue;
            }
            if let Ok(index) = usize::try_from(stat_index)
                && index < per_stat.len()
            {
                per_stat[index] = per_stat[index].saturating_add(amount);
            }
        }
        per_stat
    }

    /// C++ `GetTotalAuraModifier(auraType)` for the avoidance percentages
    /// (`Player::UpdateBlockPercentage`/`UpdateParryPercentage`/
    /// `UpdateDodgePercentage`): the sum of every active effect amount.
    fn represented_total_aura_modifier_like_cpp(&self, aura_type: i32) -> f32 {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, amount)| amount)
            .sum::<i32>() as f32
    }

    pub(super) fn represented_player_gear_stats_like_cpp(
        &self,
        _include_represented_item_bonuses: bool,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        let mut gear = RepresentedPlayerGearStatsLikeCpp::default();
        // C++ keeps the result of `_ApplyItemBonuses` on Player and uses that
        // accumulator for every subsequent stat calculation. Reading the
        // inventory here as well would count an item once through its sparse
        // row and again through the canonical Player modifier state after an
        // equip/swap. Login seeds the same accumulator before this projection,
        // so this is the single contribution path for every lifecycle.
        let bonuses = self.resolved_item_bonus_state_like_cpp()?;
        for (target, amount) in gear.stats.iter_mut().zip(bonuses.stats_base) {
            *target = target.saturating_add(amount);
        }
        gear.attack_power = gear.attack_power.saturating_add(bonuses.attack_power_total);
        gear.ranged_attack_power = gear
            .ranged_attack_power
            .saturating_add(bonuses.ranged_attack_power_total);
        gear.health = gear.health.saturating_add(bonuses.health_base);
        gear.mana = gear.mana.saturating_add(bonuses.mana_base);
        for (target, amount) in gear.combat_ratings.iter_mut().zip(bonuses.combat_ratings) {
            *target = target.saturating_add(amount);
        }
        gear.spell_power = gear.spell_power.saturating_add(bonuses.spell_power_bonus);
        gear.armor = gear
            .armor
            .saturating_add(bonuses.armor_base)
            .saturating_add(bonuses.armor_total)
            .saturating_add(bonuses.resistances_base[0]);
        for (target, amount) in gear.resistances.iter_mut().zip(bonuses.resistances_base) {
            *target = target.saturating_add(amount);
        }
        gear.mana_regen_bonus = bonuses.mana_regen_bonus;
        gear.health_regen_bonus = bonuses.health_regen_bonus;
        gear.spell_penetration_bonus = bonuses.spell_penetration_bonus;
        gear.shield_block_base_mod = bonuses.shield_block_base_mod;
        gear.shield_block_value = bonuses.shield_block_value;
        gear.weapon_damage = bonuses.weapon_damage;
        gear.base_attack_time = bonuses.base_attack_time;

        Some(gear)
    }

    pub(super) fn player_stat_system_projection_like_cpp(
        &self,
        race: u8,
        class: u8,
        level: u8,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> Option<PlayerStatSystemProjectionLikeCpp> {
        let base = *self.player_stats()?.get(race, class, level)?;
        let (attack_power_per_strength, attack_power_per_agility, ranged_attack_power_per_agility) =
            self.player_class_attack_power_coefficients_like_cpp(class)?;
        let rating_bonuses = std::array::from_fn(|index| {
            gear.combat_ratings[index] as f32
                * self.combat_rating_multiplier_like_cpp(level, index as u32)
        });
        let (can_parry, can_block) = self.canonical_player_parry_block_snapshot_like_cpp();
        let spell_bonus = self.represented_spell_bonus_like_cpp(gear);

        Some(calculate_player_stat_system_like_cpp(
            PlayerStatSystemInputLikeCpp {
                base,
                class,
                level,
                attack_power_per_strength,
                attack_power_per_agility,
                ranged_attack_power_per_agility,
                stat_total_multipliers: self
                    .resolved_represented_total_stat_multipliers_like_cpp()?,
                stat_buff_total_multipliers: self
                    .resolved_represented_total_stat_buff_multipliers_like_cpp()?,
                gear_stats: gear.stats,
                gear_health: gear.health,
                gear_mana: gear.mana,
                gear_armor: gear.armor,
                armor_base_pct: self.represented_resistance_aura_multiplier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_BASE_RESISTANCE_PCT,
                    SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
                ),
                armor_flat_aura: self
                    .represented_resistance_aura_flat_like_cpp(SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP)
                    as i32,
                armor_of_stat_percent: self.represented_armor_of_stat_percent_like_cpp(),
                armor_total_pct: self.represented_resistance_aura_multiplier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE_PCT,
                    SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
                ),
                armor_bonus_pct: self.represented_total_aura_multiplier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_BONUS_ARMOR_PCT,
                ),
                spell_dodge_pct: self.represented_total_aura_modifier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT,
                ),
                spell_parry_pct: self.represented_total_aura_modifier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_PARRY_PERCENT,
                ),
                spell_block_pct: self.represented_total_aura_modifier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_BLOCK_PERCENT,
                ),
                crit_mainhand_aura_pct: self.represented_weapon_crit_aura_modifier_like_cpp(
                    wow_constants::WeaponAttackType::BaseAttack,
                ),
                crit_offhand_aura_pct: self.represented_weapon_crit_aura_modifier_like_cpp(
                    wow_constants::WeaponAttackType::OffAttack,
                ),
                crit_ranged_aura_pct: self.represented_weapon_crit_aura_modifier_like_cpp(
                    wow_constants::WeaponAttackType::RangedAttack,
                ),
                spell_crit_aura_pct: self.represented_total_aura_modifier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_CRIT_CHANCE,
                ) + self.represented_total_aura_modifier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_PCT,
                ),
                gear_attack_power: gear.attack_power,
                gear_ranged_attack_power: gear.ranged_attack_power,
                attack_power_flat_aura: self.represented_attack_power_flat_aura_like_cpp(),
                attack_power_total_pct: self.represented_total_aura_multiplier_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACK_POWER_PCT,
                ),
                ranged_attack_power_flat_aura: self
                    .represented_ranged_attack_power_flat_aura_like_cpp(class),
                ranged_attack_power_total_pct: self
                    .represented_ranged_attack_power_total_pct_like_cpp(class),
                attack_power_override_by_spell_power_pct: self
                    .represented_override_attack_power_by_spell_power_pct_like_cpp(),
                spell_bonus,
                rating_bonuses,
                can_parry,
                can_block,
            },
        ))
    }

    /// Publish one complete `UpdateAllStats` projection on the canonical
    /// Player. Packet adapters may format this value, while combat systems can
    /// consume it without reaching through Session or rebuilding item input.
    /// The snapshot is derived and is intentionally not a persistence record.
    pub(crate) fn publish_player_effective_combat_stats_like_cpp(
        &self,
        level: u8,
        projection: PlayerStatSystemProjectionLikeCpp,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) {
        let mut resistances = gear.resistances;
        // Physical resistance is the final armor value after agility and flat
        // armor; the six magic schools come from `Unit::UpdateResistances`
        // (item `BASE_VALUE` plus the resistance aura producers).
        resistances[0] = projection.armor;
        for (index, value) in self
            .represented_school_resistances_like_cpp(gear)
            .into_iter()
            .enumerate()
        {
            resistances[index + 1] = value;
        }
        let weapon_damage = wow_data::player::effective_weapon_damage_ranges_like_cpp(
            projection,
            gear.weapon_damage,
            gear.base_attack_time,
        );
        // C++ `Player::UpdateExpertise` (`StatSystem.cpp:759-786`) truncates the
        // combat-rating bonus to `int32`, adds the `SPELL_AURA_MOD_EXPERTISE`
        // sum whose spell is fit for that attack's weapon, clamps at zero and
        // writes `MainhandExpertise`/`OffhandExpertise` per attack.
        // `ActivePlayerData::RangedExpertise`/`CombatRatingExpertise` are never
        // written by C++ (`UpdateExpertise` returns early for `RANGED_ATTACK`)
        // and stay at their zero create value.
        let rating_expertise = (gear.combat_ratings[23] as f32
            * self.combat_rating_multiplier_like_cpp(level, 23))
        .trunc();
        let mainhand_expertise = (rating_expertise
            + self.represented_expertise_aura_modifier_like_cpp(
                wow_constants::WeaponAttackType::BaseAttack,
            ) as f32)
            .max(0.0);
        let offhand_expertise = (rating_expertise
            + self.represented_expertise_aura_modifier_like_cpp(
                wow_constants::WeaponAttackType::OffAttack,
            ) as f32)
            .max(0.0);
        let mana_regen_mp5 = gear.mana_regen_bonus as f32 / 5.0
            + self.mana_regen_mp5_from_auras_like_cpp(projection.stats);
        let mana_regen_from_spirit = self.mana_regen_from_stats_like_cpp(
            level,
            self.player_class_like_cpp(),
            projection.stats,
        ) * self.mana_regen_aura_multiplier_like_cpp();
        let mana_regen_combat =
            mana_regen_mp5 + mana_regen_from_spirit * self.mana_regen_interrupt_modifier_like_cpp();
        let stats = PlayerEffectiveCombatStatsLikeCpp {
            stats: projection.stats,
            stat_pos_buff: projection.stat_pos_buff,
            stat_neg_buff: projection.stat_neg_buff,
            base_health: projection.create_health,
            max_health: projection.max_health,
            base_mana: projection.base_mana,
            max_mana: projection.max_mana,
            armor: projection.armor,
            resistances,
            attack_power: projection.attack_power,
            attack_power_mod_pos: projection.attack_power_mod_pos,
            attack_power_multiplier: projection.attack_power_multiplier,
            ranged_attack_power: projection.ranged_attack_power,
            ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
            ranged_attack_power_multiplier: projection.ranged_attack_power_multiplier,
            min_damage: weapon_damage[0][0],
            max_damage: weapon_damage[0][1],
            weapon_damage,
            min_ranged_damage: weapon_damage[2][0],
            max_ranged_damage: weapon_damage[2][1],
            combat_ratings: gear.combat_ratings,
            spell_power: gear.spell_power,
            mod_damage_done_pos: projection.mod_damage_done_pos,
            mod_damage_done_neg: projection.mod_damage_done_neg,
            mod_healing_done_pos: projection.mod_healing_done_pos,
            mod_damage_done_percent: projection.mod_damage_done_percent,
            mod_healing_done_percent: projection.mod_healing_done_percent,
            mana_regen: mana_regen_from_spirit + mana_regen_mp5,
            mana_regen_combat,
            health_regen: gear.health_regen_bonus,
            spell_penetration: gear.spell_penetration_bonus,
            mainhand_expertise,
            offhand_expertise,
            ranged_expertise: 0.0,
            combat_rating_expertise: 0.0,
            shield_block: i32::try_from(gear.shield_block_value)
                .unwrap_or(i32::MAX)
                .saturating_add(gear.shield_block_base_mod),
            block_pct: projection.block_pct,
            dodge_pct: projection.dodge_pct,
            dodge_from_attr: projection.dodge_from_attr,
            parry_pct: projection.parry_pct,
            parry_from_attr: projection.parry_from_attr,
            crit_pct: projection.crit_pct,
            ranged_crit_pct: projection.ranged_crit_pct,
            offhand_crit_pct: projection.offhand_crit_pct,
            spell_crit_pct: projection.spell_crit_pct,
            ..PlayerEffectiveCombatStatsLikeCpp::default()
        };
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(stats);
        });
    }

    pub(super) fn publish_effective_stats_like_cpp(
        &self,
        level: u8,
        _include_represented_item_bonuses: bool,
        projection: PlayerStatSystemProjectionLikeCpp,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) {
        // The canonical Player accumulator is always the source for this
        // projection. The boolean remains at the adapter boundary for
        // compatibility with callers that already name the C++ option, but a
        // second inventory-derived path is deliberately impossible here.
        self.publish_player_effective_combat_stats_like_cpp(level, projection, gear);
    }
}
