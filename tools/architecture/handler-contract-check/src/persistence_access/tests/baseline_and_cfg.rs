//! Regressions for baseline and cfg.

use super::*;

#[test]
fn persistence_inventory_unions_cfg_alternative_function_signatures() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);

                #[cfg(feature = "database")]
                fn make() -> Holder { todo!() }

                #[cfg(not(feature = "database"))]
                fn make() -> u8 { 0 }

                #[cfg(feature = "database")]
                fn persistent() {
                    consume(make().0.pool());
                }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_is_cfg_aware_and_malformed_cfg_fails_closed() {
    let baseline = inventory(
        r#"
                #[cfg(test)]
                fn test_only(pool: sqlx::PgPool) { pool.begin(); }

                #[cfg(any(test, feature = "live-db"))]
                fn production_capable(pool: sqlx::PgPool) { pool.begin(); }
            "#,
    )
    .expect("production and test satisfiability are classified");
    assert!(
        baseline
            .accesses
            .iter()
            .filter(|record| record.enclosing == "fn test_only")
            .all(|record| record.source_class == "test_fixture")
    );
    assert!(
        baseline
            .accesses
            .iter()
            .any(|record| record.enclosing == "fn test_only")
    );
    assert!(
        baseline
            .accesses
            .iter()
            .filter(|record| record.enclosing == "fn production_capable")
            .all(|record| record.source_class == "production")
    );

    let error = inventory(
        r#"
                #[cfg_attr(test)]
                fn malformed(pool: sqlx::PgPool) { pool.begin(); }
            "#,
    )
    .expect_err("malformed cfg_attr must fail closed");
    assert!(error.contains("invalid cfg"), "{error}");
}

#[test]
fn persistence_inventory_keeps_production_and_test_alias_graphs_separate() {
    let baseline = inventory(
        r#"
                #[cfg(not(test))]
                use sqlx::MySqlPool as Pool;
                #[cfg(test)]
                use sqlx::PgPool as Pool;

                #[cfg(not(test))]
                fn production_only(pool: Pool) { pool.begin(); }
                #[cfg(test)]
                fn test_only(pool: Pool) { pool.begin(); }

                #[cfg(test)]
                macro_rules! generated_test_query {
                    () => { sqlx::query("SELECT 1") };
                }
            "#,
    )
    .expect("mutually exclusive aliases resolve in their logical cfg views");

    let production_targets = baseline
        .accesses
        .iter()
        .filter(|record| record.enclosing == "fn production_only")
        .map(|record| (record.target, record.source_class.as_str()))
        .collect::<BTreeSet<_>>();
    assert!(production_targets.contains(&(PersistenceTarget::MySqlPool, "production")));
    assert!(
        !production_targets
            .iter()
            .any(|(target, _)| *target == PersistenceTarget::PgPool)
    );

    let test_targets = baseline
        .accesses
        .iter()
        .filter(|record| record.enclosing == "fn test_only")
        .map(|record| (record.target, record.source_class.as_str()))
        .collect::<BTreeSet<_>>();
    assert!(test_targets.contains(&(PersistenceTarget::PgPool, "test_fixture")));
    assert!(
        !test_targets
            .iter()
            .any(|(target, _)| *target == PersistenceTarget::MySqlPool)
    );

    assert!(baseline.accesses.iter().any(|record| {
        record.operation == PersistenceOperation::MacroReference
            && record.symbol == "generated_test_query"
            && record.source_class == "test_fixture"
    }));
}

#[test]
fn persistence_inventory_accepts_test_only_mounts_and_ratchets_their_growth() {
    let test_cfg = vec!["cfg(test)".to_owned()];
    let test_mount = |source| ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::tests",
        source_path: "src/tests.rs",
        inherited_cfg: &test_cfg,
        source,
    };
    let expected = inventory_persistence_accesses(&[test_mount(
        "fn existing(pool: sqlx::PgPool) { pool.begin(); }",
    )])
    .expect("test-only source mounts are part of the baseline");
    assert!(!expected.accesses.is_empty());
    assert!(
        expected
            .accesses
            .iter()
            .all(|record| record.source_class == "test_fixture")
    );

    let actual = inventory_persistence_accesses(&[test_mount(
        r#"
                fn existing(pool: sqlx::PgPool) { pool.begin(); }
                fn added(pool: sqlx::PgPool) { pool.begin(); }
            "#,
    )])
    .unwrap();
    let error = compare_persistence_access_baseline(&expected, &actual)
        .expect_err("new test-only concrete persistence must trip the ratchet");
    assert!(
        error.contains("untracked direct persistence access"),
        "{error}"
    );
    assert!(error.contains("test_fixture"), "{error}");
}

#[test]
fn persistence_baseline_detects_same_count_substitution_and_multiplicity() {
    let expected = inventory(
        r#"
                use sqlx::PgPool;
                fn transaction(pool: &PgPool) { pool.begin(); }
            "#,
    )
    .unwrap();
    let actual = inventory(
        r#"
                use sqlx::PgPool;
                fn transaction(pool: &PgPool) { pool.execute("DELETE"); }
            "#,
    )
    .unwrap();
    let error = compare_persistence_access_baseline(&expected, &actual)
        .expect_err("same-count operation swap must fail");
    assert!(
        error.contains("untracked direct persistence access"),
        "{error}"
    );
    assert!(
        error.contains("obsolete direct persistence baseline row"),
        "{error}"
    );

    let mut noncanonical = expected.clone();
    noncanonical.accesses[0].count = 0;
    assert!(
        compare_persistence_access_baseline(&noncanonical, &actual)
            .unwrap_err()
            .contains("zero-count")
    );

    let mut multiplicity = expected.clone();
    multiplicity
        .accesses
        .iter_mut()
        .find(|row| row.operation == PersistenceOperation::Begin)
        .expect("begin row")
        .count += 1;
    assert!(
        compare_persistence_access_baseline(&multiplicity, &expected)
            .unwrap_err()
            .contains("multiplicity changed")
    );
}

#[test]
fn persistence_baseline_is_serializable_and_input_order_independent() {
    let first = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "a",
        module: "crate::a",
        source_path: "src/a.rs",
        inherited_cfg: &[],
        source: "fn a(pool: sqlx::PgPool) { pool.begin(); }",
    };
    let second = ClassifiedPersistenceSource {
        classification: "wow_world_concrete_persistence_leaks",
        package: "b",
        module: "crate::b",
        source_path: "src/b.rs",
        inherited_cfg: &[],
        source: "fn b(pool: sqlx::MySqlPool) { pool.begin(); }",
    };
    let forward = inventory_persistence_accesses(&[first, second]).unwrap();
    let reverse = inventory_persistence_accesses(&[second, first]).unwrap();
    assert_eq!(forward, reverse);
    let json = serde_json::to_string(&forward).expect("baseline serializes");
    assert_eq!(
        serde_json::from_str::<PersistenceAccessBaseline>(&json).expect("baseline deserializes"),
        forward
    );
}

#[test]
fn persistence_inventory_inspects_generated_attributes_nested_in_cfg_attr() {
    let baseline = inventory(
        r#"
                #[cfg_attr(test, derive(sqlx::FromRow))]
                struct Row;
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.source_class == "test_fixture"
            && row.operation == PersistenceOperation::MacroReference
            && row.generated_input
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.source_class == "production"
            && row.operation == PersistenceOperation::MacroReference
            && row.generated_input
    }));
}
