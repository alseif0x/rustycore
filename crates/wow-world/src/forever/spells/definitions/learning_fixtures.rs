//! Narrow synthetic startup fixtures; not source-container or live acceptance.
use super::*;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{BirthRecords, SkillAbilityRecord},
    forever_spells::*,
};
pub(super) fn ability(id: u32, before: u32, after: u32) -> SkillAbilityRecord {
    SkillAbilityRecord {
        id,
        skill_line: 0,
        spell: after as i32,
        min_skill_rank: 0,
        class_mask: 0,
        supercedes_spell: before as i32,
        acquire_method: 0,
        trivial_rank_high: 0,
        trivial_rank_low: 0,
        flags: 0,
        num_skill_ups: 0,
        unique_bit: 0,
        trade_skill_category: 0,
        skillup_skill_line: 0,
        field_5_5_4_67090_014: [0; 2],
        race_mask: 0,
    }
}
pub(super) fn effect(kind: u32, aura: u32, misc: i32, base: f32) -> SpellEffectValues {
    SpellEffectValues {
        effect: kind,
        aura,
        misc_values: [misc, 0],
        base_points: base,
        ..Default::default()
    }
}
pub(super) fn definition(effects: Vec<SpellEffectValues>) -> Definition {
    let mut result = Definition::empty_server(vec![]);
    result.effects = effects;
    result
}
pub(super) fn seeds(
    rows: Vec<(Key, Definition)>,
    raw: SpellRecords,
    birth: BirthRecords,
) -> SpellDefinitionSeeds {
    let catalog = Arc::new(
        raw.finish(
            Default::default(),
            Default::default(),
            6,
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap(),
    );
    let birth = Arc::new(
        birth
            .finish(
                Default::default(),
                Default::default(),
                &Db2HotfixRemovalStoreLikeCpp::default(),
            )
            .unwrap(),
    );
    let mut s = SpellLoadPlan::build(catalog)
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap();
    let primary = rows.iter().map(|&(key, _)| key).collect::<Vec<_>>();
    let mut by_spell = BTreeMap::<u32, Vec<Key>>::new();
    for &key in &primary {
        by_spell.entry(key.0).or_default().push(key);
    }
    s.definitions = rows.into_iter().collect();
    s.source_order = Some(DefinitionOrder { primary, by_spell });
    s.global_corrections = Some(Default::default());
    s = s.with_skill_line_abilities(birth).unwrap();
    s.target_caps = Some(Default::default());
    s
}
pub(super) fn items() -> wow_data::forever_birth::item_records::ItemCatalog {
    wow_data::forever_birth::item_records::ItemRecords::default()
        .finish(
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap()
}
pub(super) fn no_draw(_: f32, _: f32) -> Result<f32, SpellValueError> {
    panic!("unexpected draw")
}
pub(super) fn before_skills(rows: Vec<(Key, Definition)>) -> SpellDefinitionSeeds {
    seeds(rows, Default::default(), Default::default())
        .with_spell_ranks()
        .unwrap()
        .with_spell_required(vec![])
        .unwrap()
}
pub(super) fn before_specific(rows: Vec<(Key, Definition)>) -> SpellDefinitionSeeds {
    before_skills(rows)
        .with_learn_skills(&items(), &mut no_draw)
        .unwrap()
}
pub(super) fn before_learn(
    rows: Vec<(Key, Definition)>,
    raw: SpellRecords,
) -> SpellDefinitionSeeds {
    seeds(rows, raw, Default::default())
        .with_spell_ranks()
        .unwrap()
        .with_spell_required(vec![])
        .unwrap()
        .with_learn_skills(&items(), &mut no_draw)
        .unwrap()
        .with_specific_and_aura_state()
        .unwrap()
}
