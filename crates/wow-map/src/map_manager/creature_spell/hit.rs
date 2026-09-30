//! One represented melee-spell hit roll, including NO_ATTACK_MISS.
use super::*;

pub fn represented_hit_profile(
    metadata: &SpellHitFacts,
    active_effect_indices: &[u32],
    attributes: [u32; 15],
) -> Option<SpellHitProfile> {
    const SPELL_DAMAGE_CLASS_MELEE_LIKE_CPP: i8 = 2;
    const SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP: u8 = 0x01;
    const SPELL_ATTR0_IS_ABILITY_LIKE_CPP: u32 = 0x0000_0010;
    const SPELL_ATTR3_NO_AVOIDANCE_LIKE_CPP: u32 = 0x0000_0040;
    const SPELL_ATTR3_ALWAYS_HIT_LIKE_CPP: u32 = 0x0004_0000;
    const SPELL_ATTR7_ALLOW_SPELL_REFLECTION_LIKE_CPP: u32 = 0x0000_0001;
    const SPELL_ATTR7_NO_ATTACK_MISS_LIKE_CPP: u32 = 0x0200_0000;

    if metadata.defense_type != SPELL_DAMAGE_CLASS_MELEE_LIKE_CPP
        || metadata.school_mask != SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP
        || metadata.spell_mechanic != 0
        || active_effect_indices.is_empty()
        || active_effect_indices
            .iter()
            .any(|effect_index| metadata.effect_mechanics.get(effect_index).copied() != Some(0))
        || attributes[0] & SPELL_ATTR0_IS_ABILITY_LIKE_CPP == 0
        || attributes[3] & (SPELL_ATTR3_NO_AVOIDANCE_LIKE_CPP | SPELL_ATTR3_ALWAYS_HIT_LIKE_CPP)
            != 0
        || attributes[7] & SPELL_ATTR7_ALLOW_SPELL_REFLECTION_LIKE_CPP != 0
    {
        return None;
    }

    Some(
        if attributes[7] & SPELL_ATTR7_NO_ATTACK_MISS_LIKE_CPP != 0 {
            SpellHitProfile::NoAttackMissAfterRequiredRoll
        } else {
            SpellHitProfile::BaseMeleeMiss {
                miss_threshold_per_ten_thousand: 500,
            }
        },
    )
}

pub fn resolve_hit_profile(profile: SpellHitProfile, roll: Option<u32>) -> Option<SpellHit> {
    // C++ `Unit::MeleeSpellHitResult` draws `urand(0, 9999)` before applying
    // NO_ATTACK_MISS to the miss-chance bucket. Both currently represented
    // profiles therefore require exactly one authoritative draw.
    let roll = roll.filter(|roll| *roll <= 9_999)?;
    match profile {
        SpellHitProfile::NoAttackMissAfterRequiredRoll => Some(SpellHit::Hit),
        SpellHitProfile::BaseMeleeMiss {
            miss_threshold_per_ten_thousand,
        } => Some(if roll < miss_threshold_per_ten_thousand {
            SpellHit::Miss
        } else {
            SpellHit::Hit
        }),
    }
}
