//! The apply-time SQL guard against the vectors compose.py's guard also uses.

use crate::migration::module_sql::{check_script, table_prefix};

const VECTORS: &str = include_str!("../../../../../tools/modules/sql_guard_vectors.json");

#[test]
fn the_rust_guard_matches_the_shared_vectors() {
    let vectors: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
    let prefix = vectors["prefix"].as_str().unwrap();
    assert_eq!(table_prefix("example.milestones"), prefix);
    let mut checked = 0;
    for (kind, purge) in [("migration", false), ("purge", true)] {
        for statement in vectors[kind]["allow"].as_array().unwrap() {
            let statement = statement.as_str().unwrap();
            assert_eq!(
                check_script(statement, prefix, purge),
                Vec::<String>::new(),
                "{statement}"
            );
            checked += 1;
        }
        for statement in vectors[kind]["deny"].as_array().unwrap() {
            let statement = statement.as_str().unwrap();
            assert!(
                !check_script(statement, prefix, purge).is_empty(),
                "{statement}"
            );
            checked += 1;
        }
    }
    assert!(checked > 50, "the vectors were read");
}

#[test]
fn a_script_reports_each_refused_statement() {
    let problems = check_script(
        "CREATE TABLE mod_a_x (id INT);\nUPDATE characters SET money = 0;\nINSERT INTO mod_a_x VALUES ('select; drop');\n",
        &table_prefix("a"),
        false,
    );
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].starts_with("statement 2:"), "{problems:?}");
}

#[test]
fn dash_dash_starts_a_comment_only_before_whitespace() {
    // MariaDB: `1--1` is arithmetic, so the line after it is still SQL.
    assert_eq!(
        crate::migration::split_sql("SELECT 1--1;\nSELECT 2"),
        vec!["SELECT 1--1", "SELECT 2"]
    );
    assert_eq!(
        crate::migration::split_sql("SELECT 1 -- note; still a comment\n;SELECT 2 --\n"),
        vec!["SELECT 1 -- note; still a comment", "SELECT 2 --"]
    );
}
