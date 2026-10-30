// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Resolve the cast and per-effect target data before spell-go publication.

use super::{
    SpellExecutionHostLikeCpp, represented_db_caster_destination_target_data_like_cpp,
    represented_focus_destination_target_data_like_cpp,
    represented_home_destination_target_data_like_cpp,
    represented_nearby_entry_destination_target_data_like_cpp,
    represented_transport_destination_world_position_like_cpp,
};
use crate::RepresentedSpellFocusObjectLikeCpp;
use wow_core::ObjectGuid;
use wow_packet::packets::spell::SpellTargetData;

const SPELL_TARGET_FLAG_UNIT_LIKE_CPP: u32 = 0x0000_0002;
// SpellDefines.h:302-303, including validation-only unit flags.
const SPELL_TARGET_FLAG_UNIT_MASK_LIKE_CPP: u32 = 0x0011_058e;

pub(super) struct ResolvedSpellEffectTargetsLikeCpp {
    pub(super) target_data: SpellTargetData,
    pub(super) spell_go_target_data: SpellTargetData,
    pub(super) effect_target_data_like_cpp: Vec<(u32, SpellTargetData)>,
}

pub(super) fn resolve_spell_effect_target_data_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    spell_info: &wow_data::SpellInfo,
    mut target_data: SpellTargetData,
    represented_focus_object: Option<RepresentedSpellFocusObjectLikeCpp>,
    selected_full_server_aura: bool,
) -> ResolvedSpellEffectTargetsLikeCpp {
    let mut represented_implicit_destination = false;
    let mut effect_target_data_like_cpp = Vec::new();
    for effect in spell_info.effects() {
        if effect.effect == 0 {
            continue;
        }
        for implicit_target in [effect.implicit_target_1, effect.implicit_target_2] {
            let mut isolated_effect = effect.clone();
            isolated_effect.implicit_target_1 = implicit_target;
            isolated_effect.implicit_target_2 = 0;
            let mut selection_input = target_data.clone();
            selection_input.flags &= !0x0000_0040;
            selection_input.dst_location = None;
            selection_input.map_id = None;
            let selected = match implicit_target {
                wow_data::spell::implicit_targets::TARGET_DEST_HOME => {
                    represented_home_destination_target_data_like_cpp(
                        host,
                        caster_guid,
                        &isolated_effect,
                        &selection_input,
                    )
                }
                wow_data::spell::implicit_targets::TARGET_DEST_DB => {
                    represented_db_caster_destination_target_data_like_cpp(
                        host,
                        caster_guid,
                        spell_info,
                        &isolated_effect,
                        &selection_input,
                    )
                }
                wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY
                | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
                | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB => {
                    represented_focus_destination_target_data_like_cpp(
                        host,
                        spell_id,
                        &isolated_effect,
                        &selection_input,
                        represented_focus_object,
                    )
                    .or_else(|| {
                        represented_nearby_entry_destination_target_data_like_cpp(
                            host,
                            spell_id,
                            &isolated_effect,
                            &selection_input,
                        )
                    })
                }
                _ => None,
            };
            if let Some(selected) = selected {
                target_data = selected;
                represented_implicit_destination = true;
            }
        }
        // C++ snapshots m_targets.GetDst() into
        // m_destTargets[EffectIndex] after selecting each effect.
        let mut effect_target_data = target_data.clone();
        if let Some(destination) = effect_target_data.dst_location.as_mut()
            && !destination.transport.is_empty()
            && let Some(world_position) = represented_transport_destination_world_position_like_cpp(
                host,
                destination.transport,
                destination.position,
            )
        {
            // C++ serializes _transportOffset but keeps _position as
            // world coordinates for effect execution.
            destination.position = world_position;
        }
        effect_target_data_like_cpp.push((effect.effect_index, effect_target_data));
    }

    let mut spell_go_target_data = target_data.clone();
    if represented_implicit_destination {
        // C++ retains map identity on SpellDestination for effect
        // execution, while SpellCastTargets::Write emits DstLocation XYZ.
        spell_go_target_data.map_id = None;
    }
    add_implicit_unit_caster_flag_for_selected_full_server_aura_like_cpp(
        &mut spell_go_target_data,
        spell_info,
        selected_full_server_aura,
    );

    ResolvedSpellEffectTargetsLikeCpp {
        target_data,
        spell_go_target_data,
        effect_target_data_like_cpp,
    }
}

fn add_implicit_unit_caster_flag_for_selected_full_server_aura_like_cpp(
    target_data: &mut SpellTargetData,
    spell_info: &wow_data::SpellInfo,
    selected_full_server_aura: bool,
) {
    if selected_full_server_aura
        && spell_info.effects().iter().any(|effect| {
            effect.effect != 0 && (effect.implicit_target_1 == 1 || effect.implicit_target_2 == 1)
        })
    {
        // Spell.cpp:747-755 publishes an implicit caster-unit target on
        // SpellCastTargets while retaining its empty explicit Unit GUID.
        target_data.flags |= SPELL_TARGET_FLAG_UNIT_LIKE_CPP;
    }
}

pub(super) fn spell_go_hit_targets_like_cpp(
    spell_info: &wow_data::SpellInfo,
    target_data: &SpellTargetData,
    target_guid: ObjectGuid,
) -> Vec<ObjectGuid> {
    use wow_data::spell::spell_effect_types as effect_types;

    let mut active_effects = spell_info
        .effects()
        .iter()
        .filter(|effect| effect.effect != 0)
        .peekable();
    // SpellEffects.cpp:3263-3275 and 3757-3765 execute these effects on the
    // caster; Spell.cpp:2008-2052 adds no effect-type target to the hit list.
    let caster_only_self_resurrection_or_stuck = active_effects.peek().is_some()
        && active_effects.all(|effect| {
            matches!(
                effect.effect,
                effect_types::SPELL_EFFECT_SELF_RESURRECT | effect_types::SPELL_EFFECT_STUCK
            ) && effect.implicit_target_1 == 0
                && effect.implicit_target_2 == 0
        })
        && target_data.unit.is_empty()
        && target_data.flags & SPELL_TARGET_FLAG_UNIT_MASK_LIKE_CPP == 0;

    if caster_only_self_resurrection_or_stuck {
        Vec::new()
    } else {
        vec![target_guid]
    }
}

#[cfg(test)]
#[path = "../../unit_tests/execution_session/target_resolution.rs"]
mod tests;
