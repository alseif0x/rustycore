// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Exact selected spell catalog reads and their source difficulty chain.

use wow_data::{
    DifficultyStore, SpellAuraOptionsEntry, SpellAuraOptionsStore, SpellClassOptionsStore,
    SpellLevelsStore, SpellMiscEntry, SpellMiscStore, SpellStore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedCatalogError {
    DifficultyUnavailable,
    DifficultyCycle,
}

fn resolve_selected_value<T>(
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
    has_any_row: bool,
    mut exact_value: impl FnMut(u8) -> Option<T>,
) -> Result<Option<T>, SelectedCatalogError> {
    if !has_any_row {
        return Ok(None);
    }

    let mut current_difficulty = requested_difficulty;
    let mut visited = [false; 256];
    loop {
        let index = usize::from(current_difficulty);
        if visited[index] {
            return Err(SelectedCatalogError::DifficultyCycle);
        }
        visited[index] = true;

        if let Some(value) = exact_value(current_difficulty) {
            return Ok(Some(value));
        }

        let difficulty_store = difficulties.ok_or(SelectedCatalogError::DifficultyUnavailable)?;
        let Some(difficulty) = difficulty_store.get(u32::from(current_difficulty)) else {
            return Ok(None);
        };
        current_difficulty = difficulty.fallback_difficulty_id;
    }
}

pub(crate) fn selected_spell_attributes(
    store: &SpellStore,
    spell_id: i32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<[u32; 15]>, SelectedCatalogError> {
    let has_any_row = (u8::MIN..=u8::MAX).any(|difficulty_id| {
        store
            .misc_attributes_for_exact_difficulty(spell_id, difficulty_id)
            .is_some()
    });

    resolve_selected_value(
        requested_difficulty,
        difficulties,
        has_any_row,
        |difficulty_id| store.misc_attributes_for_exact_difficulty(spell_id, difficulty_id),
    )
}

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

/// Resolve SpellClassOptions from difficulty zero through the source chain.
///
/// The store is keyed by spell and record ID; its selected row is available
/// only when the chain reaches difficulty zero. The highest record ID wins.
pub(crate) fn selected_spell_family(
    store: &SpellClassOptionsStore,
    spell_id: i32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<(u16, [u32; 4]), SelectedCatalogError> {
    let Some(row) = store
        .entries_like_cpp()
        .filter(|entry| entry.spell_id == spell_id)
        .max_by_key(|entry| entry.id)
    else {
        return Ok((0, [0; 4]));
    };

    resolve_selected_value(requested_difficulty, difficulties, true, |difficulty_id| {
        (difficulty_id == 0).then_some((u16::from(row.spell_class_set), row.spell_class_mask))
    })
    .map(|family| family.unwrap_or((0, [0; 4])))
}

pub(crate) fn selected_aura_options_entry<'a>(
    store: &'a SpellAuraOptionsStore,
    spell_id: u32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<&'a SpellAuraOptionsEntry>, SelectedCatalogError> {
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
