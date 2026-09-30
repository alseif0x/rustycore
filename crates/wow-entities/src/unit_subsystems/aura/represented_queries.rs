//! Represented Player/Unit aura predicates and modifier rules.

use std::collections::HashMap;
use super::{AuraApplicationLikeCpp, AuraSubsystem, RepresentedAuraEffectLikeCpp};

impl AuraSubsystem {
    pub fn has_represented_effect(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> bool {
        auras
            .values()
            .any(|aura| aura.represented_effect == Some(effect))
    }

    pub fn has_represented_effect_with_misc(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> bool {
        auras.values().any(|aura| {
            aura.represented_effect == Some(effect)
                && aura.represented_misc_value == Some(misc_value)
        })
    }

    pub fn represented_modifier(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> i32 {
        auras
            .values()
            .filter(|aura| aura.represented_effect == Some(effect))
            .map(|aura| aura.represented_amount)
            .sum()
    }

    pub fn represented_modifier_by_misc(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> i32 {
        auras
            .values()
            .filter(|aura| {
                aura.represented_effect == Some(effect)
                    && aura.represented_misc_value == Some(misc_value)
            })
            .map(|aura| aura.represented_amount)
            .sum()
    }

    pub fn represented_multiplier(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> f32 {
        auras
            .values()
            .filter(|aura| aura.represented_effect == Some(effect))
            .fold(1.0, |acc, aura| acc * aura.represented_multiplier)
    }

    pub fn maximum_represented_amount(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> i32 {
        auras
            .values()
            .filter(|aura| aura.represented_effect == Some(effect))
            .map(|aura| aura.represented_amount)
            .filter(|amount| *amount > 0)
            .max()
            .unwrap_or(0)
    }

    pub fn minimum_represented_amount(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> i32 {
        auras
            .values()
            .filter(|aura| aura.represented_effect == Some(effect))
            .map(|aura| aura.represented_amount)
            .filter(|amount| *amount < 0)
            .min()
            .unwrap_or(0)
    }

    pub fn represented_amount_multiplier(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> f32 {
        auras
            .values()
            .filter(|aura| aura.represented_effect == Some(effect))
            .fold(1.0, |multiplier, aura| {
                multiplier * (1.0 + aura.represented_amount.max(0) as f32 / 100.0)
            })
    }
}
