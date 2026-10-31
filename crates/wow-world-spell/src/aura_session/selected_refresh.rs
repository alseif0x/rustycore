// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read the original selected Aura facts used by stacking and timer refresh.

use wow_data::{
    DifficultyStore, SpellAuraOptionsStore, SpellDurationStore, SpellMiscStore, SpellStore,
};

use super::selected_catalog::{
    SelectedCatalogError, selected_aura_options_entry, selected_spell_info_key_difficulty,
    selected_spell_misc_entry,
};

const SPELL_ATTR0_PASSIVE: u32 = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;

/// The immutable spell facts owned by an existing selected Aura.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedAuraRefreshPolicy {
    pub actual_difficulty: u8,
    pub stack_amount: u32,
    pub attributes1: u32,
    pub attributes5: u32,
    pub attributes13: u32,
    /// Missing duration data is harmless until the old Aura refreshes timers.
    pub base_duration_ms: Option<i32>,
}

/// Resolve old Aura policy facts from its raw application difficulty.
///
/// `SpellAuras.cpp:933-952,1049-1090` and `Unit.cpp:3247-3276` at
/// `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` keep stack and refresh metadata
/// on the existing Aura's `SpellInfo`. The canonical key is resolved before
/// reading misc, AuraOptions, or duration rows. Duration remains optional so a
/// non-timer refresh does not depend on an unused duration catalog.
pub fn selected_aura_refresh_policy(
    spell_id: u32,
    raw_difficulty: u8,
    spells: &SpellStore,
    misc_store: Option<&SpellMiscStore>,
    duration_store: Option<&SpellDurationStore>,
    aura_options: Option<&SpellAuraOptionsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<SelectedAuraRefreshPolicy, &'static str> {
    let actual_difficulty =
        selected_spell_info_key_difficulty(spells, spell_id, raw_difficulty, difficulties)
            .map_err(catalog_error)?
            .ok_or("Existing selected Aura SpellInfo key is missing")?;

    let misc = match misc_store {
        Some(store) => selected_spell_misc_entry(store, spell_id, actual_difficulty, difficulties)
            .map_err(catalog_error)?,
        None => return Err("Missing SpellMisc catalog for existing selected Aura policy"),
    };
    let options =
        aura_options.ok_or("Missing SpellAuraOptions catalog for existing selected Aura policy")?;
    let stack_amount =
        selected_aura_options_entry(options, spell_id, actual_difficulty, difficulties)
            .map_err(catalog_error)?
            .map_or(0, |entry| entry.cumulative_aura);

    let attributes = misc.map_or([0; 15], |entry| entry.attributes.map(|value| value as u32));
    let base_duration_ms = match (misc, duration_store) {
        (Some(entry), Some(durations)) => {
            let index = u32::from(entry.duration_index);
            let duration = if index == 0 {
                None
            } else {
                durations.get(index)
            };
            Some(match duration {
                Some(row) if row.duration == -1 => -1,
                Some(row) => row
                    .duration
                    .checked_abs()
                    .ok_or("Existing selected Aura duration exceeds the supported range")?,
                None if attributes[0] & SPELL_ATTR0_PASSIVE != 0 => -1,
                None => 0,
            })
        }
        _ => None,
    };

    Ok(SelectedAuraRefreshPolicy {
        actual_difficulty,
        stack_amount,
        attributes1: attributes[1],
        attributes5: attributes[5],
        attributes13: attributes[13],
        base_duration_ms,
    })
}

fn catalog_error(error: SelectedCatalogError) -> &'static str {
    match error {
        wow_data::spell::DataSpellCatalogResolutionError::DifficultyUnavailable => {
            "Selected Aura catalog difficulty is unavailable"
        }
        wow_data::spell::DataSpellCatalogResolutionError::DifficultyCycle => {
            "Selected Aura catalog difficulty fallback contains a cycle"
        }
    }
}
