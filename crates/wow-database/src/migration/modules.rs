//! Per-module SQL history from `modules.lock.toml` (#1444 H6).
//!
//! `tools/modules/compose.py` records, for every composed module that ships
//! `data/sql/manifest.toml`, its migrations with their checksums. A removed
//! module keeps that record as a tombstone, written by
//! `rustycore-db module remove`. The composed `world-modules` binary embeds the
//! lock, and [`MigrationManifest::with_module_schema`] turns it into manifest
//! entries so the startup schema check accepts module history:
//!
//! - a composed module's migrations are `active`: one not applied yet is a
//!   pending migration, and the server refuses to start until
//!   `rustycore-db module apply <id>` ran;
//! - a tombstoned module's migrations are `archived`: their history is
//!   accepted, with its checksums, and nothing is required.
//!
//! History rows use the existing `component` column as the namespace:
//! `module:<id>`. Modules may only change the `characters` and `world`
//! databases.

use anyhow::{Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;

use super::{DatabaseKind, Migration, MigrationManifest, MigrationState};

/// The history `component` prefix of module migrations.
pub const MODULE_COMPONENT_PREFIX: &str = "module:";

/// The databases a module may change.
pub const MODULE_DATABASES: [DatabaseKind; 2] = [DatabaseKind::Characters, DatabaseKind::World];

/// The history component of one module's migrations.
#[must_use]
pub fn module_component(module_id: &str) -> String {
    format!("{MODULE_COMPONENT_PREFIX}{module_id}")
}

/// Whether a history component belongs to a module.
#[must_use]
pub fn is_module_component(component: &str) -> bool {
    component.starts_with(MODULE_COMPONENT_PREFIX)
}

/// A dot-separated numeric version, compared numerically.
pub fn parse_schema_version(text: &str) -> Option<Vec<u64>> {
    let parts: Option<Vec<u64>> = text
        .split('.')
        .map(|part| {
            (!part.is_empty() && part.len() <= 9 && part.bytes().all(|b| b.is_ascii_digit()))
                .then(|| part.parse().ok())
                .flatten()
        })
        .collect();
    parts.filter(|parts| !parts.is_empty() && parts.len() <= 8)
}

#[derive(Debug, Clone, Default, Deserialize)]
struct LockFile {
    #[serde(default)]
    module: Vec<LockedModule>,
    #[serde(default)]
    tombstone: Vec<LockedModule>,
}

/// One `[[module]]` or `[[tombstone]]` entry of the lock.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LockedModule {
    pub id: String,
    #[serde(default)]
    pub enabled_order: u32,
    #[serde(default)]
    pub sql: Option<ModuleSql>,
}

/// A module's recorded SQL: what it needs, where it lives and its checksums.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModuleSql {
    pub min_core_schema: String,
    /// `data/sql` of the checkout (or the retained copy of a tombstone),
    /// relative to the repository root.
    pub source: String,
    #[serde(default)]
    pub migration: Vec<ModuleMigration>,
    #[serde(default)]
    pub purge: Vec<ModulePurge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModuleMigration {
    pub database: DatabaseKind,
    pub version: String,
    pub description: String,
    /// Relative to [`ModuleSql::source`].
    pub file: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModulePurge {
    pub database: DatabaseKind,
    pub file: String,
    pub sha256: String,
}

/// The module SQL history one server build accepts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleSchema {
    /// Composed modules with SQL, in `enabled_order`.
    pub modules: Vec<LockedModule>,
    /// Removed modules whose history and data were kept.
    pub tombstones: Vec<LockedModule>,
}

impl ModuleSchema {
    /// No module SQL: the plain `world-server` and a zero-module composition.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Parse `modules.lock.toml`. An empty text is an empty schema.
    pub fn from_lock(text: &str) -> Result<Self> {
        if text.trim().is_empty() {
            return Ok(Self::empty());
        }
        let lock: LockFile = toml::from_str(text)
            .map_err(|error| anyhow::anyhow!("invalid modules.lock.toml: {error}"))?;
        let mut modules: Vec<_> = lock
            .module
            .into_iter()
            .filter(|module| module.sql.is_some())
            .collect();
        modules.sort_by_key(|module| (module.enabled_order, module.id.clone()));
        let schema = Self {
            modules,
            tombstones: lock.tombstone,
        };
        schema.validate()?;
        Ok(schema)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty() && self.tombstones.is_empty()
    }

    /// The composed module `id`, if it has SQL.
    #[must_use]
    pub fn active(&self, id: &str) -> Option<&LockedModule> {
        self.modules.iter().find(|module| module.id == id)
    }

    /// The tombstone of `id`, if the module was removed with its data kept.
    #[must_use]
    pub fn tombstone(&self, id: &str) -> Option<&LockedModule> {
        self.tombstones.iter().find(|module| module.id == id)
    }

    fn validate(&self) -> Result<()> {
        let mut ids = BTreeSet::new();
        for module in &self.modules {
            if !ids.insert(module.id.as_str()) {
                bail!("modules.lock.toml composes module {} twice", module.id);
            }
        }
        let mut tombstoned = BTreeSet::new();
        for tombstone in &self.tombstones {
            if !tombstoned.insert(tombstone.id.as_str()) {
                bail!("modules.lock.toml has two tombstones for {}", tombstone.id);
            }
            if tombstone.sql.is_none() {
                bail!("tombstone {} records no SQL", tombstone.id);
            }
        }
        for module in self.modules.iter().chain(&self.tombstones) {
            validate_module(module)?;
        }
        // A re-installed module continues its tombstoned history.
        for tombstone in &self.tombstones {
            let Some(active) = self.active(&tombstone.id) else {
                continue;
            };
            let active_sql = active.sql.as_ref().expect("filtered on sql");
            for migration in &tombstone.sql.as_ref().expect("validated").migration {
                if !active_sql.migration.contains(migration) {
                    bail!(
                        "module {} is composed again but does not continue its tombstoned migration {} {}",
                        tombstone.id,
                        migration.database.as_str(),
                        migration.version
                    );
                }
            }
        }
        Ok(())
    }
}

fn validate_module(module: &LockedModule) -> Result<()> {
    let id = &module.id;
    let valid_id = id.len() <= 64
        && id.starts_with(|c: char| c.is_ascii_lowercase())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.');
    if !valid_id {
        bail!("invalid module id {id:?} in modules.lock.toml");
    }
    let Some(sql) = &module.sql else {
        return Ok(());
    };
    if parse_schema_version(&sql.min_core_schema).is_none() {
        bail!(
            "module {id}: invalid min_core_schema {:?}",
            sql.min_core_schema
        );
    }
    let mut seen = BTreeSet::new();
    for migration in &sql.migration {
        if !MODULE_DATABASES.contains(&migration.database) {
            bail!(
                "module {id}: migration {} targets {}; a module may change only characters and world",
                migration.version,
                migration.database.as_str()
            );
        }
        if parse_schema_version(&migration.version).is_none() {
            bail!(
                "module {id}: invalid migration version {:?}",
                migration.version
            );
        }
        if !seen.insert((migration.database, migration.version.as_str())) {
            bail!(
                "module {id}: duplicate migration {} {}",
                migration.database.as_str(),
                migration.version
            );
        }
        check_sha(id, &migration.file, &migration.sha256)?;
    }
    for purge in &sql.purge {
        if !MODULE_DATABASES.contains(&purge.database) {
            bail!(
                "module {id}: purge script targets {}",
                purge.database.as_str()
            );
        }
        check_sha(id, &purge.file, &purge.sha256)?;
    }
    Ok(())
}

fn check_sha(id: &str, file: &str, sha256: &str) -> Result<()> {
    if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("module {id}: {file} has an invalid SHA-256");
    }
    Ok(())
}

impl ModuleMigration {
    fn manifest_entry(&self, module_id: &str, state: MigrationState) -> Migration {
        Migration {
            component: module_component(module_id),
            database: self.database,
            version: self.version.clone(),
            description: self.description.clone(),
            file: self.file.clone(),
            sha256: self.sha256.clone(),
            state,
            adopt_query: None,
        }
    }
}

impl MigrationManifest {
    /// This manifest plus the module history `schema` accepts: composed
    /// modules' migrations are `active`, tombstoned ones `archived`.
    pub fn with_module_schema(&self, schema: &ModuleSchema) -> Result<Self> {
        let mut manifest = self.clone();
        for module in &schema.modules {
            let sql = module
                .sql
                .as_ref()
                .expect("from_lock keeps modules with SQL");
            manifest.migrations.extend(
                sql.migration
                    .iter()
                    .map(|migration| migration.manifest_entry(&module.id, MigrationState::Active)),
            );
        }
        for tombstone in &schema.tombstones {
            // A composed module of the same id already lists this history.
            if schema.active(&tombstone.id).is_some() {
                continue;
            }
            let sql = tombstone.sql.as_ref().expect("validated");
            manifest
                .migrations
                .extend(sql.migration.iter().map(|migration| {
                    migration.manifest_entry(&tombstone.id, MigrationState::Archived)
                }));
        }
        manifest.validate_shape()?;
        Ok(manifest)
    }
}
