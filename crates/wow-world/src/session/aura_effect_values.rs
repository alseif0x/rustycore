// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Aura effect values: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::RepresentedAuraEffectAmountLikeCpp;

pub(in crate::session) type CanonicalThreatAuraSnapshotLikeCpp =
    wow_entities::AuraThreatSnapshotLikeCpp;

pub(in crate::session) fn represented_aura_effect_amounts_like_cpp(
    effect: &wow_data::SpellEffectInfo,
) -> Vec<RepresentedAuraEffectAmountLikeCpp> {
    let Some(effect_index) = u8::try_from(effect.effect_index).ok() else {
        return Vec::new();
    };
    vec![RepresentedAuraEffectAmountLikeCpp {
        effect_index,
        amount: effect.effect_base_points,
    }]
}

pub(in crate::session) fn unit_owned_apply_aura_effect_mask_like_cpp(
    spell: &wow_data::SpellInfo,
) -> u32 {
    use wow_data::spell::spell_effect_types::{
        SPELL_EFFECT_APPLY_AREA_AURA_ENEMY, SPELL_EFFECT_APPLY_AREA_AURA_FRIEND,
        SPELL_EFFECT_APPLY_AREA_AURA_OWNER, SPELL_EFFECT_APPLY_AREA_AURA_PARTY,
        SPELL_EFFECT_APPLY_AREA_AURA_PET, SPELL_EFFECT_APPLY_AREA_AURA_RAID,
        SPELL_EFFECT_APPLY_AURA, SPELL_EFFECT_APPLY_AURA_ON_PET,
    };

    const SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS: u32 = 202;
    const SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM: u32 = 271;

    spell.effects().iter().fold(0, |mask, effect| {
        let unit_owned = matches!(
            effect.effect,
            SPELL_EFFECT_APPLY_AURA
                | SPELL_EFFECT_APPLY_AURA_ON_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
                | SPELL_EFFECT_APPLY_AREA_AURA_RAID
                | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
                | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
                | SPELL_EFFECT_APPLY_AREA_AURA_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
                | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
        );
        if unit_owned && effect.effect_index < u32::BITS {
            mask | (1u32 << effect.effect_index)
        } else {
            mask
        }
    })
}
