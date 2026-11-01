// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The spell-state field-split queries (#1363 slice 12b3), moved from the
//! World `session/spell_state` aura, aura-publication, cast and spell files.

use wow_entities::RepresentedAuraEffectLikeCpp;

use crate::SpellbookHostLikeCpp;

pub fn resolved_has_represented_aura_effect_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    effect: RepresentedAuraEffectLikeCpp,
) -> Option<bool> {
    let (state, hub) = host.split_spell_state_ref();
    state.resolved_has_represented_aura_effect_like_cpp(hub, effect)
}

pub fn resolved_total_represented_aura_modifier_by_misc_value_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    effect: RepresentedAuraEffectLikeCpp,
    misc_value: i32,
) -> Option<i32> {
    let (state, hub) = host.split_spell_state_ref();
    state.resolved_total_represented_aura_modifier_by_misc_value_like_cpp(hub, effect, misc_value)
}

pub fn resolved_total_represented_aura_multiplier_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    effect: RepresentedAuraEffectLikeCpp,
) -> Option<f32> {
    let (state, hub) = host.split_spell_state_ref();
    state.resolved_total_represented_aura_multiplier_like_cpp(hub, effect)
}

pub fn set_player_aura_authority_complete_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    complete: bool,
) -> bool {
    let (state, mut hub) = host.split_spell_state_mut();
    state.set_player_aura_authority_complete_like_cpp(&mut hub, complete)
}

pub fn represented_active_glyph_aura_source_is_empty_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> bool {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_active_glyph_aura_source_is_empty_like_cpp(hub)
}

pub fn represented_add_player_to_map_aura_source_is_empty_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> bool {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_add_player_to_map_aura_source_is_empty_like_cpp(hub)
}

pub fn represented_cast_speed_multiplier_like_cpp<H: SpellbookHostLikeCpp>(host: &H) -> f32 {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_cast_speed_multiplier_like_cpp(hub)
}

pub fn represented_player_is_polymorphed_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> Option<bool> {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_player_is_polymorphed_like_cpp(hub)
}

pub fn represented_update_zone_script_aura_source_is_hit_inert_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> bool {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_update_zone_script_aura_source_is_hit_inert_like_cpp(hub)
}

pub fn represented_update_area_pvp_rule_aura_source_is_empty_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> bool {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(hub)
}

pub fn represented_login_passive_spell_cast_gate_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> bool {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_login_passive_spell_cast_gate_like_cpp(hub, spell_id)
}

pub fn player_target_spell_is_hit_inert_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    spell_id: u32,
    difficulty_id: u8,
) -> bool {
    let (state, hub) = host.split_spell_state_ref();
    state.player_target_spell_is_hit_inert_like_cpp(hub, spell_id, difficulty_id)
}
