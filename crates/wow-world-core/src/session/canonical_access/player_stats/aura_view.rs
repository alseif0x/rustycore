// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed aura inputs and pure queries shared by Session and Player owners.

use std::collections::{BTreeMap, HashMap};

use wow_data::{
    SpellChainStoreLikeCpp, SpellGroupStackRuleStoreLikeCpp, SpellGroupStoreLikeCpp, SpellStore,
};
use wow_entities::AuraApplicationLikeCpp;

use crate::session::state::hub_support::{
    RepresentedPlayerGearStatsLikeCpp, SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP, class_uses_wands_like_cpp,
};

/// Immutable spell catalogs required to interpret active Player aura effects.
#[derive(Clone, Copy)]
pub struct PlayerStatAuraCatalogs<'a> {
    spell_store: &'a SpellStore,
    spell_group_store: Option<&'a SpellGroupStoreLikeCpp>,
    spell_chain_store: Option<&'a SpellChainStoreLikeCpp>,
    spell_group_stack_rule_store: Option<&'a SpellGroupStackRuleStoreLikeCpp>,
}

impl<'a> PlayerStatAuraCatalogs<'a> {
    pub fn new(
        spell_store: &'a SpellStore,
        spell_group_store: Option<&'a SpellGroupStoreLikeCpp>,
        spell_chain_store: Option<&'a SpellChainStoreLikeCpp>,
        spell_group_stack_rule_store: Option<&'a SpellGroupStackRuleStoreLikeCpp>,
    ) -> Self {
        Self {
            spell_store,
            spell_group_store,
            spell_chain_store,
            spell_group_stack_rule_store,
        }
    }
}

/// Pure read-only stat queries over one borrowed canonical aura snapshot.
#[derive(Clone, Copy)]
pub struct PlayerStatAuraView<'a> {
    auras: &'a HashMap<u8, AuraApplicationLikeCpp>,
    catalogs: PlayerStatAuraCatalogs<'a>,
}

impl<'a> PlayerStatAuraView<'a> {
    pub fn new(
        auras: &'a HashMap<u8, AuraApplicationLikeCpp>,
        catalogs: PlayerStatAuraCatalogs<'a>,
    ) -> Self {
        Self { auras, catalogs }
    }

    pub fn effects_by_aura_type(&self, aura_type: i32) -> Vec<(i32, i32)> {
        crate::session::player_aura_effects_by_spell_aura_type_like_cpp(
            self.auras,
            self.catalogs.spell_store,
            aura_type,
        )
    }

    pub fn effects_with_misc_values_by_aura_type(&self, aura_type: i32) -> Vec<(i32, i32, i32)> {
        crate::session::aura_effects_with_misc_values_from_snapshot_like_cpp(
            self.auras,
            self.catalogs.spell_store,
            aura_type,
        )
    }

    pub fn effects_with_spell_and_misc_by_aura_type(&self, aura_type: i32) -> Vec<(i32, i32, i32)> {
        crate::session::aura_effects_with_spell_and_misc_from_snapshot_like_cpp(
            self.auras,
            self.catalogs.spell_store,
            aura_type,
        )
    }

    pub fn effect_amounts_by_aura_type(&self, aura_type: i32) -> Vec<(i32, i32)> {
        crate::session::aura_effect_amounts_by_spell_from_snapshot_like_cpp(
            self.auras,
            self.catalogs.spell_store,
            aura_type,
        )
    }

    pub fn total_aura_multiplier_by_type_and_misc_value(
        &self,
        aura_type: i32,
        misc_value: i32,
    ) -> f32 {
        self.effects_by_aura_type(aura_type)
            .into_iter()
            .filter(|(effect_misc_value, _)| *effect_misc_value == misc_value)
            .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
    }

    pub fn total_aura_modifier_by_type_and_misc_value(
        &self,
        aura_type: i32,
        misc_value: i32,
    ) -> i32 {
        self.effects_by_aura_type(aura_type)
            .into_iter()
            .filter(|(effect_misc_value, _)| *effect_misc_value == misc_value)
            .map(|(_, amount)| amount)
            .sum()
    }

    pub fn total_stat_multipliers(&self) -> [f32; 5] {
        std::array::from_fn(|stat| {
            self.stat_multiplier(
                wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                stat,
                true,
            )
        })
    }

    pub fn total_stat_buff_multipliers(&self) -> [f32; 5] {
        std::array::from_fn(|stat| {
            self.stat_multiplier(
                wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                stat,
                false,
            )
        })
    }

    pub fn stat_base_multipliers(&self) -> [f32; 5] {
        std::array::from_fn(|stat| {
            self.stat_multiplier(
                wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
                stat,
                false,
            )
        })
    }

    fn stat_multiplier(&self, aura_type: i32, stat: usize, uses_misc_value_b: bool) -> f32 {
        let mut multiplier = 1.0f32;
        let mut same_effect_spell_groups = BTreeMap::<u32, i32>::new();

        for aura in self.auras.values() {
            let Some(spell) = self.catalogs.spell_store.get(aura.spell_id) else {
                continue;
            };

            for effect in spell.effects().iter().filter(|effect| {
                1u32.checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                    && effect.effect_aura == aura_type
                    && if uses_misc_value_b {
                        effect.effect_misc_value_2 == 0
                            || effect.effect_misc_value_2 & (1 << stat) != 0
                    } else {
                        effect.effect_misc_value_1 == -1
                            || effect.effect_misc_value_1 == stat as i32
                    }
            }) {
                let amount = aura
                    .represented_effect_amounts
                    .iter()
                    .find(|represented| u32::from(represented.effect_index) == effect.effect_index)
                    .map(|represented| represented.amount)
                    .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());

                let same_effect_group = self
                    .spell_group_map_bounds(aura.spell_id as u32)
                    .iter()
                    .copied()
                    .find(|group_id| {
                        self.same_effect_stack_rule_aura_types(*group_id)
                            .is_some_and(|aura_types| aura_types.contains(&aura_type))
                    });
                if let Some(group_id) = same_effect_group {
                    same_effect_spell_groups
                        .entry(group_id)
                        .and_modify(|current| {
                            if current.unsigned_abs() < amount.unsigned_abs() {
                                *current = amount;
                            }
                        })
                        .or_insert(amount);
                } else {
                    multiplier += multiplier * amount as f32 / 100.0;
                }
            }
        }

        for amount in same_effect_spell_groups.into_values() {
            multiplier += multiplier * amount as f32 / 100.0;
        }
        multiplier
    }

    pub fn spell_group_map_bounds(&self, spell_id: u32) -> &[u32] {
        self.catalogs
            .spell_group_store
            .map(|store| {
                store.spell_spell_group_map_bounds_like_cpp(spell_id, |lookup_spell_id| {
                    self.catalogs
                        .spell_chain_store
                        .map(|chains| chains.first_spell_in_chain_like_cpp(lookup_spell_id))
                        .unwrap_or(lookup_spell_id)
                })
            })
            .unwrap_or(&[])
    }

    pub fn same_effect_stack_rule_aura_types(
        &self,
        group_id: u32,
    ) -> Option<&std::collections::BTreeSet<i32>> {
        self.catalogs
            .spell_group_stack_rule_store
            .and_then(|store| store.same_effect_stack_rule_aura_types_like_cpp(group_id))
    }

    pub fn mana_regen_aura_multiplier(&self) -> f32 {
        let mana = wow_constants::PowerType::Mana as i32;
        self.total_aura_multiplier_by_type_and_misc_value(
            wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN_PERCENT,
            mana,
        ) * self.total_aura_multiplier_by_type_and_misc_value(
            wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_PCT,
            mana,
        )
    }

    pub fn mana_regen_mp5_from_auras(&self, stats: [i32; 5]) -> f32 {
        let mana = wow_constants::PowerType::Mana as i32;
        let flat = self.total_aura_modifier_by_type_and_misc_value(
            wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN,
            mana,
        ) as f32
            / 5.0;
        let from_stat = self
            .effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_FROM_STAT)
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

    pub fn mana_regen_interrupt_modifier(&self) -> f32 {
        self.effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_INTERRUPT)
            .into_iter()
            .map(|(_, amount)| amount)
            .sum::<i32>()
            .min(100) as f32
            / 100.0
    }

    /// C++ `GetTotalAuraMultiplierByMiscMask(aura_type, school_mask)` for
    /// `Player::UpdateArmor` (`StatSystem.cpp:251-276`) and
    /// `Unit::UpdateResistances` (`Unit.cpp:9148-9163`).
    pub fn resistance_aura_multiplier(&self, aura_type: i32, school_mask: i32) -> f32 {
        self.effects_by_aura_type(aura_type)
            .into_iter()
            .filter(|(misc_value, _)| *misc_value & school_mask != 0)
            .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
    }

    /// C++ `GetTotalAuraMultiplier(aura_type)` without a school mask.
    pub fn total_aura_multiplier(&self, aura_type: i32) -> f32 {
        self.effects_by_aura_type(aura_type)
            .into_iter()
            .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
    }

    /// C++ `ActivePlayerData::OverrideAPBySpellPowerPercent` from
    /// `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT`
    /// (`SpellAuraDefines.h:499`, `SpellAuraEffects.cpp:3785-3796`); an active
    /// zero-sum set remains present and still overrides the base
    /// (`StatSystem.cpp:341`).
    pub fn override_attack_power_by_spell_power_percent(&self) -> Option<f32> {
        let effects = self.effects_by_aura_type(
            wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
        );
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

    /// C++ `GetFlatModifierValue(UNIT_MOD_ATTACK_POWER, TOTAL_VALUE)`.
    pub fn attack_power_flat_aura(&self) -> i32 {
        self.effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACK_POWER)
            .into_iter()
            .map(|(_, amount)| amount)
            .sum()
    }

    /// C++ `GetFlatModifierValue(UNIT_MOD_ATTACK_POWER_RANGED, TOTAL_VALUE)`;
    /// wand-user classes ignore this aura producer.
    pub fn ranged_attack_power_flat_aura(&self, class: u8) -> i32 {
        if class_uses_wands_like_cpp(class) {
            return 0;
        }
        self.effects_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_RANGED_ATTACK_POWER)
            .into_iter()
            .map(|(_, amount)| amount)
            .sum()
    }

    /// C++ `GetPctModifierValue(UNIT_MOD_ATTACK_POWER_RANGED, TOTAL_PCT)`;
    /// wand-user classes ignore this aura producer.
    pub fn ranged_attack_power_total_pct(&self, class: u8) -> f32 {
        if class_uses_wands_like_cpp(class) {
            return 1.0;
        }
        self.total_aura_multiplier(
            wow_data::spell::aura_types::SPELL_AURA_MOD_RANGED_ATTACK_POWER_PCT,
        )
    }

    /// C++ resistance flat modifiers from `SPELL_AURA_MOD_RESISTANCE` and
    /// `SPELL_AURA_MOD_BASE_RESISTANCE`, filtered by school mask.
    pub fn resistance_aura_flat(&self, school_mask: i32) -> f32 {
        [
            wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE,
            wow_data::spell::aura_types::SPELL_AURA_MOD_BASE_RESISTANCE,
        ]
        .into_iter()
        .flat_map(|aura_type| self.effects_by_aura_type(aura_type))
        .filter(|(misc_value, _)| *misc_value & school_mask != 0)
        .map(|(_, amount)| amount as f32)
        .sum()
    }

    /// C++ `Unit::UpdateResistances` (`Unit.cpp:9148-9163`) for the six magic
    /// schools. Physical armor remains part of the stat projection.
    pub fn school_resistances(&self, gear: &RepresentedPlayerGearStatsLikeCpp) -> [i32; 6] {
        std::array::from_fn(|index| {
            let school = index + 1;
            let mask = 1_i32 << school;
            let mut value = gear.resistances[school] as f32
                * self.resistance_aura_multiplier(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_BASE_RESISTANCE_PCT,
                    mask,
                );
            value += self.resistance_aura_flat(mask);
            value *= self.resistance_aura_multiplier(
                wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE_PCT,
                mask,
            );
            value as i32
        })
    }

    /// C++ `Player::UpdateArmor`'s `SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT`
    /// aggregation by `MiscValueB` stat index.
    pub fn armor_of_stat_percent(&self) -> [i32; 5] {
        let mut per_stat = [0i32; 5];
        for (school_mask, stat_index, amount) in self.effects_with_misc_values_by_aura_type(
            wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT,
        ) {
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

    /// C++ total aura modifier used for avoidance percentages.
    pub fn total_aura_modifier(&self, aura_type: i32) -> f32 {
        self.effects_by_aura_type(aura_type)
            .into_iter()
            .map(|(_, amount)| amount)
            .sum::<i32>() as f32
    }
}
