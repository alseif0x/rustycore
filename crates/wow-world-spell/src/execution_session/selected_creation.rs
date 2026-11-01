// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Prepare the source flags and caster provenance needed only by a new Aura.

use wow_core::ObjectGuid;

use super::SpellExecutionHostLikeCpp;

pub(super) fn prepare<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    spell_info: &wow_data::SpellInfo,
    active_effect_mask: u32,
    cast_id: ObjectGuid,
) -> Result<(u32, wow_entities::AuraCastProvenanceLikeCpp, u8), &'static str> {
    let canonical_player_guid = host
        .player_guid()
        .ok_or("Missing canonical Player for selected aura creation")?;
    let spell_id_u32 = u32::try_from(spell_id).map_err(|_| "Invalid selected aura spell id")?;
    let difficulty_id = host.core().current_map_difficulty_id_like_cpp();

    let catalogs = host.hub_ref().catalogs;
    let spell_store = catalogs
        .spell_store()
        .ok_or("Missing SpellStore for selected aura creation")?;
    let difficulties = catalogs.difficulty_store().map(AsRef::as_ref);
    let actual_difficulty =
        crate::aura_session::selected_catalog::selected_spell_info_key_difficulty(
            spell_store,
            spell_id_u32,
            difficulty_id,
            difficulties,
        )
        .map_err(|_| "Unable to resolve selected aura SpellInfo key")?
        .ok_or("Spell not found")?;
    let class_options = catalogs
        .spell_catalogs
        .spell_class_options_store()
        .map(|store| store.as_ref());
    let metadata = crate::aura_session::selected_spell_metadata(
        spell_store,
        spell_id,
        actual_difficulty,
        spell_info.effects(),
        class_options,
        difficulties,
    )
    .map_err(|_| "Unable to resolve selected aura polarity metadata")?;
    let label_store = catalogs
        .spell_catalogs
        .spell_label_store()
        .map(AsRef::as_ref);
    let modifier_absence = crate::aura_session::validate_selected_aura_modifier_absence(
        spell_id_u32,
        actual_difficulty,
        metadata.attributes,
        class_options,
        label_store,
        difficulties,
    );
    match modifier_absence {
        Ok(()) => {}
        Err(error)
            if error == "Selected aura may have applicable class-mask SpellMods"
                || error == "Selected aura may have applicable label SpellMods" =>
        {
            let hub = host.hub_ref();
            let player_auras = hub.player_aura_subsystem_snapshot_like_cpp();
            crate::aura_session::validate_selected_aura_modifier_absence_from_player_auras(
                spell_id_u32,
                actual_difficulty,
                metadata.attributes,
                caster_guid,
                canonical_player_guid,
                class_options,
                label_store,
                difficulties,
                Some(spell_store.as_ref()),
                catalogs.spell_catalogs.serverside_spell_store.as_deref(),
                player_auras.as_ref(),
            )?;
        }
        Err(error) => return Err(error),
    }
    let charges = crate::aura_session::selected_aura_max_charges(
        spell_id_u32,
        actual_difficulty,
        catalogs.spell_catalogs.spell_aura_options_store.as_deref(),
        catalogs.spell_catalogs.spell_proc_store.as_deref(),
        difficulties,
    )?;
    let (flags, provenance) = crate::aura_session::prepare_selected_aura_creation(
        host,
        difficulty_id,
        metadata,
        spell_id_u32,
        caster_guid,
        active_effect_mask,
        cast_id,
    )?;
    Ok((flags, provenance, charges))
}
