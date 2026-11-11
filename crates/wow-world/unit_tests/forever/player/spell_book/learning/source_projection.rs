//! Actual World admission/projection half of the private-domain fixture split.
use super::*;
use wow_data::forever_birth::BirthRecords;
use wow_entities::{SpellBookEffect, SpellDefinitionSource, SpellLearningSource};
use wow_persistence::forever::{
    creation::SkillTierRow,
    spells::{SpellLearnRow, SpellRequiredRow},
};

#[test]
fn admitted_rank_projection_preserves_chain_keys_ranks_and_missing_definition_links() {
    let mut fixture = Fixture::new(
        vec![row(1), row(2), row(3), row(4)],
        BirthRecords {
            abilities: vec![
                ability(1, 1, 2, 0),
                ability(2, 2, 3, 0),
                ability(3, 3, 4, 0),
                ability(4, 0, 99, 0),
            ],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    // Original direct source assertions retained from the moved profession case.
    assert_eq!(fixture.spells.spell_rank_node(1).unwrap().rank, 1);
    assert_eq!(fixture.spells.spell_rank_node(2).unwrap().rank, 2);
    let sources = fixture.sources();
    for (id, rank, previous, next) in [(1, 1, 0, 2), (2, 2, 1, 3), (3, 3, 2, 4), (4, 4, 3, 0)] {
        let node = sources.spell_rank_node(id).unwrap();
        assert_eq!((node.first, node.rank), ((1, 0), rank));
        assert_eq!(sources.previous_spell_in_chain(id), previous);
        assert_eq!(sources.next_spell_in_chain(id), next);
        assert_eq!(sources.first_spell_in_chain(id), 1);
    }
    for id in [0, 99] {
        assert!(sources.spell_rank_node(id).is_none());
        assert_eq!(sources.first_spell_in_chain(id), id);
        assert_eq!(sources.previous_spell_in_chain(id), 0);
        assert_eq!(sources.next_spell_in_chain(id), 0);
    }
    // The cfg(test) helper seeds definitions for ability spell/before IDs,
    // including before=0. This is synthetic helper coverage, not evidence that
    // real DB2/native startup admits a zero-ID definition.
    assert!(sources.definition(0, 0).unwrap().is_some());
    // Deliberate post-admission test inconsistency: rank links remain while a
    // definition disappears. This protects the missing asserted-next guard.
    crate::forever::spells::skill_set_test_spell_fields(&mut fixture.spells, 2, None);
    let sources = fixture.sources();
    assert!(sources.definition(2, 0).unwrap().is_none());
    assert_eq!(sources.next_spell_in_chain(1), 2);
    assert_eq!(sources.spell_rank_node(2).unwrap().first, (1, 0));
}

#[test]
fn admitted_learn_skill_projection_preserves_physical_precedence_steps_and_tier_answers() {
    let mut rows = vec![];
    for (id, skill, step) in [
        (0, 9, 1.),
        (1, 5, 0.),
        (2, 5, 1.),
        (3, 5, 2.),
        (4, 6, 2.),
        (5, 7, 3.),
    ] {
        let mut spell = row(id);
        spell.3.push(spell_effect(118, 0, skill, step));
        rows.push(spell);
    }
    let mut weapon = row(6);
    weapon.3 = vec![
        spell_effect(155, 0, 0, 0.),
        spell_effect(0, 0, 0, 0.),
        spell_effect(40, 0, 0, 0.),
    ];
    rows.push(weapon);
    let mut values = [0; 16];
    values[0] = 75;
    values[15] = 70000;
    let fixture = Fixture::new(
        rows,
        BirthRecords {
            skill_lines: vec![line(5, 9), line(6, 7), line(7, 7), line(9, 7)],
            race_class: vec![rc(1, 5, 1)],
            ..Default::default()
        },
        vec![SkillTierRow { id: 1, values }],
        vec![],
        vec![],
    );
    let sources = fixture.sources();
    // Zero here is an explicit synthetic raw/helper input, not native admission
    // proof (the helper can also seed zero via ability before/spell placeholders).
    for (id, expected) in [
        (0, (9, 1, 0, 0)),
        (1, (5, 0, 0, 0)),
        (2, (5, 1, 0, 0)),
        (3, (5, 2, 0, 0)),
        (4, (6, 2, 0, 0)),
        (5, (7, 3, 0, 0)),
        (6, (118, 1, 1, 1)),
    ] {
        let node = sources.spell_learn_skill(id).unwrap();
        assert_eq!(
            (node.skill, node.step, node.value, node.max_value),
            expected
        );
    }
    let view = sources.definition(6, 0).unwrap().unwrap();
    assert_eq!(view.effect_count(), 3);
    assert_eq!(
        view.effects()
            .map(|effect| effect.effect_kind())
            .collect::<Vec<_>>(),
        [155, 0, 40]
    );
    assert_eq!(sources.skill_tier_value(1, 0), Some(75));
    assert_eq!(sources.skill_tier_value(1, u32::MAX), Some(70000));
}

fn relations(
    nodes: impl Iterator<Item = wow_entities::SpellLearningNode>,
) -> Vec<(u32, u32, u32, bool, bool)> {
    nodes
        .map(|node| {
            (
                node.source,
                node.spell,
                node.overrides_spell,
                node.active,
                node.auto_learned,
            )
        })
        .collect()
}

#[test]
fn admitted_required_and_learning_projections_preserve_input_orders_and_cycles() {
    let mut fixture = Fixture::new(
        vec![row(1), row(2), row(3)],
        Default::default(),
        vec![],
        vec![
            SpellRequiredRow {
                spell: 3,
                required: 1,
            },
            SpellRequiredRow {
                spell: 2,
                required: 1,
            },
            SpellRequiredRow {
                spell: 1,
                required: 2,
            },
        ],
        vec![
            SpellLearnRow {
                source: 1,
                learned: 2,
                active: true,
            },
            SpellLearnRow {
                source: 2,
                learned: 1,
                active: true,
            },
            SpellLearnRow {
                source: 1,
                learned: 3,
                active: false,
            },
            SpellLearnRow {
                source: 2,
                learned: 3,
                active: true,
            },
        ],
    );
    // Numeric override/auto injection after admission, not DB2 derivation proof.
    crate::forever::spells::spell_book_test_learn_flags(&mut fixture.spells, 1, 3, 777, true);
    let sources = fixture.sources();
    assert_eq!(
        sources.spells_requiring(1).unwrap().collect::<Vec<_>>(),
        [3, 2]
    );
    assert_eq!(
        sources.spells_requiring(2).unwrap().collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        relations(sources.spell_learn_nodes(1).unwrap()),
        [(1, 2, 0, true, false), (1, 3, 777, false, true)]
    );
    assert_eq!(
        relations(sources.spell_learn_nodes(2).unwrap()),
        [(2, 1, 0, true, false), (2, 3, 0, true, false)]
    );
    assert_eq!(
        relations(sources.spell_learned_by(1).unwrap()),
        [(2, 1, 0, true, false)]
    );
    assert_eq!(
        relations(sources.spell_learned_by(2).unwrap()),
        [(1, 2, 0, true, false)]
    );
    assert_eq!(
        relations(sources.spell_learned_by(3).unwrap()),
        [(1, 3, 777, false, true), (2, 3, 0, true, false)]
    );
    let mut teacher = row(4);
    let mut effect = spell_effect(36, 0, 0, 0.);
    effect.trigger_spell = 7;
    teacher.3.push(effect);
    let empty_sql = Fixture::simple(vec![teacher, row(7)]);
    let sources = empty_sql.sources();
    assert_eq!(relations(sources.spell_learn_nodes(4).unwrap()), []);
    assert_eq!(relations(sources.spell_learned_by(7).unwrap()), []);
}

#[test]
fn known_invalid_lower_rank_keeps_rank_metadata_and_real_template_validity() {
    let mut lower = row(1);
    let mut craft = spell_effect(24, 0, 0, 0.);
    craft.item_type = 99;
    lower.3.push(craft);
    let fixture = Fixture::new(
        vec![lower, row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let sources = fixture.sources();
    assert!(sources.definition(1, 0).unwrap().is_some());
    // Actual admitted NumericItemTemplates is empty: item 99 cannot satisfy the
    // crafting definition. An absent definition would exercise a different path.
    assert!(!sources.spell_is_valid(1).unwrap());
    assert!(sources.spell_is_valid(2).unwrap());
    assert_eq!(
        (
            sources.spell_rank_node(1).unwrap().first,
            sources.spell_rank_node(1).unwrap().rank
        ),
        ((1, 0), 1)
    );
    assert_eq!(
        (
            sources.spell_rank_node(2).unwrap().first,
            sources.spell_rank_node(2).unwrap().rank
        ),
        ((1, 0), 2)
    );
}
