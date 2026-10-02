// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::state::hub_support::{
    class_uses_wands_like_cpp, RepresentedPlayerGearStatsLikeCpp,
    SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
};
use wow_constants::PowerType;

impl crate::session::HubRef<'_> {
    pub fn mana_regen_from_stats_like_cpp(
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
            * self.catalogs.mana_regen_ratio_like_cpp(level, class)
    }

    pub fn mana_regen_aura_multiplier_like_cpp(&self) -> f32 {
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

    pub fn mana_regen_mp5_from_auras_like_cpp(&self, stats: [i32; 5]) -> f32 {
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

    pub fn mana_regen_interrupt_modifier_like_cpp(&self) -> f32 {
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
    pub fn represented_resistance_aura_multiplier_like_cpp(
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
    pub fn represented_total_aura_multiplier_like_cpp(&self, aura_type: i32) -> f32 {
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
    pub fn represented_override_attack_power_by_spell_power_pct_like_cpp(&self) -> Option<f32> {
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

    /// C++ `GetFlatModifierValue(UNIT_MOD_ATTACK_POWER, TOTAL_VALUE)` from the
    /// `SPELL_AURA_MOD_ATTACK_POWER` producers
    /// (`HandleAuraModAttackPower`).
    pub fn represented_attack_power_flat_aura_like_cpp(&self) -> i32 {
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
    pub fn represented_ranged_attack_power_flat_aura_like_cpp(&self, class: u8) -> i32 {
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
    pub fn represented_ranged_attack_power_total_pct_like_cpp(&self, class: u8) -> f32 {
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
    pub fn represented_resistance_aura_flat_like_cpp(&self, school_mask: i32) -> f32 {
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
    pub fn represented_school_resistances_like_cpp(
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
    pub fn represented_armor_of_stat_percent_like_cpp(&self) -> [i32; 5] {
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
    pub fn represented_total_aura_modifier_like_cpp(&self, aura_type: i32) -> f32 {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, amount)| amount)
            .sum::<i32>() as f32
    }
}
