//! Exact spell metadata reads and difficulty fallback traversal.

use super::{MAX_SPELL_EFFECTS_LIKE_CPP, SpellEffectInfo, SpellStore};
use crate::difficulty::DifficultyStore;

/// A difficulty catalog failure while resolving spell metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSpellCatalogResolutionError {
    DifficultyUnavailable,
    DifficultyCycle,
}

/// Resolve one catalog value at the requested difficulty or along its known
/// fallback chain. The exact lookup runs before the difficulty graph is read.
pub fn resolve_spell_catalog_value<T>(
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
    has_any_row: bool,
    mut exact_value: impl FnMut(u8) -> Option<T>,
) -> Result<Option<T>, DataSpellCatalogResolutionError> {
    if !has_any_row {
        return Ok(None);
    }

    let mut current_difficulty = requested_difficulty;
    let mut visited = [false; 256];
    loop {
        let index = usize::from(current_difficulty);
        if visited[index] {
            return Err(DataSpellCatalogResolutionError::DifficultyCycle);
        }
        visited[index] = true;

        if let Some(value) = exact_value(current_difficulty) {
            return Ok(Some(value));
        }

        let difficulty_store =
            difficulties.ok_or(DataSpellCatalogResolutionError::DifficultyUnavailable)?;
        let Some(difficulty) = difficulty_store.get(u32::from(current_difficulty)) else {
            return Ok(None);
        };
        current_difficulty = difficulty.fallback_difficulty_id;
    }
}

/// Resolve the `SpellMisc` attribute words for one spell through its source
/// difficulty chain.
pub fn resolve_spell_attributes(
    store: &SpellStore,
    spell_id: i32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<[u32; 15]>, DataSpellCatalogResolutionError> {
    let has_any_row = (u8::MIN..=u8::MAX).any(|difficulty_id| {
        store
            .misc_attributes_for_exact_difficulty(spell_id, difficulty_id)
            .is_some()
    });

    resolve_spell_catalog_value(
        requested_difficulty,
        difficulties,
        has_any_row,
        |difficulty_id| store.misc_attributes_for_exact_difficulty(spell_id, difficulty_id),
    )
}

/// A malformed effect vector or difficulty graph that prevents exact effect
/// resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellEffectResolutionError {
    Catalog(DataSpellCatalogResolutionError),
    InvalidEffectIndex { effect_index: u32 },
    DuplicateEffectIndex { effect_index: u32 },
}

/// Resolve each indexed effect independently through the spell's difficulty
/// chain. Returned rows are borrowed from the canonical store and ordered by
/// effect index; explicit inert rows remain present and stop inheritance.
pub fn resolve_spell_effect_rows<'a>(
    store: &'a SpellStore,
    spell_id: i32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<Vec<&'a SpellEffectInfo>, SpellEffectResolutionError> {
    const EFFECT_SLOT_COUNT: usize = MAX_SPELL_EFFECTS_LIKE_CPP as usize;

    let mut rows_by_difficulty: [Option<&[SpellEffectInfo]>; 256] = [None; 256];
    let mut available_effect_indices = [false; EFFECT_SLOT_COUNT];

    for difficulty_id in u8::MIN..=u8::MAX {
        let Some(rows) = store.effect_rows_for_exact_difficulty(spell_id, difficulty_id) else {
            continue;
        };
        rows_by_difficulty[usize::from(difficulty_id)] = Some(rows);

        let mut seen_effect_indices = [false; EFFECT_SLOT_COUNT];
        for row in rows {
            let effect_index = usize::try_from(row.effect_index).map_err(|_| {
                SpellEffectResolutionError::InvalidEffectIndex {
                    effect_index: row.effect_index,
                }
            })?;
            if effect_index >= EFFECT_SLOT_COUNT {
                return Err(SpellEffectResolutionError::InvalidEffectIndex {
                    effect_index: row.effect_index,
                });
            }
            if seen_effect_indices[effect_index] {
                return Err(SpellEffectResolutionError::DuplicateEffectIndex {
                    effect_index: row.effect_index,
                });
            }
            seen_effect_indices[effect_index] = true;
            available_effect_indices[effect_index] = true;
        }
    }

    if !available_effect_indices.iter().any(|available| *available) {
        return Ok(Vec::new());
    }

    let mut resolved_effects = Vec::new();
    for (effect_index, available) in available_effect_indices.into_iter().enumerate() {
        if !available {
            continue;
        }

        let resolved = resolve_spell_catalog_value(
            requested_difficulty,
            difficulties,
            true,
            |difficulty_id| {
                rows_by_difficulty[usize::from(difficulty_id)]?
                    .iter()
                    .find(|row| usize::try_from(row.effect_index).ok() == Some(effect_index))
            },
        )
        .map_err(SpellEffectResolutionError::Catalog)?;

        if let Some(effect) = resolved {
            resolved_effects.push(effect);
        }
    }

    Ok(resolved_effects)
}
