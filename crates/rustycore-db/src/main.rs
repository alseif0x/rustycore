//! Explicit RustyCore database administration.

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::process::ExitCode;
use wow_config::DatabaseInfo;
use wow_database::migration::{
    DatabaseKind, DatabaseReport, MigrationManifest, inspect_database_for_admin, migrate_database,
};
use wow_database::{Database, StatementDef, build_connection_string_with_ssl_like_cpp};

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 2;
const EXIT_INCOMPATIBLE: u8 = 3;
const EXIT_OPERATIONAL: u8 = 4;
const DEFAULT_MANIFEST: &str = "database/migrations/manifest.toml";
const DEFAULT_CONFIG: &str = "worldserver.conf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Status,
    Validate,
    Migrate { dry_run: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cli {
    command: Command,
    json: bool,
    manifest: PathBuf,
    config: PathBuf,
}

#[derive(Debug, Serialize)]
struct Output {
    command: &'static str,
    compatible: bool,
    mutated: bool,
    databases: Vec<DatabaseReport>,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()).await {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("rustycore-db: {error:#}");
            ExitCode::from(EXIT_OPERATIONAL)
        }
    }
}

async fn run(args: Vec<String>) -> Result<u8> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        print_help();
        return Ok(EXIT_OK);
    }
    let cli = match Cli::parse(args) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("{message}\n");
            print_help();
            return Ok(EXIT_USAGE);
        }
    };
    wow_config::load_config(cli.config.to_string_lossy().as_ref())
        .with_context(|| format!("cannot load database config {}", cli.config.display()))?;
    let manifest = MigrationManifest::load(&cli.manifest)?;
    let mut reports = Vec::new();
    for database in DatabaseKind::ALL {
        reports.push(run_one(&cli, &manifest, database).await?);
    }
    let compatible = reports.iter().all(|report| report.compatible);
    let output = Output {
        command: match cli.command {
            Command::Status => "status",
            Command::Validate => "validate",
            Command::Migrate { dry_run: true } => "migrate-dry-run",
            Command::Migrate { dry_run: false } => "migrate",
        },
        compatible,
        mutated: matches!(cli.command, Command::Migrate { dry_run: false }),
        databases: reports,
    };
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human(&output);
    }
    Ok(if compatible {
        EXIT_OK
    } else {
        EXIT_INCOMPATIBLE
    })
}

async fn run_one(
    cli: &Cli,
    manifest: &MigrationManifest,
    database: DatabaseKind,
) -> Result<DatabaseReport> {
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
        match cli.command {
            Command::Migrate { dry_run: false } => "write",
            _ => "read-only",
        },
        database.as_str(),
        info.database,
        info.host,
        info.port_or_socket
    );
    match database {
        DatabaseKind::Auth => {
            with_database::<wow_database::LoginStatements>(cli, manifest, database, &connection)
                .await
        }
        DatabaseKind::Characters => {
            with_database::<wow_database::CharStatements>(cli, manifest, database, &connection)
                .await
        }
        DatabaseKind::World => {
            with_database::<wow_database::WorldStatements>(cli, manifest, database, &connection)
                .await
        }
        DatabaseKind::Hotfixes => {
            with_database::<wow_database::HotfixStatements>(cli, manifest, database, &connection)
                .await
        }
    }
}

async fn with_database<S: StatementDef>(
    cli: &Cli,
    manifest: &MigrationManifest,
    database: DatabaseKind,
    connection: &str,
) -> Result<DatabaseReport> {
    let db = Database::<S>::open(connection).await?;
    let report = match cli.command {
        Command::Migrate { dry_run: false } => {
            migrate_database(db.pool(), manifest, database).await?
        }
        Command::Status | Command::Validate | Command::Migrate { dry_run: true } => {
            inspect_database_for_admin(db.pool(), manifest, database).await?
        }
    };
    db.close().await;
    Ok(report)
}

fn database_info(database: DatabaseKind) -> DatabaseInfo {
    let (key, default_name) = match database {
        DatabaseKind::Auth => ("Login", "auth"),
        DatabaseKind::Characters => ("Character", "characters"),
        DatabaseKind::World => ("World", "world"),
        DatabaseKind::Hotfixes => ("Hotfix", "hotfixes"),
    };
    wow_config::get_database_info_default(
        key,
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", default_name),
    )
}

impl Cli {
    fn parse(args: Vec<String>) -> std::result::Result<Self, String> {
        let mut command = None;
        let mut json = false;
        let mut dry_run = false;
        let mut manifest = PathBuf::from(DEFAULT_MANIFEST);
        let mut config = PathBuf::from(DEFAULT_CONFIG);
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "status" if command.is_none() => command = Some(Command::Status),
                "validate" if command.is_none() => command = Some(Command::Validate),
                "migrate" if command.is_none() => {
                    command = Some(Command::Migrate { dry_run: false })
                }
                "--json" => json = true,
                "--dry-run" => dry_run = true,
                "--manifest" | "--config" => {
                    let option = args[index].clone();
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| format!("{option} requires a path"))?;
                    if option == "--manifest" {
                        manifest = PathBuf::from(value);
                    } else {
                        config = PathBuf::from(value);
                    }
                }
                unknown => return Err(format!("unknown or duplicate argument: {unknown}")),
            }
            index += 1;
        }
        let mut command = command.ok_or_else(|| "missing command".to_string())?;
        if dry_run {
            if command != (Command::Migrate { dry_run: false }) {
                return Err("--dry-run is valid only with migrate".to_string());
            }
            command = Command::Migrate { dry_run: true };
        }
        Ok(Self {
            command,
            json,
            manifest,
            config,
        })
    }
}

fn print_human(output: &Output) {
    println!(
        "rustycore-db {}: {}",
        output.command,
        if output.compatible {
            "compatible"
        } else {
            "action required"
        }
    );
    for report in &output.databases {
        println!(
            "{}: baseline={}, history={}, compatible={}",
            report.database.as_str(),
            report.baseline_compatible,
            report.history_present,
            report.compatible
        );
        for migration in &report.migrations {
            println!(
                "  {:?} {}:{} {} {} [{}]",
                migration.status,
                migration.component,
                migration.version,
                migration.file,
                migration.description,
                migration.sha256
            );
        }
        for problem in &report.problems {
            println!("  problem: {problem}");
        }
    }
}

fn print_help() {
    println!(
        "rustycore-db <status|validate|migrate> [--dry-run] [--json] [--config PATH] [--manifest PATH]\n\
         \nExit codes:\n  0 compatible/success\n  2 invalid command line\n  3 schema incompatible or migrations pending\n  4 operational/database/lock/migration failure"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_required_command_surface() {
        assert_eq!(
            Cli::parse(vec!["status".into()]).unwrap().command,
            Command::Status
        );
        assert_eq!(
            Cli::parse(vec!["migrate".into(), "--dry-run".into(), "--json".into()])
                .unwrap()
                .command,
            Command::Migrate { dry_run: true }
        );
        assert!(Cli::parse(vec!["validate".into(), "--dry-run".into()]).is_err());
    }

    #[test]
    fn exit_codes_are_stable_and_distinct() {
        assert_eq!(
            [EXIT_OK, EXIT_USAGE, EXIT_INCOMPATIBLE, EXIT_OPERATIONAL],
            [0, 2, 3, 4]
        );
    }
}
