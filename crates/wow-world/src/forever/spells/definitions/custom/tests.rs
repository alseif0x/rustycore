//! Synthetic source-rule coverage; native replay/client/durability are separate.
use super::super::{Definition, SpellEffectValues};
use super::*;
use crate::forever::spells::{SpellLoadPlan, SpellTraversal};
use std::sync::Arc;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, forever_birth::*, forever_game_tables::SpellValueGameTables,
    forever_spells::*,
};
mod binary;
mod effect_flags;
mod tail;
// Reuse synthetic full-schema rows, not private client assets or a raw mirror.
#[allow(dead_code)]
#[path = "../../../../../../wow-data/src/forever_spells/custom_source_fixtures.rs"]
mod dependency_rows;

fn effect(kind: u32, aura: u32, bp: f32) -> SpellEffectValues {
    SpellEffectValues {
        effect: kind,
        aura,
        base_points: bp,
        ..Default::default()
    }
}
fn definition(mut effects: Vec<SpellEffectValues>) -> Definition {
    for (index, effect) in effects.iter_mut().enumerate() {
        effect.index = index as u32;
    }
    let mut definition = Definition::empty_server(Vec::new());
    definition.effects = effects;
    definition.fields.attributes[12] = 0x0020_0000;
    definition
}
fn fresh(
    raw: SpellRecords,
    rows: impl IntoIterator<Item = (Key, Definition)>,
) -> SpellDefinitionSeeds {
    let catalog = raw
        .finish(
            Default::default(),
            Default::default(),
            6,
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap();
    let mut seeds = SpellLoadPlan::build(Arc::new(catalog))
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap();
    seeds.definitions = rows.into_iter().collect();
    seeds
}
fn prefix(
    seeds: SpellDefinitionSeeds,
    primary: Vec<Key>,
    birth: BirthRecords,
) -> SpellDefinitionSeeds {
    let secondary = seeds.definitions.keys().rev().copied().collect();
    let birth = birth
        .finish(
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap();
    seeds
        .with_id_corrections()
        .unwrap()
        .with_global_corrections(SpellTraversal::new(primary, secondary))
        .unwrap()
        .with_skill_line_abilities(Arc::new(birth))
        .unwrap()
        .with_sql_custom_attributes(vec![])
        .unwrap()
}
fn tables() -> Arc<SpellValueGameTables> {
    let columns = (0..24)
        .map(|i| format!("C{i}"))
        .collect::<Vec<_>>()
        .join("\t");
    let values = ["1"; 24].join("\t");
    Arc::new(
        SpellValueGameTables::parse_strs(
            &format!("id\t{columns}\n1\t{values}\n"),
            "id\tA\tB\tC\tD\n1\t1\t1\t1\t1\n",
            "id\tA\tB\tC\tD\n1\t1\t1\t1\t1\n",
        )
        .unwrap(),
    )
}
fn admitted(
    raw: SpellRecords,
    rows: impl IntoIterator<Item = (Key, Definition)>,
    order: Vec<Key>,
) -> SpellDefinitionSeeds {
    prefix(fresh(raw, rows), order, Default::default())
        .with_value_game_tables(tables())
        .unwrap()
}
fn items() -> item_records::ItemCatalog {
    item_records::ItemRecords::default()
        .finish(
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap()
}
fn no_draw(_: f32, _: f32) -> Result<f32, SpellValueError> {
    panic!("unexpected variance draw")
}
fn flags(seeds: &SpellDefinitionSeeds, key: Key) -> u32 {
    seeds.definitions[&key].custom_attributes
}

#[test]
fn admission_rejects_missing_prefix_tables_unknown_dependencies_and_reapplication() {
    assert!(matches!(
        fresh(Default::default(), []).with_custom_attributes(&items(), &mut no_draw),
        Err(SpellCustomAttributeError::RequiresSqlPrefixAndSourceOrder)
    ));
    assert!(matches!(
        prefix(fresh(Default::default(), []), vec![], Default::default())
            .with_custom_attributes(&items(), &mut no_draw),
        Err(SpellCustomAttributeError::RequiresValueGameTables)
    ));
    for index in 36..42 {
        let mut raw = SpellRecords::default();
        raw.unknown_baseline_records[index] = 1;
        assert!(matches!(
            admitted(raw, [], vec![]).with_custom_attributes(&items(), &mut no_draw),
            Err(SpellCustomAttributeError::IncompleteDependencies)
        ));
    }
    let complete = admitted(Default::default(), [], vec![])
        .with_custom_attributes(&items(), &mut no_draw)
        .unwrap();
    assert_eq!(
        complete.custom_attribute_counts(),
        Some(CustomAttributeCounts::default())
    );
    assert!(matches!(
        complete.with_custom_attributes(&items(), &mut no_draw),
        Err(SpellCustomAttributeError::AlreadyApplied)
    ));
}

#[test]
fn full_phase_retains_primary_order_binary_before_positivity_and_once_only_masks() {
    let a = (900_001, -1);
    let b = (900_002, 0);
    let make = |bp, variance| {
        let mut e = effect(3, 0, bp);
        e.scaling_variance = variance;
        definition(vec![e])
    };
    let mut one = make(10.0, 0.2);
    one.fields.school_mask = 0x8000_0003;
    one.fields.targets = 0x10;
    let mut two = make(20.0, 0.8);
    two.fields.targets = 0x40;
    let input = admitted(Default::default(), [(a, one), (b, two)], vec![b, a]);
    assert_eq!(
        input
            .get_exact(a.0, a.1)
            .unwrap()
            .explicit_target_masks()
            .available,
        0
    );
    let raw = input.raw_catalog();
    let mut ranges = vec![];
    let complete = input
        .with_custom_attributes(&items(), &mut |lo, hi| {
            ranges.push((lo, hi));
            Ok(0.0)
        })
        .unwrap();
    assert_eq!(ranges, [(-0.4, 0.4), (-0.4, 0.4), (-0.1, 0.1), (-0.1, 0.1)]);
    assert_eq!(
        flags(&complete, a) & (BINARY | MIXED_SCHOOL),
        BINARY | MIXED_SCHOOL
    );
    assert_eq!(complete.definitions[&a].fields.school_mask, 0x8000_0002);
    assert_eq!(
        complete
            .get_exact(a.0, a.1)
            .unwrap()
            .explicit_target_masks()
            .available,
        0x10
    );
    assert_eq!(
        complete
            .get_exact(b.0, b.1)
            .unwrap()
            .explicit_target_masks()
            .available,
        0x40
    );
    assert!(Arc::ptr_eq(&raw, &complete.raw_catalog()));
    let counts = complete.custom_attribute_counts().unwrap();
    assert_eq!(
        (
            counts.definitions,
            counts.effect_slots,
            counts.binary_assignments,
            counts.normal_school_clears,
            counts.explicit_masks_initialized
        ),
        (2, 2, 2, 1, 2)
    );
    assert_eq!(complete.source_order.as_ref().unwrap().primary, [b, a]);
}

#[test]
fn primary_order_exposes_live_child_negative_bits_without_recursive_publication() {
    let parent = (900_001, 0);
    let child = (900_002, 0);
    for (order, expected_ranges) in [
        (vec![child, parent], vec![(-0.25, 0.25), (-0.1, 0.1)]),
        (
            vec![parent, child],
            vec![(-0.1, 0.1), (-0.25, 0.25), (-0.25, 0.25)],
        ),
    ] {
        let mut p = effect(64, 0, 10.0);
        p.scaling_variance = 0.2;
        p.trigger_spell = child.0;
        let mut c = effect(2, 0, 20.0);
        c.scaling_variance = 0.5;
        let input = admitted(
            Default::default(),
            [(parent, definition(vec![p])), (child, definition(vec![c]))],
            order,
        );
        let mut ranges = vec![];
        let result = input
            .with_custom_attributes(&items(), &mut |lo, hi| {
                ranges.push((lo, hi));
                Ok(0.0)
            })
            .unwrap();
        assert_eq!(ranges, expected_ranges);
        assert!(result.definitions[&parent].negative_effects[0]);
        assert!(result.definitions[&child].negative_effects[0]);
        assert_eq!(
            result
                .custom_attribute_counts()
                .unwrap()
                .new_negative_effects,
            2
        );
    }
}

#[test]
fn full_phase_propagates_random_errors_without_returning_a_partial_owner() {
    let key = (900_001, 0);
    let mut e = effect(3, 0, 10.0);
    e.scaling_variance = 0.5;
    assert!(matches!(
        admitted(Default::default(), [(key, definition(vec![e]))], vec![key])
            .with_custom_attributes(&items(), &mut |_, _| Err(
                SpellValueError::RandomSourceUnavailable
            )),
        Err(SpellCustomAttributeError::Value(
            SpellValueError::RandomSourceUnavailable
        ))
    ));
}
