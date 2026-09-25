// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Aura-effect projections over canonical player and creature applications.

use crate::session::*;
use std::collections::HashMap;
use wow_data::SpellStore;
use wow_entities::{AppliedAuraRef, AuraApplicationLikeCpp};

/// One resolved effect of a player's applied auras, the shape C++
/// `Unit::GetAuraEffectsByType` exposes to every predicate that also reads
/// `MiscValueB`, the effect's caster or its spell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlayerAuraEffectLikeCpp {
    /// C++ `AuraApplication::GetSlot()`: identifies the owning application
    /// when callers also need base-aura metadata such as cast provenance.
    pub slot: u8,
    /// C++ `AuraEffect::GetId()`: the owning spell.
    pub spell_id: i32,
    /// C++ `AuraEffect::GetAuraType()`.
    pub aura_type: i32,
    /// C++ `AuraEffect::GetMiscValue()`.
    pub misc_value: i32,
    /// C++ `AuraEffect::GetMiscValueB()`, read by predicates such as
    /// `SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH`'s health threshold.
    pub misc_value_b: i32,
    pub amount: i32,
    /// C++ `AuraEffect::GetCasterGUID()`.
    pub caster_guid: wow_core::ObjectGuid,
}

impl PlayerAuraEffectLikeCpp {
    /// The creature-aura projection's shape, which the shared
    /// `MeleeDamageBonusTaken` chain consumes.
    pub(crate) fn as_applied_like_cpp(&self) -> AppliedAuraEffectLikeCpp {
        AppliedAuraEffectLikeCpp {
            slot: self.slot,
            spell_id: self.spell_id,
            caster_guid: self.caster_guid,
            aura_type: self.aura_type,
            misc_value: self.misc_value,
            misc_value_b: self.misc_value_b,
            amount: self.amount,
        }
    }
}

/// C++ `Unit::GetAuraEffectsByType(auraType)` for one player's applied auras.
///
/// Auras are visited in ascending slot order so the projection is deterministic
/// (C++ walks its aura list in application order); the amount prefers the
/// represented `AuraEffect` value and falls back to the spell effect's
/// no-caster calculation, exactly like the session adapter used to do.
pub(crate) fn player_aura_effects_full_by_spell_aura_type_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<PlayerAuraEffectLikeCpp> {
    player_aura_effects_filtered_like_cpp(auras, spell_store, Some(aura_type))
}

/// Every active effect of a player's applied auras, the input C++
/// `MeleeDamageBonusTaken`'s `GetTotalAuraModifier*` chain reads across several
/// aura types at once.
pub(crate) fn player_aura_effects_all_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
) -> Vec<PlayerAuraEffectLikeCpp> {
    player_aura_effects_filtered_like_cpp(auras, spell_store, None)
}

fn player_aura_effects_filtered_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: Option<i32>,
) -> Vec<PlayerAuraEffectLikeCpp> {
    let mut slots: Vec<u8> = auras.keys().copied().collect();
    slots.sort_unstable();
    let mut effects = Vec::new();
    for slot in slots {
        let aura = &auras[&slot];
        let Some(spell) = spell_store.get(aura.spell_id) else {
            continue;
        };
        for effect in spell.effects().iter().filter(|effect| {
            aura_type.is_none_or(|aura_type| effect.effect_aura == aura_type)
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| {
                    u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                })
                .map(|represented| represented.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            effects.push(PlayerAuraEffectLikeCpp {
                slot,
                spell_id: aura.spell_id,
                aura_type: effect.effect_aura,
                misc_value: effect.effect_misc_value_1,
                misc_value_b: effect.effect_misc_value_2,
                amount,
                caster_guid: aura.caster_guid,
            });
        }
    }
    effects
}

/// The `(MiscValue, amount)` projection every `GetTotalAuraModifier*` caller
/// reads.
pub(crate) fn player_aura_effects_by_spell_aura_type_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<(i32, i32)> {
    player_aura_effects_full_by_spell_aura_type_like_cpp(auras, spell_store, aura_type)
        .into_iter()
        .map(|effect| (effect.misc_value, effect.amount))
        .collect()
}

/// One represented `SPELL_AURA_SCHOOL_ABSORB` shield of a player victim.
///
/// C++ `Unit::CalcAbsorbResist` (`Unit.cpp:1813-1880`) copies
/// `GetAuraEffectsByType(SPELL_AURA_SCHOOL_ABSORB)`, sorts it with
/// `Trinity::AbsorbAuraOrderPred` and depletes each effect's amount. The
/// application slot and effect index are carried here because the depletion is
/// a canonical aura-amount write, not a recomputation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RepresentedAbsorbShieldLikeCpp {
    /// The aura application slot that owns the effect.
    pub slot: u8,
    /// C++ `AuraEffect::GetEffIndex()`.
    pub effect_index: u8,
    /// C++ `AuraEffect::GetId()`.
    pub spell_id: i32,
    /// C++ `SpellInfo::GetCategory()` (`SpellInfo.cpp:1361-1364`), the
    /// `AbsorbAuraOrderPred` Ice Barrier rank.
    pub category_id: u32,
    /// C++ `AuraEffect::GetAmount()`. A negative amount is an infinite-absorb
    /// script shield, which C++ clamps to zero before absorbing.
    pub amount: i32,
    /// C++ `SpellInfo::HasAttribute(SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE)`: an
    /// absorb that an attacker's `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` cannot
    /// reduce (`Unit.cpp:1830-1832`).
    pub cannot_be_ignored: bool,
}

/// C++ `Unit::CalcAbsorbResist`'s `SPELL_AURA_SCHOOL_ABSORB` selection
/// (`Unit.cpp:1812-1825`): every active absorb effect whose `MiscValue` covers
/// the incoming school mask.
///
/// Auras are visited in ascending slot order so the input to the priority sort
/// is deterministic; the amount prefers the represented `AuraEffect` value and
/// falls back to the spell effect's no-caster calculation, exactly like the
/// generic aura projection.
pub(crate) fn player_absorb_shields_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    school_mask: u32,
) -> Vec<RepresentedAbsorbShieldLikeCpp> {
    let mut slots: Vec<u8> = auras.keys().copied().collect();
    slots.sort_unstable();
    let mut shields = Vec::new();
    for slot in slots {
        let aura = &auras[&slot];
        let Some(spell) = spell_store.get(aura.spell_id) else {
            continue;
        };
        let category_id = spell_store
            .hit_metadata_for_difficulty_like_cpp(aura.spell_id, difficulty_id, difficulty_store)
            .map_or(0, |metadata| metadata.category_id);
        for effect in spell.effects().iter().filter(|effect| {
            effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                // C++ `!(absorbAurEff->GetMiscValue() & damageInfo.GetSchoolMask())`.
                && (effect.effect_misc_value_1 as u32) & school_mask != 0
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| {
                    u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                })
                .map(|represented| represented.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            shields.push(RepresentedAbsorbShieldLikeCpp {
                slot,
                effect_index: u8::try_from(effect.effect_index).unwrap_or(0),
                spell_id: aura.spell_id,
                category_id,
                amount,
                cannot_be_ignored: spell_store.has_attribute_for_difficulty_like_cpp(
                    aura.spell_id,
                    difficulty_id,
                    difficulty_store,
                    6,
                    wow_data::spell::attributes::SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE,
                ),
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
pub(crate) fn creature_absorb_shields_like_cpp(
    auras: &wow_entities::AuraSubsystem,
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    school_mask: u32,
) -> Vec<RepresentedAbsorbShieldLikeCpp> {
    let mut applied = auras.applied_auras.clone();
    applied.sort_by_key(|aura| (aura.slot, aura.effect_mask));
    let mut shields = Vec::new();
    for aura in applied {
        let spell_id = i32::try_from(aura.spell_id).unwrap_or(0);
        let Some(spell) = spell_store.get(spell_id) else {
            continue;
        };
        let category_id = spell_store
            .hit_metadata_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
            .map_or(0, |metadata| metadata.category_id);
        for effect in spell.effects().iter().filter(|effect| {
            effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                && (effect.effect_misc_value_1 as u32) & school_mask != 0
        }) {
            let applied_effect = wow_entities::AppliedAuraRef::new(
                aura.spell_id,
                aura.caster_guid,
                aura.slot,
                1_u32 << effect.effect_index,
            );
            let amount = auras
                .applied_aura_amounts
                .get(&applied_effect)
                .copied()
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            shields.push(RepresentedAbsorbShieldLikeCpp {
                slot: aura.slot,
                effect_index: u8::try_from(effect.effect_index).unwrap_or(0),
                spell_id,
                category_id,
                amount,
                cannot_be_ignored: spell_store.has_attribute_for_difficulty_like_cpp(
                    spell_id,
                    difficulty_id,
                    difficulty_store,
                    6,
                    wow_data::spell::attributes::SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE,
                ),
            });
        }
    }
    shields
}

/// One represented `SPELL_AURA_MANA_SHIELD` of a player victim.
///
/// C++ `Unit::CalcAbsorbResist`'s mana-shield loop (`Unit.cpp:1886-1930`) reads
/// the effect's amount as the damage cap and
/// `SpellEffectInfo::CalcValueMultiplier` (`Amplitude`) as the mana drained per
/// absorbed point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct RepresentedManaShieldLikeCpp {
    /// The aura application slot that owns the effect.
    pub slot: u8,
    /// C++ `AuraEffect::GetEffIndex()`.
    pub effect_index: u8,
    /// C++ `AuraEffect::GetId()`.
    pub spell_id: i32,
    /// C++ `AuraEffect::GetAmount()`. A negative amount is an infinite-absorb
    /// script shield, which C++ clamps to zero.
    pub amount: i32,
    /// C++ `SpellEffectInfo::CalcValueMultiplier(caster)`'s data term: the mana
    /// the shield drains per point of absorbed damage.
    pub mana_multiplier: f32,
    /// C++ `SpellInfo::HasAttribute(SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE)`.
    pub cannot_be_ignored: bool,
}

/// C++ `Unit::CalcAbsorbResist`'s `SPELL_AURA_MANA_SHIELD` selection
/// (`Unit.cpp:1886-1897`): every active mana-shield effect whose `MiscValue`
/// covers the incoming school mask.
///
/// C++ iterates `GetAuraEffectsByType` in application order; the represented
/// projection visits auras in ascending slot order so the loop is
/// deterministic.
pub(crate) fn player_mana_shields_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    school_mask: u32,
) -> Vec<RepresentedManaShieldLikeCpp> {
    let mut slots: Vec<u8> = auras.keys().copied().collect();
    slots.sort_unstable();
    let mut shields = Vec::new();
    for slot in slots {
        let aura = &auras[&slot];
        let Some(spell) = spell_store.get(aura.spell_id) else {
            continue;
        };
        for effect in spell.effects().iter().filter(|effect| {
            effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MANA_SHIELD
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                // C++ `!(absorbAurEff->GetMiscValue() & damageInfo.GetSchoolMask())`.
                && (effect.effect_misc_value_1 as u32) & school_mask != 0
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| {
                    u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                })
                .map(|represented| represented.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            shields.push(RepresentedManaShieldLikeCpp {
                slot,
                effect_index: u8::try_from(effect.effect_index).unwrap_or(0),
                spell_id: aura.spell_id,
                amount,
                mana_multiplier: effect.calc_value_multiplier_like_cpp(),
                cannot_be_ignored: spell_store.has_attribute_for_difficulty_like_cpp(
                    aura.spell_id,
                    difficulty_id,
                    difficulty_store,
                    6,
                    wow_data::spell::attributes::SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE,
                ),
            });
        }
    }
    shields
}

/// One represented `SPELL_AURA_SCHOOL_HEAL_ABSORB` of a player victim.
///
/// C++ `Unit::CalcHealAbsorb` (`Unit.cpp:2020-2084`) copies
/// `GetAuraEffectsByType(SPELL_AURA_SCHOOL_HEAL_ABSORB)` and depletes each
/// effect's amount by the heal it consumed; the application slot and effect
/// index are carried so the depletion is a canonical aura-amount write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RepresentedHealAbsorbShieldLikeCpp {
    /// The aura application slot that owns the effect.
    pub slot: u8,
    /// C++ `AuraEffect::GetEffIndex()`.
    pub effect_index: u8,
    /// C++ `AuraEffect::GetAmount()`. A negative amount is an infinite-absorb
    /// script shield, which C++ clamps to zero.
    pub amount: i32,
}

/// C++ `Unit::CalcHealAbsorb`'s `SPELL_AURA_SCHOOL_HEAL_ABSORB` selection
/// (`Unit.cpp:2025-2034`): every active heal-absorb effect whose `MiscValue`
/// covers the heal's school mask, in the aura order C++
/// `GetAuraEffectsByType` returns.
pub(crate) fn player_heal_absorb_shields_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    school_mask: u32,
) -> Vec<RepresentedHealAbsorbShieldLikeCpp> {
    let mut slots: Vec<u8> = auras.keys().copied().collect();
    slots.sort_unstable();
    let mut shields = Vec::new();
    for slot in slots {
        let aura = &auras[&slot];
        let Some(spell) = spell_store.get(aura.spell_id) else {
            continue;
        };
        for effect in spell.effects().iter().filter(|effect| {
            effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_SCHOOL_HEAL_ABSORB
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                // C++ `!(absorbAurEff->GetMiscValue() & healInfo.GetSchoolMask())`.
                && (effect.effect_misc_value_1 as u32) & school_mask != 0
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| {
                    u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                })
                .map(|represented| represented.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            shields.push(RepresentedHealAbsorbShieldLikeCpp {
                slot,
                effect_index: u8::try_from(effect.effect_index).unwrap_or(0),
                amount,
            });
        }
    }
    shields
}

/// C++ `Unit::HasAuraWithMechanic` (`Unit.cpp:4714-4729`) over a canonical
/// Player's `AuraApplication` map: the union of each application's
/// `SpellInfo::Mechanic` and its applied `IsEffect()` slots' mechanics, read at
/// the application's own difficulty.
pub(crate) fn aura_application_mechanic_mask_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> u64 {
    auras.values().fold(0_u64, |mask, aura| {
        mask | spell_mechanic_mask_like_cpp(
            spell_store,
            difficulty_store,
            aura.spell_id,
            aura.difficulty_id,
            aura.effect_mask,
        )
    })
}

/// C++ `Unit::HasAuraWithMechanic` over a creature's `AppliedAuraRef` list.
/// Those refs do not retain a difficulty, so the caller supplies the one it
/// resolves the rest of the victim's state with.
pub(crate) fn applied_aura_mechanic_mask_like_cpp(
    applied_auras: &[AppliedAuraRef],
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> u64 {
    applied_auras.iter().fold(0_u64, |mask, aura| {
        mask | spell_mechanic_mask_like_cpp(
            spell_store,
            difficulty_store,
            i32::try_from(aura.spell_id).unwrap_or(0),
            difficulty_id,
            aura.effect_mask,
        )
    })
}

/// The mechanic bits one `AuraApplication`/`AppliedAuraRef` contributes: its
/// spell's own `Mechanic` plus every applied `IsEffect()` slot that carries one.
fn spell_mechanic_mask_like_cpp(
    spell_store: &SpellStore,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    spell_id: i32,
    difficulty_id: u8,
    effect_mask: u32,
) -> u64 {
    let Some(metadata) =
        spell_store.hit_metadata_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
    else {
        return 0;
    };
    let mut mask = mechanic_bit_like_cpp(i32::from(metadata.spell_mechanic)).unwrap_or(0);
    let effects =
        spell_store.effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store);
    for (effect_index, mechanic) in metadata.effect_mechanics {
        if !(1..32).contains(&effect_index) || effect_mask & (1_u32 << effect_index) == 0 {
            continue;
        }
        let is_effect = effects.is_some_and(|effects| {
            effects
                .iter()
                .any(|effect| effect.effect_index == effect_index && effect.effect != 0)
        });
        if is_effect {
            mask |= mechanic_bit_like_cpp(mechanic).unwrap_or(0);
        }
    }
    mask
}

/// C++ `UI64LIT(1) << mechanic` for a positive mechanic; `None` for the unset
/// or out-of-range value a malformed row could carry.
fn mechanic_bit_like_cpp(mechanic: i32) -> Option<u64> {
    (1..64).contains(&mechanic).then(|| 1_u64 << mechanic)
}

/// One resolved effect of a creature's `AppliedAuraRef`, the shape every
/// victim-side C++ `GetTotalAuraModifier*` query reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AppliedAuraEffectLikeCpp {
    /// C++ `AuraApplication::GetSlot()` for the effect's owning application.
    pub slot: u8,
    pub spell_id: i32,
    pub caster_guid: wow_core::ObjectGuid,
    /// The effect's `AuraType`.
    pub aura_type: i32,
    /// The effect's `MiscValue`, matched against a school mask where C++ uses
    /// `GetTotalAuraModifierByMiscMask`.
    pub misc_value: i32,
    /// The effect's `MiscValueB`, read by C++ predicates such as
    /// `SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH`'s health threshold.
    pub misc_value_b: i32,
    pub amount: i32,
}

/// C++ `Unit::GetAuraEffectsByType`/`GetTotalAuraModifier*` over a creature's
/// `AuraApplicationMap`: every active effect of the unit's applied auras,
/// resolved at the caller's difficulty.
///
/// The player-side counterpart is
/// [`player_aura_effects_by_spell_aura_type_like_cpp`]; a creature's
/// `AppliedAuraRef` carries no represented effect amounts, so the effect's
/// no-caster calculation is the value, exactly like the mechanic-mask
/// projection.
pub(crate) fn creature_aura_effects_like_cpp(
    applied_auras: &[AppliedAuraRef],
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Vec<AppliedAuraEffectLikeCpp> {
    let mut effects = Vec::new();
    for aura in applied_auras {
        let spell_id = i32::try_from(aura.spell_id).unwrap_or(0);
        let Some(spell_effects) =
            spell_store.effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
        else {
            continue;
        };
        for effect in spell_effects.iter().filter(|effect| {
            effect.effect_aura != 0
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
        }) {
            effects.push(AppliedAuraEffectLikeCpp {
                slot: aura.slot,
                spell_id,
                caster_guid: aura.caster_guid,
                aura_type: effect.effect_aura,
                misc_value: effect.effect_misc_value_1,
                misc_value_b: effect.effect_misc_value_2,
                amount: effect.calc_value_no_caster_like_cpp(),
            });
        }
    }
    effects
}
