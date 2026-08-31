//! Composition boundary for the effective Phase DB2 authorities.

use anyhow::{Context, Result, bail};
use tracing::info;
use wow_persistence::{
    PhaseGroupHotfixRowLikeCpp, PhaseHotfixLoadOutcomeLikeCpp, PhaseHotfixPersistencePortLikeCpp,
    PhaseHotfixRowLikeCpp,
};

fn phase_entry_like_cpp(row: PhaseHotfixRowLikeCpp) -> wow_data::PhaseEntry {
    wow_data::PhaseEntry {
        id: row.id,
        flags: row.flags,
    }
}

fn phase_group_entry_like_cpp(row: PhaseGroupHotfixRowLikeCpp) -> wow_data::PhaseXPhaseGroupEntry {
    wow_data::PhaseXPhaseGroupEntry {
        id: row.id,
        phase_id: row.phase_id,
        phase_group_id: row.phase_group_id,
    }
}

fn loaded_rows_like_cpp<T>(outcome: PhaseHotfixLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        PhaseHotfixLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        PhaseHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

async fn overlay_phase_stores_like_cpp<F>(
    mut phase_store: wow_data::PhaseStore,
    persistence: &dyn PhaseHotfixPersistencePortLikeCpp,
    load_phase_groups: F,
) -> Result<(wow_data::PhaseStore, wow_data::PhaseGroupStore)>
where
    F: FnOnce(&wow_data::PhaseStore) -> Result<wow_data::PhaseGroupStore>,
{
    let phase_rows = loaded_rows_like_cpp(persistence.load_phase_hotfix_rows_like_cpp().await)
        .context("Failed to load Phase.db2 / hotfix rows")?;
    let phase_hotfix_count =
        phase_store.apply_hotfix_entries_like_cpp(phase_rows.into_iter().map(phase_entry_like_cpp));
    if phase_hotfix_count != 0 {
        info!("Loaded {phase_hotfix_count} Phase hotfix rows");
    }

    // The group WDC4 rows depend on the effective Phase store, matching the
    // existing Rust order and C++'s post-load group-index validation.
    let mut phase_group_store = load_phase_groups(&phase_store)
        .context("Failed to load PhaseXPhaseGroup.db2 / hotfix rows")?;
    let group_rows =
        loaded_rows_like_cpp(persistence.load_phase_group_hotfix_rows_like_cpp().await)
            .context("Failed to load PhaseXPhaseGroup.db2 / hotfix rows")?;
    let group_hotfix_count = phase_group_store.apply_hotfix_entries_like_cpp(
        &phase_store,
        group_rows.into_iter().map(phase_group_entry_like_cpp),
    );
    if group_hotfix_count != 0 {
        info!("Loaded {group_hotfix_count} PhaseXPhaseGroup hotfix rows");
    }

    Ok((phase_store, phase_group_store))
}

pub(super) async fn load_phase_stores_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn PhaseHotfixPersistencePortLikeCpp,
) -> Result<(wow_data::PhaseStore, wow_data::PhaseGroupStore)> {
    let phase_store = wow_data::PhaseStore::load(data_dir, locale)?;
    overlay_phase_stores_like_cpp(phase_store, persistence, |phase_store| {
        wow_data::PhaseGroupStore::load(data_dir, locale, phase_store)
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct RecordingPort {
        calls: Mutex<Vec<&'static str>>,
        fail_at: Option<&'static str>,
    }

    impl PhaseHotfixPersistencePortLikeCpp for RecordingPort {
        fn load_phase_hotfix_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, PhaseHotfixLoadOutcomeLikeCpp<PhaseHotfixRowLikeCpp>>
        {
            Box::pin(async move {
                self.calls.lock().unwrap().push("phase_hotfix");
                if self.fail_at == Some("phase_hotfix") {
                    PhaseHotfixLoadOutcomeLikeCpp::Failed {
                        reason: "phase read failed".into(),
                    }
                } else {
                    PhaseHotfixLoadOutcomeLikeCpp::Loaded(vec![PhaseHotfixRowLikeCpp {
                        id: 10,
                        flags: 0x0020, // C++ `PHASE_ENTRY_FLAG_PERSONAL`
                    }])
                }
            })
        }

        fn load_phase_group_hotfix_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, PhaseHotfixLoadOutcomeLikeCpp<PhaseGroupHotfixRowLikeCpp>>
        {
            Box::pin(async move {
                self.calls.lock().unwrap().push("phase_group_hotfix");
                if self.fail_at == Some("phase_group_hotfix") {
                    PhaseHotfixLoadOutcomeLikeCpp::Failed {
                        reason: "phase-group read failed".into(),
                    }
                } else {
                    PhaseHotfixLoadOutcomeLikeCpp::Loaded(vec![PhaseGroupHotfixRowLikeCpp {
                        id: 1,
                        phase_id: 10,
                        phase_group_id: 7,
                    }])
                }
            })
        }
    }

    fn phase_base() -> wow_data::PhaseStore {
        wow_data::PhaseStore::from_entries([wow_data::PhaseEntry { id: 10, flags: 0 }])
    }

    #[tokio::test]
    async fn startup_preserves_phase_overlay_then_group_load_then_group_overlay_order() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
        };

        let (phases, groups) = overlay_phase_stores_like_cpp(phase_base(), &port, |phases| {
            port.calls.lock().unwrap().push("phase_group_db2");
            assert!(phases.is_personal_phase(10));
            Ok(wow_data::PhaseGroupStore::from_entries(phases, []))
        })
        .await
        .unwrap();

        assert!(phases.is_personal_phase(10));
        assert_eq!(groups.phases_for_group(7), Some([10].as_slice()));
        assert_eq!(
            *port.calls.lock().unwrap(),
            ["phase_hotfix", "phase_group_db2", "phase_group_hotfix"]
        );
    }

    #[tokio::test]
    async fn phase_failure_stops_before_group_db2_and_group_query() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: Some("phase_hotfix"),
        };

        let result = overlay_phase_stores_like_cpp(phase_base(), &port, |_| {
            panic!("phase-group DB2 load must not run after Phase query failure")
        })
        .await;

        let error = match result {
            Ok(_) => panic!("a failed Phase read must not publish stores"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "Failed to load Phase.db2 / hotfix rows");
        assert_eq!(
            error.root_cause().to_string(),
            "phase read failed",
            "the adapter reason remains available"
        );
        assert_eq!(*port.calls.lock().unwrap(), ["phase_hotfix"]);
    }

    #[tokio::test]
    async fn group_failure_returns_no_partially_assembled_pair() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: Some("phase_group_hotfix"),
        };

        let result = overlay_phase_stores_like_cpp(phase_base(), &port, |phases| {
            port.calls.lock().unwrap().push("phase_group_db2");
            Ok(wow_data::PhaseGroupStore::from_entries(phases, []))
        })
        .await;

        let error = match result {
            Ok(_) => panic!("a failed group read must not publish stores"),
            Err(error) => error,
        };
        assert_eq!(
            error.to_string(),
            "Failed to load PhaseXPhaseGroup.db2 / hotfix rows"
        );
        assert_eq!(
            error.root_cause().to_string(),
            "phase-group read failed",
            "the adapter reason remains available"
        );
        assert_eq!(
            *port.calls.lock().unwrap(),
            ["phase_hotfix", "phase_group_db2", "phase_group_hotfix"]
        );
    }

    #[test]
    fn typed_rows_preserve_every_consumed_phase_field() {
        assert_eq!(
            phase_entry_like_cpp(PhaseHotfixRowLikeCpp { id: 1, flags: 2 }),
            wow_data::PhaseEntry { id: 1, flags: 2 }
        );
        assert_eq!(
            phase_group_entry_like_cpp(PhaseGroupHotfixRowLikeCpp {
                id: 3,
                phase_id: 4,
                phase_group_id: 5,
            }),
            wow_data::PhaseXPhaseGroupEntry {
                id: 3,
                phase_id: 4,
                phase_group_id: 5,
            }
        );
    }
}
