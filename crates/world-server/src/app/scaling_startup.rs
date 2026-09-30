//! Player level scaling and shared curve authority.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct ScalingCatalogs {
    pub(super) scaling_stat_values_store:
        Arc<wow_data::progression_rewards::ScalingStatValuesStore>,
    pub(super) scaling_stat_distribution_store:
        Arc<wow_data::progression_rewards::ScalingStatDistributionStore>,
    pub(super) curve_point_store: Arc<wow_data::progression_rewards::CurvePointStore>,
    pub(super) curve_store: Arc<wow_data::progression_rewards::CurveStore>,
}

pub(super) fn load(data_dir: &str, locale: &str) -> anyhow::Result<ScalingCatalogs> {
    let curve_store = Arc::new(
        wow_data::progression_rewards::CurveStore::load(data_dir, locale)
            .context("Failed to load Curve.db2 for C++ curve validation")?,
    );
    let curve_point_store = Arc::new(
        wow_data::progression_rewards::CurvePointStore::load(data_dir, locale)
            .context("Failed to load CurvePoint.db2 for C++ curve evaluation")?,
    );
    let scaling_stat_distribution_store = Arc::new(
        wow_data::progression_rewards::ScalingStatDistributionStore::load(data_dir, locale)
            .context(
                "Failed to load ScalingStatDistribution.db2 — check DataDir and DBC.Locale config",
            )?,
    );
    let scaling_stat_values_store = Arc::new(
        wow_data::progression_rewards::ScalingStatValuesStore::load(data_dir, locale).context(
            "Failed to load ScalingStatValues.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} curves, {} curve points, {} scaling-stat distributions, and {} scaling-stat values from DB2",
        curve_store.len(),
        curve_point_store.len(),
        scaling_stat_distribution_store.len(),
        scaling_stat_values_store.len()
    );
    Ok(ScalingCatalogs {
        curve_store,
        curve_point_store,
        scaling_stat_distribution_store,
        scaling_stat_values_store,
    })
}
