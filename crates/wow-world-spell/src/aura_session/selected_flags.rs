// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure selected-effect polarity and Aura application-flag rules.

use wow_core::ObjectGuid;
use wow_data::SpellEffectInfo;

const SPELL_EFFECT_APPLY_AURA: u32 = 6;

const SPELL_AURA_MOD_INCREASE_SPEED: i32 = 31;
const SPELL_AURA_TRANSFORM: i32 = 56;
const SPELL_AURA_MOD_INCREASE_SWIM_SPEED: i32 = 58;
const SPELL_AURA_MOD_DAMAGE_PERCENT_DONE: i32 = 79;
const SPELL_AURA_MOD_PERCENT_STAT: i32 = 80;
const SPELL_AURA_GHOST: i32 = 95;
const SPELL_AURA_NO_PVP_CREDIT: i32 = 159;

const ATTR0_PASSIVE: u32 = 0x0000_0040;
const ATTR0_AURA_IS_DEBUFF: u32 = 0x0400_0000;
const ATTR1_AURA_UNIQUE: u32 = 0x0000_0800;
const ATTR1_NO_AURA_ICON: u32 = 0x1000_0000;
const ATTR4_AURA_IS_BUFF: u32 = 0x0000_1000;
const ATTR8_AURA_POINTS_ON_CLIENT: u32 = 0x0000_1000;

const EFFECT_ATTRIBUTE_IS_HARMFUL: u32 = 0x0000_1000;
const SPELL_MECHANIC_IMMUNE_SHIELD: i32 = 29;

const SPELL_FAMILY_GENERIC: u16 = 0;
const SPELL_FAMILY_WARRIOR: u16 = 4;
const SPELL_FAMILY_ROGUE: u16 = 8;

const TARGET_NONE: u32 = 0;
const TARGET_UNIT_CASTER: u32 = 1;
const TARGET_UNIT_TARGET_ENEMY: u32 = 6;

const APPLICATION_FLAG_NO_CASTER: u16 = 0x0001;
const APPLICATION_FLAG_CANCELABLE: u16 = 0x0002;
const APPLICATION_FLAG_SCALABLE: u16 = 0x0008;
const APPLICATION_FLAG_NEGATIVE: u16 = 0x0010;
const APPLICATION_FLAG_POSITIVE: u16 = 0x0100;
const APPLICATION_FLAG_PASSIVE: u16 = 0x0200;

/// Spell-wide facts that the caller must resolve from the effective spell
/// metadata and its already-established negativity authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectedSpellMetadataFacts {
    pub(crate) spell_id: i32,
    pub(crate) attributes: [u32; 15],
    pub(crate) family_name: Option<u16>,
    pub(crate) family_flags: Option<[u32; 4]>,
    pub(crate) spell_mechanic: Option<i32>,
    pub(crate) preexisting_negative_effect_mask: u32,
}

/// A transient polarity calculation; it is not canonical spell state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectedSpellPolarity {
    pub(crate) available_effect_mask: u32,
    pub(crate) negative_effect_mask: u32,
    pub(crate) positive_effect_mask: u32,
    pub(crate) whole_spell_positive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedSpellPolarityError {
    EmptyActiveSpell,
    InvalidEffectIndex(u32),
    DuplicateEffectIndex(u32),
    UnsupportedEffect { index: u32, effect: u32 },
    UnsupportedAuraType { index: u32, aura_type: i32 },
    UnsupportedTarget { index: u32 },
    MissingEffectMetadata { index: u32 },
    MissingSpellFamilyName { index: u32 },
    MissingSpellFamilyFlags { index: u32 },
    MissingSpellMechanic { index: u32 },
    NonFinitePolarityData { index: u32 },
    RoundedValueOutOfRange { index: u32 },
    AmbiguousCalcValueSign { index: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedAuraFlagsError {
    EmptyActiveEffectMask,
    ActiveMaskOutsideAvailableEffects { requested: u32, available: u32 },
    UnknownForeignCasterRelation,
}

/// Classify the closed selected-aura spell subset using only supplied facts.
///
/// C++ anchors: `SpellInfo.cpp:4517-4962` and `:4964-5000` at
/// `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.
pub(crate) fn classify_selected_spell_polarity(
    spell: SelectedSpellMetadataFacts,
    effects: &[SpellEffectInfo],
) -> Result<SelectedSpellPolarity, SelectedSpellPolarityError> {
    let mut indexed_effects = [None; 32];
    let mut effect_vector_len = 0_usize;
    let mut available_effect_mask = 0_u32;

    for effect in effects {
        let index = effect.effect_index;
        let Some(bit) = 1_u32.checked_shl(index) else {
            return Err(SelectedSpellPolarityError::InvalidEffectIndex(index));
        };
        let Some(slot) = indexed_effects.get_mut(index as usize) else {
            return Err(SelectedSpellPolarityError::InvalidEffectIndex(index));
        };
        if slot.replace(effect).is_some() {
            return Err(SelectedSpellPolarityError::DuplicateEffectIndex(index));
        }
        effect_vector_len = effect_vector_len.max(index as usize + 1);

        if effect.effect == 0 {
            continue;
        }
        if effect.effect != SPELL_EFFECT_APPLY_AURA {
            return Err(SelectedSpellPolarityError::UnsupportedEffect {
                index,
                effect: effect.effect,
            });
        }
        if !is_supported_aura(effect.effect_aura) {
            return Err(SelectedSpellPolarityError::UnsupportedAuraType {
                index,
                aura_type: effect.effect_aura,
            });
        }
        available_effect_mask |= bit;
    }

    if available_effect_mask == 0 {
        return Err(SelectedSpellPolarityError::EmptyActiveSpell);
    }

    // Keep pre-existing bits even when they identify an inactive or absent
    // effect slot; C++ IsPositive() observes the complete NegativeEffects set.
    let mut negative_effect_mask = spell.preexisting_negative_effect_mask;

    for effect in effects.iter().filter(|effect| effect.effect != 0) {
        let index = effect.effect_index;
        let bit = 1_u32 << index;
        if negative_effect_mask & bit != 0 {
            continue;
        }

        let attributes = spell.attributes;
        if attributes[0] & ATTR0_PASSIVE != 0 {
            continue;
        }
        if attributes[0] & ATTR0_AURA_IS_DEBUFF != 0 {
            negative_effect_mask |= bit;
            continue;
        }
        if attributes[4] & ATTR4_AURA_IS_BUFF != 0 {
            continue;
        }

        let effect_metadata = effect
            .polarity_metadata
            .ok_or(SelectedSpellPolarityError::MissingEffectMetadata { index })?;
        if effect_metadata.attributes & EFFECT_ATTRIBUTE_IS_HARMFUL != 0 {
            negative_effect_mask |= bit;
            continue;
        }

        if let Some(is_negative) = family_decision(spell, index)? {
            if is_negative {
                negative_effect_mask |= bit;
            }
            continue;
        }
        let spell_mechanic = spell
            .spell_mechanic
            .ok_or(SelectedSpellPolarityError::MissingSpellMechanic { index })?;
        if spell_mechanic == SPELL_MECHANIC_IMMUNE_SHIELD {
            continue;
        }

        if spell.attributes[1] & ATTR1_AURA_UNIQUE != 0 {
            match unique_aura_targets_positive(effects)? {
                false => {
                    negative_effect_mask |= bit;
                    continue;
                }
                true => {}
            }
        }

        let is_negative = match effect.effect_aura {
            SPELL_AURA_GHOST => true,
            SPELL_AURA_MOD_INCREASE_SPEED => !target_polarity(effect)
                .ok_or(SelectedSpellPolarityError::UnsupportedTarget { index })?,
            SPELL_AURA_MOD_INCREASE_SWIM_SPEED | SPELL_AURA_MOD_PERCENT_STAT => {
                let real_points_per_level = effect_metadata.real_points_per_level;
                if !real_points_per_level.is_finite() {
                    return Err(SelectedSpellPolarityError::NonFinitePolarityData { index });
                }
                real_points_per_level < 0.0 || calc_value_is_negative(effect, index)?
            }
            SPELL_AURA_MOD_DAMAGE_PERCENT_DONE => match target_polarity(effect) {
                Some(false) => true,
                Some(true) => calc_value_is_negative(effect, index)?,
                None => {
                    if calc_value_is_negative(effect, index)? {
                        true
                    } else {
                        return Err(SelectedSpellPolarityError::UnsupportedTarget { index });
                    }
                }
            },
            SPELL_AURA_TRANSFORM => !target_polarity(effect)
                .ok_or(SelectedSpellPolarityError::UnsupportedTarget { index })?,
            SPELL_AURA_NO_PVP_CREDIT => false,
            _ => unreachable!("unsupported aura types were rejected above"),
        };
        if is_negative {
            negative_effect_mask |= bit;
        }
    }

    // C++'s post-pass compares each currently-positive special aura with
    // negative effects at later effect indexes, including retained hole bits.
    for effect in effects {
        if effect.effect == 0
            || effect.effect_aura != SPELL_AURA_TRANSFORM
            || negative_effect_mask & (1_u32 << effect.effect_index) != 0
        {
            continue;
        }
        let has_later_negative_same_target =
            ((effect.effect_index as usize + 1)..effect_vector_len).any(|later_index| {
                if negative_effect_mask & (1_u32 << later_index) == 0 {
                    return false;
                }
                let (target_a, target_b) = indexed_effects[later_index]
                    .map(|later| (later.implicit_target_1, later.implicit_target_2))
                    .unwrap_or((TARGET_NONE, TARGET_NONE));
                target_a == effect.implicit_target_1 && target_b == effect.implicit_target_2
            });
        if has_later_negative_same_target {
            negative_effect_mask |= 1_u32 << effect.effect_index;
        }
    }

    let positive_effect_mask = available_effect_mask & !negative_effect_mask;
    Ok(SelectedSpellPolarity {
        available_effect_mask,
        negative_effect_mask,
        positive_effect_mask,
        whole_spell_positive: negative_effect_mask == 0,
    })
}

/// Build the C++ `AuraApplication::_InitFlags` result for an actual active mask.
///
/// C++ anchors: `SpellAuras.cpp:110-161` and `SpellAuraDefines.h:34-43` at
/// `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.
pub(crate) fn build_selected_aura_application_flags(
    polarity: SelectedSpellPolarity,
    active_effect_mask: u32,
    raw_caster_guid: ObjectGuid,
    owner_guid: ObjectGuid,
    target_guid: ObjectGuid,
    attributes: &[u32; 15],
    foreign_caster_is_friendly: Option<bool>,
) -> Result<u16, SelectedAuraFlagsError> {
    if active_effect_mask == 0 {
        return Err(SelectedAuraFlagsError::EmptyActiveEffectMask);
    }
    if active_effect_mask & !polarity.available_effect_mask != 0 {
        return Err(SelectedAuraFlagsError::ActiveMaskOutsideAvailableEffects {
            requested: active_effect_mask,
            available: polarity.available_effect_mask,
        });
    }

    let positive_active_mask = active_effect_mask & polarity.positive_effect_mask;
    let negative_active_mask = active_effect_mask & polarity.negative_effect_mask;
    let is_self_cast = raw_caster_guid == target_guid;
    let is_positive = if positive_active_mask == active_effect_mask {
        true
    } else if negative_active_mask == active_effect_mask {
        false
    } else if is_self_cast || foreign_caster_is_friendly == Some(false) {
        negative_active_mask == 0
    } else if foreign_caster_is_friendly == Some(true) {
        positive_active_mask != 0
    } else {
        return Err(SelectedAuraFlagsError::UnknownForeignCasterRelation);
    };

    let mut flags = if is_self_cast {
        APPLICATION_FLAG_NO_CASTER
    } else {
        0
    };
    flags |= if is_positive {
        APPLICATION_FLAG_POSITIVE
    } else {
        APPLICATION_FLAG_NEGATIVE
    };

    let is_passive = attributes[0] & ATTR0_PASSIVE != 0;
    if is_positive
        && !is_passive
        && attributes[1] & ATTR1_NO_AURA_ICON == 0
        && owner_guid == target_guid
    {
        flags |= APPLICATION_FLAG_CANCELABLE;
    }
    if attributes[8] & ATTR8_AURA_POINTS_ON_CLIENT != 0 {
        flags |= APPLICATION_FLAG_SCALABLE;
    }
    if is_passive {
        flags |= APPLICATION_FLAG_PASSIVE;
    }

    Ok(flags)
}

fn is_supported_aura(aura_type: i32) -> bool {
    matches!(
        aura_type,
        SPELL_AURA_GHOST
            | SPELL_AURA_MOD_INCREASE_SPEED
            | SPELL_AURA_MOD_INCREASE_SWIM_SPEED
            | SPELL_AURA_TRANSFORM
            | SPELL_AURA_MOD_PERCENT_STAT
            | SPELL_AURA_MOD_DAMAGE_PERCENT_DONE
            | SPELL_AURA_NO_PVP_CREDIT
    )
}

fn family_decision(
    spell: SelectedSpellMetadataFacts,
    index: u32,
) -> Result<Option<bool>, SelectedSpellPolarityError> {
    let family_name = spell
        .family_name
        .ok_or(SelectedSpellPolarityError::MissingSpellFamilyName { index })?;
    let decision = match family_name {
        SPELL_FAMILY_GENERIC => match spell.spell_id {
            40_268 | 61_987 | 61_988 | 64_412 | 72_410 | 71_204 => Some(true),
            24_732 | 30_877 | 61_716 | 61_734 | 62_344 | 50_344 | 61_819 | 61_834 | 73_523 => {
                Some(false)
            }
            _ => None,
        },
        SPELL_FAMILY_ROGUE => match spell.spell_id {
            32_645 => Some(false),
            40_251 => Some(true),
            _ => None,
        },
        SPELL_FAMILY_WARRIOR => {
            let flags = spell
                .family_flags
                .ok_or(SelectedSpellPolarityError::MissingSpellFamilyFlags { index })?;
            (flags[0] & 0x2020_0000 != 0).then_some(true)
        }
        _ => None,
    };
    Ok(decision)
}

fn unique_aura_targets_positive(
    effects: &[SpellEffectInfo],
) -> Result<bool, SelectedSpellPolarityError> {
    let mut unknown_index = None;
    for effect in effects {
        match target_polarity(effect) {
            Some(false) => return Ok(false),
            Some(true) => {}
            None => unknown_index = Some(effect.effect_index),
        }
    }
    if let Some(index) = unknown_index {
        return Err(SelectedSpellPolarityError::UnsupportedTarget { index });
    }
    Ok(true)
}

/// `None` means the closed target contract has no source-backed answer.
fn target_polarity(effect: &SpellEffectInfo) -> Option<bool> {
    if effect.effect == 0 {
        return Some(true);
    }
    if effect.implicit_target_1 == TARGET_UNIT_TARGET_ENEMY
        || effect.implicit_target_2 == TARGET_UNIT_TARGET_ENEMY
    {
        return Some(false);
    }
    (effect.implicit_target_2 == TARGET_NONE
        && matches!(effect.implicit_target_1, TARGET_NONE | TARGET_UNIT_CASTER))
    .then_some(true)
}

fn calc_value_is_negative(
    effect: &SpellEffectInfo,
    index: u32,
) -> Result<bool, SelectedSpellPolarityError> {
    let metadata = effect
        .polarity_metadata
        .ok_or(SelectedSpellPolarityError::MissingEffectMetadata { index })?;

    if !metadata.scaling_coefficient.is_finite() || !metadata.scaling_variance.is_finite() {
        return Err(SelectedSpellPolarityError::NonFinitePolarityData { index });
    }

    // C++ SpellEffectInfo::CalcBaseValue returns zero for nonzero coefficients
    // in this target: SpellInfo construction sets Scaling.Class to zero and
    // the scaling table contribution is commented out.
    let base_value = if metadata.scaling_coefficient != 0.0 {
        0.0
    } else {
        let source_base_points = (effect.effect_base_points as f32).round();
        f64::from(checked_round_to_i32(f64::from(source_base_points), index)?)
    };

    // C++ uses BasePoints * frand(-abs(Variance * 0.5), +abs(...)).
    let delta = (metadata.scaling_variance * 0.5_f32).abs();
    if !delta.is_finite() {
        return Err(SelectedSpellPolarityError::NonFinitePolarityData { index });
    }
    let variance_extent = base_value.abs() * f64::from(delta);
    let mut minimum = base_value - variance_extent;
    let mut maximum = base_value + variance_extent;

    match effect.effect_die_sides {
        0 => {}
        1 => {
            minimum += 1.0;
            maximum += 1.0;
        }
        die_sides if die_sides > 1 => {
            minimum += 1.0;
            maximum += f64::from(die_sides);
        }
        die_sides => {
            minimum += f64::from(die_sides);
            maximum += 1.0;
        }
    }

    let minimum_rounded = checked_round_to_i32(minimum, index)?;
    let maximum_rounded = checked_round_to_i32(maximum, index)?;
    match (minimum_rounded < 0, maximum_rounded < 0) {
        (true, true) => Ok(true),
        (false, false) => Ok(false),
        _ => Err(SelectedSpellPolarityError::AmbiguousCalcValueSign { index }),
    }
}

fn checked_round_to_i32(value: f64, index: u32) -> Result<i32, SelectedSpellPolarityError> {
    let rounded = value.round();
    if !rounded.is_finite() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return Err(SelectedSpellPolarityError::RoundedValueOutOfRange { index });
    }
    Ok(rounded as i32)
}
