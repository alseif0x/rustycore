//! Migration regressions.
//!
//! Separated from migration.rs under #685.

use super::*;
use sqlx::mysql::MySqlPoolOptions;
use std::time::{SystemTime, UNIX_EPOCH};

fn migration(component: &str, version: &str, state: MigrationState) -> Migration {
    Migration {
        component: component.to_string(),
        database: DatabaseKind::Auth,
        version: version.to_string(),
        description: "fixture".to_string(),
        file: format!("{component}-{version}.sql"),
        sha256: format!("hash-{component}-{version}"),
        state,
        adopt_query: None,
    }
}

fn fixture_manifest(migrations: Vec<Migration>) -> MigrationManifest {
    MigrationManifest {
        format: 1,
        baselines: vec![],
        migrations,
        root: PathBuf::new(),
    }
}

mod scenarios;
