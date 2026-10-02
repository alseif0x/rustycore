// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session item scaling values shared with World.

use std::collections::BTreeMap;
use std::sync::Arc;
use wow_data::PlayerStatsStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedScalingStatContextLikeCpp {
    pub stat_id: [i32; 10],
    pub bonus: [i32; 10],
    pub ssd_multiplier: i32,
    pub spell_bonus: i32,
    pub armor_mod: i32,
    pub dps_mod: i32,
    pub is_two_hand: bool,
}

pub fn player_class_mask_for_transmog_like_cpp(class_id: u8) -> u32 {
    if class_id == 0 || class_id > 32 {
        0
    } else {
        1_u32 << u32::from(class_id - 1)
    }
}

impl crate::session::HubRef<'_> {
    pub(in crate::session) fn resolved_represented_total_stat_multiplier_for_stat_like_cpp(
        &self,
        stat: usize,
        uses_misc_value_b: bool,
    ) -> Option<f32> {
        let spell_store = self.catalogs.spell_store()?;
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let aura_type = wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE;
        let mut multiplier = 1.0f32;
        let mut same_effect_spell_groups = BTreeMap::<u32, i32>::new();

        for aura in visible_auras.values() {
            let Some(spell) = spell_store.get(aura.spell_id) else {
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
                    .catalogs
                    .spell_spell_group_map_bounds_like_cpp(aura.spell_id as u32)
                    .iter()
                    .copied()
                    .find(|group_id| {
                        self.catalogs
                            .same_effect_stack_rule_aura_types_like_cpp(*group_id)
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
        Some(multiplier)
    }

    pub fn resolved_represented_total_stat_multipliers_like_cpp(&self) -> Option<[f32; 5]> {
        let mut multipliers = [1.0; 5];
        for (stat, multiplier) in multipliers.iter_mut().enumerate() {
            *multiplier =
                self.resolved_represented_total_stat_multiplier_for_stat_like_cpp(stat, true)?;
        }
        Some(multipliers)
    }

    pub fn resolved_represented_total_stat_buff_multipliers_like_cpp(
        &self,
    ) -> Option<[f32; 5]> {
        let mut multipliers = [1.0; 5];
        for (stat, multiplier) in multipliers.iter_mut().enumerate() {
            *multiplier =
                self.resolved_represented_total_stat_multiplier_for_stat_like_cpp(stat, false)?;
        }
        Some(multipliers)
    }
}

impl crate::session::state::SessionCatalogs {
    /// Get the player stats store reference.
    pub fn player_stats(&self) -> Option<&Arc<PlayerStatsStore>> {
        self.player_stats.as_ref()
    }
}

pub fn player_class_mask_for_talent_like_cpp(class_id: u8) -> Option<u32> {
    if class_id == 0 || class_id > 32 {
        None
    } else {
        Some(1_u32 << u32::from(class_id - 1))
    }
}
