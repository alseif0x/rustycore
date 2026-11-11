//! Synthetic semantic definitions for the cast resolver, not real data/Unit QA.
use super::{
    Definition, SpellConstructorFields, SpellDefinitionSeeds, SpellEffectValues, learning_fixtures,
};
use wow_data::forever_spells::{DifficultyRecord, SpellRecords};

type CastRow = ((u32, i16), SpellConstructorFields, Vec<SpellEffectValues>);

/// Independent derived definitions with the same borrowed raw authority. Sharing
/// the Arc here does not clone the catalog or make the derived owners identical.
pub(crate) fn cast_test_shared_catalog_pair(
    first_rows: Vec<CastRow>,
    second_rows: Vec<CastRow>,
) -> (SpellDefinitionSeeds, SpellDefinitionSeeds) {
    let first = cast_test_definitions(first_rows, vec![]);
    let mut second = cast_test_definitions(second_rows, vec![]);
    second.catalog = std::sync::Arc::clone(&first.catalog);
    assert!(std::sync::Arc::ptr_eq(&first.catalog, &second.catalog));
    (first, second)
}

pub(crate) fn cast_test_definitions(
    rows: Vec<((u32, i16), SpellConstructorFields, Vec<SpellEffectValues>)>,
    difficulties: Vec<DifficultyRecord>,
) -> SpellDefinitionSeeds {
    let rows = rows
        .into_iter()
        .map(|(key, fields, effects)| {
            let mut definition = Definition::empty_server(vec![]);
            definition.fields = fields;
            definition.effects = effects;
            (key, definition)
        })
        .collect();
    learning_fixtures::seeds(
        rows,
        SpellRecords {
            difficulties,
            ..Default::default()
        },
        Default::default(),
    )
}
