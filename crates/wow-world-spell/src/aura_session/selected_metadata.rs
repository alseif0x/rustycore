// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Resolve effective selected spell metadata before pure aura polarity rules.

use wow_data::{DifficultyStore, SpellClassOptionsStore, SpellEffectInfo, SpellStore};

use super::{
    SelectedSpellMetadataFacts, SelectedSpellPolarity, SelectedSpellPolarityError,
    classify_selected_spell_polarity,
    selected_catalog::{
        SelectedCatalogError, selected_spell_attributes, selected_spell_family,
        selected_spell_info_key_difficulty, selected_spell_mechanic,
    },
};

const EARTHQUAKE_SPELL_ID: i32 = 61_882;
const EARTHQUAKE_NEGATIVE_EFFECT_INDEX: u32 = 2;

/// Transient attributes and polarity for the complete selected spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectedSpellMetadata {
    pub(crate) attributes: [u32; 15],
    pub(crate) polarity: SelectedSpellPolarity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedSpellMetadataError {
    Catalog {
        spell_id: i32,
        requested_difficulty: u8,
        source: SelectedCatalogError,
    },
    SpellInfoUnavailable {
        spell_id: i32,
        requested_difficulty: u8,
    },
    Polarity(SelectedSpellPolarityError),
}

/// Resolve effective spell facts and classify every effect in the spell.
///
/// C++ anchors: `SpellMgr.cpp:675-693,2540-2544,2614-2687,4766-4769`,
/// `SpellInfo.cpp:1240-1258,4517-5000` at
/// `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.
pub(crate) fn selected_spell_metadata(
    spells: &SpellStore,
    spell_id: i32,
    requested_difficulty: u8,
    effects: &[SpellEffectInfo],
    class_options: Option<&SpellClassOptionsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<SelectedSpellMetadata, SelectedSpellMetadataError> {
    let attributes =
        selected_spell_attributes(spells, spell_id, requested_difficulty, difficulties)
            .map_err(|source| catalog_error(spell_id, requested_difficulty, source))?;
    let attributes = match attributes {
        Some(attributes) => attributes,
        None => {
            let has_regular_key = match u32::try_from(spell_id) {
                Ok(key_spell_id) => selected_spell_info_key_difficulty(
                    spells,
                    key_spell_id,
                    requested_difficulty,
                    difficulties,
                )
                .map_err(|source| catalog_error(spell_id, requested_difficulty, source))?
                .is_some(),
                Err(_) => false,
            };
            if !has_regular_key {
                return Err(SelectedSpellMetadataError::SpellInfoUnavailable {
                    spell_id,
                    requested_difficulty,
                });
            }
            [0; 15]
        }
    };

    let family = class_options
        .map(|store| selected_spell_family(store, spell_id, requested_difficulty, difficulties));
    let mechanic = selected_spell_mechanic(spells, spell_id, requested_difficulty, difficulties);

    let (family_name, family_flags) = match family.as_ref() {
        Some(Ok((family_name, family_flags))) => (Some(*family_name), Some(*family_flags)),
        _ => (None, None),
    };
    let spell_mechanic = mechanic.as_ref().ok().copied();
    let mut preexisting_negative_effect_mask = 0;
    if spell_id == EARTHQUAKE_SPELL_ID {
        preexisting_negative_effect_mask |= 1_u32 << EARTHQUAKE_NEGATIVE_EFFECT_INDEX;
    }

    let facts = SelectedSpellMetadataFacts {
        spell_id,
        attributes,
        family_name,
        family_flags,
        spell_mechanic,
        preexisting_negative_effect_mask,
    };
    let polarity =
        classify_selected_spell_polarity(facts, effects).map_err(|error| match error {
            error @ (SelectedSpellPolarityError::MissingSpellFamilyName { .. }
            | SelectedSpellPolarityError::MissingSpellFamilyFlags { .. }) => match family {
                Some(Err(source)) => catalog_error(spell_id, requested_difficulty, source),
                _ => SelectedSpellMetadataError::Polarity(error),
            },
            error @ SelectedSpellPolarityError::MissingSpellMechanic { .. } => match mechanic {
                Err(source) => catalog_error(spell_id, requested_difficulty, source),
                _ => SelectedSpellMetadataError::Polarity(error),
            },
            error => SelectedSpellMetadataError::Polarity(error),
        })?;

    Ok(SelectedSpellMetadata {
        attributes,
        polarity,
    })
}

fn catalog_error(
    spell_id: i32,
    requested_difficulty: u8,
    source: SelectedCatalogError,
) -> SelectedSpellMetadataError {
    SelectedSpellMetadataError::Catalog {
        spell_id,
        requested_difficulty,
        source,
    }
}

#[cfg(test)]
#[path = "../../unit_tests/aura_session/selected_metadata.rs"]
mod tests;
