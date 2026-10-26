// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact selected spell catalog reads and their source difficulty chain.

pub(crate) use wow_data::spell::{
    DataSpellCatalogResolutionError as SelectedCatalogError,
    resolve_spell_aura_options_entry as selected_aura_options_entry,
    resolve_spell_attributes as selected_spell_attributes,
    resolve_spell_catalog_value as resolve_selected_value,
    resolve_spell_family as selected_spell_family,
};
use wow_data::{DifficultyStore, SpellLevelsStore, SpellMiscEntry, SpellMiscStore, SpellStore};

/// Resolve the effective `SpellCategories` mechanic along a spell's source
/// difficulty chain. An absent category row uses SpellInfo's zero default.
pub(crate) fn selected_spell_mechanic(
    store: &SpellStore,
    spell_id: i32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<i32, SelectedCatalogError> {
    let has_any_row = (u8::MIN..=u8::MAX).any(|difficulty_id| {
        store
            .spell_mechanic_for_exact_difficulty(spell_id, difficulty_id)
            .is_some()
    });

    resolve_selected_value(
        requested_difficulty,
        difficulties,
        has_any_row,
        |difficulty_id| store.spell_mechanic_for_exact_difficulty(spell_id, difficulty_id),
    )
    .map(|mechanic| i32::from(mechanic.unwrap_or(0)))
}

pub(crate) fn selected_spell_level(
    store: &SpellLevelsStore,
    spell_id: u32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<u8, SelectedCatalogError> {
    let has_any_row = store
        .entries_like_cpp()
        .any(|entry| entry.spell_id == spell_id);

    resolve_selected_value(
        requested_difficulty,
        difficulties,
        has_any_row,
        |difficulty_id| {
            store
                .entries_like_cpp()
                .filter(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
                .max_by_key(|entry| entry.id)
                .map(|entry| entry.spell_level as u8)
        },
    )
    .map(|level| level.unwrap_or(0))
}

pub(crate) fn selected_spell_misc_entry<'a>(
    store: &'a SpellMiscStore,
    spell_id: u32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<&'a SpellMiscEntry>, SelectedCatalogError> {
    let has_any_row = store
        .entries_like_cpp()
        .any(|entry| entry.spell_id == spell_id);

    resolve_selected_value(
        requested_difficulty,
        difficulties,
        has_any_row,
        |difficulty_id| {
            store
                .entries_like_cpp()
                .filter(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
                .max_by_key(|entry| entry.id)
        },
    )
}

pub(crate) fn selected_spell_info_key_difficulty(
    store: &SpellStore,
    spell_id: u32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<u8>, SelectedCatalogError> {
    let has_any_row = store.contains_spell_info_any_difficulty_like_cpp(spell_id);

    resolve_selected_value(
        requested_difficulty,
        difficulties,
        has_any_row,
        |difficulty_id| {
            store
                .contains_spell_info_exact_like_cpp(spell_id, difficulty_id)
                .then_some(difficulty_id)
        },
    )
}

#[cfg(test)]
#[path = "../../unit_tests/aura_session/selected_catalog.rs"]
mod tests;
