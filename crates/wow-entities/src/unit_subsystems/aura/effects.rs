//! Aura-effect selection over canonical applications and borrowed catalog rows.
//! C++ Unit::GetAuraEffectsByType; catalog lookup remains with the caller.

use super::{AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem};
use std::collections::HashMap;
use wow_data_model::aura_effects::AppliedAuraEffectLikeCpp;

impl AuraSubsystem {
    /// Ascending application-slot order, retaining the represented amount before
    /// lazily calculating the same borrowed row's no-caster value.
    /// The field callback returns (index, opcode, aura type, misc value, misc value B).
    pub fn player_effects<'catalog, E: 'catalog, T>(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        aura_type: Option<i32>,
        mut select: impl FnMut(i32) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
        mut base_amount: impl FnMut(&E) -> i32,
        mut project: impl FnMut(AppliedAuraEffectLikeCpp) -> T,
    ) -> Vec<T> {
        let mut slots: Vec<u8> = auras.keys().copied().collect();
        slots.sort_unstable();
        project_player_effects(
            slots.into_iter().map(|slot| (slot, &auras[&slot])),
            aura_type,
            &mut select,
            &fields,
            &mut base_amount,
            &mut project,
        )
    }

    /// These projections preserve the existing HashMap value iteration order.
    pub fn player_effects_in_map_order<'catalog, E: 'catalog, T>(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        aura_type: i32,
        mut select: impl FnMut(i32) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
        mut base_amount: impl FnMut(&E) -> i32,
        mut project: impl FnMut(AppliedAuraEffectLikeCpp) -> T,
    ) -> Vec<T> {
        project_player_effects(
            auras.values().map(|aura| (aura.slot, aura)),
            Some(aura_type),
            &mut select,
            &fields,
            &mut base_amount,
            &mut project,
        )
    }

    /// Creature projections retain applied-list and difficulty-selected row order;
    /// their amounts remain no-caster calculations, without represented overrides.
    pub fn creature_effects<'catalog, E: 'catalog>(
        applied_auras: &[AppliedAuraRef],
        difficulty_id: u8,
        mut select: impl FnMut(i32, u8) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
        mut base_amount: impl FnMut(&E) -> i32,
    ) -> Vec<AppliedAuraEffectLikeCpp> {
        let mut effects = Vec::new();
        for aura in applied_auras {
            let spell_id = i32::try_from(aura.spell_id).unwrap_or(0);
            let Some(spell_effects) = select(spell_id, difficulty_id) else {
                continue;
            };
            for effect in spell_effects.iter().filter(|effect| {
                fields(effect).2 != 0
                    && 1u32
                        .checked_shl(fields(effect).0)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
            }) {
                effects.push(AppliedAuraEffectLikeCpp {
                    slot: aura.slot,
                    spell_id,
                    caster_guid: aura.caster_guid,
                    aura_type: fields(effect).2,
                    misc_value: fields(effect).3,
                    misc_value_b: fields(effect).4,
                    amount: base_amount(effect),
                });
            }
        }
        effects
    }

    /// Unit::GetTotalAuraMultiplierByMiscValue over already resolved effects.
    pub fn effect_multiplier_by_misc(effects: Vec<(i32, i32)>, misc_value: i32) -> f32 {
        effects
            .into_iter()
            .filter(|(effect_misc_value, _)| *effect_misc_value == misc_value)
            .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
    }

    /// Unit::GetTotalAuraModifierByMiscValue over already resolved effects.
    pub fn effect_modifier_by_misc(effects: Vec<(i32, i32)>, misc_value: i32) -> i32 {
        effects
            .into_iter()
            .filter(|(effect_misc_value, _)| *effect_misc_value == misc_value)
            .map(|(_, amount)| amount)
            .sum()
    }

    pub fn has_total_stat_percentage<'catalog, E: 'catalog>(
        aura: &AuraApplicationLikeCpp,
        mut select: impl FnMut(i32) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
    ) -> bool {
        select(aura.spell_id).is_some_and(|effects| {
            effects.iter().any(|effect| {
                1u32.checked_shl(fields(effect).0)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                    && fields(effect).2
                        == wow_constants::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
            })
        })
    }

    /// SpellInfo::HasAttribute(IS_ABILITY) precedes the row lookup; these
    /// predicates never calculate amounts (including random base values).
    pub fn total_stat_percentage_preserves_health<'catalog, E: 'catalog>(
        aura: &AuraApplicationLikeCpp,
        mut is_ability: impl FnMut(i32) -> bool,
        mut select: impl FnMut(i32) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
    ) -> bool {
        is_ability(aura.spell_id)
            && select(aura.spell_id).is_some_and(|effects| {
                effects.iter().any(|effect| {
                    1u32.checked_shl(fields(effect).0)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                    && fields(effect).2
                        == wow_constants::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                    && (fields(effect).4 == 0 || fields(effect).4 & (1 << 2) != 0)
                })
            })
    }
}

fn project_player_effects<'application, 'catalog, E: 'catalog, T>(
    auras: impl IntoIterator<Item = (u8, &'application AuraApplicationLikeCpp)>,
    aura_type: Option<i32>,
    select: &mut impl FnMut(i32) -> Option<&'catalog [E]>,
    fields: &impl Fn(&E) -> (u32, u32, i32, i32, i32),
    base_amount: &mut impl FnMut(&E) -> i32,
    project: &mut impl FnMut(AppliedAuraEffectLikeCpp) -> T,
) -> Vec<T> {
    let mut effects = Vec::new();
    for (slot, aura) in auras {
        let Some(spell_effects) = select(aura.spell_id) else {
            continue;
        };
        for effect in spell_effects.iter().filter(|effect| {
            aura_type.is_none_or(|aura_type| fields(effect).2 == aura_type)
                && 1u32
                    .checked_shl(fields(effect).0)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| {
                    u8::try_from(fields(effect).0).ok() == Some(represented.effect_index)
                })
                .map(|represented| represented.amount)
                .unwrap_or_else(|| base_amount(effect));
            effects.push(project(AppliedAuraEffectLikeCpp {
                slot,
                spell_id: aura.spell_id,
                aura_type: fields(effect).2,
                misc_value: fields(effect).3,
                misc_value_b: fields(effect).4,
                amount,
                caster_guid: aura.caster_guid,
            }));
        }
    }
    effects
}
