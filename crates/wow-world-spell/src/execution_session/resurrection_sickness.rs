// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The race-selected self aura used by spirit-healer resurrection.
//!
//! C++ anchors: aura creation in `Spell.cpp:3225` and
//! `Spell::EffectApplyAura` (`SpellEffects.cpp:1037-1050`),
//! `Aura::TryRefreshStackOrCreate` (`SpellAuras.cpp:344-382`),
//! `Unit::_TryStackingOrRefreshingExistingAura` (`Unit.cpp:3224-3283`) and
//! `SpellInfo::GetDuration` (`SpellInfo.cpp:3905`).

use wow_core::ObjectGuid;
use wow_data::spell_duration_ms_like_cpp;

use super::SpellExecutionHostLikeCpp;

pub(super) fn resurrection_sickness_cast_metadata<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> Result<(u32, u32), &'static str> {
    let spell_id = u32::try_from(spell_id).map_err(|_| "Invalid resurrection sickness spell")?;
    let hub = host.hub_ref();
    let difficulty_id = host.core().current_map_difficulty_id_like_cpp();
    let config = &hub.config.creature_aggro_config_like_cpp;
    let visual_id =
        wow_world_entities::creature_spell_metadata::creature_ai_spell_x_spell_visual_id_like_cpp(
            spell_id,
            difficulty_id,
            config,
        )
        .map_err(|()| "Resurrection sickness visual metadata is missing or conditional")?;
    let cast_flags =
        wow_world_entities::creature_spell_metadata::creature_ai_spell_go_cast_flags_like_cpp(
            spell_id,
            difficulty_id,
            config,
        ) | 0x0000_0001;
    Ok((visual_id, cast_flags))
}

pub(super) fn should_send_resurrection_sickness_spell_go<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
    spell_visual_id: u32,
) -> Result<bool, &'static str> {
    let spell_id = u32::try_from(spell_id).map_err(|_| "Invalid resurrection sickness spell")?;
    let hub = host.hub_ref();
    let difficulty_id = host.core().current_map_difficulty_id_like_cpp();
    let misc = hub
        .catalogs
        .spell_catalogs
        .spell_misc_store
        .as_deref()
        .and_then(|store| {
            store.entry_for_spell_difficulty_with_fallback_like_cpp(
                spell_id,
                difficulty_id,
                hub.catalogs.difficulty_store.as_deref(),
            )
        })
        .ok_or("Missing resurrection sickness SpellMisc row")?;
    let attr1 = misc.attributes[1] as u32;
    let attr7 = misc.attributes[7] as u32;
    let attr8 = misc.attributes[8] as u32;
    Ok(spell_visual_id != 0
        || attr1
            & (wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED
                | wow_data::spell::attributes::SPELL_ATTR1_IS_SELF_CHANNELLED)
            != 0
        || attr8 & wow_data::spell::attributes::SPELL_ATTR8_AURA_POINTS_ON_CLIENT != 0
        || attr7 & wow_data::spell::attributes::SPELL_ATTR7_ALWAYS_CAST_LOG != 0
        || misc.speed > 0.0
        || misc.launch_delay > 0.0)
}

pub(super) fn is_resurrection_sickness_spell<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> bool {
    let hub = host.hub_ref();
    let race_id = hub.player_race_like_cpp();
    hub.catalogs
        .chr
        .races_store
        .as_ref()
        .and_then(|store| store.get(u32::from(race_id)))
        .is_some_and(|race| race.res_sickness_spell_id == spell_id)
}

pub(super) fn apply_resurrection_sickness_aura<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    spell_info: &wow_data::SpellInfo,
    cast_id: ObjectGuid,
    spell_visual_id: u32,
    remaining_ms: Option<u32>,
) -> Result<(), &'static str> {
    let spell_id_u32 =
        u32::try_from(spell_id).map_err(|_| "Invalid resurrection sickness spell")?;
    let (duration_ms, visible_auras) = {
        let hub = host.hub_ref();
        let difficulty_id = host.core().current_map_difficulty_id_like_cpp();
        let duration_index = hub
            .catalogs
            .spell_catalogs
            .spell_misc_store
            .as_deref()
            .and_then(|store| {
                store.entry_for_spell_difficulty_with_fallback_like_cpp(
                    spell_id_u32,
                    difficulty_id,
                    hub.catalogs.difficulty_store.as_deref(),
                )
            })
            .map(|entry| u32::from(entry.duration_index))
            .ok_or("Missing resurrection sickness SpellMisc row")?;
        let duration_ms = spell_duration_ms_like_cpp(
            duration_index,
            hub.catalogs.spell_catalogs.spell_duration_store.as_deref(),
        );
        let duration_ms = u32::try_from(duration_ms)
            .ok()
            .filter(|duration| *duration > 0)
            .ok_or("Missing finite resurrection sickness SpellDuration")?;
        let visible_auras = hub
            .resolved_player_visible_auras_like_cpp()
            .ok_or("Missing canonical Player aura applications")?;
        (duration_ms, visible_auras)
    };

    let effect_mask = spell_info
        .effects()
        .iter()
        .filter(|effect| {
            effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
        })
        .try_fold(0u32, |mask, effect| {
            let bit = 1u32
                .checked_shl(effect.effect_index)
                .ok_or("Invalid resurrection sickness effect index")?;
            Ok::<_, &'static str>(mask | bit)
        })?;
    if effect_mask == 0 {
        return Err("Resurrection sickness has no APPLY_AURA effects");
    }

    let existing = visible_auras.values().any(|aura| {
        aura.spell_id == spell_id
            && aura.caster_guid == caster_guid
            && aura.effect_mask == effect_mask
    });
    if existing {
        if crate::set_player_aura_duration(
            host,
            spell_id,
            caster_guid,
            remaining_ms.unwrap_or(duration_ms),
        ) == 0
        {
            return Err("Resurrection sickness aura disappeared during refresh");
        }
        return Ok(());
    }

    let remaining_ms = remaining_ms.unwrap_or(duration_ms);
    host.apply_aura_with_effect_mask_and_provenance_like_cpp(
        spell_id,
        caster_guid,
        duration_ms,
        0x0000_0015,
        effect_mask,
        wow_entities::AuraCastProvenanceLikeCpp {
            cast_id,
            spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
        },
        false,
    )?;
    if crate::set_player_aura_duration(host, spell_id, caster_guid, remaining_ms) == 0 {
        return Err("Resurrection sickness aura disappeared after application");
    }
    Ok(())
}
