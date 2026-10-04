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

pub(in crate::session) use wow_world_spell::unit_owned_apply_aura_effect_mask_like_cpp;
