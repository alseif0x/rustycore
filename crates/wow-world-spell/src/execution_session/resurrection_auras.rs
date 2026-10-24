// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The server-triggered Ghost and Wisp self-aura cast path.
//!
//! C++ anchors: `Player.cpp:4262-4318` casts Wisp Spirit before Ghost during
//! repop; `Spell.cpp:8072` decides whether a triggered cast needs SpellGo;
//! `SpellAuras.cpp:110-161` derives AuraApplication flags and `:225-267`
//! projects the queued client update. Effect metadata comes from the selected
//! SpellMisc/SpellDuration rows; the permanent Duration entry is represented
//! by zero in the Rust runtime.

use wow_core::ObjectGuid;

use super::SpellExecutionHostLikeCpp;

const GHOST_SPELL: i32 = 8326;
const WISP_SPELL: i32 = 20584;
const AURA_FLAG_NO_CASTER: u32 = 0x0001;
const AURA_FLAG_CANCELABLE: u32 = 0x0002;
const AURA_FLAG_SCALABLE: u32 = 0x0008;
const AURA_FLAG_NEGATIVE: u32 = 0x0010;
const AURA_FLAG_POSITIVE: u32 = 0x0100;
const AURA_FLAG_PASSIVE: u32 = 0x0200;

pub(super) fn is_ghost_wisp_spell(spell_id: i32) -> bool {
    matches!(spell_id, GHOST_SPELL | WISP_SPELL)
}

pub(super) fn is_selected_self_cast<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
    target_guid: ObjectGuid,
    caster_guid: ObjectGuid,
    from_client: bool,
    triggered: bool,
) -> bool {
    !from_client
        && triggered
        && target_guid == caster_guid
        && target_guid == host.player_guid().unwrap_or(ObjectGuid::EMPTY)
        && is_ghost_wisp_spell(spell_id)
}

pub(super) fn cast_visual_id<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> Result<u32, &'static str> {
    if spell_id == WISP_SPELL {
        return Ok(0);
    }
    if spell_id != GHOST_SPELL {
        return Err("Spell is not a selected resurrection ghost aura");
    }
    let difficulty = host.core().current_map_difficulty_id_like_cpp();
    wow_world_entities::creature_spell_metadata::creature_ai_spell_x_spell_visual_id_like_cpp(
        u32::try_from(spell_id).map_err(|_| "Invalid Ghost spell id")?,
        difficulty,
        &host.hub_ref().config.creature_aggro_config_like_cpp,
    )
    .map_err(|()| "Ghost SpellXSpellVisual is missing or conditional")
}

pub(super) fn apply<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    spell_info: &wow_data::SpellInfo,
    cast_id: ObjectGuid,
    spell_visual_id: u32,
) -> Result<(), &'static str> {
    if !is_ghost_wisp_spell(spell_id) {
        return Err("Spell is not a selected resurrection ghost aura");
    }
    if caster_guid != host.player_guid().unwrap_or(ObjectGuid::EMPTY) {
        return Err("Selected resurrection ghost aura requires the Player as its caster");
    }
    let (duration_ms, aura_flags, effect_mask) = {
        let hub = host.hub_ref();
        let difficulty = host.core().current_map_difficulty_id_like_cpp();
        let misc = hub
            .catalogs
            .spell_catalogs
            .spell_misc_store
            .as_deref()
            .and_then(|store| {
                store.entry_for_spell_difficulty_with_fallback_like_cpp(
                    u32::try_from(spell_id).ok()?,
                    difficulty,
                    hub.catalogs.difficulty_store.as_deref(),
                )
            })
            .ok_or("Missing selected resurrection ghost SpellMisc row")?;
        let duration_index = u32::from(misc.duration_index);
        let duration = wow_data::spell_duration_ms_like_cpp(
            duration_index,
            hub.catalogs
                .spell_catalogs
                .spell_duration_store
                .as_deref(),
        );
        if duration != -1 {
            return Err("Selected resurrection ghost aura is not permanent in SpellDuration");
        }

        let mut effect_mask = 0_u32;
        for effect in spell_info.effects().iter().filter(|effect| {
            effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
        }) {
            let bit = 1_u32
                .checked_shl(effect.effect_index)
                .ok_or("Invalid resurrection ghost effect index")?;
            effect_mask |= bit;
        }
        if effect_mask != 0b111 {
            return Err("Selected resurrection ghost aura does not use all three effect slots");
        }

        let attrs = misc.attributes.map(|value| value as u32);
        let mut any_negative_effect = false;
        for effect in spell_info.effects().iter().filter(|effect| {
            effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                && effect_mask & 1u32.checked_shl(effect.effect_index).unwrap_or(0) != 0
        }) {
            let mut single_effect = spell_info.clone();
            single_effect.effects = vec![effect.clone()];
            if !wow_data::represented_spell_is_positive_like_cpp(&single_effect) {
                any_negative_effect = true;
            }
        }
        let positive = !any_negative_effect;
        let mut aura_flags = AURA_FLAG_NO_CASTER;
        if positive {
            aura_flags |= AURA_FLAG_POSITIVE;
        } else {
            aura_flags |= AURA_FLAG_NEGATIVE;
        }
        let passive = attrs[0] & wow_data::spell::attributes::SPELL_ATTR0_PASSIVE != 0;
        if positive
            && !passive
            && attrs[1] & wow_data::spell::attributes::SPELL_ATTR1_NO_AURA_ICON == 0
        {
            aura_flags |= AURA_FLAG_CANCELABLE;
        }
        if passive {
            aura_flags |= AURA_FLAG_PASSIVE;
        }
        if attrs[8] & wow_data::spell::attributes::SPELL_ATTR8_AURA_POINTS_ON_CLIENT != 0 {
            aura_flags |= AURA_FLAG_SCALABLE;
        }
        (0, aura_flags, effect_mask)
    };

    host.apply_ghost_wisp_aura(
        spell_id,
        caster_guid,
        spell_info,
        duration_ms,
        aura_flags,
        effect_mask,
        wow_entities::AuraCastProvenanceLikeCpp {
            cast_id,
            spell_visual_id: i32::try_from(spell_visual_id).unwrap_or(i32::MAX),
        },
    )
}
