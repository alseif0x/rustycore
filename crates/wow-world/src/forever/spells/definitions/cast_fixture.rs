//! Synthetic semantic definitions for the cast resolver, not real data/Unit QA.
use super::{
    Definition, SpellConstructorFields, SpellDefinitionSeeds, SpellEffectValues, learning_fixtures,
};
use wow_data::forever_spells::{DifficultyRecord, SpellRecords};

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
