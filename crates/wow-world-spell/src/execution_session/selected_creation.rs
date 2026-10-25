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
    spell_visual_id: u32,
) -> Result<(u32, wow_entities::AuraCastProvenanceLikeCpp), &'static str> {
    let spell_id_u32 = u32::try_from(spell_id).map_err(|_| "Invalid selected aura spell id")?;
    let owner_guid = host
        .player_guid()
        .ok_or("Missing canonical Player for selected aura creation")?;
    let difficulty_id = host.core().current_map_difficulty_id_like_cpp();

    let (
        metadata,
        spell_levels,
        difficulties,
        faction_store,
        faction_templates,
        friendship_reactions,
    ) = {
        let hub = host.hub_ref();
        let spell_store = hub
            .catalogs
            .spell_catalogs
            .spell_store
            .as_deref()
            .ok_or("Missing SpellStore for selected aura creation")?;
        let metadata = crate::aura_session::selected_spell_metadata(
            spell_store,
            spell_id,
            difficulty_id,
            spell_info.effects(),
            hub.catalogs
                .spell_catalogs
                .spell_class_options_store()
                .map(|store| store.as_ref()),
            hub.catalogs.difficulty_store.as_deref(),
        )
        .map_err(|_| "Unable to resolve selected aura polarity metadata")?;
        (
            metadata,
            hub.catalogs.spell_catalogs.spell_levels_store.clone(),
            hub.catalogs.difficulty_store.clone(),
            hub.catalogs.factions.store.clone(),
            hub.catalogs.factions.template_store.clone(),
            hub.catalogs.friendship_rep_reaction_store.clone(),
        )
    };

    let known_flags = crate::aura_session::build_selected_aura_application_flags(
        metadata.polarity,
        active_effect_mask,
        caster_guid,
        owner_guid,
        owner_guid,
        &metadata.attributes,
        None,
    );
    let known_flags = match known_flags {
        Ok(flags) => Some(flags),
        Err(crate::aura_session::SelectedAuraFlagsError::UnknownForeignCasterRelation) => None,
        Err(_) => return Err("Unable to build selected aura application flags"),
    };

    let (aura_flags, caster_level) = {
        let core = host.core();
        let owner = core
            .player_handle_like_cpp
            .ok_or("Missing canonical Player owner for selected aura creation")?;
        let manager = core
            .canonical_map_manager
            .as_ref()
            .ok_or("Missing canonical MapManager for selected aura creation")?;
        let manager = manager
            .lock()
            .map_err(|_| "Canonical MapManager is unavailable for selected aura creation")?;
        let caster_level = crate::aura_session::selected_caster_level(
            &manager,
            owner,
            caster_guid,
            spell_id_u32,
            difficulty_id,
            spell_levels.as_deref(),
            difficulties.as_deref(),
        )
        .map_err(|_| "Unable to resolve selected aura caster level")?;
        let aura_flags = if let Some(flags) = known_flags {
            flags
        } else {
            let friendly = crate::aura_session::selected_caster_is_friendly(
                &manager,
                owner,
                caster_guid,
                faction_store.as_deref(),
                faction_templates.as_deref(),
                friendship_reactions.as_deref(),
            )
            .map_err(|_| "Unable to resolve selected aura caster relation")?;
            crate::aura_session::build_selected_aura_application_flags(
                metadata.polarity,
                active_effect_mask,
                caster_guid,
                owner_guid,
                owner_guid,
                &metadata.attributes,
                Some(friendly),
            )
            .map_err(|_| "Unable to build selected aura application flags")?
        };
        (aura_flags, caster_level)
    };

    Ok((
        u32::from(aura_flags),
        wow_entities::AuraCastProvenanceLikeCpp {
            cast_id,
            spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
            caster_level: Some(caster_level),
        },
    ))
}
