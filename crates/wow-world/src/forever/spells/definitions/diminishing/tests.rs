//! Synthetic rules/composition only, not actual assets or live acceptance.
use super::super::{SpellEffectValues, SpellLoadPlan, SpellTraversal};
use super::*;
use std::sync::Arc;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, forever_birth::BirthRecords,
    forever_game_tables::SpellValueGameTables, forever_spells::*,
};
mod groups;
mod visual;

fn definition(family: u32, flags: [u32; 4]) -> Definition {
    let mut d = Definition::empty_server(Vec::new());
    d.fields.spell_family_name = family;
    d.fields.spell_family_flags = flags;
    d.negative_effects[31] = true; // Source IsPositive uses all 32 bits, not active slots.
    d
}
fn no_visual() -> Result<u32, SpellDiminishingError> {
    panic!("unexpected visual query")
}
fn no_select(_: &[f64]) -> Result<usize, SpellDiminishingError> {
    panic!("unexpected draw")
}
fn raw(records: SpellRecords) -> Arc<SpellCatalog> {
    Arc::new(
        records
            .finish(
                Default::default(),
                Default::default(),
                6,
                Default::default(),
                Default::default(),
                &Db2HotfixRemovalStoreLikeCpp::default(),
            )
            .unwrap(),
    )
}
fn input(rows: Vec<(Key, Definition)>, records: SpellRecords) -> SpellDefinitionSeeds {
    let mut s = SpellLoadPlan::build(raw(records))
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap();
    s.definitions = rows.into_iter().collect();
    s
}
fn admitted(
    rows: Vec<(Key, Definition)>,
    records: SpellRecords,
    order: Vec<Key>,
) -> SpellDefinitionSeeds {
    let s = input(rows, records);
    let secondary = s.definitions.keys().rev().copied().collect();
    let birth = BirthRecords::default()
        .finish(
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap();
    let columns = (0..24)
        .map(|i| format!("C{i}"))
        .collect::<Vec<_>>()
        .join("\t");
    let cells = ["1"; 24].join("\t");
    let gt = SpellValueGameTables::parse_strs(
        &format!("id\t{columns}\n1\t{cells}\n"),
        "id\tA\tB\tC\tD\n1\t1\t1\t1\t1\n",
        "id\tA\tB\tC\tD\n1\t1\t1\t1\t1\n",
    )
    .unwrap();
    let items = wow_data::forever_birth::item_records::ItemRecords::default()
        .finish(
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap();
    s.with_id_corrections()
        .unwrap()
        .with_global_corrections(SpellTraversal::new(order, secondary))
        .unwrap()
        .with_skill_line_abilities(Arc::new(birth))
        .unwrap()
        .with_sql_custom_attributes(vec![])
        .unwrap()
        .with_value_game_tables(Arc::new(gt))
        .unwrap()
        .with_custom_attributes(&items, &mut |_, _| panic!("no effects"))
        .unwrap()
}

#[test]
fn admission_constructor_defaults_and_repeat_guard_are_phase_specific() {
    let key = (900_001, -1);
    let s = input(vec![(key, definition(0, [0; 4]))], Default::default());
    assert_eq!(
        s.get_exact(key.0, key.1).unwrap().diminishing_info(),
        DiminishingInfo::default()
    );
    assert!(matches!(
        s.with_diminishing_info(&mut no_select),
        Err(SpellDiminishingError::RequiresCustomAttributes)
    ));
    let s = admitted(vec![], Default::default(), vec![])
        .with_diminishing_info(&mut no_select)
        .unwrap();
    assert_eq!(s.diminishing_counts(), Some(DiminishingCounts::default()));
    assert!(matches!(
        s.with_diminishing_info(&mut no_select),
        Err(SpellDiminishingError::AlreadyApplied)
    ));
}

#[test]
fn complete_phase_retains_primary_order_per_query_draws_and_raw_identity() {
    let a = (900_001, -1);
    let b = (900_002, 0);
    let mut d = definition(6, [0, 0, 0x20, 0]);
    d.visuals = vec![1, 2];
    let s = admitted(
        vec![(a, d), (b, definition(0, [0; 4]))],
        SpellRecords {
            spell_x_spell_visuals: vec![
                visual::relation(1, 52021, 0, 0, 1, 0.2),
                visual::relation(2, 52019, 0, 0, 1, 0.8),
            ],
            ..Default::default()
        },
        vec![b, a],
    );
    let before = s.raw_catalog();
    let mut calls = 0;
    let s = s
        .with_diminishing_info(&mut |weights| {
            assert_eq!(weights, [f64::from(0.2f32), f64::from(0.8f32)]);
            calls += 1;
            Ok(if calls == 1 { 1 } else { 0 })
        })
        .unwrap();
    // First query sees 52019 (not stun); second sees 52021 (not incap).
    assert_eq!(
        s.get_exact(a.0, a.1).unwrap().diminishing_info().group,
        DiminishingGroup::None
    );
    assert_eq!(
        s.diminishing_counts(),
        Some(DiminishingCounts {
            definitions: 2,
            visual_queries: 2,
            selections: 2
        })
    );
    assert_eq!(s.source_order.as_ref().unwrap().primary, [b, a]);
    assert!(Arc::ptr_eq(&before, &s.raw_catalog()));
}

#[test]
fn group_type_level_and_duration_are_independent_of_positive_early_returns() {
    use DiminishingGroup::*;
    for (family, id, flags, group, millis) in [
        (3, 900_001, [0x800000, 0, 0, 0], Incapacitate, 3000),
        (5, 170995, [0; 4], LimitOnly, 4000),
        (9, 117526, [0; 4], Stun, 3000),
        (9, 900_001, [0, 0x1000, 0, 0], Incapacitate, 6000),
        (53, 900_001, [0, 0, 0x800000, 0], Incapacitate, 4000),
        (107, 217832, [0; 4], Incapacitate, 4000),
        (0, 108199, [0; 4], AoeKnockback, 8000),
    ] {
        for positive in [false, true] {
            let mut d = definition(family, flags);
            d.negative_effects = [!positive; 32];
            assert_eq!(
                super::groups::compute(&d, id, &mut no_visual).unwrap(),
                if positive { None } else { group }
            );
            assert_eq!(duration(&d, (id, 0)), millis);
        }
    }
    for (family, flags, expected) in [
        (
            3,
            [0x40, 0, 0, 0],
            (Root, DiminishingType::Player, DiminishingLevel::Immune),
        ),
        (
            4,
            [0, 0x8000, 0, 0],
            (Stun, DiminishingType::All, DiminishingLevel::Immune),
        ),
        (
            57,
            [0x8000000, 0, 0, 0],
            (
                AoeKnockback,
                DiminishingType::Player,
                DiminishingLevel::Second,
            ),
        ),
        (
            0,
            [0; 4],
            (None, DiminishingType::None, DiminishingLevel::Immune),
        ),
    ] {
        let key = (900_001, 0);
        let s = admitted(
            vec![(key, definition(family, flags))],
            Default::default(),
            vec![key],
        )
        .with_diminishing_info(&mut no_select)
        .unwrap();
        let info = s.get_exact(key.0, key.1).unwrap().diminishing_info();
        assert_eq!((info.group, info.return_type, info.maximum_level), expected);
    }
}
