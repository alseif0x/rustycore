//! Module SQL history in the lock (#1444 H6): parsing, merging and the
//! startup acceptance of active, pending and tombstoned module history.

use super::*;

const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A lock as compose.py renders it, plus a tombstone block.
fn lock(database: &str, tombstone: &str) -> String {
    format!(
        r#"source_api = "2"

[[module]]
id = "example.milestones"
version = "1.0.0"
enabled_order = 0

[module.sql]
min_core_schema = "2026.08.02.00"
source = "modules/milestones/data/sql"

[[module.sql.migration]]
database = "{database}"
version = "2026.10.21.00"
description = "counts"
file = "characters/2026_10_21_00_counts.sql"
sha256 = "{SHA_A}"

[[module.sql.purge]]
database = "characters"
file = "purge/characters.sql"
sha256 = "{SHA_B}"

[[module]]
id = "plain.module"
version = "1.0.0"
enabled_order = 1
{tombstone}"#
    )
}

const TOMBSTONE: &str = r#"
# --- tombstones: written by rustycore-db module remove; kept as is by compose.py ---
[[tombstone]]
id = "old.module"
enabled_order = 3

[tombstone.sql]
min_core_schema = "2026.08.02.00"
source = "modules/.retired/old.module/data/sql"

[[tombstone.sql.migration]]
database = "world"
version = "1"
description = "old table"
file = "world/1.sql"
sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
"#;

fn row(component: &str, version: &str, checksum: &str) -> HistoryRow {
    HistoryRow {
        component: component.into(),
        version: version.into(),
        checksum: checksum.into(),
        success: true,
    }
}

fn core() -> MigrationManifest {
    MigrationManifest {
        format: 1,
        baselines: DatabaseKind::ALL
            .into_iter()
            .map(|database| BaselineRequirement {
                database,
                marker_table: "marker".into(),
                content_version: None,
                cache_id: None,
            })
            .collect(),
        migrations: vec![],
        root: PathBuf::new(),
    }
}

#[test]
fn a_lock_without_module_sql_is_an_empty_schema() {
    assert!(ModuleSchema::from_lock("").unwrap().is_empty());
    let plain = "source_api = \"2\"\n\n[[module]]\nid = \"plain.module\"\nversion = \"1.0.0\"\n";
    assert!(ModuleSchema::from_lock(plain).unwrap().is_empty());
}

#[test]
fn composed_module_sql_is_active_and_tombstoned_sql_is_archived() {
    let schema = ModuleSchema::from_lock(&lock("characters", TOMBSTONE)).unwrap();
    assert_eq!(schema.modules.len(), 1, "plain.module has no SQL");
    assert!(schema.active("example.milestones").is_some());
    assert!(schema.tombstone("old.module").is_some());

    let manifest = core().with_module_schema(&schema).unwrap();
    let characters = manifest.migrations_for(DatabaseKind::Characters);
    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0].component, "module:example.milestones");
    assert_eq!(characters[0].state, MigrationState::Active);
    let world = manifest.migrations_for(DatabaseKind::World);
    assert_eq!(world[0].component, "module:old.module");
    assert_eq!(world[0].state, MigrationState::Archived);
}

#[test]
fn startup_requires_composed_module_sql_and_accepts_tombstoned_history() {
    let schema = ModuleSchema::from_lock(&lock("characters", TOMBSTONE)).unwrap();
    let manifest = core().with_module_schema(&schema).unwrap();

    let pending = build_report(
        &manifest,
        DatabaseKind::Characters,
        true,
        true,
        &[],
        &BTreeSet::new(),
    );
    assert!(!pending.compatible);
    assert_eq!(
        pending.problems,
        vec!["pending migration module:example.milestones:2026.10.21.00".to_string()]
    );

    let applied = [row("module:example.milestones", "2026.10.21.00", SHA_A)];
    let report = build_report(
        &manifest,
        DatabaseKind::Characters,
        true,
        true,
        &applied,
        &BTreeSet::new(),
    );
    assert!(report.compatible, "{:?}", report.problems);

    // The removed module's history stays acceptable, and nothing is required.
    let removed = [row("module:old.module", "1", SHA_B)];
    let report = build_report(
        &manifest,
        DatabaseKind::World,
        true,
        true,
        &removed,
        &BTreeSet::new(),
    );
    assert!(report.compatible, "{:?}", report.problems);
    let report = build_report(
        &manifest,
        DatabaseKind::World,
        true,
        true,
        &[],
        &BTreeSet::new(),
    );
    assert!(report.compatible, "an archived migration is never pending");

    // A changed applied file is refused.
    let changed = [row("module:old.module", "1", SHA_A)];
    let report = build_report(
        &manifest,
        DatabaseKind::World,
        true,
        true,
        &changed,
        &BTreeSet::new(),
    );
    assert!(
        report.problems[0].contains("checksum differs"),
        "{:?}",
        report.problems
    );
}

#[test]
fn a_build_without_the_module_refuses_its_history() {
    let plain = core().with_module_schema(&ModuleSchema::empty()).unwrap();
    let applied = [row("module:example.milestones", "2026.10.21.00", SHA_A)];
    let report = build_report(
        &plain,
        DatabaseKind::Characters,
        true,
        true,
        &applied,
        &BTreeSet::new(),
    );
    assert!(!report.compatible);
    assert!(report.problems[0].contains("absent from the manifest"));
}

#[test]
fn module_sql_outside_characters_and_world_is_refused() {
    for database in ["auth", "hotfixes"] {
        let error = ModuleSchema::from_lock(&lock(database, ""))
            .unwrap_err()
            .to_string();
        assert!(error.contains("only characters and world"), "{error}");
    }
}

#[test]
fn a_reinstalled_module_must_continue_its_tombstoned_history() {
    let shadow = TOMBSTONE.replace("old.module", "example.milestones");
    let error = ModuleSchema::from_lock(&lock("characters", &shadow))
        .unwrap_err()
        .to_string();
    assert!(error.contains("does not continue"), "{error}");

    let continued = format!(
        "\n# --- tombstones ---\n[[tombstone]]\nid = \"example.milestones\"\n\n[tombstone.sql]\nmin_core_schema = \"1\"\nsource = \"x\"\n\n[[tombstone.sql.migration]]\ndatabase = \"characters\"\nversion = \"2026.10.21.00\"\ndescription = \"counts\"\nfile = \"characters/2026_10_21_00_counts.sql\"\nsha256 = \"{SHA_A}\"\n"
    );
    let schema = ModuleSchema::from_lock(&lock("characters", &continued)).unwrap();
    let manifest = core().with_module_schema(&schema).unwrap();
    let characters = manifest.migrations_for(DatabaseKind::Characters);
    assert_eq!(characters.len(), 1, "the composed module owns the history");
    assert_eq!(characters[0].state, MigrationState::Active);
}

#[test]
fn invalid_module_records_are_refused() {
    let bad_sha = lock("characters", "").replace(SHA_A, "nothex");
    assert!(ModuleSchema::from_lock(&bad_sha).is_err());
    let bad_version = lock("characters", "").replace("2026.10.21.00", "v1");
    assert!(ModuleSchema::from_lock(&bad_version).is_err());
    let bad_id = lock("characters", "").replace("example.milestones", "Example");
    assert!(ModuleSchema::from_lock(&bad_id).is_err());
    assert!(ModuleSchema::from_lock("[[module]\n").is_err());
}

#[test]
fn schema_versions_compare_numerically() {
    assert!(
        crate::migration::modules::parse_schema_version("2026.10.2").unwrap()
            < crate::migration::modules::parse_schema_version("2026.10.10").unwrap()
    );
    for bad in ["", "1..2", "a", "1.2.", "1234567890"] {
        assert!(
            crate::migration::modules::parse_schema_version(bad).is_none(),
            "{bad}"
        );
    }
}

#[test]
fn core_migrate_skips_module_migrations() {
    assert!(is_module_component("module:x"));
    assert!(!is_module_component("core"));
    assert_eq!(module_component("a.b"), "module:a.b");
}

/// Two modules listed against `enabled_order`, with versions listed against
/// numeric order ("2026.10.10" before "2026.10.9").
const WRONG_ORDER: &str = r#"source_api = "2"

[[module]]
id = "a.late"
enabled_order = 1

[module.sql]
min_core_schema = "1"
source = "a"

[[module.sql.migration]]
database = "characters"
version = "2026.10.10"
description = "second"
file = "2.sql"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[[module.sql.migration]]
database = "characters"
version = "2026.10.9"
description = "first"
file = "1.sql"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[[module]]
id = "z.early"
enabled_order = 0

[module.sql]
min_core_schema = "1"
source = "z"

[[module.sql.migration]]
database = "characters"
version = "1"
description = "only"
file = "1.sql"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
"#;

#[test]
fn apply_order_is_enabled_order_then_numeric_version() {
    let schema = ModuleSchema::from_lock(WRONG_ORDER).unwrap();
    let order: Vec<_> = schema
        .ordered_migrations(DatabaseKind::Characters)
        .into_iter()
        .map(|(module, migration)| format!("{}:{}", module.id, migration.version))
        .collect();
    assert_eq!(
        order,
        ["z.early:1", "a.late:2026.10.9", "a.late:2026.10.10"],
        "enabled_order beats the id, numbers beat text"
    );
    let manifest = core().with_module_schema(&schema).unwrap();
    let versions: Vec<_> = manifest
        .migrations_for(DatabaseKind::Characters)
        .into_iter()
        .filter(|migration| migration.component == "module:a.late")
        .map(|migration| migration.version.as_str())
        .collect();
    assert_eq!(versions, ["2026.10.9", "2026.10.10"]);
}

#[test]
fn unknown_module_history_and_changed_active_checksums_are_refused() {
    let schema = ModuleSchema::from_lock(&lock("characters", "")).unwrap();
    let manifest = core().with_module_schema(&schema).unwrap();
    let unknown = [
        row("module:example.milestones", "2026.10.21.00", SHA_A),
        row("module:someone.else", "1", SHA_A),
    ];
    let report = build_report(
        &manifest,
        DatabaseKind::Characters,
        true,
        true,
        &unknown,
        &BTreeSet::new(),
    );
    assert_eq!(
        report.problems,
        vec!["applied migration module:someone.else:1 is absent from the manifest".to_string()]
    );
    let changed = [row("module:example.milestones", "2026.10.21.00", SHA_B)];
    let report = build_report(
        &manifest,
        DatabaseKind::Characters,
        true,
        true,
        &changed,
        &BTreeSet::new(),
    );
    assert!(!report.compatible);
    assert!(
        report.problems[0].contains("checksum differs"),
        "{:?}",
        report.problems
    );
}

#[test]
fn mixed_version_kinds_sort_by_one_total_key() {
    let mut manifest = core();
    for version in ["b", "10", "9", "a", "1.x", "2", "1.10", "1.9"] {
        manifest.migrations.push(Migration {
            component: "module:mixed".into(),
            database: DatabaseKind::World,
            version: version.into(),
            description: "mixed".into(),
            file: format!("{version}.sql"),
            sha256: SHA_A.into(),
            state: MigrationState::Archived,
            adopt_query: None,
        });
    }
    let order: Vec<_> = manifest
        .migrations_for(DatabaseKind::World)
        .into_iter()
        .map(|migration| migration.version.as_str())
        .collect();
    assert_eq!(order, ["1.9", "1.10", "2", "9", "10", "1.x", "a", "b"]);
}

#[test]
fn nested_prefixes_and_escaping_paths_are_refused() {
    let nested = lock("characters", "").replace("plain.module\"", "example.milestones.x\"")
        + "\n[module.sql]\nmin_core_schema = \"1\"\nsource = \"b\"\n";
    let error = ModuleSchema::from_lock(&nested).unwrap_err().to_string();
    assert!(error.contains("share a table prefix"), "{error}");
    let tombstoned = lock(
        "characters",
        &TOMBSTONE.replace("old.module", "example.milestones.old"),
    );
    let error = ModuleSchema::from_lock(&tombstoned)
        .unwrap_err()
        .to_string();
    assert!(error.contains("share a table prefix"), "{error}");
    for bad in ["../x.sql", "/etc/x.sql", "a//b.sql", "./a.sql"] {
        let escaping = lock("characters", "").replace("characters/2026_10_21_00_counts.sql", bad);
        assert!(ModuleSchema::from_lock(&escaping).is_err(), "{bad}");
    }
    let source = lock("characters", "").replace("modules/milestones/data/sql", "../outside");
    assert!(ModuleSchema::from_lock(&source).is_err());
}
