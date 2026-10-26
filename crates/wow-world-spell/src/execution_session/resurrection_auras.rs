// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The server-triggered Ghost and Wisp self-aura cast path.
//!
//! C++ anchors: `Player.cpp:4262-4318` casts Wisp Spirit before Ghost during
//! repop; `Spell.cpp:8072` decides whether a triggered cast needs SpellGo;
//! `SpellAuras.cpp:110-161` derives AuraApplication flags and `:225-267`
//! projects the queued client update. Effect metadata comes from the selected
//! SpellMisc/SpellDuration rows; the permanent Duration entry is represented
//! by the canonical `-1/-1` lifetime pair (legacy `0/0` inputs are adapted at
//! their boundary).

use wow_core::ObjectGuid;

use super::SpellExecutionHostLikeCpp;

const GHOST_SPELL: i32 = 8326;
const WISP_SPELL: i32 = 20584;

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

pub(super) fn apply<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    spell_info: &wow_data::SpellInfo,
    cast_id: ObjectGuid,
    _spell_visual_id: u32,
) -> Result<(), &'static str> {
    if !is_ghost_wisp_spell(spell_id) {
        return Err("Spell is not a selected resurrection ghost aura");
    }
    if caster_guid != host.player_guid().unwrap_or(ObjectGuid::EMPTY) {
        return Err("Selected resurrection ghost aura requires the Player as its caster");
    }
    let (duration_ms, effect_mask, stack_amount, attributes1, attributes5) = {
        let hub = host.hub_ref();
        let difficulty = host.core().current_map_difficulty_id_like_cpp();
        let difficulties = hub.catalogs.difficulty_store.as_deref();
        let misc_store = hub
            .catalogs
            .spell_catalogs
            .spell_misc_store
            .as_deref()
            .ok_or("Missing selected resurrection ghost SpellMisc catalog")?;
        let misc = crate::aura_session::selected_catalog::selected_spell_misc_entry(
            misc_store,
            u32::try_from(spell_id).map_err(|_| "Invalid selected resurrection ghost spell")?,
            difficulty,
            difficulties,
        )
        .map_err(|_| "Unable to resolve selected resurrection ghost SpellMisc row")?
        .ok_or("Missing selected resurrection ghost SpellMisc row")?;
        let duration_index = u32::from(misc.duration_index);
        let duration = wow_data::spell_duration_ms_like_cpp(
            duration_index,
            hub.catalogs.spell_catalogs.spell_duration_store.as_deref(),
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

        let spell_id_u32 =
            u32::try_from(spell_id).map_err(|_| "Invalid selected resurrection ghost spell")?;
        let options = crate::aura_session::selected_catalog::selected_aura_options_entry(
            hub.catalogs.spell_catalogs.spell_aura_options_store.as_deref()
                .ok_or("Missing selected resurrection ghost SpellAuraOptions catalog")?,
            spell_id_u32,
            difficulty,
            difficulties,
        )
        .map_err(|_| "Unable to resolve selected resurrection ghost SpellAuraOptions row")?;
        let stack_amount = options.map_or(0, |row| row.cumulative_aura);
        let attrs = misc.attributes.map(|value| value as u32);
        (0, effect_mask, stack_amount, attrs[1], attrs[5])
    };
    let creation = super::selected_creation::prepare(
        host,
        spell_id,
        caster_guid,
        spell_info,
        effect_mask,
        cast_id,
    );

    host.apply_ghost_wisp_aura(
        spell_id,
        caster_guid,
        spell_info,
        duration_ms,
        effect_mask,
        stack_amount,
        attributes1,
        attributes5,
        creation,
    )
}
