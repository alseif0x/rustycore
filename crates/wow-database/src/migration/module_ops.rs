//! `rustycore-db module apply|remove|purge` (#1444 H6).
//!
//! Every operation is a dry run unless the operator passes `--yes`:
//!
//! - **apply** runs a composed module's pending migrations, in version order,
//!   after the core schema is current and every module earlier in
//!   `enabled_order` is applied. Each file must still match the checksum the
//!   lock recorded and pass the static guard (`module_sql`).
//! - **remove** keeps every table and history row. It copies the module's
//!   `data/sql` to `modules/.retired/<id>/data/sql` and writes the module's
//!   tombstone into `modules.lock.toml`, so the server keeps accepting its
//!   history once the checkout is gone.
//! - **purge** runs a tombstoned module's reviewed DROP scripts (guarded:
//!   only its own tables), deletes its history rows and its tombstone.
//!
//! Module SQL changes only `characters` and `world`.

use anyhow::{Context, Result, bail};
use serde::Serialize;
use sqlx::{MySql, MySqlPool, pool::PoolConnection};
use std::fs;
use std::path::{Path, PathBuf};

use super::module_sql::{check_script, table_prefix};
use super::modules::{
    LockedModule, ModuleMigration, ModuleSchema, ModuleSql, module_component, parse_schema_version,
};
use super::{
    DatabaseKind, HISTORY_TABLE, LOCK_PREFIX, MigrationManifest, MigrationState, MigrationStatus,
    apply_one_content, ensure_history_table, read_history, read_history_from, sha256_hex,
    split_sql, table_exists,
};

/// Where `remove` keeps a module's SQL once its checkout is gone.
pub const RETIRED_DIR: &str = "modules/.retired";
/// The line that opens the lock's tombstone block (shared with compose.py).
pub const TOMBSTONE_MARKER: &str =
    "# --- tombstones: written by rustycore-db module remove; kept as is by compose.py ---";

/// What one module operation found or did in one database.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ModuleDatabaseReport {
    pub database: Option<DatabaseKind>,
    /// Migrations applied by this run, or that a dry run would apply.
    pub pending: Vec<String>,
    /// History rows of the module already in the database.
    pub history: Vec<String>,
    /// The module's tables present in the database.
    pub tables: Vec<String>,
    /// Purge statements a run executes.
    pub statements: Vec<String>,
}

fn source_file(repo_root: &Path, sql: &ModuleSql, file: &str, sha256: &str) -> Result<String> {
    let path = repo_root.join(&sql.source).join(file);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("missing module SQL file {}", path.display()))?
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let actual = sha256_hex(&content);
    if actual != sha256 {
        bail!(
            "{} changed since modules.lock.toml recorded it (lock {sha256}, file {actual}); a recorded migration never changes — add a new one and sync",
            path.display()
        );
    }
    Ok(content)
}

fn guarded(module_id: &str, file: &str, content: &str, purge: bool) -> Result<()> {
    let problems = check_script(content, &table_prefix(module_id), purge);
    if !problems.is_empty() {
        bail!(
            "module {module_id}: {file} is refused by the module SQL guard: {}",
            problems.join("; ")
        );
    }
    Ok(())
}

fn active<'a>(schema: &'a ModuleSchema, id: &str) -> Result<&'a LockedModule> {
    schema.active(id).with_context(|| {
        if schema.tombstone(id).is_some() {
            format!("module {id} is removed (tombstoned); re-install and sync it before applying")
        } else {
            format!("module {id} has no SQL in modules.lock.toml; install it and run compose sync")
        }
    })
}

async fn module_tables(pool: &MySqlPool, module_id: &str) -> Result<Vec<String>> {
    let prefix = table_prefix(module_id).replace('_', "\\_");
    Ok(sqlx::query_scalar(
        "SELECT CAST(table_name AS CHAR) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name LIKE ? ORDER BY table_name",
    )
    .bind(format!("{prefix}%"))
    .fetch_all(pool)
    .await?)
}

async fn module_history(pool: &MySqlPool, module_id: &str) -> Result<Vec<String>> {
    if !table_exists(pool, HISTORY_TABLE).await? {
        return Ok(Vec::new());
    }
    let component = module_component(module_id);
    Ok(read_history(pool)
        .await?
        .into_iter()
        .filter(|row| row.component == component)
        .map(|row| {
            format!(
                "{}{}",
                row.version,
                if row.success { "" } else { " (incomplete)" }
            )
        })
        .collect())
}

async fn lock_database(
    connection: &mut PoolConnection<MySql>,
    database: DatabaseKind,
) -> Result<String> {
    let lock_name = format!("{LOCK_PREFIX}{}", database.as_str());
    let acquired: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, 0)")
        .bind(&lock_name)
        .fetch_one(&mut **connection)
        .await?;
    if acquired != Some(1) {
        bail!(
            "migration lock for {} is held by another process; nothing was changed",
            database.as_str()
        );
    }
    Ok(lock_name)
}

async fn unlock_database(connection: &mut PoolConnection<MySql>, lock_name: &str) -> Result<()> {
    let released: Option<i64> = sqlx::query_scalar("SELECT RELEASE_LOCK(?)")
        .bind(lock_name)
        .fetch_one(&mut **connection)
        .await?;
    if released != Some(1) {
        bail!("migration advisory lock release returned {released:?}");
    }
    Ok(())
}

/// `rustycore-db module apply <id>` for one database. `manifest` is the core
/// manifest merged with `schema`.
pub async fn apply_module(
    pool: &MySqlPool,
    manifest: &MigrationManifest,
    schema: &ModuleSchema,
    repo_root: &Path,
    module_id: &str,
    database: DatabaseKind,
    execute: bool,
) -> Result<ModuleDatabaseReport> {
    let module = active(schema, module_id)?;
    let sql = module.sql.as_ref().expect("active modules have SQL");
    let mut report = ModuleDatabaseReport {
        database: Some(database),
        history: module_history(pool, module_id).await?,
        tables: module_tables(pool, module_id).await?,
        ..ModuleDatabaseReport::default()
    };
    // The one apply order: enabled_order, id, numeric version (#1444 H6a F3).
    let migrations: Vec<&ModuleMigration> = schema
        .ordered_migrations(database)
        .into_iter()
        .filter(|(owner, _)| owner.id == module.id)
        .map(|(_, migration)| migration)
        .collect();
    if migrations.is_empty() {
        return Ok(report);
    }

    // The core schema it builds on must be there, and current.
    let minimum = parse_schema_version(&sql.min_core_schema).expect("validated");
    if !manifest.migrations.iter().any(|migration| {
        migration.component == "core"
            && parse_schema_version(&migration.version).is_some_and(|version| version >= minimum)
    }) {
        bail!(
            "module {module_id} needs core schema {} or newer; this core's manifest is older",
            sql.min_core_schema
        );
    }
    let state = super::inspect_database(pool, manifest, database).await?;
    let core_problems: Vec<_> = state
        .problems
        .iter()
        .filter(|problem| !problem.contains(super::modules::MODULE_COMPONENT_PREFIX))
        .collect();
    if !core_problems.is_empty() {
        bail!(
            "{} core schema is not current ({}); run `rustycore-db migrate` first",
            database.as_str(),
            core_problems
                .iter()
                .map(|problem| problem.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
    // Earlier modules first: `enabled_order`, then version.
    for earlier in schema
        .modules
        .iter()
        .take_while(|other| other.id != module.id)
    {
        let component = module_component(&earlier.id);
        if let Some(pending) = state
            .migrations
            .iter()
            .find(|view| view.component == component && view.status != MigrationStatus::Applied)
        {
            bail!(
                "module {} comes earlier in enabled_order and has {:?} migration {}; apply it first",
                earlier.id,
                pending.status,
                pending.version
            );
        }
    }
    let component = module_component(module_id);
    let mut todo = Vec::new();
    for migration in migrations {
        let view = state
            .migrations
            .iter()
            .find(|view| view.component == component && view.version == migration.version)
            .expect("merged manifest lists every module migration");
        match view.status {
            MigrationStatus::Applied => continue,
            MigrationStatus::Incomplete => bail!(
                "module {module_id} migration {} is incomplete; MariaDB DDL may already be committed — inspect and repair it explicitly",
                migration.version
            ),
            _ => {}
        }
        let content = source_file(repo_root, sql, &migration.file, &migration.sha256)?;
        guarded(module_id, &migration.file, &content, false)?;
        report.pending.push(migration.version.clone());
        todo.push((migration, content));
    }
    if !execute || todo.is_empty() {
        return Ok(report);
    }

    let mut connection = pool.acquire().await?;
    let lock_name = lock_database(&mut connection, database).await?;
    let result = async {
        ensure_history_table(&mut connection).await?;
        // Re-read under the lock: another operator may have applied some.
        let history = read_history_from(&mut connection).await?;
        for (migration, content) in &todo {
            if history
                .iter()
                .any(|row| row.component == component && row.version == migration.version)
            {
                bail!(
                    "module {module_id} migration {} appeared while waiting; rerun the dry run",
                    migration.version
                );
            }
            let entry = super::Migration {
                component: component.clone(),
                database,
                version: migration.version.clone(),
                description: migration.description.clone(),
                file: format!("{}/{}", sql.source, migration.file),
                sha256: migration.sha256.clone(),
                state: MigrationState::Active,
                adopt_query: None,
            };
            apply_one_content(&mut connection, &entry, content).await?;
        }
        Ok(())
    }
    .await;
    let released = unlock_database(&mut connection, &lock_name).await;
    result?;
    released?;
    report.history = module_history(pool, module_id).await?;
    report.tables = module_tables(pool, module_id).await?;
    Ok(report)
}

/// The read-only part of `remove`: what stays in `database`.
pub async fn retained_data(
    pool: &MySqlPool,
    module_id: &str,
    database: DatabaseKind,
) -> Result<ModuleDatabaseReport> {
    Ok(ModuleDatabaseReport {
        database: Some(database),
        history: module_history(pool, module_id).await?,
        tables: module_tables(pool, module_id).await?,
        ..ModuleDatabaseReport::default()
    })
}

/// The purge statements of `module_id` for `database`, verified and guarded.
pub fn purge_statements(
    schema: &ModuleSchema,
    repo_root: &Path,
    module_id: &str,
    database: DatabaseKind,
) -> Result<Vec<String>> {
    if schema.active(module_id).is_some() {
        bail!("module {module_id} is still composed; remove it (and sync) before purging");
    }
    let tombstone = schema.tombstone(module_id).with_context(|| {
        format!("module {module_id} has no tombstone; only a removed module can be purged")
    })?;
    let sql = tombstone.sql.as_ref().expect("validated");
    let mut statements = Vec::new();
    for purge in sql.purge.iter().filter(|purge| purge.database == database) {
        let content = source_file(repo_root, sql, &purge.file, &purge.sha256)?;
        guarded(module_id, &purge.file, &content, true)?;
        statements.extend(split_sql(&content).into_iter().map(str::to_owned));
    }
    Ok(statements)
}

/// `rustycore-db module purge <id>` for one database.
pub async fn purge_module(
    pool: &MySqlPool,
    schema: &ModuleSchema,
    repo_root: &Path,
    module_id: &str,
    database: DatabaseKind,
    execute: bool,
) -> Result<ModuleDatabaseReport> {
    let statements = purge_statements(schema, repo_root, module_id, database)?;
    let mut report = retained_data(pool, module_id, database).await?;
    report.statements = statements.clone();
    if !execute {
        return Ok(report);
    }
    let mut connection = pool.acquire().await?;
    let lock_name = lock_database(&mut connection, database).await?;
    let component = module_component(module_id);
    let result = async {
        for statement in &statements {
            sqlx::query(statement)
                .execute(&mut *connection)
                .await
                .with_context(|| format!("purge statement failed: {statement}"))?;
        }
        if super::table_exists_on(&mut connection, HISTORY_TABLE).await? {
            sqlx::query("DELETE FROM `rustycore_schema_history` WHERE `component` = ?")
                .bind(&component)
                .execute(&mut *connection)
                .await?;
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    let released = unlock_database(&mut connection, &lock_name).await;
    result?;
    released?;
    report.history = module_history(pool, module_id).await?;
    report.tables = module_tables(pool, module_id).await?;
    Ok(report)
}

// ------------------------------------------------------------- lock edits

fn render_entry(module: &LockedModule) -> String {
    let sql = module.sql.as_ref().expect("tombstones carry SQL");
    let mut text = format!(
        "[[tombstone]]\nid = \"{}\"\nenabled_order = {}\n\n[tombstone.sql]\nmin_core_schema = \"{}\"\nsource = \"{}\"\n\n",
        module.id, module.enabled_order, sql.min_core_schema, sql.source
    );
    for migration in &sql.migration {
        text += &format!(
            "[[tombstone.sql.migration]]\ndatabase = \"{}\"\nversion = \"{}\"\ndescription = \"{}\"\nfile = \"{}\"\nsha256 = \"{}\"\n\n",
            migration.database.as_str(),
            migration.version,
            migration.description,
            migration.file,
            migration.sha256
        );
    }
    for purge in &sql.purge {
        text += &format!(
            "[[tombstone.sql.purge]]\ndatabase = \"{}\"\nfile = \"{}\"\nsha256 = \"{}\"\n\n",
            purge.database.as_str(),
            purge.file,
            purge.sha256
        );
    }
    text
}

/// The lock with its tombstone block replaced by `tombstones`.
#[must_use]
pub fn lock_with_tombstones(lock: &str, tombstones: &[LockedModule]) -> String {
    let generated = lock
        .find(TOMBSTONE_MARKER)
        .map_or(lock, |index| &lock[..index])
        .trim_end_matches('\n');
    if tombstones.is_empty() {
        return format!("{generated}\n");
    }
    let mut ordered: Vec<_> = tombstones.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    let block: String = ordered.into_iter().map(render_entry).collect();
    format!(
        "{generated}\n\n{TOMBSTONE_MARKER}\n{}\n",
        block.trim_end_matches('\n')
    )
}

/// What `remove --yes` writes: the retained SQL path and the new lock text.
pub struct RemovalPlan {
    pub retained: PathBuf,
    pub lock: String,
    pub tombstone: LockedModule,
}

/// Plan `remove`: the tombstone and where the module's SQL is retained.
pub fn plan_removal(lock_text: &str, repo_root: &Path, module_id: &str) -> Result<RemovalPlan> {
    let schema = ModuleSchema::from_lock(lock_text)?;
    let module = schema.active(module_id).with_context(|| {
        if schema.tombstone(module_id).is_some() {
            format!("module {module_id} is already removed (tombstoned)")
        } else {
            format!("module {module_id} has no SQL in modules.lock.toml; `rustycore-module remove` is enough")
        }
    })?;
    let sql = module.sql.as_ref().expect("active modules have SQL");
    // Every recorded file must still be the one recorded.
    for migration in &sql.migration {
        source_file(repo_root, sql, &migration.file, &migration.sha256)?;
    }
    for purge in &sql.purge {
        source_file(repo_root, sql, &purge.file, &purge.sha256)?;
    }
    let retained = PathBuf::from(RETIRED_DIR).join(module_id).join("data/sql");
    let mut tombstone = module.clone();
    tombstone.sql.as_mut().expect("checked").source = retained.to_string_lossy().into_owned();
    let mut tombstones: Vec<_> = schema
        .tombstones
        .iter()
        .filter(|other| other.id != module_id)
        .cloned()
        .collect();
    tombstones.push(tombstone.clone());
    let lock = lock_with_tombstones(lock_text, &tombstones);
    // The new lock must parse into the same history.
    ModuleSchema::from_lock(&lock).context("the tombstoned lock does not parse")?;
    Ok(RemovalPlan {
        retained,
        lock,
        tombstone,
    })
}

/// Copy the recorded SQL files into the retained directory.
pub fn retain_sql(repo_root: &Path, sql: &ModuleSql, plan: &RemovalPlan) -> Result<()> {
    let target = repo_root.join(&plan.retained);
    for file in sql
        .migration
        .iter()
        .map(|migration| (&migration.file, &migration.sha256))
        .chain(sql.purge.iter().map(|purge| (&purge.file, &purge.sha256)))
    {
        let content = source_file(repo_root, sql, file.0, file.1)?;
        let destination = target.join(file.0);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, content)?;
    }
    Ok(())
}

/// The lock once `module_id`'s tombstone is purged.
pub fn lock_without_tombstone(lock_text: &str, module_id: &str) -> Result<String> {
    let schema = ModuleSchema::from_lock(lock_text)?;
    let tombstones: Vec<_> = schema
        .tombstones
        .iter()
        .filter(|other| other.id != module_id)
        .cloned()
        .collect();
    Ok(lock_with_tombstones(lock_text, &tombstones))
}
