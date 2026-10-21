//! `rustycore-db module apply|remove|purge <id>` (#1444 H6).
//!
//! Dry run by default; `--yes` executes. Module SQL changes only the
//! `characters` and `world` databases; see
//! `wow_database::migration::module_ops` for the rules each command enforces.

use anyhow::{Context, Result, bail};
use serde::Serialize;
use sqlx::MySqlPool;
use std::path::PathBuf;
use wow_database::migration::module_ops::{
    self, ModuleDatabaseReport, apply_module, plan_removal, purge_module, retain_sql, retained_data,
};
use wow_database::migration::modules::MODULE_DATABASES;
use wow_database::migration::{DatabaseKind, MigrationManifest, ModuleSchema};
use wow_database::{Database, build_connection_string_with_ssl_like_cpp};

use crate::{DEFAULT_CONFIG, DEFAULT_LOCK, DEFAULT_MANIFEST, EXIT_OK, EXIT_USAGE, database_info};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Action {
    Apply,
    Remove,
    Purge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModuleCli {
    pub(crate) action: Action,
    pub(crate) id: String,
    pub(crate) execute: bool,
    pub(crate) json: bool,
    pub(crate) manifest: PathBuf,
    pub(crate) config: PathBuf,
    pub(crate) lock: PathBuf,
}

#[derive(Debug, Serialize)]
struct Output {
    command: &'static str,
    action: Action,
    module: String,
    executed: bool,
    /// Lock and filesystem changes made (or, in a dry run, that would be).
    changes: Vec<String>,
    databases: Vec<ModuleDatabaseReport>,
}

impl ModuleCli {
    pub(crate) fn parse(args: Vec<String>) -> std::result::Result<Self, String> {
        let mut args = args.into_iter();
        let action = match args.next().as_deref() {
            Some("apply") => Action::Apply,
            Some("remove") => Action::Remove,
            Some("purge") => Action::Purge,
            other => {
                return Err(format!(
                    "module needs apply, remove or purge, not {other:?}"
                ));
            }
        };
        let mut id = None;
        let mut execute = false;
        let mut json = false;
        let mut manifest = PathBuf::from(DEFAULT_MANIFEST);
        let mut config = PathBuf::from(DEFAULT_CONFIG);
        let mut lock = PathBuf::from(DEFAULT_LOCK);
        let mut config_given = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--yes" => execute = true,
                "--json" => json = true,
                "--manifest" | "--config" | "--lock" => {
                    let value = args
                        .next()
                        .ok_or_else(|| format!("{arg} requires a path"))?;
                    match arg.as_str() {
                        "--manifest" => manifest = PathBuf::from(value),
                        "--config" => {
                            config_given = true;
                            config = PathBuf::from(value);
                        }
                        _ => lock = PathBuf::from(value),
                    }
                }
                value if !value.starts_with('-') && id.is_none() => id = Some(value.to_owned()),
                unknown => return Err(format!("unknown or duplicate argument: {unknown}")),
            }
        }
        if execute && !config_given {
            return Err(
                "--yes changes databases or the lock: name the configuration with --config".into(),
            );
        }
        Ok(Self {
            action,
            id: id.ok_or("module needs a module id")?,
            execute,
            json,
            manifest,
            config,
            lock,
        })
    }

    /// The repository root: the lock's directory, where `sql.source` paths start.
    fn repo_root(&self) -> PathBuf {
        match self.lock.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => PathBuf::from("."),
        }
    }
}

/// Run `op` against one module database, opened with the configured credentials.
async fn with_pool<T>(
    database: DatabaseKind,
    write: bool,
    op: impl AsyncFnOnce(&MySqlPool) -> Result<T>,
) -> Result<T> {
    let info = database_info(database);
    let connection = build_connection_string_with_ssl_like_cpp(
        &info.host,
        &info.port_or_socket,
        &info.username,
        &info.password,
        &info.database,
        info.ssl,
    );
    eprintln!(
        "{}: connecting to {} database {} on {}:{}",
        if write { "write" } else { "read-only" },
        database.as_str(),
        info.database,
        info.host,
        info.port_or_socket
    );
    match database {
        DatabaseKind::Characters => {
            let db = Database::<wow_database::CharStatements>::open(&connection).await?;
            let result = op(db.pool()).await;
            db.close().await;
            result
        }
        DatabaseKind::World => {
            let db = Database::<wow_database::WorldStatements>::open(&connection).await?;
            let result = op(db.pool()).await;
            db.close().await;
            result
        }
        other => bail!("module SQL never touches the {} database", other.as_str()),
    }
}

pub(crate) async fn run(args: Vec<String>) -> Result<u8> {
    let cli = match ModuleCli::parse(args) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("{message}\n");
            crate::print_help();
            return Ok(EXIT_USAGE);
        }
    };
    wow_config::load_config(cli.config.to_string_lossy().as_ref())
        .with_context(|| format!("cannot load database config {}", cli.config.display()))?;
    let lock_text = std::fs::read_to_string(&cli.lock)
        .with_context(|| format!("cannot read {}", cli.lock.display()))?;
    let schema = ModuleSchema::from_lock(&lock_text)?;
    let manifest = MigrationManifest::load(&cli.manifest)?.with_module_schema(&schema)?;
    let root = cli.repo_root();
    let id = cli.id.as_str();
    let mut output = Output {
        command: "module",
        action: cli.action,
        module: cli.id.clone(),
        executed: cli.execute,
        changes: Vec::new(),
        databases: Vec::new(),
    };
    match cli.action {
        Action::Apply => {
            for database in MODULE_DATABASES {
                let report = with_pool(database, cli.execute, async |pool| {
                    apply_module(pool, &manifest, &schema, &root, id, database, cli.execute).await
                })
                .await?;
                output.databases.push(report);
            }
        }
        Action::Remove => {
            let plan = plan_removal(&lock_text, &root, id)?;
            for database in MODULE_DATABASES {
                let report = with_pool(database, false, async |pool| {
                    retained_data(pool, id, database).await
                })
                .await?;
                output.databases.push(report);
            }
            output.changes = vec![
                format!("retain data/sql in {}", plan.retained.display()),
                format!("write the {id} tombstone into {}", cli.lock.display()),
                "keep every table and history row".to_owned(),
            ];
            if cli.execute {
                let sql = schema
                    .active(id)
                    .and_then(|module| module.sql.as_ref())
                    .expect("plan_removal found it");
                retain_sql(&root, sql, &plan)?;
                write_atomically(&cli.lock, &plan.lock)?;
            }
        }
        Action::Purge => {
            for database in MODULE_DATABASES {
                // Verify every database's scripts before dropping anything.
                module_ops::purge_statements(&schema, &root, id, database)?;
            }
            for database in MODULE_DATABASES {
                let report = with_pool(database, cli.execute, async |pool| {
                    purge_module(pool, &schema, &root, id, database, cli.execute).await
                })
                .await?;
                output.databases.push(report);
            }
            let retained = root.join(module_ops::RETIRED_DIR).join(id);
            output.changes = vec![
                format!("delete the {id} history rows"),
                format!("drop the {id} tombstone from {}", cli.lock.display()),
                format!("delete {}", retained.display()),
            ];
            if cli.execute {
                write_atomically(
                    &cli.lock,
                    &module_ops::lock_without_tombstone(&lock_text, id)?,
                )?;
                if retained.is_dir() {
                    std::fs::remove_dir_all(&retained)?;
                }
            }
        }
    }
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human(&output);
    }
    Ok(EXIT_OK)
}

/// Replace `path` with `text` through a sibling temporary file and a rename,
/// so a crash never leaves a half-written lock.
fn write_atomically(path: &std::path::Path, text: &str) -> Result<()> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    std::fs::write(&temporary, text)
        .with_context(|| format!("cannot write {}", temporary.display()))?;
    std::fs::rename(&temporary, path).with_context(|| format!("cannot replace {}", path.display()))
}

fn print_human(output: &Output) {
    println!(
        "rustycore-db module {:?} {}: {}",
        output.action,
        output.module,
        if output.executed {
            "executed"
        } else {
            "dry run (pass --yes to execute)"
        }
    );
    for change in &output.changes {
        println!("  {change}");
    }
    for report in &output.databases {
        let database = report.database.map_or("?", DatabaseKind::as_str);
        println!(
            "{database}: history={:?} tables={:?}",
            report.history, report.tables
        );
        for version in &report.pending {
            println!(
                "  {} {version}",
                if output.executed {
                    "applied"
                } else {
                    "would apply"
                }
            );
        }
        for statement in &report.statements {
            println!(
                "  {} {statement}",
                if output.executed { "ran" } else { "would run" }
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> std::result::Result<ModuleCli, String> {
        ModuleCli::parse(args.iter().map(|arg| (*arg).to_owned()).collect())
    }

    #[test]
    fn module_commands_are_dry_runs_unless_yes() {
        let cli = parse(&["apply", "example.milestones"]).unwrap();
        assert_eq!(cli.action, Action::Apply);
        assert!(!cli.execute);
        assert!(
            parse(&["purge", "example.milestones", "--yes"]).is_err(),
            "--yes needs --config"
        );
        let cli = parse(&[
            "purge",
            "example.milestones",
            "--yes",
            "--config",
            "w.conf",
            "--lock",
            "x/modules.lock.toml",
        ])
        .unwrap();
        assert_eq!(cli.action, Action::Purge);
        assert!(cli.execute);
        assert_eq!(cli.repo_root(), PathBuf::from("x"));
        assert_eq!(
            parse(&["remove", "a.b"]).unwrap().repo_root(),
            PathBuf::from(".")
        );
        assert!(parse(&["drop", "a.b"]).is_err());
        assert!(parse(&["apply"]).is_err());
        assert!(parse(&["apply", "a.b", "c.d"]).is_err());
        assert!(parse(&["apply", "a.b", "--dry-run"]).is_err());
    }
}
