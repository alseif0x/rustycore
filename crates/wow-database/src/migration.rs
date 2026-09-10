//! Explicit, manifest-driven MariaDB migrations.
//!
//! Runtime servers only use [`validate_runtime_schema`]. Schema mutation is
//! deliberately confined to the `rustycore-db` administrative executable.

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use sqlx::{MySql, MySqlPool, Row, pool::PoolConnection};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const HISTORY_TABLE: &str = "rustycore_schema_history";
const LOCK_PREFIX: &str = "rustycore-db:migrate:";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseKind {
    Auth,
    Characters,
    World,
    Hotfixes,
}

impl DatabaseKind {
    pub const ALL: [Self; 4] = [Self::Auth, Self::Characters, Self::World, Self::Hotfixes];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::Characters => "characters",
            Self::World => "world",
            Self::Hotfixes => "hotfixes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MigrationState {
    Active,
    Archived,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationManifest {
    pub format: u32,
    pub baselines: Vec<BaselineRequirement>,
    pub migrations: Vec<Migration>,
    #[serde(skip)]
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BaselineRequirement {
    pub database: DatabaseKind,
    pub marker_table: String,
    pub content_version: Option<String>,
    pub cache_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Migration {
    pub component: String,
    pub database: DatabaseKind,
    pub version: String,
    pub description: String,
    pub file: String,
    pub sha256: String,
    pub state: MigrationState,
    /// Read-only scalar query used only to adopt a pre-manifest installation.
    /// It must return exactly `1` when the final schema is already present.
    pub adopt_query: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MigrationView {
    pub component: String,
    pub database: DatabaseKind,
    pub version: String,
    pub description: String,
    pub file: String,
    pub sha256: String,
    pub state: MigrationState,
    pub status: MigrationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    Pending,
    TransitionImport,
    Applied,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatabaseReport {
    pub database: DatabaseKind,
    pub baseline_compatible: bool,
    pub history_present: bool,
    pub compatible: bool,
    pub migrations: Vec<MigrationView>,
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HistoryRow {
    component: String,
    version: String,
    checksum: String,
    success: bool,
}

impl MigrationManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("cannot read migration manifest {}", path.display()))?;
        let mut manifest: Self = toml::from_str(&text)
            .with_context(|| format!("invalid migration manifest {}", path.display()))?;
        if manifest.format != 1 {
            bail!("unsupported migration manifest format {}", manifest.format);
        }
        manifest.root = path
            .parent()
            .ok_or_else(|| anyhow!("manifest has no parent directory"))?
            .to_path_buf();
        manifest.validate_sources()?;
        Ok(manifest)
    }

    /// Parse the source-controlled manifest embedded in runtime binaries.
    ///
    /// Unlike [`Self::load`], this performs no filesystem access. Runtime
    /// compatibility checks use the compiled identities and checksums only.
    pub fn load_embedded(text: &str) -> Result<Self> {
        let mut manifest: Self =
            toml::from_str(text).context("invalid embedded migration manifest")?;
        if manifest.format != 1 {
            bail!("unsupported migration manifest format {}", manifest.format);
        }
        manifest.root = PathBuf::new();
        manifest.validate_shape()?;
        Ok(manifest)
    }

    pub fn baseline(&self, database: DatabaseKind) -> Result<&BaselineRequirement> {
        self.baselines
            .iter()
            .find(|baseline| baseline.database == database)
            .ok_or_else(|| anyhow!("manifest has no {} baseline", database.as_str()))
    }

    pub fn migrations_for(&self, database: DatabaseKind) -> Vec<&Migration> {
        let mut migrations: Vec<_> = self
            .migrations
            .iter()
            .filter(|migration| migration.database == database)
            .collect();
        migrations.sort_by_key(|migration| (&migration.component, &migration.version));
        migrations
    }

    pub fn source_path(&self, migration: &Migration) -> PathBuf {
        self.root.join(&migration.file)
    }

    fn validate_sources(&self) -> Result<()> {
        self.validate_shape()?;
        for migration in &self.migrations {
            let path = self.source_path(migration);
            let content = normalized_source(&path)?;
            let actual = sha256_hex(&content);
            if actual != migration.sha256 {
                bail!(
                    "checksum mismatch for {}: manifest {}, actual {}",
                    path.display(),
                    migration.sha256,
                    actual
                );
            }
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<()> {
        let mut baseline_databases = BTreeSet::new();
        for baseline in &self.baselines {
            if !baseline_databases.insert(baseline.database) {
                bail!("duplicate {} baseline", baseline.database.as_str());
            }
            if baseline.content_version.is_some() != baseline.cache_id.is_some() {
                bail!(
                    "{} baseline must declare content_version and cache_id together",
                    baseline.database.as_str()
                );
            }
        }
        if baseline_databases != DatabaseKind::ALL.into_iter().collect() {
            bail!("manifest must declare exactly one baseline for every database");
        }

        let mut keys = BTreeSet::new();
        for migration in &self.migrations {
            if migration.component.trim().is_empty() || migration.version.trim().is_empty() {
                bail!("migration component and version must not be empty");
            }
            let key = (
                migration.database,
                migration.component.as_str(),
                migration.version.as_str(),
            );
            if !keys.insert(key) {
                bail!(
                    "duplicate migration identity ({}, {}, {})",
                    migration.component,
                    migration.database.as_str(),
                    migration.version
                );
            }
            if migration.sha256.len() != 64
                || !migration
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                bail!(
                    "migration {}:{} has an invalid SHA-256",
                    migration.component,
                    migration.version
                );
            }
            if migration.adopt_query.as_ref().is_some_and(|query| {
                !query
                    .trim_start()
                    .to_ascii_uppercase()
                    .starts_with("SELECT ")
                    || query.contains(';')
            }) {
                bail!(
                    "migration {}:{} adopt_query must be one SELECT without a semicolon",
                    migration.component,
                    migration.version
                );
            }
        }
        Ok(())
    }
}

pub fn bundled_manifest() -> Result<MigrationManifest> {
    MigrationManifest::load_embedded(include_str!("../../../database/migrations/manifest.toml"))
}

pub async fn inspect_database(
    pool: &MySqlPool,
    manifest: &MigrationManifest,
    database: DatabaseKind,
) -> Result<DatabaseReport> {
    let baseline = manifest.baseline(database)?;
    let baseline_compatible = baseline_matches(pool, baseline).await?;
    let history_present = table_exists(pool, HISTORY_TABLE).await?;
    let history = if history_present {
        read_history(pool).await?
    } else {
        Vec::new()
    };
    Ok(build_report(
        manifest,
        database,
        baseline_compatible,
        history_present,
        &history,
        &BTreeSet::new(),
    ))
}

/// Administrative read-only view, including exact legacy-history imports that
/// the next `migrate` would record without reapplying SQL.
pub async fn inspect_database_for_admin(
    pool: &MySqlPool,
    manifest: &MigrationManifest,
    database: DatabaseKind,
) -> Result<DatabaseReport> {
    let baseline = manifest.baseline(database)?;
    let baseline_compatible = baseline_matches(pool, baseline).await?;
    let history_present = table_exists(pool, HISTORY_TABLE).await?;
    let history = if history_present {
        read_history(pool).await?
    } else {
        Vec::new()
    };
    let legacy_imports = if history_present {
        BTreeSet::new()
    } else {
        legacy_import_candidates(pool, manifest, database).await?
    };
    Ok(build_report(
        manifest,
        database,
        baseline_compatible,
        history_present,
        &history,
        &legacy_imports,
    ))
}

/// Bounded, read-only compatibility gate used before a server opens listeners.
pub async fn validate_runtime_schema(
    pool: &MySqlPool,
    manifest: &MigrationManifest,
    database: DatabaseKind,
) -> Result<DatabaseReport> {
    let report = inspect_database(pool, manifest, database).await?;
    if !report.compatible {
        bail!(
            "{} database schema is incompatible: {}. Run `rustycore-db status` then `rustycore-db migrate` before starting the server",
            database.as_str(),
            report.problems.join("; ")
        );
    }
    Ok(report)
}

pub async fn migrate_database(
    pool: &MySqlPool,
    manifest: &MigrationManifest,
    database: DatabaseKind,
) -> Result<DatabaseReport> {
    let baseline = manifest.baseline(database)?;
    if !baseline_matches(pool, baseline).await? {
        bail!(
            "{} database does not match the supported baseline; import the pinned baseline through the #255 bootstrap flow",
            database.as_str()
        );
    }

    let mut connection = pool.acquire().await?;
    let lock_name = format!("{LOCK_PREFIX}{}", database.as_str());
    let acquired: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, 0)")
        .bind(&lock_name)
        .fetch_one(&mut *connection)
        .await?;
    if acquired != Some(1) {
        bail!(
            "migration lock for {} is held by another process; no migration was started",
            database.as_str()
        );
    }

    let result = migrate_while_locked(&mut connection, manifest, database).await;
    let release_result: Result<Option<i64>, sqlx::Error> =
        sqlx::query_scalar("SELECT RELEASE_LOCK(?)")
            .bind(&lock_name)
            .fetch_one(&mut *connection)
            .await;
    match release_result {
        Ok(Some(1)) => {}
        Ok(other) => bail!("migration advisory lock release returned {other:?}"),
        Err(error) => {
            return Err(error).context("migration completed but advisory lock release failed");
        }
    }
    drop(connection);
    result?;
    inspect_database(pool, manifest, database).await
}

async fn migrate_while_locked(
    connection: &mut PoolConnection<MySql>,
    manifest: &MigrationManifest,
    database: DatabaseKind,
) -> Result<()> {
    ensure_history_table(connection).await?;
    import_transition_history(connection, manifest, database).await?;
    let history = read_history_from(connection).await?;
    let report = build_report(manifest, database, true, true, &history, &BTreeSet::new());
    if report
        .migrations
        .iter()
        .any(|migration| migration.status == MigrationStatus::Incomplete)
    {
        bail!(
            "{} has an incomplete migration. MariaDB DDL may already be committed; inspect the recorded migration and database state, restore a backup or repair it explicitly, then update history deliberately",
            database.as_str()
        );
    }
    let immutable_problems: Vec<_> = report
        .problems
        .iter()
        .filter(|problem| !problem.starts_with("pending migration"))
        .collect();
    if !immutable_problems.is_empty() {
        bail!(
            "migration validation failed: {}",
            report.problems.join("; ")
        );
    }

    for migration in manifest.migrations_for(database) {
        if migration.state == MigrationState::Archived
            || history
                .iter()
                .any(|row| row.component == migration.component && row.version == migration.version)
        {
            continue;
        }
        apply_one(connection, manifest, migration).await?;
    }
    Ok(())
}

async fn apply_one(
    connection: &mut PoolConnection<MySql>,
    manifest: &MigrationManifest,
    migration: &Migration,
) -> Result<()> {
    let next_rank: u64 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(MAX(`installed_rank`), 0) + 1 AS UNSIGNED) FROM `rustycore_schema_history`",
    )
    .fetch_one(&mut **connection)
    .await?;
    sqlx::query(
        "INSERT INTO `rustycore_schema_history` \
         (`installed_rank`, `component`, `database_name`, `version`, `description`, `script`, `checksum_sha256`, `execution_time_ms`, `success`, `failure_message`) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, 0, NULL)",
    )
    .bind(next_rank)
    .bind(&migration.component)
    .bind(migration.database.as_str())
    .bind(&migration.version)
    .bind(&migration.description)
    .bind(&migration.file)
    .bind(&migration.sha256)
    .execute(&mut **connection)
    .await?;

    let content = normalized_source(&manifest.source_path(migration))?;
    let started = Instant::now();
    for statement in split_sql(&content) {
        if let Err(error) = sqlx::query(statement).execute(&mut **connection).await {
            let message = format!("{error}");
            let _ = sqlx::query(
                "UPDATE `rustycore_schema_history` SET `execution_time_ms` = ?, `failure_message` = ? WHERE `component` = ? AND `version` = ?",
            )
            .bind(elapsed_millis(started))
            .bind(&message)
            .bind(&migration.component)
            .bind(&migration.version)
            .execute(&mut **connection)
            .await;
            bail!(
                "migration {}:{} failed and remains marked incomplete; MariaDB DDL may already be committed: {}",
                migration.component,
                migration.version,
                error
            );
        }
    }
    sqlx::query(
        "UPDATE `rustycore_schema_history` SET `execution_time_ms` = ?, `success` = 1, `failure_message` = NULL WHERE `component` = ? AND `version` = ?",
    )
    .bind(elapsed_millis(started))
    .bind(&migration.component)
    .bind(&migration.version)
    .execute(&mut **connection)
    .await?;
    Ok(())
}

fn elapsed_millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

async fn ensure_history_table(connection: &mut PoolConnection<MySql>) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS `rustycore_schema_history` (\
         `installed_rank` bigint unsigned NOT NULL,\
         `component` varchar(96) NOT NULL,\
         `database_name` varchar(32) NOT NULL,\
         `version` varchar(96) NOT NULL,\
         `description` varchar(255) NOT NULL,\
         `script` varchar(512) NOT NULL,\
         `checksum_sha256` char(64) NOT NULL,\
         `installed_on` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,\
         `execution_time_ms` bigint unsigned NOT NULL,\
         `success` tinyint(1) NOT NULL,\
         `failure_message` text DEFAULT NULL,\
         PRIMARY KEY (`component`, `version`),\
         UNIQUE KEY `uq_rustycore_schema_history_rank` (`installed_rank`)\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci",
    )
    .execute(&mut **connection)
    .await?;
    Ok(())
}

async fn import_transition_history(
    connection: &mut PoolConnection<MySql>,
    manifest: &MigrationManifest,
    database: DatabaseKind,
) -> Result<()> {
    let history_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM `rustycore_schema_history`")
        .fetch_one(&mut **connection)
        .await?;
    if history_count != 0 {
        return Ok(());
    }
    let old_rows: Vec<(String, String)> = if table_exists_on(connection, "updates").await? {
        sqlx::query_as("SELECT `name`, `hash` FROM `updates`")
            .fetch_all(&mut **connection)
            .await?
    } else {
        Vec::new()
    };
    let old: BTreeMap<_, _> = old_rows.into_iter().collect();
    let mut imports = Vec::new();
    for migration in manifest.migrations_for(database) {
        let Some(file_name) = Path::new(&migration.file)
            .file_name()
            .and_then(|name| name.to_str())
        else {
            bail!("migration file has no UTF-8 filename: {}", migration.file);
        };
        if let Some(old_hash) = old.get(file_name) {
            let content = normalized_source(&manifest.source_path(migration))?;
            let expected_old_hash = sha1_hex(&content);
            if old_hash != &expected_old_hash {
                bail!(
                    "legacy history checksum mismatch for {file_name}; refusing to infer that it was applied"
                );
            }
            imports.push(migration);
            continue;
        }
        if let Some(query) = &migration.adopt_query {
            let matches: i64 = sqlx::query_scalar(query)
                .fetch_one(&mut **connection)
                .await
                .with_context(|| {
                    format!(
                        "schema adoption probe failed for {}:{}",
                        migration.component, migration.version
                    )
                })?;
            if matches == 1 {
                imports.push(migration);
            }
        }
    }
    if imports.is_empty() {
        return Ok(());
    }

    sqlx::query("START TRANSACTION")
        .execute(&mut **connection)
        .await?;
    let import_result: Result<()> = async {
        for (index, migration) in imports.into_iter().enumerate() {
            let rank = i64::try_from(index + 1)?;
            sqlx::query(
                "INSERT INTO `rustycore_schema_history` (`installed_rank`, `component`, `database_name`, `version`, `description`, `script`, `checksum_sha256`, `execution_time_ms`, `success`, `failure_message`) VALUES (?, ?, ?, ?, ?, ?, ?, 0, 1, NULL)",
            )
            .bind(rank)
            .bind(&migration.component)
            .bind(database.as_str())
            .bind(&migration.version)
            .bind(&migration.description)
            .bind(&migration.file)
            .bind(&migration.sha256)
            .execute(&mut **connection)
            .await?;
        }
        Ok(())
    }
    .await;
    if let Err(error) = import_result {
        let _ = sqlx::query("ROLLBACK").execute(&mut **connection).await;
        return Err(error).context("legacy history import rolled back");
    }
    sqlx::query("COMMIT").execute(&mut **connection).await?;
    Ok(())
}

fn build_report(
    manifest: &MigrationManifest,
    database: DatabaseKind,
    baseline_compatible: bool,
    history_present: bool,
    history: &[HistoryRow],
    legacy_imports: &BTreeSet<(String, String)>,
) -> DatabaseReport {
    let expected: BTreeMap<_, _> = manifest
        .migrations_for(database)
        .into_iter()
        .map(|migration| {
            (
                (migration.component.as_str(), migration.version.as_str()),
                migration,
            )
        })
        .collect();
    let applied: BTreeMap<_, _> = history
        .iter()
        .map(|row| ((row.component.as_str(), row.version.as_str()), row))
        .collect();
    let mut problems = Vec::new();
    if !baseline_compatible {
        problems.push("supported baseline marker/version is absent".to_string());
    }
    if !history_present {
        problems.push("migration history table is absent".to_string());
    }
    for row in history {
        let Some(migration) = expected.get(&(row.component.as_str(), row.version.as_str())) else {
            problems.push(format!(
                "applied migration {}:{} is absent from the manifest",
                row.component, row.version
            ));
            continue;
        };
        if row.checksum != migration.sha256 {
            problems.push(format!(
                "applied migration {}:{} checksum differs from the immutable manifest",
                row.component, row.version
            ));
        }
        if !row.success {
            problems.push(format!(
                "incomplete migration {}:{} requires explicit recovery",
                row.component, row.version
            ));
        }
    }
    let migrations = manifest
        .migrations_for(database)
        .into_iter()
        .map(|migration| {
            let row = applied.get(&(migration.component.as_str(), migration.version.as_str()));
            let status = match row {
                Some(row) if row.success => MigrationStatus::Applied,
                Some(_) => MigrationStatus::Incomplete,
                None if legacy_imports
                    .contains(&(migration.component.clone(), migration.version.clone())) =>
                {
                    MigrationStatus::TransitionImport
                }
                None => MigrationStatus::Pending,
            };
            if status == MigrationStatus::Pending && migration.state == MigrationState::Active {
                problems.push(format!(
                    "pending migration {}:{}",
                    migration.component, migration.version
                ));
            }
            MigrationView {
                component: migration.component.clone(),
                database: migration.database,
                version: migration.version.clone(),
                description: migration.description.clone(),
                file: migration.file.clone(),
                sha256: migration.sha256.clone(),
                state: migration.state.clone(),
                status,
            }
        })
        .collect();
    DatabaseReport {
        database,
        baseline_compatible,
        history_present,
        compatible: problems.is_empty(),
        migrations,
        problems,
    }
}

async fn legacy_import_candidates(
    pool: &MySqlPool,
    manifest: &MigrationManifest,
    database: DatabaseKind,
) -> Result<BTreeSet<(String, String)>> {
    let old_rows: Vec<(String, String)> = if table_exists(pool, "updates").await? {
        sqlx::query_as("SELECT `name`, `hash` FROM `updates`")
            .fetch_all(pool)
            .await?
    } else {
        Vec::new()
    };
    let old: BTreeMap<_, _> = old_rows.into_iter().collect();
    let mut candidates = BTreeSet::new();
    for migration in manifest.migrations_for(database) {
        let Some(file_name) = Path::new(&migration.file)
            .file_name()
            .and_then(|name| name.to_str())
        else {
            bail!("migration file has no UTF-8 filename: {}", migration.file);
        };
        if let Some(old_hash) = old.get(file_name) {
            let content = normalized_source(&manifest.source_path(migration))?;
            if old_hash != &sha1_hex(&content) {
                bail!(
                    "legacy history checksum mismatch for {file_name}; refusing to infer that it was applied"
                );
            }
            candidates.insert((migration.component.clone(), migration.version.clone()));
            continue;
        }
        if let Some(query) = &migration.adopt_query {
            let matches: i64 = sqlx::query_scalar(query)
                .fetch_one(pool)
                .await
                .with_context(|| {
                    format!(
                        "schema adoption probe failed for {}:{}",
                        migration.component, migration.version
                    )
                })?;
            if matches == 1 {
                candidates.insert((migration.component.clone(), migration.version.clone()));
            }
        }
    }
    Ok(candidates)
}

async fn baseline_matches(pool: &MySqlPool, baseline: &BaselineRequirement) -> Result<bool> {
    if !table_exists(pool, &baseline.marker_table).await? {
        return Ok(false);
    }
    let (Some(expected_version), Some(expected_cache)) =
        (&baseline.content_version, baseline.cache_id)
    else {
        return Ok(true);
    };
    let row = sqlx::query("SELECT `db_version`, `cache_id` FROM `version` LIMIT 1")
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some_and(|row| {
        row.try_get::<String, _>(0).ok().as_deref() == Some(expected_version.as_str())
            && row.try_get::<i32, _>(1).ok() == Some(expected_cache)
    }))
}

async fn table_exists(pool: &MySqlPool, table: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
    )
    .bind(table)
    .fetch_one(pool)
    .await?;
    Ok(count == 1)
}

async fn table_exists_on(connection: &mut PoolConnection<MySql>, table: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
    )
    .bind(table)
    .fetch_one(&mut **connection)
    .await?;
    Ok(count == 1)
}

async fn read_history(pool: &MySqlPool) -> Result<Vec<HistoryRow>> {
    let rows = sqlx::query(
        "SELECT `component`, `version`, `checksum_sha256`, `success` FROM `rustycore_schema_history` ORDER BY `installed_rank`",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(history_row).collect())
}

async fn read_history_from(connection: &mut PoolConnection<MySql>) -> Result<Vec<HistoryRow>> {
    let rows = sqlx::query(
        "SELECT `component`, `version`, `checksum_sha256`, `success` FROM `rustycore_schema_history` ORDER BY `installed_rank`",
    )
    .fetch_all(&mut **connection)
    .await?;
    Ok(rows.into_iter().map(history_row).collect())
}

fn history_row(row: sqlx::mysql::MySqlRow) -> HistoryRow {
    HistoryRow {
        component: row.get(0),
        version: row.get(1),
        checksum: row.get(2),
        success: row.get::<i8, _>(3) != 0,
    }
}

fn normalized_source(path: &Path) -> Result<String> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("missing migration file {}", path.display()))?;
    Ok(content.replace("\r\n", "\n").replace('\r', "\n"))
}

fn sha256_hex(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn sha1_hex(content: &str) -> String {
    format!("{:x}", Sha1::digest(content.as_bytes()))
}

fn split_sql(content: &str) -> Vec<&str> {
    let bytes = content.as_bytes();
    let mut statements = Vec::new();
    let (mut start, mut index) = (0, 0);
    while index < bytes.len() {
        match bytes[index] {
            b'-' if index + 1 < bytes.len() && bytes[index + 1] == b'-' => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'#' => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                {
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            quote @ (b'\'' | b'"') => {
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes[index] == quote {
                        index += 1;
                        break;
                    } else {
                        index += 1;
                    }
                }
            }
            b';' => {
                let statement = content[start..index].trim();
                if !statement.is_empty() {
                    statements.push(statement);
                }
                index += 1;
                start = index;
            }
            _ => index += 1,
        }
    }
    let tail = content[start..].trim();
    if !tail.is_empty() {
        statements.push(tail);
    }
    statements
}

#[cfg(test)]
#[path = "migration/tests/mod.rs"]
mod tests;
