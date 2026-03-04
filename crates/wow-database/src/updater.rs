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

use anyhow::{bail, Result};
use sha1::{Digest, Sha1};
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tracing::{info, warn};

// ─── Public API ─────────────────────────────────────────────────────────────

pub struct DbUpdater {
    /// Already-connected sqlx pool (for metadata queries).
    pool: MySqlPool,
    /// Raw connection params (for mysql CLI when executing large SQL files).
    host: String,
    port: u16,
    user: String,
    pass: String,
    db: String,
}

impl DbUpdater {
    pub fn new(pool: MySqlPool, host: &str, port: u16, user: &str, pass: &str, db: &str) -> Self {
        Self {
            pool,
            host: host.to_string(),
            port,
            user: user.to_string(),
            pass: pass.to_string(),
            db: db.to_string(),
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

        if !Path::new(base_sql).exists() {
            bail!(
                "Base SQL file not found: '{}'\n\
                 Download the TrinityCore WotLK Classic DB release and place it under sql/base/\n\
                 https://github.com/TrinityCore/TrinityCore/releases",
                base_sql
            );
        }

        self.apply_file_cli(base_sql)?;
        info!("Done populating '{}'", self.db);
        Ok(true)
    }

    /// Scan `updates_include` paths, apply new/changed SQL files.
    /// `source_dir` is the project root (where `sql/` lives).
    pub async fn update(&self, source_dir: &str) -> Result<()> {
        info!("Checking '{}' database for pending updates...", self.db);

        self.ensure_updates_table().await?;
        self.ensure_updates_include_table().await?;

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

        // Load already-applied files from DB
        let applied = self.read_applied_files().await?;

        let mut updated = 0u32;

        for (path, state) in &available {
            let name = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let content = match fs::read_to_string(path) {
                Ok(c) => c.replace("\r\n", "\n").replace('\r', "\n"),
                Err(e) => {
                    warn!("Cannot read '{}': {}", path.display(), e);
                    continue;
                }
            };

            let hash = sha1_hex(&content);

            match applied.get(&name) {
                None => {
                    // Check for renamed file (same hash, different name)
                    if let Some(old_name) = applied
                        .iter()
                        .find_map(|(n, h)| if h == &hash { Some(n) } else { None })
                    {
                        info!("Renaming update '{}' → '{}'", old_name, name);
                        sqlx::query(
                            "UPDATE `updates` SET `name` = ? WHERE `name` = ?",
                        )
                        .bind(&name)
                        .bind(old_name)
                        .execute(&self.pool)
                        .await?;
                    } else {
                        info!(
                            "Applying '{}' [{}]...",
                            name,
                            &hash[..7]
                        );
                        let t = Instant::now();
                        self.apply_sql_file(path, &content).await?;
                        let ms = t.elapsed().as_millis() as u32;
                        sqlx::query(
                            "REPLACE INTO `updates` (`name`, `hash`, `state`, `speed`) \
                             VALUES (?, ?, ?, ?)",
                        )
                        .bind(&name)
                        .bind(&hash)
                        .bind(state.as_str())
                        .bind(ms)
                        .execute(&self.pool)
                        .await?;
                        updated += 1;
                    }
                }
                Some(existing_hash) => {
                    if existing_hash != &hash && state != "ARCHIVED" {
                        info!(
                            "Reapplying '{}' (hash changed {} → {})...",
                            name,
                            &existing_hash[..7.min(existing_hash.len())],
                            &hash[..7]
                        );
                        let t = Instant::now();
                        self.apply_sql_file(path, &content).await?;
                        let ms = t.elapsed().as_millis() as u32;
                        sqlx::query(
                            "UPDATE `updates` SET `hash` = ?, `speed` = ? WHERE `name` = ?",
                        )
                        .bind(&hash)
                        .bind(ms)
                        .bind(&name)
                        .execute(&self.pool)
                        .await?;
                        updated += 1;
                    }
                    // else: hash matches → already up-to-date, skip
                }
            }
        }

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
        cmd.arg(format!("-h{}", self.host))
            .arg(format!("-P{}", self.port))
            .arg(format!("-u{}", self.user))
            .arg("--default-character-set=utf8")
            .arg("--max-allowed-packet=1073741824")
            .arg("-e")
            .arg(format!("SOURCE {};", path))
            .arg(&self.db);

        if !self.pass.is_empty() {
            cmd.arg(format!("-p{}", self.pass));
        }

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
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(|e| {
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
                `state`     ENUM('RELEASED','ARCHIVED','CUSTOM') NOT NULL DEFAULT 'RELEASED',
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
                `state` ENUM('RELEASED','ARCHIVED','CUSTOM') NOT NULL DEFAULT 'RELEASED',
                PRIMARY KEY (`path`)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn read_updates_include(&self) -> Result<Vec<(String, String)>> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT `path`, `state` FROM `updates_include`")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn read_applied_files(&self) -> Result<HashMap<String, String>> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT `name`, `hash` FROM `updates`")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().collect())
    }
}

// ─── SQL file helpers ────────────────────────────────────────────────────────

fn sha1_hex(content: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
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
                while i < len && bytes[i] != b'\n' { i += 1; }
            }
            b'#' => {
                while i < len && bytes[i] != b'\n' { i += 1; }
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
            _ => { i += 1; }
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
