// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Preserve the legacy primary-effect fallback while rejecting unknown routes.

use super::SpellExecutionHostLikeCpp;
use super::execution_support::{
    SpellEffectExecutionRouteLikeCpp, SpellEffectRouteContextLikeCpp,
    classify_spell_effect_route_like_cpp,
};
use wow_core::ObjectGuid;

pub(super) fn apply_primary_spell_effect_fallback_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    effect_type: u32,
    spell_info: &wow_data::SpellInfo,
    spell_id: i32,
    player_guid: ObjectGuid,
    cast_id: ObjectGuid,
    spell_visual_id: u32,
    route_context: SpellEffectRouteContextLikeCpp,
) -> Result<(), &'static str> {
    let primary_index = spell_info
        .effects()
        .iter()
        .find(|effect| effect.effect == effect_type)
        .map(|effect| effect.effect_index);
    match classify_spell_effect_route_like_cpp(
        spell_info,
        effect_type,
        primary_index,
        route_context,
    ) {
        SpellEffectExecutionRouteLikeCpp::SourceInert
        | SpellEffectExecutionRouteLikeCpp::Direct
        | SpellEffectExecutionRouteLikeCpp::PostGo => {}
        SpellEffectExecutionRouteLikeCpp::ApplyAura => {
            if spell_info.effects().is_empty()
                && effect_type == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            {
                host.apply_aura_with_effect_mask_and_provenance_like_cpp(
                    spell_id,
                    player_guid,
                    30_000,
                    0x0000_0001,
                    0x0000_0001,
                    wow_entities::AuraCastProvenanceLikeCpp {
                        cast_id,
                        spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                        caster_level: None,
                    },
                    true,
                )?;
            }
        }
        SpellEffectExecutionRouteLikeCpp::Unavailable => {
            return Err("primary spell effect has no represented execution route");
        }
    }
    Ok(())
}
