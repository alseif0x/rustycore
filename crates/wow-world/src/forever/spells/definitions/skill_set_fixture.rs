//! Narrow cross-owner SetSkill/rank fixture. Synthesized order is NOT native
//! replay evidence; callers execute actual rank derivation on one BirthCatalog.
use super::*;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, forever_birth::BirthCatalog, forever_spells::SpellRecords,
};

pub(crate) fn skill_set_test_definitions(birth: Arc<BirthCatalog>) -> SpellDefinitionSeeds {
    let catalog = Arc::new(
        SpellRecords::default()
            .finish(
                Default::default(),
                Default::default(),
                6,
                Default::default(),
                Default::default(),
                &Db2HotfixRemovalStoreLikeCpp::default(),
            )
            .unwrap(),
    );
    let mut seeds = SpellLoadPlan::build(catalog)
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap();
    let keys: BTreeSet<_> = birth
        .skill_ability_records()
        .flat_map(|row| [row.spell as u32, row.supercedes_spell as u32])
        .map(|id| (id, 0))
        .collect();
    seeds.definitions = keys
        .iter()
        .map(|&key| (key, Definition::empty_server(vec![])))
        .collect();
    seeds.source_order = Some(DefinitionOrder {
        primary: keys.iter().copied().collect(),
        by_spell: keys.iter().map(|&key| (key.0, vec![key])).collect(),
    });
    seeds.global_corrections = Some(Default::default());
    seeds.target_caps = Some(Default::default());
    seeds.with_skill_line_abilities(birth).unwrap()
}

pub(crate) fn skill_set_test_spell_fields(
    seeds: &mut SpellDefinitionSeeds,
    spell: u32,
    fields: Option<(u32, u32, u32)>,
) {
    if let Some((base_level, spell_level, condition)) = fields {
        let definition = seeds.definitions.get_mut(&(spell, 0)).unwrap();
        definition.fields.base_level = base_level;
        definition.fields.spell_level = spell_level;
        definition.fields.show_future_spell_player_condition_id = condition;
    } else {
        seeds.definitions.remove(&(spell, 0));
    }
}
