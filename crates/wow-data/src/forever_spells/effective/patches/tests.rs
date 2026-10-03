//! Exclusive raw record patching, never resurrection or another catalog.
use super::super::*;
use crate::Db2HotfixRemovalStoreLikeCpp;

fn row(id: u32) -> SummonPropertiesRecord {
    SummonPropertiesRecord {
        id,
        control: -9,
        faction: 8,
        title: 99,
        slot: 5,
        flags: [i32::MIN, -1],
    }
}
fn patches() -> [SummonPropertiesPatch; 3] {
    [
        SummonPropertiesPatch {
            id: 121,
            title: Some(4),
            control: None,
        },
        SummonPropertiesPatch {
            id: 647,
            title: Some(4),
            control: None,
        },
        SummonPropertiesPatch {
            id: 628,
            title: None,
            control: Some(2),
        },
    ]
}
fn finish(rows: SpellRecords, removals: &Db2HotfixRemovalStoreLikeCpp) -> SpellCatalog {
    rows.finish(
        SpellRecords::default(),
        SpellRecords::default(),
        6,
        SpellLocaleRecords::default(),
        SpellLocaleRecords::default(),
        removals,
    )
    .unwrap()
}

#[test]
fn existing_records_mutate_exact_fields_in_place_and_retain_catalog_coverage() {
    let mut rows = SpellRecords {
        summon_properties: vec![row(121), row(647), row(628)],
        ..Default::default()
    };
    rows.unknown_baseline_records[33] = 7;
    let mut result = finish(
        rows,
        &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
    );
    let pointer = result.summon_properties(121).unwrap() as *const SummonPropertiesRecord;
    let counts = result.counts();
    assert_eq!(
        result.apply_summon_properties_patches(patches()),
        SummonPropertiesPatchCounts {
            applied: 3,
            missing: 0
        }
    );
    for id in [121, 647] {
        let changed = result.summon_properties(id).unwrap();
        assert_eq!((changed.title, changed.control), (4, -9));
        assert_eq!(
            (changed.faction, changed.slot, changed.flags),
            (8, 5, [i32::MIN, -1])
        );
    }
    let changed = result.summon_properties(628).unwrap();
    assert_eq!((changed.title, changed.control), (99, 2));
    assert_eq!(
        pointer,
        result.summon_properties(121).unwrap() as *const SummonPropertiesRecord
    );
    assert_eq!(counts, result.counts());
}

#[test]
fn absent_removed_records_are_not_resurrected_or_invented() {
    let rows = SpellRecords {
        summon_properties: vec![row(121), row(647)],
        ..Default::default()
    };
    let removals =
        Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(SPELL_TABLE_HASHES[33], 647, 2)]);
    let mut result = finish(rows, &removals);
    let counts = result.counts();
    assert_eq!(
        result.apply_summon_properties_patches(patches()),
        SummonPropertiesPatchCounts {
            applied: 1,
            missing: 2
        }
    );
    assert!(result.summon_properties(647).is_none());
    assert!(result.summon_properties(628).is_none());
    assert_eq!(counts, result.counts());
}

#[test]
fn generic_adapter_preserves_signed_width_and_input_order_without_domain_enum_guessing() {
    let mut result = finish(
        SpellRecords {
            summon_properties: vec![row(1)],
            ..Default::default()
        },
        &Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([]),
    );
    let count = result.apply_summon_properties_patches([
        SummonPropertiesPatch {
            id: 1,
            title: Some(i32::MIN),
            control: Some(i32::MAX),
        },
        SummonPropertiesPatch {
            id: 1,
            title: None,
            control: Some(-1),
        },
    ]);
    assert_eq!(
        count,
        SummonPropertiesPatchCounts {
            applied: 2,
            missing: 0
        }
    );
    let row = result.summon_properties(1).unwrap();
    assert_eq!((row.title, row.control), (i32::MIN, -1));
}
