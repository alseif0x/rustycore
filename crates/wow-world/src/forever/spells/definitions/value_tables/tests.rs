use super::*;
use crate::forever::spells::SpellLoadPlan;
use wow_data::{Db2HotfixRemovalStoreLikeCpp, forever_spells::SpellRecords};

fn seeds() -> SpellDefinitionSeeds {
    let raw = SpellRecords::default()
        .finish(
            Default::default(),
            Default::default(),
            6,
            Default::default(),
            Default::default(),
            &Db2HotfixRemovalStoreLikeCpp::default(),
        )
        .unwrap();
    SpellLoadPlan::build(Arc::new(raw))
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap()
}
fn tables() -> Arc<SpellValueGameTables> {
    let scaling = format!(
        "id\t{}\n1\t{}\n",
        (0..24)
            .map(|i| format!("C{i}"))
            .collect::<Vec<_>>()
            .join("\t"),
        (0..24).map(|_| "0").collect::<Vec<_>>().join("\t")
    );
    Arc::new(
        SpellValueGameTables::parse_strs(
            &scaling,
            "id\tA\tB\tC\tD\n1\t1\t2\t3\t4\n",
            "id\tA\tB\tC\tD\n1\t5\t6\t7\t8\n",
        )
        .unwrap(),
    )
}

#[test]
fn admitted_value_tables_share_one_allocation_without_claiming_a_custom_phase() {
    let original = seeds();
    assert!(original.value_game_tables().is_none());
    let raw = original.raw_catalog();
    let tables = tables();
    let admitted = original.with_value_game_tables(tables.clone()).unwrap();
    assert!(std::ptr::eq(
        tables.as_ref(),
        admitted.value_game_tables().unwrap()
    ));
    assert!(Arc::ptr_eq(&raw, &admitted.raw_catalog()));
    assert_eq!(admitted.value_game_tables().unwrap().counts(), [2, 2, 2]);
    assert_eq!(admitted.sql_custom_attribute_counts(), None);
    assert_eq!(admitted.global_correction_counts(), None);
    assert!(admitted.is_empty());
}

#[test]
fn second_input_admission_rejects_replacement_of_the_canonical_value_tables() {
    assert!(matches!(
        seeds()
            .with_value_game_tables(tables())
            .unwrap()
            .with_value_game_tables(tables()),
        Err(SpellDefinitionError::SpellValueTablesAlreadyLoaded)
    ));
}
