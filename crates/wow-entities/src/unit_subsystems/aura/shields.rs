//! Shield selection over canonical aura amounts and borrowed spell rows.

use super::{AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem};
use std::collections::HashMap;
use wow_data_model::aura_effects::{
    RepresentedAbsorbShieldLikeCpp, RepresentedHealAbsorbShieldLikeCpp,
    RepresentedManaShieldLikeCpp,
};

impl AuraSubsystem {
    /// C++ `Unit::CalcAbsorbResist`'s `SPELL_AURA_SCHOOL_ABSORB` selection
    /// (`Unit.cpp:1812-1825`): every active absorb effect whose `MiscValue` covers
    /// the incoming school mask.
    ///
    /// Auras are visited in ascending slot order so the input to the priority sort
    /// is deterministic; the amount prefers the represented `AuraEffect` value and
    /// falls back to the spell effect's no-caster calculation, exactly like the
    /// generic aura projection.
    pub fn player_absorb_shields<'catalog, E: 'catalog>(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        school_mask: u32,
        mut select: impl FnMut(i32) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
        mut base_amount: impl FnMut(&E) -> i32,
        mut category: impl FnMut(i32) -> u32,
        mut cannot_be_ignored: impl FnMut(i32) -> bool,
    ) -> Vec<RepresentedAbsorbShieldLikeCpp> {
        let mut slots: Vec<u8> = auras.keys().copied().collect();
        slots.sort_unstable();
        let mut shields = Vec::new();
        for slot in slots {
            let aura = &auras[&slot];
            let Some(spell_effects) = select(aura.spell_id) else {
                continue;
            };
            let category_id = category(aura.spell_id);
            for effect in spell_effects.iter().filter(|effect| {
                fields(effect).2 == wow_constants::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB
                    && 1u32
                        .checked_shl(fields(effect).0)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
                    // C++ `!(absorbAurEff->GetMiscValue() & damageInfo.GetSchoolMask())`.
                    && (fields(effect).3 as u32) & school_mask != 0
            }) {
                let amount = aura
                    .represented_effect_amounts
                    .iter()
                    .find(|represented| {
                        u8::try_from(fields(effect).0).ok() == Some(represented.effect_index)
                    })
                    .map(|represented| represented.amount)
                    .unwrap_or_else(|| base_amount(effect));
                shields.push(RepresentedAbsorbShieldLikeCpp {
                    slot,
                    effect_index: u8::try_from(fields(effect).0).unwrap_or(0),
                    spell_id: aura.spell_id,
                    category_id,
                    amount,
                    cannot_be_ignored: cannot_be_ignored(aura.spell_id),
                });
            }
        }
        shields
    }

    /// C++ `Unit::CalcAbsorbResist`'s school-absorb selection for a creature
    /// victim. Creature aura applications use the canonical `AppliedAuraRef`
    /// tables rather than the player's runtime-application map, but the mutable
    /// amount still belongs to the same canonical aura subsystem. Reading that
    /// amount here keeps a later hit from recreating an already-spent creature
    /// shield from the spell's base points.
    pub fn creature_absorb_shields<'catalog, E: 'catalog>(
        &self,
        school_mask: u32,
        mut select: impl FnMut(i32) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
        mut base_amount: impl FnMut(&E) -> i32,
        mut category: impl FnMut(i32) -> u32,
        mut cannot_be_ignored: impl FnMut(i32) -> bool,
    ) -> Vec<RepresentedAbsorbShieldLikeCpp> {
        let mut applied = self.applied_auras.clone();
        applied.sort_by_key(|aura| (aura.slot, aura.effect_mask));
        let mut shields = Vec::new();
        for aura in applied {
            let spell_id = i32::try_from(aura.spell_id).unwrap_or(0);
            let Some(spell_effects) = select(spell_id) else {
                continue;
            };
            let category_id = category(spell_id);
            for effect in spell_effects.iter().filter(|effect| {
                fields(effect).2 == wow_constants::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB
                    && 1u32
                        .checked_shl(fields(effect).0)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
                    && (fields(effect).3 as u32) & school_mask != 0
            }) {
                let applied_effect = AppliedAuraRef::new(
                    aura.spell_id,
                    aura.caster_guid,
                    aura.slot,
                    1_u32 << fields(effect).0,
                );
                let amount = self
                    .applied_aura_amounts
                    .get(&applied_effect)
                    .copied()
                    .unwrap_or_else(|| base_amount(effect));
                shields.push(RepresentedAbsorbShieldLikeCpp {
                    slot: aura.slot,
                    effect_index: u8::try_from(fields(effect).0).unwrap_or(0),
                    spell_id,
                    category_id,
                    amount,
                    cannot_be_ignored: cannot_be_ignored(spell_id),
                });
            }
        }
        shields
    }

    /// C++ `Unit::CalcAbsorbResist`'s `SPELL_AURA_MANA_SHIELD` selection
    /// (`Unit.cpp:1886-1897`): every active mana-shield effect whose `MiscValue`
    /// covers the incoming school mask.
    ///
    /// C++ iterates `GetAuraEffectsByType` in application order; the represented
    /// projection visits auras in ascending slot order so the loop is
    /// deterministic.
    pub fn player_mana_shields<'catalog, E: 'catalog>(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        school_mask: u32,
        mut select: impl FnMut(i32) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
        mut base_amount: impl FnMut(&E) -> i32,
        mut mana_multiplier: impl FnMut(&E) -> f32,
        mut cannot_be_ignored: impl FnMut(i32) -> bool,
    ) -> Vec<RepresentedManaShieldLikeCpp> {
        let mut slots: Vec<u8> = auras.keys().copied().collect();
        slots.sort_unstable();
        let mut shields = Vec::new();
        for slot in slots {
            let aura = &auras[&slot];
            let Some(spell_effects) = select(aura.spell_id) else {
                continue;
            };
            for effect in spell_effects.iter().filter(|effect| {
                fields(effect).2 == wow_constants::spell::aura_types::SPELL_AURA_MANA_SHIELD
                    && 1u32
                        .checked_shl(fields(effect).0)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
                    // C++ `!(absorbAurEff->GetMiscValue() & damageInfo.GetSchoolMask())`.
                    && (fields(effect).3 as u32) & school_mask != 0
            }) {
                let amount = aura
                    .represented_effect_amounts
                    .iter()
                    .find(|represented| {
                        u8::try_from(fields(effect).0).ok() == Some(represented.effect_index)
                    })
                    .map(|represented| represented.amount)
                    .unwrap_or_else(|| base_amount(effect));
                shields.push(RepresentedManaShieldLikeCpp {
                    slot,
                    effect_index: u8::try_from(fields(effect).0).unwrap_or(0),
                    spell_id: aura.spell_id,
                    amount,
                    mana_multiplier: mana_multiplier(effect),
                    cannot_be_ignored: cannot_be_ignored(aura.spell_id),
                });
            }
        }
        shields
    }

    /// C++ `Unit::CalcHealAbsorb`'s `SPELL_AURA_SCHOOL_HEAL_ABSORB` selection
    /// (`Unit.cpp:2025-2034`): every active heal-absorb effect whose `MiscValue`
    /// covers the heal's school mask, in the aura order C++
    /// `GetAuraEffectsByType` returns.
    pub fn player_heal_absorb_shields<'catalog, E: 'catalog>(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        school_mask: u32,
        mut select: impl FnMut(i32) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, u32, i32, i32, i32),
        mut base_amount: impl FnMut(&E) -> i32,
    ) -> Vec<RepresentedHealAbsorbShieldLikeCpp> {
        let mut slots: Vec<u8> = auras.keys().copied().collect();
        slots.sort_unstable();
        let mut shields = Vec::new();
        for slot in slots {
            let aura = &auras[&slot];
            let Some(spell_effects) = select(aura.spell_id) else {
                continue;
            };
            for effect in spell_effects.iter().filter(|effect| {
                fields(effect).2 == wow_constants::spell::aura_types::SPELL_AURA_SCHOOL_HEAL_ABSORB
                    && 1u32
                        .checked_shl(fields(effect).0)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
                    // C++ `!(absorbAurEff->GetMiscValue() & healInfo.GetSchoolMask())`.
                    && (fields(effect).3 as u32) & school_mask != 0
            }) {
                let amount = aura
                    .represented_effect_amounts
                    .iter()
                    .find(|represented| {
                        u8::try_from(fields(effect).0).ok() == Some(represented.effect_index)
                    })
                    .map(|represented| represented.amount)
                    .unwrap_or_else(|| base_amount(effect));
                shields.push(RepresentedHealAbsorbShieldLikeCpp {
                    slot,
                    effect_index: u8::try_from(fields(effect).0).unwrap_or(0),
                    amount,
                });
            }
        }
        shields
    }
}
