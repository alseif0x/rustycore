// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Prepare flags and caster provenance for a newly-created selected Aura.

use wow_core::ObjectGuid;
use wow_world_core::session::SessionHubHostLikeCpp;

use super::{
    SelectedAuraFlagsError, SelectedSpellMetadata, build_selected_aura_application_flags,
    selected_caster_is_friendly, selected_caster_level,
    selected_visual::selected_aura_spell_visual_id,
};

pub(crate) fn prepare_selected_aura_creation<H: SessionHubHostLikeCpp + ?Sized>(
    host: &H,
    difficulty_id: u8,
    metadata: SelectedSpellMetadata,
    spell_id: u32,
    caster_guid: ObjectGuid,
    active_effect_mask: u32,
    cast_id: ObjectGuid,
) -> Result<(u32, wow_entities::AuraCastProvenanceLikeCpp), &'static str> {
    let owner_guid = host
        .player_guid()
        .ok_or("Missing canonical Player for selected aura creation")?;
    let known_flags = build_selected_aura_application_flags(
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
        Err(SelectedAuraFlagsError::UnknownForeignCasterRelation) => None,
        Err(_) => return Err("Unable to build selected aura application flags"),
    };

    let hub = host.hub_ref();
    let catalogs = hub.catalogs;
    let (aura_flags, caster_level, spell_visual_id) = {
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
        let caster_level = selected_caster_level(
            &manager,
            owner,
            caster_guid,
            spell_id,
            difficulty_id,
            catalogs.spell_catalogs.spell_levels_store.as_deref(),
            catalogs.difficulty_store.as_deref(),
        )
        .map_err(|_| "Unable to resolve selected aura caster level")?;
        let aura_flags = if let Some(flags) = known_flags {
            flags
        } else {
            let friendly = selected_caster_is_friendly(
                &manager,
                owner,
                caster_guid,
                catalogs.factions.store.as_deref(),
                catalogs.factions.template_store.as_deref(),
                catalogs.friendship_rep_reaction_store.as_deref(),
            )
            .map_err(|_| "Unable to resolve selected aura caster relation")?;
            build_selected_aura_application_flags(
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
        let spell_visual_id = selected_aura_spell_visual_id(
            &manager,
            owner,
            caster_guid,
            spell_id,
            difficulty_id,
            catalogs.spell_catalogs.spell_store.as_deref(),
            catalogs.spell_catalogs.serverside_spell_store.as_deref(),
            hub.config
                .creature_aggro_config_like_cpp
                .spell_x_spell_visual_store
                .as_deref(),
            catalogs.condition_presence.as_deref(),
            catalogs.difficulty_store.as_deref(),
        )?;
        (aura_flags, caster_level, spell_visual_id)
    };

    Ok((
        u32::from(aura_flags),
        wow_entities::AuraCastProvenanceLikeCpp {
            cast_id,
            spell_visual_id: spell_visual_id as i32,
            caster_level: Some(caster_level),
        },
    ))
}
