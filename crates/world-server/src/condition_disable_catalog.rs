//! Composition boundary for ConditionMgr and DisableMgr startup rows.

use anyhow::{Result, bail};
use tracing::{info, warn};
use wow_persistence::{
    ConditionDisableCatalogPersistencePortLikeCpp, ConditionDisableRowsLoadOutcomeLikeCpp,
    ConditionPersistenceRowLikeCpp, DisablePersistenceRowLikeCpp,
};

fn loaded<T>(outcome: ConditionDisableRowsLoadOutcomeLikeCpp<T>) -> Result<T> {
    match outcome {
        ConditionDisableRowsLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        ConditionDisableRowsLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_conditions_like_cpp(
    persistence: &dyn ConditionDisableCatalogPersistencePortLikeCpp,
    script_id_for_name: impl FnMut(&str) -> u32,
) -> Result<wow_data::ConditionLoadReport> {
    let rows = loaded(persistence.load_condition_rows_like_cpp().await)?;
    let report = wow_data::conditions::parse_condition_rows_like_cpp(
        rows.into_iter().map(|row: ConditionPersistenceRowLikeCpp| {
            wow_data::conditions::ConditionDbRowLikeCpp {
                source_type_or_reference_id: row.source_type_or_reference_id,
                source_group: row.source_group,
                source_entry: row.source_entry,
                source_id: row.source_id,
                else_group: row.else_group,
                condition_type_or_reference: row.condition_type_or_reference,
                condition_target: row.condition_target,
                condition_value1: row.condition_value1,
                condition_value2: row.condition_value2,
                condition_value3: row.condition_value3,
                condition_string_value1: row.condition_string_value1,
                negative_condition: row.negative_condition,
                error_type: row.error_type,
                error_text_id: row.error_text_id,
                script_name: row.script_name,
            }
        }),
        script_id_for_name,
    );
    info!(
        "Parsed {} conditions rows ({} skipped before validation, {} load warnings)",
        report.parsed_count(),
        report.skipped.len(),
        report.warnings.len()
    );
    Ok(report)
}

pub(super) async fn load_disable_mgr_like_cpp(
    persistence: &dyn ConditionDisableCatalogPersistencePortLikeCpp,
    refs: wow_data::DisableMgrRefsLikeCpp<'_>,
) -> Result<wow_data::DisableMgrLikeCpp> {
    let rows = loaded(persistence.load_disable_rows_like_cpp().await)?;
    let (mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
        rows.into_iter().map(
            |row: DisablePersistenceRowLikeCpp| wow_data::DisableDbRowLikeCpp {
                source_type: row.source_type,
                entry: row.entry,
                flags: row.flags,
                params_0: row.params_0,
                params_1: row.params_1,
            },
        ),
        refs,
    );
    if report.loaded_count == 0 {
        info!("Loaded 0 disables. DB table `disables` is empty");
    } else {
        info!("Loaded {} disables", report.loaded_count);
    }
    for skipped in &report.skipped_rows {
        warn!(
            "Skipped disable type {} entry {}: {}",
            skipped.row.source_type, skipped.row.entry, skipped.reason
        );
    }
    for warning in &report.warnings {
        warn!("{warning}");
    }
    Ok(mgr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_is_distinct_from_an_empty_table() {
        let error = loaded::<Vec<ConditionPersistenceRowLikeCpp>>(
            ConditionDisableRowsLoadOutcomeLikeCpp::Failed {
                reason: "read failed".into(),
            },
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "read failed");
    }
}
