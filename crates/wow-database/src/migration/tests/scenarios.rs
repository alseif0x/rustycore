//! Migration regressions.
//!
//! Moved out of migration.rs under #685; every test is unchanged.

use super::*;

#[test]
fn component_namespace_isolates_overlapping_versions_deterministically() {
    let manifest = fixture_manifest(vec![
        migration("fixture-module", "1", MigrationState::Active),
        migration("core", "1", MigrationState::Active),
    ]);
    let keys: Vec<_> = manifest
        .migrations_for(DatabaseKind::Auth)
        .into_iter()
        .map(|migration| (migration.component.as_str(), migration.version.as_str()))
        .collect();
    assert_eq!(keys, [("core", "1"), ("fixture-module", "1")]);
}

#[test]
fn changed_checksum_and_incomplete_rows_are_never_treated_as_applied() {
    let manifest = fixture_manifest(vec![migration("core", "1", MigrationState::Active)]);
    let report = build_report(
        &manifest,
        DatabaseKind::Auth,
        true,
        true,
        &[HistoryRow {
            component: "core".to_string(),
            version: "1".to_string(),
            checksum: "changed".to_string(),
            success: false,
        }],
        &BTreeSet::new(),
    );
    assert!(!report.compatible);
    assert_eq!(report.migrations[0].status, MigrationStatus::Incomplete);
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.contains("checksum"))
    );
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.contains("recovery"))
    );
}

#[test]
fn squash_excludes_archived_history_from_pending_chain() {
    let manifest = fixture_manifest(vec![
        migration("core", "1", MigrationState::Archived),
        migration("core", "2", MigrationState::Active),
    ]);
    let report = build_report(
        &manifest,
        DatabaseKind::Auth,
        true,
        true,
        &[],
        &BTreeSet::new(),
    );
    assert_eq!(report.migrations[0].status, MigrationStatus::Pending);
    assert_eq!(report.migrations[1].status, MigrationStatus::Pending);
    assert_eq!(report.problems, ["pending migration core:2"]);
}

#[test]
fn splitter_keeps_semicolons_inside_strings() {
    assert_eq!(
        split_sql("INSERT INTO t VALUES ('a;b'); SELECT 1;"),
        ["INSERT INTO t VALUES ('a;b')", "SELECT 1"]
    );
}

#[test]
fn bundled_manifest_has_four_explicit_baselines_and_exact_sources() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database/migrations/manifest.toml");
    let manifest = MigrationManifest::load(&path).expect("bundled manifest must validate");
    assert_eq!(manifest.baselines.len(), 4);
    assert_eq!(manifest.migrations.len(), 4);
    assert_eq!(
        manifest
            .baseline(DatabaseKind::World)
            .unwrap()
            .content_version
            .as_deref(),
        Some("TDB 343.24081")
    );
}

#[test]
fn missing_and_changed_migration_files_fail_closed() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "rustycore-migration-manifest-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let manifest_path = root.join("manifest.toml");
    let baseline = |database: &str, marker: &str| {
        format!("[[baselines]]\ndatabase = \"{database}\"\nmarker_table = \"{marker}\"\n")
    };
    let manifest_text = format!(
        "format = 1\n{}{}{}{}\n[[migrations]]\ncomponent = \"core\"\ndatabase = \"auth\"\nversion = \"1\"\ndescription = \"fixture\"\nfile = \"one.sql\"\nsha256 = \"{}\"\nstate = \"active\"\n",
        baseline("auth", "account"),
        baseline("characters", "characters"),
        baseline("world", "version"),
        baseline("hotfixes", "hotfix_blob"),
        sha256_hex("SELECT 1;\n")
    );
    fs::write(&manifest_path, &manifest_text).unwrap();
    let missing = MigrationManifest::load(&manifest_path)
        .unwrap_err()
        .to_string();
    assert!(missing.contains("missing migration file"));

    fs::write(root.join("one.sql"), "SELECT 2;\n").unwrap();
    let changed = MigrationManifest::load(&manifest_path)
        .unwrap_err()
        .to_string();
    assert!(changed.contains("checksum mismatch"));
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
#[ignore = "requires a disposable live MariaDB; set RUSTYCORE_DB_IT_USER and optional HOST/PORT/PASS"]
async fn live_mariadb_lock_order_and_incomplete_state_fail_closed() -> Result<()> {
    let Some(user) = std::env::var("RUSTYCORE_DB_IT_USER").ok() else {
        eprintln!("skipping: RUSTYCORE_DB_IT_USER is not set");
        return Ok(());
    };
    let host = std::env::var("RUSTYCORE_DB_IT_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("RUSTYCORE_DB_IT_PORT").unwrap_or_else(|_| "3306".into());
    let password = std::env::var("RUSTYCORE_DB_IT_PASS").unwrap_or_default();
    let server_url = format!("mysql://{user}:{password}@{host}:{port}?ssl-mode=DISABLED");
    let server = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&server_url)
        .await?;
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let database_name = format!("rustycore_migration_it_{}_{}", std::process::id(), unique);
    sqlx::query(&format!(
        "CREATE DATABASE `{database_name}` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"
    ))
    .execute(&server)
    .await?;

    let root = std::env::temp_dir().join(&database_name);
    fs::create_dir_all(&root)?;
    fs::write(
        root.join("1.sql"),
        "CREATE TABLE one (id INT PRIMARY KEY);\n",
    )?;
    fs::write(
        root.join("2.sql"),
        "CREATE TABLE partial_ddl (id INT PRIMARY KEY);\nTHIS IS INVALID SQL;\n",
    )?;
    let migrations = vec![
        Migration {
            component: "core".into(),
            database: DatabaseKind::Auth,
            version: "1".into(),
            description: "ordered first".into(),
            file: "1.sql".into(),
            sha256: sha256_hex("CREATE TABLE one (id INT PRIMARY KEY);\n"),
            state: MigrationState::Active,
            adopt_query: None,
        },
        Migration {
            component: "core".into(),
            database: DatabaseKind::Auth,
            version: "2".into(),
            description: "durable failure".into(),
            file: "2.sql".into(),
            sha256: sha256_hex(
                "CREATE TABLE partial_ddl (id INT PRIMARY KEY);\nTHIS IS INVALID SQL;\n",
            ),
            state: MigrationState::Active,
            adopt_query: None,
        },
    ];
    let manifest = MigrationManifest {
        format: 1,
        baselines: vec![BaselineRequirement {
            database: DatabaseKind::Auth,
            marker_table: "account".into(),
            content_version: None,
            cache_id: None,
        }],
        migrations,
        root: root.clone(),
    };
    let database_url =
        format!("mysql://{user}:{password}@{host}:{port}/{database_name}?ssl-mode=DISABLED");
    let pool = MySqlPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await?;
    sqlx::query("CREATE TABLE account (id INT PRIMARY KEY)")
        .execute(&pool)
        .await?;

    let mut lock_connection = pool.acquire().await?;
    let lock_name = format!("{LOCK_PREFIX}auth");
    let acquired: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, 0)")
        .bind(&lock_name)
        .fetch_one(&mut *lock_connection)
        .await?;
    assert_eq!(acquired, Some(1));
    let lock_error = migrate_database(&pool, &manifest, DatabaseKind::Auth)
        .await
        .unwrap_err()
        .to_string();
    assert!(lock_error.contains("held by another process"));
    sqlx::query_scalar::<_, Option<i64>>("SELECT RELEASE_LOCK(?)")
        .bind(&lock_name)
        .fetch_one(&mut *lock_connection)
        .await?;
    drop(lock_connection);

    let migration_error = migrate_database(&pool, &manifest, DatabaseKind::Auth)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        migration_error.contains("remains marked incomplete"),
        "unexpected migration error: {migration_error}"
    );
    let report = inspect_database(&pool, &manifest, DatabaseKind::Auth).await?;
    assert_eq!(report.migrations[0].status, MigrationStatus::Applied);
    assert_eq!(report.migrations[1].status, MigrationStatus::Incomplete);
    assert!(table_exists(&pool, "partial_ddl").await?);

    pool.close().await;
    sqlx::query(&format!("DROP DATABASE `{database_name}`"))
        .execute(&server)
        .await?;
    server.close().await;
    fs::remove_dir_all(root)?;
    Ok(())
}
