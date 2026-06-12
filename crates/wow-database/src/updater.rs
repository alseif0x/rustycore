// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Database auto-updater — hybrid TrinityCore/modern approach.
//!
//! - **File format**: TrinityCore compatible (`sql/base/*.sql`, `sql/updates/**/*.sql`)
//! - **Tracking**: `updates` table with SHA1 hash per file (TrinityCore style)
//! - **Metadata queries**: async via sqlx pool (modern)
//! - **SQL execution**: `mysql` CLI for large base files; statement-by-statement via sqlx for updates
//!
//! # Flow
//! 1. `populate()` — if DB has 0 tables, apply base SQL via mysql CLI
//! 2. `update()`   — scan `updates_include` paths, apply new/changed SQL files

use anyhow::{Result, bail};
use sha1::{Digest, Sha1};
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tracing::{info, warn};

const DELETE_UPDATE_ENTRY_BY_NAME_SQL_LIKE_CPP: &str = "DELETE FROM `updates` WHERE `name` = ?";
const RENAME_UPDATE_ENTRY_SQL_LIKE_CPP: &str = "UPDATE `updates` SET `name` = ? WHERE `name` = ?";

// ─── Public API ─────────────────────────────────────────────────────────────

pub struct DbUpdater {
    /// Already-connected sqlx pool (for metadata queries).
    pool: MySqlPool,
    /// Raw connection params (for mysql CLI when executing large SQL files).
    host: String,
    port_or_socket: String,
    user: String,
    pass: String,
    db: String,
    ssl: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateDecisionLikeCpp {
    Skip,
    Apply,
    Rehash,
    UpdateState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UpdateConfigLikeCpp {
    redundancy_checks: bool,
    allow_rehash: bool,
    archived_redundancy: bool,
    clean_dead_references_max_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppliedUpdateFileLikeCpp {
    hash: String,
    state: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateDatabaseKindLikeCpp {
    Auth,
    Characters,
    World,
    Hotfixes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PopulateBaseActionLikeCpp {
    SkipNoBaseFile,
    ApplyBaseFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RenamedUpdateDecisionLikeCpp {
    ApplyAsNew,
    CopyConflict { old_name: String },
    Rename { old_name: String },
}

impl UpdateConfigLikeCpp {
    fn from_config_like_cpp() -> Self {
        Self {
            redundancy_checks: wow_config::get_value_default("Updates.Redundancy", true),
            allow_rehash: wow_config::get_value_default("Updates.AllowRehash", true),
            archived_redundancy: wow_config::get_value_default("Updates.ArchivedRedundancy", false),
            clean_dead_references_max_count: wow_config::get_value_default(
                "Updates.CleanDeadRefMaxCount",
                3,
            ),
        }
    }
}

fn update_decision_like_cpp(
    existing_hash: &str,
    available_hash: &str,
    applied_state: &str,
    available_state: &str,
    config: &UpdateConfigLikeCpp,
) -> UpdateDecisionLikeCpp {
    if !config.redundancy_checks {
        return UpdateDecisionLikeCpp::Skip;
    }

    if !config.archived_redundancy && applied_state == "ARCHIVED" && available_state == "ARCHIVED" {
        return UpdateDecisionLikeCpp::Skip;
    }

    if config.allow_rehash && existing_hash.is_empty() {
        return UpdateDecisionLikeCpp::Rehash;
    }

    if existing_hash != available_hash {
        return UpdateDecisionLikeCpp::Apply;
    }

    if applied_state != available_state {
        return UpdateDecisionLikeCpp::UpdateState;
    }

    UpdateDecisionLikeCpp::Skip
}

impl DbUpdater {
    pub fn new(
        pool: MySqlPool,
        host: &str,
        port_or_socket: &str,
        user: &str,
        pass: &str,
        db: &str,
        ssl: bool,
    ) -> Self {
        Self {
            pool,
            host: host.to_string(),
            port_or_socket: port_or_socket.to_string(),
            user: user.to_string(),
            pass: pass.to_string(),
            db: db.to_string(),
            ssl,
        }
    }

    /// If the database has no tables, apply the base SQL file.
    /// Returns `true` if it was populated, `false` if already had data.
    pub async fn populate(&self, base_sql: &str) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE()",
        )
        .fetch_one(&self.pool)
        .await?;

        if row.0 > 0 {
            return Ok(false);
        }

        info!(
            "Database '{}' is empty — auto-populating from {}...",
            self.db, base_sql
        );

        if populate_base_action_like_cpp(base_sql) == PopulateBaseActionLikeCpp::SkipNoBaseFile {
            info!(
                "No base SQL file provided for '{}'; skipping populate like TC.",
                self.db
            );
            self.ensure_updates_include_table().await?;
            self.bootstrap_updates_include_if_empty_like_cpp(None)
                .await?;
            return Ok(false);
        }

        if !Path::new(base_sql).exists() {
            bail!(
                "Base SQL file not found: '{}'\n\
                 Download the TrinityCore WotLK Classic DB release and place it under sql/base/\n\
                 https://github.com/TrinityCore/TrinityCore/releases",
                base_sql
            );
        }

        self.apply_file_cli(base_sql)?;
        self.ensure_updates_include_table().await?;
        self.bootstrap_updates_include_if_empty_like_cpp(Some(base_sql))
            .await?;
        info!("Done populating '{}'", self.db);
        Ok(true)
    }

    /// Scan `updates_include` paths, apply new/changed SQL files.
    /// `source_dir` is the project root (where `sql/` lives).
    pub async fn update(&self, source_dir: &str) -> Result<()> {
        info!("Checking '{}' database for pending updates...", self.db);
        let update_config = UpdateConfigLikeCpp::from_config_like_cpp();
        self.ensure_updates_table().await?;
        self.ensure_updates_include_table().await?;
        self.bootstrap_updates_include_if_empty_like_cpp(None)
            .await?;

        let includes = self.read_updates_include().await?;
        if includes.is_empty() {
            info!("'{}' has no updates_include entries — skipping.", self.db);
            return Ok(());
        }

        // Collect all available SQL files in order
        let mut available: Vec<(PathBuf, String)> = vec![];
        for (raw_path, state) in &includes {
            let resolved = raw_path.replacen('$', source_dir, 1);
            let dir = Path::new(&resolved);
            if !dir.exists() {
                warn!("Update directory '{}' not found, skipping.", resolved);
                continue;
            }
            let mut files = collect_sql_files(dir)?;
            files.sort();
            for f in files {
                available.push((f, state.clone()));
            }
        }
        sort_update_files_like_cpp(&mut available)?;
        let available_names: HashSet<String> = available
            .iter()
            .filter_map(|(path, _)| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect();

        // Load already-applied files from DB
        let mut applied = self.read_applied_files().await?;

        let mut updated = 0u32;

        for (path, state) in &available {
            let name = path.file_name().unwrap().to_string_lossy().to_string();

            let content = match fs::read_to_string(path) {
                Ok(c) => c.replace("\r\n", "\n").replace('\r', "\n"),
                Err(e) => {
                    warn!("Cannot read '{}': {}", path.display(), e);
                    continue;
                }
            };

            let hash = sha1_hex(&content);

            match applied.get(&name).cloned() {
                None => match renamed_update_decision_like_cpp(&applied, &available_names, &hash) {
                    RenamedUpdateDecisionLikeCpp::Rename { old_name } => {
                        info!("Renaming update '{}' → '{}'", old_name, name);
                        sqlx::query(DELETE_UPDATE_ENTRY_BY_NAME_SQL_LIKE_CPP)
                            .bind(&name)
                            .execute(&self.pool)
                            .await?;
                        sqlx::query(RENAME_UPDATE_ENTRY_SQL_LIKE_CPP)
                            .bind(&name)
                            .bind(&old_name)
                            .execute(&self.pool)
                            .await?;
                        applied.remove(&old_name);
                    }
                    RenamedUpdateDecisionLikeCpp::CopyConflict { old_name } => {
                        warn!(
                            "It seems like the update '{}' was renamed, but the old file '{}' is still there; treating it as a new file.",
                            name, old_name
                        );
                        info!("Applying '{}' [{}]...", name, &hash[..7]);
                        let t = Instant::now();
                        self.apply_sql_file(path, &content).await?;
                        let ms = t.elapsed().as_millis() as u32;
                        sqlx::query(
                            "REPLACE INTO `updates` (`name`, `hash`, `state`, `speed`) \
                                 VALUES (?, ?, ?, ?)",
                        )
                        .bind(&name)
                        .bind(&hash)
                        .bind(update_state_like_cpp(state))
                        .bind(ms)
                        .execute(&self.pool)
                        .await?;
                        updated += 1;
                    }
                    RenamedUpdateDecisionLikeCpp::ApplyAsNew => {
                        info!("Applying '{}' [{}]...", name, &hash[..7]);
                        let t = Instant::now();
                        self.apply_sql_file(path, &content).await?;
                        let ms = t.elapsed().as_millis() as u32;
                        sqlx::query(
                            "REPLACE INTO `updates` (`name`, `hash`, `state`, `speed`) \
                                 VALUES (?, ?, ?, ?)",
                        )
                        .bind(&name)
                        .bind(&hash)
                        .bind(update_state_like_cpp(state))
                        .bind(ms)
                        .execute(&self.pool)
                        .await?;
                        updated += 1;
                    }
                },
                Some(applied_entry) => {
                    match update_decision_like_cpp(
                        &applied_entry.hash,
                        &hash,
                        &applied_entry.state,
                        state,
                        &update_config,
                    ) {
                        UpdateDecisionLikeCpp::Skip => {}
                        UpdateDecisionLikeCpp::UpdateState => {
                            info!("Updating state for '{}' to '{}'...", name, state);
                            sqlx::query("UPDATE `updates` SET `state` = ? WHERE `name` = ?")
                                .bind(update_state_like_cpp(state))
                                .bind(&name)
                                .execute(&self.pool)
                                .await?;
                        }
                        UpdateDecisionLikeCpp::Rehash => {
                            info!("Rehashing '{}' [{}]...", name, &hash[..7]);
                            sqlx::query(
                                "UPDATE `updates` SET `hash` = ?, `state` = ? WHERE `name` = ?",
                            )
                            .bind(&hash)
                            .bind(update_state_like_cpp(state))
                            .bind(&name)
                            .execute(&self.pool)
                            .await?;
                        }
                        UpdateDecisionLikeCpp::Apply => {
                            info!(
                                "Reapplying '{}' (hash changed {} → {})...",
                                name,
                                &applied_entry.hash[..7.min(applied_entry.hash.len())],
                                &hash[..7]
                            );
                            let t = Instant::now();
                            self.apply_sql_file(path, &content).await?;
                            let ms = t.elapsed().as_millis() as u32;
                            sqlx::query(
                                "UPDATE `updates` SET `hash` = ?, `state` = ?, `speed` = ? WHERE `name` = ?",
                            )
                            .bind(&hash)
                            .bind(update_state_like_cpp(state))
                            .bind(ms)
                            .bind(&name)
                            .execute(&self.pool)
                            .await?;
                            updated += 1;
                        }
                    }
                    applied.remove(&name);
                }
            }
        }

        self.cleanup_orphaned_updates_like_cpp(
            &applied,
            update_config.clean_dead_references_max_count,
        )
        .await?;

        if updated == 0 {
            info!("'{}' database is up-to-date.", self.db);
        } else {
            info!("Applied {} update(s) to '{}'.", updated, self.db);
        }

        Ok(())
    }
}

// ─── Internal helpers ────────────────────────────────────────────────────────

impl DbUpdater {
    /// Execute a SQL file via the `mysql` CLI — used for large base SQL dumps
    /// where splitting statements would be unreliable (triggers, DELIMITER, etc.).
    fn apply_file_cli(&self, path: &str) -> Result<()> {
        let mut cmd = Command::new("mysql");
        cmd.args(mysql_cli_args_like_cpp(
            &self.host,
            &self.port_or_socket,
            &self.user,
            &self.pass,
            &self.db,
            self.ssl,
            path,
        ));

        let out = cmd.output()?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            bail!("mysql CLI error applying '{}': {}", path, err);
        }
        Ok(())
    }

    /// Execute a SQL file via sqlx — statement by statement.
    /// Used for incremental update files which are always clean SQL (no DELIMITER).
    async fn apply_sql_file(&self, path: &Path, content: &str) -> Result<()> {
        for stmt in split_sql(content) {
            if stmt.is_empty() {
                continue;
            }
            sqlx::query(stmt).execute(&self.pool).await.map_err(|e| {
                anyhow::anyhow!(
                    "SQL error in '{}': {}\nStatement: {}",
                    path.display(),
                    e,
                    &stmt[..stmt.len().min(120)]
                )
            })?;
        }
        Ok(())
    }

    async fn ensure_updates_table(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS `updates` (
                `name`      VARCHAR(200) NOT NULL COMMENT 'filename of the update',
                `hash`      VARCHAR(40)  NOT NULL DEFAULT '' COMMENT 'SHA1 of the SQL file',
                `state`     ENUM('RELEASED','ARCHIVED') NOT NULL DEFAULT 'RELEASED',
                `timestamp` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                `speed`     INT UNSIGNED NOT NULL DEFAULT 0 COMMENT 'apply time in ms',
                PRIMARY KEY (`name`)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn ensure_updates_include_table(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS `updates_include` (
                `path`  VARCHAR(200) NOT NULL COMMENT 'path to a directory with update files',
                `state` ENUM('RELEASED','ARCHIVED') NOT NULL DEFAULT 'RELEASED',
                PRIMARY KEY (`path`)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn bootstrap_updates_include_if_empty_like_cpp(
        &self,
        base_sql_hint: Option<&str>,
    ) -> Result<()> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM `updates_include`")
            .fetch_one(&self.pool)
            .await?;

        if row.0 > 0 {
            return Ok(());
        }

        let Some(kind) = update_database_kind_like_cpp(base_sql_hint, &self.db) else {
            warn!(
                "updates_include is empty for '{}', but the database kind is unknown; skipping default include bootstrap.",
                self.db
            );
            return Ok(());
        };

        for (path, state) in default_updates_include_rows_like_cpp(kind) {
            sqlx::query("INSERT IGNORE INTO `updates_include` (`path`, `state`) VALUES (?, ?)")
                .bind(path)
                .bind(update_state_like_cpp(state))
                .execute(&self.pool)
                .await?;
        }

        info!(
            "Bootstrapped '{}' updates_include with {} TrinityCore default path(s).",
            self.db,
            default_updates_include_rows_like_cpp(kind).len()
        );

        Ok(())
    }

    async fn read_updates_include(&self) -> Result<Vec<(String, String)>> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT `path`, `state` FROM `updates_include`")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn read_applied_files(&self) -> Result<HashMap<String, AppliedUpdateFileLikeCpp>> {
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT `name`, `hash`, `state` FROM `updates`")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows
            .into_iter()
            .map(|(name, hash, state)| {
                (
                    name,
                    AppliedUpdateFileLikeCpp {
                        hash,
                        state: update_state_like_cpp(&state).to_string(),
                    },
                )
            })
            .collect())
    }

    async fn cleanup_orphaned_updates_like_cpp(
        &self,
        applied: &HashMap<String, AppliedUpdateFileLikeCpp>,
        clean_dead_references_max_count: i32,
    ) -> Result<()> {
        if applied.is_empty() {
            return Ok(());
        }

        let do_cleanup = should_cleanup_orphaned_updates_like_cpp(
            applied.len(),
            clean_dead_references_max_count,
        );

        for name in applied.keys() {
            warn!(
                "The file '{}' was applied to the database, but is missing in your update directory now!",
                name
            );

            if do_cleanup {
                info!("Deleting orphaned entry '{}'...", name);
            }
        }

        if do_cleanup {
            for name in applied.keys() {
                sqlx::query("DELETE FROM `updates` WHERE `name` = ?")
                    .bind(name)
                    .execute(&self.pool)
                    .await?;
            }
        } else {
            tracing::error!(
                "Cleanup is disabled! There were {} dirty files applied to your database, but they are now missing in your source directory!",
                applied.len()
            );
        }

        Ok(())
    }
}

// ─── SQL file helpers ────────────────────────────────────────────────────────

fn sha1_hex(content: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn mysql_cli_args_like_cpp(
    host: &str,
    port_or_socket: &str,
    user: &str,
    password: &str,
    database: &str,
    ssl: bool,
    path: &str,
) -> Vec<String> {
    let mut args = Vec::with_capacity(12);
    args.push(format!("-h{host}"));
    args.push(format!("-u{user}"));

    if !password.is_empty() {
        args.push(format!("-p{password}"));
    }

    #[cfg(windows)]
    {
        if host == "." {
            args.push("--protocol=PIPE".to_string());
        } else {
            args.push(format!("-P{port_or_socket}"));
        }
    }

    #[cfg(not(windows))]
    {
        if !port_or_socket
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
        {
            args.push("-P0".to_string());
            args.push("--protocol=SOCKET".to_string());
            args.push(format!("-S{port_or_socket}"));
        } else {
            args.push(format!("-P{port_or_socket}"));
        }
    }

    args.push("--default-character-set=utf8".to_string());
    args.push("--max-allowed-packet=1GB".to_string());

    if ssl {
        args.push(mysql_cli_ssl_arg_like_cpp().to_string());
    }

    args.push("-e".to_string());
    args.push(format!("BEGIN; SOURCE {path}; COMMIT;"));

    if !database.is_empty() {
        args.push(database.to_string());
    }

    args
}

fn mysql_cli_ssl_arg_like_cpp() -> &'static str {
    "--ssl"
}

/// Split a SQL file into individual statements by `;`.
/// Handles:
///  - Line comments (`-- ...`, `# ...`)
///  - Block comments (`/* ... */`)
///  - String literals (`'...'`, `"..."`) with escape sequences
fn split_sql(content: &str) -> Vec<&str> {
    let mut statements = vec![];
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut start = 0;
    let mut i = 0;

    while i < len {
        match bytes[i] {
            // Line comment: -- or #
            b'-' if i + 1 < len && bytes[i + 1] == b'-' => {
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'#' => {
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            // Block comment /* ... */
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            // String literal
            b'\'' | b'"' => {
                let quote = bytes[i];
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
            }
            // Statement terminator
            b';' => {
                let stmt = content[start..i].trim();
                if !stmt.is_empty() {
                    statements.push(stmt);
                }
                i += 1;
                start = i;
            }
            _ => {
                i += 1;
            }
        }
    }

    // Trailing statement without semicolon
    let tail = content[start..].trim();
    if !tail.is_empty() {
        statements.push(tail);
    }

    statements
}

/// Recursively collect all `.sql` files under a directory.
fn collect_sql_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(collect_sql_files(&path)?);
        } else if path.extension().map_or(false, |e| e == "sql") {
            files.push(path);
        }
    }
    Ok(files)
}

fn sort_update_files_like_cpp(files: &mut [(PathBuf, String)]) -> Result<()> {
    files.sort_by(|left, right| {
        update_filename_like_cpp(&left.0).cmp(&update_filename_like_cpp(&right.0))
    });

    for window in files.windows(2) {
        let left = update_filename_like_cpp(&window[0].0);
        let right = update_filename_like_cpp(&window[1].0);
        if left == right {
            bail!(
                "Duplicate filename \"{}\" occurred. Because updates are ordered by their filenames, every name needs to be unique!",
                left
            );
        }
    }

    Ok(())
}

fn update_filename_like_cpp(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn update_state_like_cpp(state: &str) -> &'static str {
    if state == "RELEASED" {
        "RELEASED"
    } else {
        "ARCHIVED"
    }
}

fn renamed_update_decision_like_cpp(
    applied: &HashMap<String, AppliedUpdateFileLikeCpp>,
    available_names: &HashSet<String>,
    hash: &str,
) -> RenamedUpdateDecisionLikeCpp {
    let Some(old_name) = applied.iter().find_map(|(name, applied)| {
        if applied.hash == hash {
            Some(name.clone())
        } else {
            None
        }
    }) else {
        return RenamedUpdateDecisionLikeCpp::ApplyAsNew;
    };

    if available_names.contains(&old_name) {
        RenamedUpdateDecisionLikeCpp::CopyConflict { old_name }
    } else {
        RenamedUpdateDecisionLikeCpp::Rename { old_name }
    }
}

fn should_cleanup_orphaned_updates_like_cpp(
    orphan_count: usize,
    clean_dead_references_max_count: i32,
) -> bool {
    orphan_count == 0
        || clean_dead_references_max_count < 0
        || orphan_count <= clean_dead_references_max_count as usize
}

fn update_database_kind_like_cpp(
    base_sql_hint: Option<&str>,
    database_name: &str,
) -> Option<UpdateDatabaseKindLikeCpp> {
    let hinted = base_sql_hint
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .map(str::to_ascii_lowercase);

    match hinted.as_deref() {
        Some("auth_database.sql") => return Some(UpdateDatabaseKindLikeCpp::Auth),
        Some("characters_database.sql") => return Some(UpdateDatabaseKindLikeCpp::Characters),
        Some("world_database.sql") => return Some(UpdateDatabaseKindLikeCpp::World),
        Some("hotfixes_database.sql") => return Some(UpdateDatabaseKindLikeCpp::Hotfixes),
        _ => {}
    }

    match database_name.to_ascii_lowercase().as_str() {
        "auth" | "login" => Some(UpdateDatabaseKindLikeCpp::Auth),
        "characters" | "character" => Some(UpdateDatabaseKindLikeCpp::Characters),
        "world" => Some(UpdateDatabaseKindLikeCpp::World),
        "hotfixes" | "hotfix" => Some(UpdateDatabaseKindLikeCpp::Hotfixes),
        _ => None,
    }
}

fn populate_base_action_like_cpp(base_sql: &str) -> PopulateBaseActionLikeCpp {
    if base_sql.trim().is_empty() {
        PopulateBaseActionLikeCpp::SkipNoBaseFile
    } else {
        PopulateBaseActionLikeCpp::ApplyBaseFile
    }
}

fn default_updates_include_rows_like_cpp(
    kind: UpdateDatabaseKindLikeCpp,
) -> &'static [(&'static str, &'static str)] {
    match kind {
        UpdateDatabaseKindLikeCpp::Auth => &[
            ("$/sql/custom/auth", "RELEASED"),
            ("$/sql/old/10.x/auth", "ARCHIVED"),
            ("$/sql/old/3.4.x/auth", "ARCHIVED"),
            ("$/sql/old/6.x/auth", "ARCHIVED"),
            ("$/sql/old/7/auth", "ARCHIVED"),
            ("$/sql/old/8.x/auth", "ARCHIVED"),
            ("$/sql/old/9.x/auth", "ARCHIVED"),
            ("$/sql/updates/auth", "RELEASED"),
        ],
        UpdateDatabaseKindLikeCpp::Characters => &[
            ("$/sql/custom/characters", "RELEASED"),
            ("$/sql/old/10.x/characters", "ARCHIVED"),
            ("$/sql/old/3.4.x/characters", "ARCHIVED"),
            ("$/sql/old/6.x/characters", "ARCHIVED"),
            ("$/sql/old/7/characters", "ARCHIVED"),
            ("$/sql/old/8.x/characters", "ARCHIVED"),
            ("$/sql/old/9.x/characters", "ARCHIVED"),
            ("$/sql/updates/characters", "RELEASED"),
        ],
        UpdateDatabaseKindLikeCpp::World => &[
            ("$/sql/custom/world", "RELEASED"),
            ("$/sql/old/10.x/world", "ARCHIVED"),
            ("$/sql/old/3.4.x/world", "ARCHIVED"),
            ("$/sql/old/6.x/world", "ARCHIVED"),
            ("$/sql/old/7/world", "ARCHIVED"),
            ("$/sql/old/8.x/world", "ARCHIVED"),
            ("$/sql/old/9.x/world", "ARCHIVED"),
            ("$/sql/updates/world", "RELEASED"),
        ],
        UpdateDatabaseKindLikeCpp::Hotfixes => &[
            ("$/sql/custom/hotfixes", "RELEASED"),
            ("$/sql/old/10.x/hotfixes", "ARCHIVED"),
            ("$/sql/old/3.4.x/hotfixes", "ARCHIVED"),
            ("$/sql/old/6.x/hotfixes", "ARCHIVED"),
            ("$/sql/old/7/hotfixes", "ARCHIVED"),
            ("$/sql/old/8.x/hotfixes", "ARCHIVED"),
            ("$/sql/old/9.x/hotfixes", "ARCHIVED"),
            ("$/sql/updates/hotfixes", "RELEASED"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppliedUpdateFileLikeCpp, DELETE_UPDATE_ENTRY_BY_NAME_SQL_LIKE_CPP,
        PopulateBaseActionLikeCpp, RENAME_UPDATE_ENTRY_SQL_LIKE_CPP, RenamedUpdateDecisionLikeCpp,
        UpdateConfigLikeCpp, UpdateDatabaseKindLikeCpp, UpdateDecisionLikeCpp,
        default_updates_include_rows_like_cpp, mysql_cli_args_like_cpp,
        populate_base_action_like_cpp, renamed_update_decision_like_cpp,
        should_cleanup_orphaned_updates_like_cpp, sort_update_files_like_cpp, split_sql,
        update_database_kind_like_cpp, update_decision_like_cpp, update_state_like_cpp,
    };
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;

    fn update_config_like_cpp(
        redundancy_checks: bool,
        allow_rehash: bool,
        archived_redundancy: bool,
    ) -> UpdateConfigLikeCpp {
        UpdateConfigLikeCpp {
            redundancy_checks,
            allow_rehash,
            archived_redundancy,
            clean_dead_references_max_count: 3,
        }
    }

    #[test]
    fn update_decision_honors_cpp_redundancy_and_archived_gates() {
        let redundancy_off = update_config_like_cpp(false, true, true);
        assert_eq!(
            update_decision_like_cpp(
                "oldhash",
                "newhash",
                "RELEASED",
                "RELEASED",
                &redundancy_off,
            ),
            UpdateDecisionLikeCpp::Skip
        );

        let archived_gate_off = update_config_like_cpp(true, true, false);
        assert_eq!(
            update_decision_like_cpp(
                "oldhash",
                "newhash",
                "ARCHIVED",
                "ARCHIVED",
                &archived_gate_off,
            ),
            UpdateDecisionLikeCpp::Skip
        );

        let archived_gate_on = update_config_like_cpp(true, true, true);
        assert_eq!(
            update_decision_like_cpp(
                "oldhash",
                "newhash",
                "ARCHIVED",
                "ARCHIVED",
                &archived_gate_on,
            ),
            UpdateDecisionLikeCpp::Apply
        );
    }

    #[test]
    fn update_decision_honors_cpp_rehash_gate_and_changed_hashes() {
        let rehash_on = update_config_like_cpp(true, true, false);
        assert_eq!(
            update_decision_like_cpp("", "newhash", "RELEASED", "RELEASED", &rehash_on),
            UpdateDecisionLikeCpp::Rehash
        );

        let rehash_off = update_config_like_cpp(true, false, false);
        assert_eq!(
            update_decision_like_cpp("", "newhash", "RELEASED", "RELEASED", &rehash_off),
            UpdateDecisionLikeCpp::Apply
        );

        assert_eq!(
            update_decision_like_cpp("same", "same", "RELEASED", "RELEASED", &rehash_on),
            UpdateDecisionLikeCpp::Skip
        );

        assert_eq!(
            update_decision_like_cpp("same", "same", "RELEASED", "ARCHIVED", &rehash_on),
            UpdateDecisionLikeCpp::UpdateState
        );
    }

    #[test]
    fn update_state_conversion_matches_cpp_two_state_model() {
        assert_eq!(update_state_like_cpp("RELEASED"), "RELEASED");
        assert_eq!(update_state_like_cpp("ARCHIVED"), "ARCHIVED");
        assert_eq!(update_state_like_cpp("CUSTOM"), "ARCHIVED");
        assert_eq!(update_state_like_cpp(""), "ARCHIVED");
        assert_eq!(update_state_like_cpp("released"), "ARCHIVED");
    }

    #[test]
    fn orphaned_update_cleanup_threshold_matches_cpp() {
        assert!(should_cleanup_orphaned_updates_like_cpp(0, 3));
        assert!(should_cleanup_orphaned_updates_like_cpp(3, 3));
        assert!(!should_cleanup_orphaned_updates_like_cpp(4, 3));
        assert!(should_cleanup_orphaned_updates_like_cpp(10, -1));
    }

    #[test]
    fn update_database_kind_uses_base_sql_hint_before_database_name() {
        assert_eq!(
            update_database_kind_like_cpp(Some("/repo/sql/base/auth_database.sql"), "custom"),
            Some(UpdateDatabaseKindLikeCpp::Auth)
        );
        assert_eq!(
            update_database_kind_like_cpp(
                Some("/repo/sql/base/dev/hotfixes_database.sql"),
                "custom"
            ),
            Some(UpdateDatabaseKindLikeCpp::Hotfixes)
        );
        assert_eq!(
            update_database_kind_like_cpp(None, "characters"),
            Some(UpdateDatabaseKindLikeCpp::Characters)
        );
        assert_eq!(update_database_kind_like_cpp(None, "unknown"), None);
    }

    #[test]
    fn default_updates_include_rows_match_wotlk_classic_layout_like_cpp() {
        let auth = default_updates_include_rows_like_cpp(UpdateDatabaseKindLikeCpp::Auth);
        assert_eq!(auth.len(), 8);
        assert_eq!(auth[0], ("$/sql/custom/auth", "RELEASED"));
        assert_eq!(auth[2], ("$/sql/old/3.4.x/auth", "ARCHIVED"));
        assert_eq!(auth[7], ("$/sql/updates/auth", "RELEASED"));

        let characters =
            default_updates_include_rows_like_cpp(UpdateDatabaseKindLikeCpp::Characters);
        assert_eq!(characters[0], ("$/sql/custom/characters", "RELEASED"));
        assert_eq!(characters[7], ("$/sql/updates/characters", "RELEASED"));

        let world = default_updates_include_rows_like_cpp(UpdateDatabaseKindLikeCpp::World);
        assert_eq!(world[0], ("$/sql/custom/world", "RELEASED"));
        assert_eq!(world[7], ("$/sql/updates/world", "RELEASED"));

        let hotfixes = default_updates_include_rows_like_cpp(UpdateDatabaseKindLikeCpp::Hotfixes);
        assert_eq!(hotfixes[0], ("$/sql/custom/hotfixes", "RELEASED"));
        assert_eq!(hotfixes[7], ("$/sql/updates/hotfixes", "RELEASED"));
    }

    #[test]
    fn empty_base_file_skips_populate_like_cpp() {
        assert_eq!(
            populate_base_action_like_cpp(""),
            PopulateBaseActionLikeCpp::SkipNoBaseFile
        );
        assert_eq!(
            populate_base_action_like_cpp("   "),
            PopulateBaseActionLikeCpp::SkipNoBaseFile
        );
        assert_eq!(
            populate_base_action_like_cpp("/repo/sql/base/auth_database.sql"),
            PopulateBaseActionLikeCpp::ApplyBaseFile
        );
    }

    #[test]
    fn update_files_sort_by_filename_and_reject_duplicates_like_cpp() {
        let mut files = vec![
            (
                PathBuf::from("/repo/sql/updates/world/2024_02_00_world.sql"),
                "RELEASED".to_string(),
            ),
            (
                PathBuf::from("/repo/sql/custom/world/2024_01_00_world.sql"),
                "RELEASED".to_string(),
            ),
            (
                PathBuf::from("/repo/sql/old/world/2023_12_00_world.sql"),
                "ARCHIVED".to_string(),
            ),
        ];

        sort_update_files_like_cpp(&mut files).unwrap();
        let names: Vec<_> = files
            .iter()
            .map(|(path, _)| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            vec![
                "2023_12_00_world.sql",
                "2024_01_00_world.sql",
                "2024_02_00_world.sql"
            ]
        );

        let mut duplicate = vec![
            (
                PathBuf::from("/repo/sql/updates/world/2024_01_00_world.sql"),
                "RELEASED".to_string(),
            ),
            (
                PathBuf::from("/repo/sql/custom/world/2024_01_00_world.sql"),
                "RELEASED".to_string(),
            ),
        ];
        assert!(sort_update_files_like_cpp(&mut duplicate).is_err());
    }

    #[test]
    fn renamed_update_decision_requires_old_file_to_be_absent_like_cpp() {
        let mut applied = HashMap::new();
        applied.insert(
            "2024_01_01_old.sql".to_string(),
            AppliedUpdateFileLikeCpp {
                hash: "samehash".to_string(),
                state: "RELEASED".to_string(),
            },
        );

        let available_without_old = HashSet::from(["2024_01_02_new.sql".to_string()]);
        assert_eq!(
            renamed_update_decision_like_cpp(&applied, &available_without_old, "samehash"),
            RenamedUpdateDecisionLikeCpp::Rename {
                old_name: "2024_01_01_old.sql".to_string(),
            }
        );

        let available_with_old = HashSet::from([
            "2024_01_01_old.sql".to_string(),
            "2024_01_02_new.sql".to_string(),
        ]);
        assert_eq!(
            renamed_update_decision_like_cpp(&applied, &available_with_old, "samehash"),
            RenamedUpdateDecisionLikeCpp::CopyConflict {
                old_name: "2024_01_01_old.sql".to_string(),
            }
        );

        assert_eq!(
            renamed_update_decision_like_cpp(&applied, &available_without_old, "different"),
            RenamedUpdateDecisionLikeCpp::ApplyAsNew
        );
    }

    #[test]
    fn renamed_update_sql_deletes_target_before_renaming_like_cpp() {
        assert_eq!(
            DELETE_UPDATE_ENTRY_BY_NAME_SQL_LIKE_CPP,
            "DELETE FROM `updates` WHERE `name` = ?"
        );
        assert_eq!(
            RENAME_UPDATE_ENTRY_SQL_LIKE_CPP,
            "UPDATE `updates` SET `name` = ? WHERE `name` = ?"
        );
    }

    #[test]
    fn mysql_cli_args_honor_socket_and_ssl_like_cpp() {
        let tcp_args = mysql_cli_args_like_cpp(
            "127.0.0.1",
            "3306",
            "trinity",
            "trinity",
            "auth",
            true,
            "/repo/sql/base/auth_database.sql",
        );
        assert_eq!(
            tcp_args,
            vec![
                "-h127.0.0.1",
                "-utrinity",
                "-ptrinity",
                "-P3306",
                "--default-character-set=utf8",
                "--max-allowed-packet=1GB",
                "--ssl",
                "-e",
                "BEGIN; SOURCE /repo/sql/base/auth_database.sql; COMMIT;",
                "auth",
            ]
        );

        let socket_args = mysql_cli_args_like_cpp(
            "localhost",
            "/var/run/mysqld/mysqld.sock",
            "trinity",
            "",
            "",
            false,
            "/repo/sql/base/auth_database.sql",
        );
        assert_eq!(
            socket_args,
            vec![
                "-hlocalhost",
                "-utrinity",
                "-P0",
                "--protocol=SOCKET",
                "-S/var/run/mysqld/mysqld.sock",
                "--default-character-set=utf8",
                "--max-allowed-packet=1GB",
                "-e",
                "BEGIN; SOURCE /repo/sql/base/auth_database.sql; COMMIT;",
            ]
        );
    }

    #[test]
    fn split_sql_handles_comments_quotes_and_escapes() {
        let sql = r#"
            -- a semicolon in a line comment; is ignored
            CREATE TABLE `a` (`text` varchar(64));
            # another comment; also ignored
            INSERT INTO `a` VALUES ('semi;colon', "double;quoted", 'escaped \'; quote');
            /* block; comment */
            UPDATE `a` SET `text` = 'tail';
        "#;

        let statements = split_sql(sql);
        assert_eq!(statements.len(), 3);
        assert!(statements[0].contains("CREATE TABLE `a`"));
        assert!(statements[1].contains("'semi;colon'"));
        assert!(statements[1].contains("\"double;quoted\""));
        assert!(statements[1].contains("'escaped \\'; quote'"));
        assert!(statements[2].contains("UPDATE `a` SET"));
    }
}
