//! Composition boundary for the effective C++ ChrSpecialization authority.

use anyhow::{Result, bail};
use wow_persistence::{
    ChrSpecializationHotfixLoadOutcomeLikeCpp, ChrSpecializationHotfixPersistencePortLikeCpp,
    ChrSpecializationHotfixRowLikeCpp,
};

fn chr_specialization_entry_like_cpp(
    row: ChrSpecializationHotfixRowLikeCpp,
) -> wow_data::ChrSpecializationEntry {
    wow_data::ChrSpecializationEntry {
        id: row.id,
        class_id: row.class_id,
        order_index: row.order_index,
        role: row.role,
    }
}

fn apply_loaded_hotfix_outcome_like_cpp(
    base: wow_data::ChrSpecializationStore,
    outcome: ChrSpecializationHotfixLoadOutcomeLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> Result<wow_data::ChrSpecializationStore> {
    let rows = match outcome {
        ChrSpecializationHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        ChrSpecializationHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    base.apply_hotfix_overlays_like_cpp(
        rows.official
            .into_iter()
            .map(chr_specialization_entry_like_cpp),
        rows.custom
            .into_iter()
            .map(chr_specialization_entry_like_cpp),
        removals,
    )
}

pub(super) async fn load_chr_specialization_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn ChrSpecializationHotfixPersistencePortLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> Result<wow_data::ChrSpecializationStore> {
    // Preserve C++ `DB2Stores.cpp::LoadDB2`: WDC4 precedes
    // `DB2StorageBase::LoadFromDB` and therefore both SQL overlays.
    let base = wow_data::ChrSpecializationStore::load(data_dir, locale)?;
    let outcome = persistence
        .load_chr_specialization_hotfix_rows_like_cpp()
        .await;
    apply_loaded_hotfix_outcome_like_cpp(base, outcome, removals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_hotfix_row_preserves_every_consumed_field() {
        assert_eq!(
            chr_specialization_entry_like_cpp(ChrSpecializationHotfixRowLikeCpp {
                id: 1,
                class_id: 2,
                order_index: -3,
                role: 4,
            }),
            wow_data::ChrSpecializationEntry {
                id: 1,
                class_id: 2,
                order_index: -3,
                role: 4,
            }
        );
    }

    #[test]
    fn failed_overlay_read_stops_before_domain_application() {
        let base = wow_data::ChrSpecializationStore::from_entries([]);
        let result = apply_loaded_hotfix_outcome_like_cpp(
            base,
            ChrSpecializationHotfixLoadOutcomeLikeCpp::Failed {
                reason: "hotfix read failed".into(),
            },
            &wow_data::Db2HotfixRemovalStoreLikeCpp::default(),
        );
        let error = match result {
            Ok(_) => panic!("failed overlay read must not publish a store"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "hotfix read failed");
    }
}
