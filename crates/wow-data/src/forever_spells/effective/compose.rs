//! Source batch semantics, especially repeated new IDs and localized slots.
use super::super::*;
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

fn baseline<T>(rows: Vec<T>, id: &impl Fn(&T) -> u32) -> Result<BTreeMap<u32, T>> {
    let mut result = BTreeMap::new();
    for row in rows {
        ensure!(
            result.insert(id(&row), row).is_none(),
            "Duplicate spell baseline ID"
        );
    }
    Ok(result)
}
pub(super) fn numeric<T>(
    rows: Vec<T>,
    official: Vec<T>,
    custom: Vec<T>,
    id: impl Fn(&T) -> u32,
) -> Result<BTreeMap<u32, T>> {
    let mut result = baseline(rows, &id)?;
    for row in official.into_iter().chain(custom) {
        result.insert(id(&row), row);
    }
    Ok(result)
}

/// Source DB2DatabaseLoader publishes index size only when a SQL batch adds
/// a record absent at that batch's start. A bigger MAX query with only
/// overwrites does NOT update GetNumRows (early return at :159-165).
/// The file's bound includes unresolved copies; final removal never shrinks.
pub(super) fn random_points(
    rows: Vec<RandPropPointsRecord>,
    official: Vec<RandPropPointsRecord>,
    custom: Vec<RandPropPointsRecord>,
    file_last_index: u32,
    official_size: Option<u32>,
    custom_size: Option<u32>,
) -> Result<(BTreeMap<u32, RandPropPointsRecord>, u32)> {
    let mut result = baseline(rows, &|row: &RandPropPointsRecord| row.id)?;
    // Wider temporary arithmetic retains an undefined source overflow as raw
    // metadata; rand_prop_points_or_last refuses it before calculation.
    let mut index_size = u64::from(file_last_index) + 1;
    let mut allocated_size = index_size;
    for (batch, observed_size) in [(official, official_size), (custom, custom_size)] {
        if batch.is_empty() {
            ensure!(
                observed_size.is_none(),
                "Empty SQL batch cannot assert a storage query"
            );
            continue;
        }
        let allocation = index_size.max(observed_size.map(u64::from).unwrap_or_else(|| {
            // SQL-free synthetic batches only. The production adapter always
            // supplies its separate query observation for a nonempty batch.
            batch
                .iter()
                .map(|row| u64::from(row.id) + 1)
                .max()
                .unwrap_or(0)
        }));
        // Source may have grown the pointer array without publishing records
        // after an overwrite-only batch. A later query grows/replaces it only
        // when its observed size exceeds records, not its physical capacity.
        if allocation > index_size {
            allocated_size = allocation;
        }
        ensure!(
            batch.iter().all(|row| u64::from(row.id) < allocated_size),
            "SQL random-property row exceeds observed source allocation"
        );
        let adds_record = batch.iter().any(|row| !result.contains_key(&row.id));
        for row in batch {
            result.insert(row.id, row);
        }
        if adds_record {
            index_size = allocation;
        }
    }
    // Concurrent SQL deletion between result and MAX can leave a source row
    // in a physical slot outside GetNumRows. LookupEntry cannot expose it.
    result.retain(|id, _| u64::from(*id) < index_size);
    Ok((result, (index_size - 1) as u32))
}
pub(super) fn localized<T>(
    rows: Vec<T>,
    official: Vec<T>,
    custom: Vec<T>,
    id: impl Fn(&T) -> u32,
    merge: impl Fn(T, Option<T>) -> Result<T>,
) -> Result<BTreeMap<u32, T>> {
    let mut result = baseline(rows, &id)?;
    for batch in [official, custom] {
        // Source new indices are published only after the SQL batch. A repeated
        // new ID starts fresh each time; an existing ID updates its live slots.
        let existing: BTreeSet<_> = result.keys().copied().collect();
        for row in batch {
            let key = id(&row);
            let previous = if existing.contains(&key) {
                result.remove(&key)
            } else {
                None
            };
            result.insert(key, merge(row, previous)?);
        }
    }
    Ok(result)
}

pub(super) fn spell_name(
    mut row: SpellNameRecord,
    previous: Option<SpellNameRecord>,
) -> Result<SpellNameRecord> {
    row.name.check_sql_main()?;
    if let Some(previous) = previous {
        row.name = previous.name.with_sql_main(row.name)?;
    }
    Ok(row)
}

pub(super) fn difficulty(
    mut row: DifficultyRecord,
    previous: Option<DifficultyRecord>,
) -> Result<DifficultyRecord> {
    row.name.check_sql_main()?;
    if let Some(previous) = previous {
        row.name = previous.name.with_sql_main(row.name)?;
    }
    Ok(row)
}

pub(super) fn spell_range(
    mut row: SpellRangeRecord,
    previous: Option<SpellRangeRecord>,
) -> Result<SpellRangeRecord> {
    row.display_name.check_sql_main()?;
    row.display_name_short.check_sql_main()?;
    if let Some(previous) = previous {
        row.display_name = previous.display_name.with_sql_main(row.display_name)?;
        row.display_name_short = previous
            .display_name_short
            .with_sql_main(row.display_name_short)?;
    }
    Ok(row)
}

pub(super) fn spell_shapeshift_form(
    mut row: SpellShapeshiftFormRecord,
    previous: Option<SpellShapeshiftFormRecord>,
) -> Result<SpellShapeshiftFormRecord> {
    row.name.check_sql_main()?;
    if let Some(previous) = previous {
        row.name = previous.name.with_sql_main(row.name)?;
    }
    Ok(row)
}

pub(super) fn battle_pet_species(
    mut row: BattlePetSpeciesRecord,
    previous: Option<BattlePetSpeciesRecord>,
) -> Result<BattlePetSpeciesRecord> {
    row.description.check_sql_main()?;
    row.source_text.check_sql_main()?;
    if let Some(previous) = previous {
        row.description = previous.description.with_sql_main(row.description)?;
        row.source_text = previous.source_text.with_sql_main(row.source_text)?;
    }
    Ok(row)
}

pub(super) fn spell_category(
    mut row: SpellCategoryRecord,
    previous: Option<SpellCategoryRecord>,
) -> Result<SpellCategoryRecord> {
    row.name.check_sql_main()?;
    if let Some(previous) = previous {
        row.name = previous.name.with_sql_main(row.name)?;
    }
    Ok(row)
}

pub(super) fn talent(
    mut row: TalentRecord,
    previous: Option<TalentRecord>,
) -> Result<TalentRecord> {
    row.description.check_sql_main()?;
    if let Some(previous) = previous {
        row.description = previous.description.with_sql_main(row.description)?;
    }
    Ok(row)
}

pub(super) fn spell_item_enchantment(
    mut row: SpellItemEnchantmentRecord,
    previous: Option<SpellItemEnchantmentRecord>,
) -> Result<SpellItemEnchantmentRecord> {
    row.name.check_sql_main()?;
    row.horde_name.check_sql_main()?;
    if let Some(previous) = previous {
        row.name = previous.name.with_sql_main(row.name)?;
        row.horde_name = previous.horde_name.with_sql_main(row.horde_name)?;
    }
    Ok(row)
}
