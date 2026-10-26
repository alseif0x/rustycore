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
) -> Result<(u32, wow_entities::AuraCastProvenanceLikeCpp), &'static str> {
    host.player_guid()
        .ok_or("Missing canonical Player for selected aura creation")?;
    let spell_id_u32 = u32::try_from(spell_id).map_err(|_| "Invalid selected aura spell id")?;
    let difficulty_id = host.core().current_map_difficulty_id_like_cpp();

    let metadata = {
        let catalogs = host.hub_ref().catalogs;
        let spell_store = catalogs
            .spell_catalogs
            .spell_store
            .as_deref()
            .ok_or("Missing SpellStore for selected aura creation")?;
        crate::aura_session::selected_spell_metadata(
            spell_store,
            spell_id,
            difficulty_id,
            spell_info.effects(),
            catalogs
                .spell_catalogs
                .spell_class_options_store()
                .map(|store| store.as_ref()),
            catalogs.difficulty_store.as_deref(),
        )
        .map_err(|_| "Unable to resolve selected aura polarity metadata")?
    };

    crate::aura_session::prepare_selected_aura_creation(
        host,
        difficulty_id,
        metadata,
        spell_id_u32,
        caster_guid,
        active_effect_mask,
        cast_id,
    )
}
