// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed catalog facts needed by selected Aura wire projection.

use super::selected_wire::SelectedAuraWireFacts;
use crate::aura_session::selected_catalog::{
    SelectedCatalogError, selected_aura_options_entry as resolve_aura_options_entry,
    selected_spell_attributes, selected_spell_info_key_difficulty,
};
use wow_data::character_progression::ChrRacesStore;
use wow_data::{DifficultyStore, SpellAuraOptionsStore, SpellStore};
use wow_entities::{
    AuraApplicationLikeCpp as AuraApplication, AuraCastProvenanceLikeCpp as AuraCastProvenance,
    AuraPersistenceMetadata,
};

/// Immutable stores consulted while preparing a selected Player Aura frame.
///
/// The view is transient; ownership remains with the session's catalog bundle.
#[derive(Clone, Copy)]
pub struct SelectedAuraPublicationCatalogs<'a> {
    pub races: Option<&'a ChrRacesStore>,
    pub spells: Option<&'a SpellStore>,
    pub aura_options: Option<&'a SpellAuraOptionsStore>,
    pub difficulties: Option<&'a DifficultyStore>,
}

pub(crate) fn sickness_spell_for_race(
    catalogs: SelectedAuraPublicationCatalogs<'_>,
    race_id: u8,
) -> Result<i32, &'static str> {
    let race = catalogs
        .races
        .ok_or("selected aura race catalog is unavailable")?
        .get(u32::from(race_id))
        .ok_or("selected aura source race is unavailable")?;
    if race.res_sickness_spell_id < 0 {
        return Err("selected aura source sickness spell is invalid");
    }
    Ok(race.res_sickness_spell_id)
}

pub(crate) fn is_selected_aura_spell(spell_id: i32, sickness_spell_id: i32) -> bool {
    matches!(spell_id, 8_326 | 20_584) || (sickness_spell_id > 0 && spell_id == sickness_spell_id)
}

pub(crate) fn selected_aura_wire_facts(
    aura: &AuraApplication,
    provenance: &AuraCastProvenance,
    persistence: Option<&AuraPersistenceMetadata>,
    catalogs: SelectedAuraPublicationCatalogs<'_>,
) -> Result<SelectedAuraWireFacts, &'static str> {
    let captured_caster_level = provenance
        .caster_level
        .ok_or("selected aura caster level is unavailable")?;
    let persistence = persistence.ok_or("selected aura persistence metadata is unavailable")?;
    let spell_store = catalogs
        .spells
        .ok_or("selected aura spell catalog is unavailable")?;
    let spell_id =
        u32::try_from(aura.spell_id).map_err(|_| "selected aura spell identity is invalid")?;
    let attributes = match selected_misc_attributes(
        spell_store,
        aura.spell_id,
        aura.difficulty_id,
        catalogs.difficulties,
    )? {
        Some(attributes) => attributes,
        None => {
            if selected_spell_info_key_difficulty(
                spell_store,
                spell_id,
                aura.difficulty_id,
                catalogs.difficulties,
            )
            .map_err(|error| selected_spell_info_error(error))?
            .is_some()
            {
                // C++ initializes SpellInfo misc attributes to zero when its
                // resolved key exists without any SpellMisc row.
                [0; 15]
            } else {
                return Err("selected aura spell attributes are unavailable");
            }
        }
    };
    let options = catalogs
        .aura_options
        .ok_or("selected aura options catalog is unavailable")?;
    let catalog_stack_amount =
        selected_aura_options_entry(options, spell_id, aura.difficulty_id, catalogs.difficulties)?
            .map_or(0, |entry| entry.cumulative_aura);

    Ok(SelectedAuraWireFacts {
        captured_caster_level,
        cast_item_level: persistence.cast_item_level,
        catalog_stack_amount,
        canonical_charges: persistence.charges,
        attributes5: attributes[5],
        attributes11: attributes[11],
    })
}

fn selected_misc_attributes(
    store: &SpellStore,
    spell_id: i32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<[u32; 15]>, &'static str> {
    selected_spell_attributes(store, spell_id, requested_difficulty, difficulties)
        .map_err(selected_spell_attributes_error)
}

fn selected_aura_options_entry<'a>(
    store: &'a SpellAuraOptionsStore,
    spell_id: u32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<&'a wow_data::SpellAuraOptionsEntry>, &'static str> {
    resolve_aura_options_entry(store, spell_id, requested_difficulty, difficulties)
        .map_err(selected_aura_options_error)
}

fn selected_spell_attributes_error(error: SelectedCatalogError) -> &'static str {
    match error {
        SelectedCatalogError::DifficultyUnavailable => {
            "selected aura spell difficulty catalog is unavailable"
        }
        SelectedCatalogError::DifficultyCycle => {
            "selected aura spell difficulty fallback contains a cycle"
        }
    }
}

fn selected_aura_options_error(error: SelectedCatalogError) -> &'static str {
    match error {
        SelectedCatalogError::DifficultyUnavailable => {
            "selected aura options difficulty catalog is unavailable"
        }
        SelectedCatalogError::DifficultyCycle => {
            "selected aura options difficulty fallback contains a cycle"
        }
    }
}

fn selected_spell_info_error(error: SelectedCatalogError) -> &'static str {
    match error {
        SelectedCatalogError::DifficultyUnavailable => {
            "selected aura spell difficulty catalog is unavailable"
        }
        SelectedCatalogError::DifficultyCycle => {
            "selected aura spell difficulty fallback contains a cycle"
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/selected_catalog.rs"]
mod tests;
