//! Synthetic rows, actual learning phase algorithms; not native startup proof.
use super::*;
use wow_data::forever_birth::BirthCatalog;
use wow_persistence::forever::spells::{SpellLearnRow, SpellRequiredRow};
pub(crate) fn spell_book_test_definitions(
    birth: Arc<BirthCatalog>,
    rows: Vec<(u32, SpellConstructorFields, u32, Vec<SpellEffectValues>)>,
    required: Vec<SpellRequiredRow>,
    learned: Vec<SpellLearnRow>,
) -> SpellDefinitionSeeds {
    let mut seeds = skill_set_test_definitions(birth);
    for (id, fields, custom_attributes, effects) in rows {
        let mut definition = Definition::empty_server(vec![]);
        definition.fields = fields;
        definition.custom_attributes = custom_attributes;
        definition.effects = effects;
        seeds.definitions.insert((id, 0), definition);
    }
    let keys = seeds.definitions.keys().copied().collect::<Vec<_>>();
    seeds.source_order = Some(DefinitionOrder {
        primary: keys.clone(),
        by_spell: keys.iter().map(|key| (key.0, vec![*key])).collect(),
    });
    seeds
        .with_spell_ranks()
        .unwrap()
        .with_spell_required(required)
        .unwrap()
        .with_learn_skills(&learning_fixtures::items(), &mut learning_fixtures::no_draw)
        .unwrap()
        .with_specific_and_aura_state()
        .unwrap()
        .with_learn_spells(learned)
        .unwrap()
}
