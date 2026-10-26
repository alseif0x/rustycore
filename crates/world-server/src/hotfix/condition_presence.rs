//! Startup composition for effective condition-row presence.

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use wow_persistence::{ConditionPresenceLoadOutcome, ConditionPresencePersistencePort};

pub(crate) async fn load_condition_presence_catalog(
    data_dir: &str,
    locale: &str,
    persistence: &dyn ConditionPresencePersistencePort,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> Result<Arc<wow_data::ConditionPresenceCatalog>> {
    // Load and validate both WDC4 tables before asking persistence for SQL rows.
    let files = wow_data::ConditionPresenceCatalogFiles::load(data_dir, locale)
        .context("failed to load complete condition DB2 files")?;
    let rows = match persistence.load_condition_presence_rows().await {
        ConditionPresenceLoadOutcome::Loaded(rows) => rows,
        ConditionPresenceLoadOutcome::Failed { reason } => bail!(reason),
    };
    let catalog = files
        .apply_hotfix_ids(rows.player_condition_ids, rows.unit_condition_ids, removals)
        .context("failed to compose effective condition presence")?;
    Ok(Arc::new(catalog))
}

#[cfg(test)]
#[path = "condition_presence/tests.rs"]
mod tests;
