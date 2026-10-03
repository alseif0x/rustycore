//! Synthetic complete source inventory, holder diagnostics and phase failures.
use super::super::{Definition, DefinitionOrder, SpellEffectValues};
use super::*;
use crate::forever::spells::SpellLoadPlan;
use std::{collections::BTreeMap, sync::Arc};
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::*};

fn items() -> ItemCatalog {
    wow_data::forever_birth::item_records::ItemRecords::default()
        .finish(
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap()
}
fn definition(values: &[f32]) -> Definition {
    let mut definition = Definition::empty_server(vec![]);
    definition.effects = values
        .iter()
        .enumerate()
        .map(|(index, &base_points)| SpellEffectValues {
            index: index as u32,
            base_points,
            scaling_variance: f32::NAN,
            // Blank effects still have a value and vector extent in source.
            ..Default::default()
        })
        .collect();
    definition
}
fn seeds(
    definitions: impl IntoIterator<Item = ((u32, i16), Definition)>,
    records: SpellRecords,
) -> SpellDefinitionSeeds {
    let raw = Arc::new(
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
    );
    let mut s = SpellLoadPlan::build(raw)
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap();
    s.definitions = definitions.into_iter().collect();
    let primary: Vec<_> = s.definitions.keys().rev().copied().collect();
    let mut by_spell: BTreeMap<u32, Vec<_>> = BTreeMap::new();
    for &key in &primary {
        by_spell.entry(key.0).or_default().push(key);
    }
    // Explicit synthetic prephase; native composition test exercises admission.
    s.source_order = Some(DefinitionOrder { primary, by_spell });
    s.immunities = Some(super::super::immunities::LoadedImmunities {
        creatures: BTreeMap::new(),
        counts: Default::default(),
    });
    s
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
fn complete_source_cap_inventory_covers_all_35_ids_and_defaults() {
    // Ordered manual contract from SpellMgr.cpp:5426-5573, not inferred families.
    let expected = [
        (198030, 5),
        (453035, 8),
        (258860, 8),
        (258926, 5),
        (390137, 5),
        (53385, 5),
        (404358, 5),
        (157997, 8),
        (400254, 5),
        (212680, 5),
        (115310, 5),
        (388615, 5),
        (121253, 5),
        (385060, 8),
        (385061, 8),
        (385062, 8),
        (205472, 8),
        (2120, 8),
        (1254851, 8),
        (351140, 8),
        (199667, 5),
        (44949, 5),
        (199852, 5),
        (199851, 5),
        (307046, 5),
        (389860, 5),
        (400370, 5),
        (435222, 5),
        (1265579, 5),
        (1265580, 5),
        (1265581, 5),
        (1265582, 5),
        (1225827, 5),
        (1279200, 5),
        (1261215, 8),
    ];
    assert_eq!(
        RULES
            .iter()
            .flat_map(|rule| rule.ids.iter().map(|&id| (id, rule.maximum)))
            .collect::<Vec<_>>(),
        expected
    );
    let s = seeds(
        expected
            .iter()
            .map(|&(id, _)| ((id, 0), definition(&[])))
            .chain([((900001, 0), definition(&[]))]),
        Default::default(),
    );
    let raw = s.raw_catalog();
    assert_eq!(
        s.get_exact(198030, 0).unwrap().sqrt_target_limit(),
        SqrtTargetLimit::default()
    );
    let s = s.with_target_caps(&items()).unwrap();
    assert!(Arc::ptr_eq(&raw, &s.raw_catalog()));
    for (id, maximum) in expected {
        assert_eq!(
            s.get_exact(id, 0).unwrap().sqrt_target_limit(),
            SqrtTargetLimit {
                max_targets: maximum,
                non_diminished_targets: 0
            }
        );
    }
    assert_eq!(
        s.get_exact(900001, 0).unwrap().sqrt_target_limit(),
        SqrtTargetLimit::default()
    );
    let counts = s.target_cap_counts().unwrap();
    assert_eq!(
        (
            counts.patch_groups,
            counts.requested_spells,
            counts.missing_spells,
            counts.applications
        ),
        (25, 35, 0, 35)
    );
}

#[test]
fn every_holder_contract_preserves_blank_slots_and_truncates_base_without_variance() {
    for rule in RULES {
        let id = rule.ids[0];
        let Some((foreign, slot)) = rule.holder else {
            continue;
        };
        for value in [rule.maximum as f32 + 0.75, rule.maximum as f32 + 1.75] {
            let holder_id = foreign.unwrap_or(id);
            let mut values = vec![0.0; slot + 1];
            values[slot] = value;
            let mut rows = BTreeMap::from([((id, 0), definition(&[]))]);
            rows.insert((holder_id, 0), definition(&values));
            let s = seeds(rows, Default::default())
                .with_target_caps(&items())
                .unwrap();
            assert_eq!(
                s.get_exact(id, 0).unwrap().sqrt_target_limit().max_targets,
                rule.maximum
            );
            // NaN variance cannot affect CalcBaseValue; no RNG capability exists.
            assert_eq!(
                s.target_cap_counts().unwrap().mismatched_values,
                usize::from(value as i32 != rule.maximum)
            );
        }
    }
}

#[test]
fn all_difficulties_keep_caps_and_holder_lookup_uses_exact_then_source_fallback() {
    let records = SpellRecords {
        difficulties: vec![difficulty(2, 1)],
        ..Default::default()
    };
    let s = seeds(
        [
            ((198030, 2), definition(&[])),
            ((198030, 1), definition(&[])),
            ((198013, 1), definition(&[0.0, 0.0, 0.0, 0.0, 5.9])),
        ],
        records,
    )
    .with_target_caps(&items())
    .unwrap();
    assert_eq!(s.target_cap_counts().unwrap().applications, 2);
    assert_eq!(s.target_cap_counts().unwrap().mismatched_values, 0);
    for difficulty in [1, 2] {
        assert_eq!(
            s.get_exact(198030, difficulty)
                .unwrap()
                .sqrt_target_limit()
                .max_targets,
            5
        );
    }
    let records = SpellRecords {
        difficulties: vec![difficulty(2, 1)],
        ..Default::default()
    };
    let s = seeds(
        [
            ((198030, 2), definition(&[])),
            ((198013, 2), definition(&[0.0, 0.0, 0.0, 0.0, 6.0])),
            ((198013, 1), definition(&[0.0, 0.0, 0.0, 0.0, 5.0])),
        ],
        records,
    )
    .with_target_caps(&items())
    .unwrap();
    assert_eq!(s.target_cap_counts().unwrap().mismatched_values, 1);
}

#[test]
fn diagnostic_absence_keeps_caps_but_undefined_lookup_or_cast_returns_no_output() {
    let s = seeds(
        [
            ((198030, 0), definition(&[])),
            ((258860, 0), definition(&[])),
        ],
        Default::default(),
    )
    .with_target_caps(&items())
    .unwrap();
    let counts = s.target_cap_counts().unwrap();
    assert_eq!(
        (counts.missing_value_holders, counts.missing_holder_effects),
        (1, 1)
    );
    assert_eq!(
        s.get_exact(198030, 0)
            .unwrap()
            .sqrt_target_limit()
            .max_targets,
        5
    );
    let s = seeds(
        [((198030, 2), definition(&[]))],
        SpellRecords {
            difficulties: vec![difficulty(2, 2)],
            ..Default::default()
        },
    );
    assert!(matches!(
        s.with_target_caps(&items()),
        Err(SpellTargetCapError::DefinitionLookup(
            SpellDefinitionError::DifficultyCycle
        ))
    ));
    for value in [f32::NAN, f32::INFINITY, -f32::INFINITY, 2147483648.0] {
        let s = seeds(
            [((258860, 0), definition(&[0.0, value]))],
            Default::default(),
        );
        assert!(matches!(
            s.with_target_caps(&items()),
            Err(SpellTargetCapError::Value(
                SpellValueError::UndefinedIntegerCast
            ))
        ));
    }
}

#[test]
fn phase_admission_repeat_and_empty_registry_do_not_manufacture_definitions() {
    let mut s = seeds([], Default::default());
    s.immunities = None;
    assert!(matches!(
        s.with_target_caps(&items()),
        Err(SpellTargetCapError::RequiresImmunities)
    ));
    let s = seeds([], Default::default())
        .with_target_caps(&items())
        .unwrap();
    assert!(s.is_empty());
    assert_eq!(s.target_cap_counts().unwrap().missing_spells, 35);
    assert!(matches!(
        s.with_target_caps(&items()),
        Err(SpellTargetCapError::AlreadyApplied)
    ));
}
