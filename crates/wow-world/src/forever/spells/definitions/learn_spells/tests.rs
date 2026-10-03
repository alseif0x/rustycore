use super::super::{Definition, SpellSpecific, learning_fixtures as f};
use super::*;
use std::sync::Arc;
use wow_data::forever_spells::{DifficultyRecord, SpellLearnSpellRecord, SpellRecords, SpellText};

fn row(source: u32, learned: u32, active: bool) -> SpellLearnRow {
    SpellLearnRow {
        source,
        learned,
        active,
    }
}
fn db2(id: u32, source: u32, learned: u32, overrides: u32) -> SpellLearnSpellRecord {
    SpellLearnSpellRecord {
        id,
        spell_id: source,
        learn_spell_id: learned as i32,
        overrides_spell_id: overrides as i32,
    }
}
fn teaching(learned: &[u32]) -> Definition {
    f::definition(
        learned
            .iter()
            .map(|&id| {
                let mut effect = f::effect(36, 0, 0, 0.0);
                effect.trigger_spell = id;
                effect
            })
            .collect(),
    )
}
fn difficulty(id: u32, fallback: i16) -> DifficultyRecord {
    DifficultyRecord {
        id,
        name: SpellText::default(),
        instance_type: 0,
        order_index: 0,
        old_enum_value: 0,
        fallback_difficulty_id: fallback,
        min_players: 0,
        max_players: 0,
        flags: 0,
        item_context: 0,
        toggle_difficulty_id: 0,
        group_size_health_curve_id: 0,
        group_size_dmg_curve_id: 0,
        group_size_spell_points_curve_id: 0,
        unknown1105: 0,
    }
}

#[test]
fn empty_sql_returns_before_effect_db2_and_reverse_passes() {
    let raw = SpellRecords {
        spell_learn_spells: vec![db2(1, 1, 2, 3)],
        ..Default::default()
    };
    let s = f::before_learn(vec![((1, 0), teaching(&[2])), ((2, 0), teaching(&[]))], raw);
    assert!(s.spell_learn_nodes(1).is_none());
    let s = s.with_learn_spells(vec![]).unwrap();
    assert_eq!(s.spell_learn_nodes(1).unwrap().count(), 0);
    assert_eq!(s.spell_learned_by(2).unwrap().count(), 0);
    assert_eq!(s.is_spell_learn_to(1, 2), Some(false));
    assert_eq!(
        s.learn_spell_counts(),
        Some(LearnSpellCounts {
            empty_sql_early_return: true,
            ..Default::default()
        })
    );
    assert!(matches!(
        s.with_learn_spells(vec![]),
        Err(SpellLearnError::AlreadyApplied)
    ));
}

#[test]
fn sql_snapshot_effect_duplicates_db2_priority_and_reverse_identity_match_multimap_order() {
    let mut talent = teaching(&[3]);
    talent.custom_attributes = TALENT;
    let definitions = vec![
        ((30, 0), talent),
        ((20, 0), teaching(&[3, 4, 4, 99])),
        ((10, 0), teaching(&[3])),
        ((5, 0), teaching(&[])),
        ((4, 0), teaching(&[])),
        ((3, 0), teaching(&[])),
        ((2, 0), teaching(&[])),
    ];
    let mut raw = SpellRecords {
        spell_learn_spells: vec![
            db2(8, 30, 3, 0),
            db2(4, 20, 4, 0),
            db2(1, 20, 2, u32::MAX),
            db2(5, 5, 3, 0),
            db2(2, 20, 2, 55),
            db2(3, 10, 3, 0),
            db2(6, 99, 3, 0),
            db2(7, 5, 99, 0),
        ],
        ..Default::default()
    };
    raw.unknown_baseline_records[31] = 7;
    let s = f::before_learn(definitions, raw);
    let raw = s.raw_catalog();
    let s = s
        .with_learn_spells(vec![
            row(20, 3, false),
            row(20, 3, true),
            row(10, 3, false),
            row(30, 3, false),
            row(99, 3, true),
            row(5, 99, true),
        ])
        .unwrap();
    assert!(Arc::ptr_eq(&raw, &s.raw_catalog()));
    let nodes = s.spell_learn_nodes(20).unwrap().collect::<Vec<_>>();
    assert_eq!(
        nodes
            .iter()
            .map(|n| (n.spell, n.active, n.auto_learned, n.overrides_spell))
            .collect::<Vec<_>>(),
        [
            (3, false, false, 0),
            (3, true, false, 0),
            (4, true, false, 0),
            (4, true, false, 0),
            (2, true, false, u32::MAX)
        ]
    );
    let reverse = s.spell_learned_by(3).unwrap().collect::<Vec<_>>();
    assert_eq!(
        reverse.iter().map(|n| n.source).collect::<Vec<_>>(),
        [5, 10, 20, 20, 30]
    );
    assert!(std::ptr::eq(nodes[0], reverse[2]));
    assert!(std::ptr::eq(nodes[1], reverse[3]));
    assert!(reverse[4].auto_learned);
    assert_eq!(
        s.learn_spell_counts(),
        Some(LearnSpellCounts {
            input_rows: 6,
            sql_nodes: 3,
            effect_nodes: 3,
            db2_nodes: 2,
            missing_source: 2,
            missing_learned: 3,
            rejected_talents: 1,
            redundant_sql: 3,
            db2_already_present: 3,
            nodes: 8,
            unavailable_baseline_db2: 7,
            empty_sql_early_return: false
        })
    );
    assert_eq!(s.is_spell_learn_to(20, 4), Some(true));
    assert_eq!(s.is_spell_learn_to(99, 4), Some(false));
}

#[test]
fn rejected_nonempty_sql_still_runs_automatic_passes_and_uses_exact_auto_learn_flags() {
    let mut pet_a = teaching(&[9]);
    pet_a.effects[0].implicit_targets[0] = 5;
    let mut pet_b = teaching(&[9]);
    pet_b.effects[0].implicit_targets[1] = 5;
    let mut talent = teaching(&[9]);
    talent.custom_attributes = TALENT;
    let mut passive = teaching(&[9]);
    passive.fields.attributes[0] = 0x40;
    let mut skill_step = teaching(&[9]);
    skill_step.effects.push(f::effect(44, 0, 0, 0.0));
    let mut different_bit = teaching(&[9]);
    different_bit.custom_attributes = 0x10000;
    let s = f::before_learn(
        vec![
            ((1, 0), pet_a),
            ((2, 0), pet_b),
            ((3, 0), talent),
            ((4, 0), passive),
            ((5, 0), skill_step),
            ((6, 0), different_bit),
            ((8, 2), teaching(&[9])),
            ((9, 0), teaching(&[])),
        ],
        Default::default(),
    )
    .with_learn_spells(vec![row(99, 9, true)])
    .unwrap();
    for (source, auto) in [
        (1, true),
        (2, false),
        (3, true),
        (4, true),
        (5, true),
        (6, false),
    ] {
        let nodes = s.spell_learn_nodes(source).unwrap().collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            *nodes[0],
            SpellLearnNode {
                source,
                spell: 9,
                overrides_spell: 0,
                active: true,
                auto_learned: auto
            }
        );
    }
    assert_eq!(s.spell_learn_nodes(8).unwrap().count(), 0);
    let counts = s.learn_spell_counts().unwrap();
    assert_eq!(
        (
            counts.input_rows,
            counts.sql_nodes,
            counts.effect_nodes,
            counts.missing_source,
            counts.nodes
        ),
        (1, 0, 6, 1, 6)
    );
    assert!(!counts.empty_sql_early_return);
}

#[test]
fn db2_signed_id_bits_and_source_sql_self_relationship_are_preserved() {
    let raw = SpellRecords {
        spell_learn_spells: vec![db2(1, 1, u32::MAX, u32::MAX - 1)],
        ..Default::default()
    };
    let s = f::before_learn(
        vec![((1, 0), teaching(&[])), ((u32::MAX, 0), teaching(&[]))],
        raw,
    )
    .with_learn_spells(vec![row(1, 1, false)])
    .unwrap();
    let nodes = s.spell_learn_nodes(1).unwrap().collect::<Vec<_>>();
    assert_eq!(
        *nodes[0],
        SpellLearnNode {
            source: 1,
            spell: 1,
            overrides_spell: 0,
            active: false,
            auto_learned: false
        }
    );
    assert_eq!(
        *nodes[1],
        SpellLearnNode {
            source: 1,
            spell: u32::MAX,
            overrides_spell: u32::MAX - 1,
            active: true,
            auto_learned: false
        }
    );
    assert!(std::ptr::eq(
        nodes[1],
        s.spell_learned_by(u32::MAX).unwrap().next().unwrap()
    ));
}

#[test]
fn baseline_sql_lookup_can_fall_back_but_nonregular_definitions_are_not_effect_teachers() {
    let raw = SpellRecords {
        difficulties: vec![difficulty(0, 2)],
        ..Default::default()
    };
    let s = f::before_learn(vec![((1, 2), teaching(&[2])), ((2, 2), teaching(&[]))], raw)
        .with_learn_spells(vec![row(1, 2, false)])
        .unwrap();
    assert_eq!(s.learn_spell_counts().unwrap().sql_nodes, 1);
    assert_eq!(s.learn_spell_counts().unwrap().effect_nodes, 0);
    assert_eq!(
        s.get_exact(1, 2).unwrap().spell_specific(),
        SpellSpecific::Normal
    );
    assert!(!s.spell_learn_nodes(1).unwrap().next().unwrap().active);
}

#[test]
fn phase_guard_and_undefined_lookup_reject_consumed_owner_without_partial_publication() {
    let s = f::before_specific(vec![]);
    assert!(matches!(
        s.with_learn_spells(vec![]),
        Err(SpellLearnError::RequiresSpecificAndAuraState)
    ));
    let raw = SpellRecords {
        difficulties: vec![difficulty(0, 1), difficulty(1, 0)],
        ..Default::default()
    };
    let s = f::before_learn(vec![((1, 0), teaching(&[]))], raw);
    assert!(matches!(
        s.with_learn_spells(vec![row(1, 99, true)]),
        Err(SpellLearnError::DefinitionLookup(
            SpellDefinitionError::DifficultyCycle
        ))
    ));
}
