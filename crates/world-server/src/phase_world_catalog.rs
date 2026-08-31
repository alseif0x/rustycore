//! Composition boundary for immutable C++ World phasing metadata.

use anyhow::{Context, Result, bail};
use tracing::info;
use wow_persistence::{
    PhaseAreaPersistenceRowLikeCpp, PhaseNamePersistenceRowLikeCpp,
    PhaseWorldCatalogLoadOutcomeLikeCpp, PhaseWorldCatalogPersistencePortLikeCpp,
    TerrainSwapDefaultPersistenceRowLikeCpp, TerrainWorldMapPersistenceRowLikeCpp,
};

fn loaded_rows_like_cpp<T>(outcome: PhaseWorldCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        PhaseWorldCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        PhaseWorldCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

fn phase_area_like_cpp(row: PhaseAreaPersistenceRowLikeCpp) -> (u32, u32) {
    (row.area_id, row.phase_id)
}

fn phase_name_like_cpp(row: PhaseNamePersistenceRowLikeCpp) -> (u32, String) {
    (row.phase_id, row.name)
}

fn terrain_world_map_like_cpp(row: TerrainWorldMapPersistenceRowLikeCpp) -> (u32, u32) {
    (row.terrain_swap_map, row.ui_map_phase_id)
}

fn terrain_swap_default_like_cpp(row: TerrainSwapDefaultPersistenceRowLikeCpp) -> (u32, u32) {
    (row.map_id, row.terrain_swap_map)
}

pub(super) async fn load_phase_world_catalogs_like_cpp(
    persistence: &dyn PhaseWorldCatalogPersistencePortLikeCpp,
    area_store: &wow_data::AreaTableStore,
    phase_store: &wow_data::PhaseStore,
    map_store: &wow_data::MapStore,
    mut is_ui_map_phase: impl FnMut(u32) -> bool,
) -> Result<(
    wow_data::PhaseInfoStore,
    wow_data::PhaseNameStoreLikeCpp,
    wow_data::TerrainSwapStore,
)> {
    // Preserve the represented Rust startup order. C++ `LoadPhases` loads the
    // terrain stages before area phases and loads phase names much later; that
    // pre-existing difference is a separate behavior-correction concern.
    let phase_area_rows = loaded_rows_like_cpp(persistence.load_phase_area_rows_like_cpp().await)
        .context("Failed to load phase_area rows")?;
    let mut phase_info_store = wow_data::PhaseInfoStore::from_phase_store_like_cpp(phase_store);
    let phase_area_count = phase_info_store.load_area_phases_from_rows_like_cpp(
        area_store,
        phase_store,
        phase_area_rows.into_iter().map(phase_area_like_cpp),
    );
    info!("Loaded {phase_area_count} phase area definitions");
    info!(
        "Seeded {} phase info records and {} phase area rows",
        phase_info_store.phase_info_count(),
        phase_info_store.phase_area_count()
    );

    let phase_name_rows = loaded_rows_like_cpp(persistence.load_phase_name_rows_like_cpp().await)
        .context("Failed to load C++ phase names")?;
    let phase_name_store = wow_data::PhaseNameStoreLikeCpp::from_rows_like_cpp(
        phase_name_rows.into_iter().map(phase_name_like_cpp),
    );
    info!("Loaded {} C++ phase names", phase_name_store.len());

    let terrain_world_map_rows =
        loaded_rows_like_cpp(persistence.load_terrain_world_map_rows_like_cpp().await)
            .context("Failed to load C++ terrain swap stores")?;
    let terrain_swap_default_rows =
        loaded_rows_like_cpp(persistence.load_terrain_swap_default_rows_like_cpp().await)
            .context("Failed to load C++ terrain swap stores")?;
    let terrain_swap_store = wow_data::TerrainSwapStore::from_rows_like_cpp(
        map_store,
        terrain_world_map_rows
            .into_iter()
            .map(terrain_world_map_like_cpp),
        terrain_swap_default_rows
            .into_iter()
            .map(terrain_swap_default_like_cpp),
        &mut is_ui_map_phase,
    );
    info!(
        "Loaded {} terrain swap definitions",
        terrain_swap_store.terrain_swap_count()
    );

    Ok((phase_info_store, phase_name_store, terrain_swap_store))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct RecordingPort {
        calls: Mutex<Vec<&'static str>>,
        fail_at: Option<&'static str>,
        empty: bool,
    }

    impl RecordingPort {
        fn outcome<T>(
            &self,
            stage: &'static str,
            rows: Vec<T>,
        ) -> PhaseWorldCatalogLoadOutcomeLikeCpp<T> {
            self.calls.lock().unwrap().push(stage);
            if self.fail_at == Some(stage) {
                PhaseWorldCatalogLoadOutcomeLikeCpp::Failed {
                    reason: format!("{stage} read failed"),
                }
            } else if self.empty {
                PhaseWorldCatalogLoadOutcomeLikeCpp::Loaded(Vec::new())
            } else {
                PhaseWorldCatalogLoadOutcomeLikeCpp::Loaded(rows)
            }
        }
    }

    impl PhaseWorldCatalogPersistencePortLikeCpp for RecordingPort {
        fn load_phase_area_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PhaseWorldCatalogLoadOutcomeLikeCpp<PhaseAreaPersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.outcome(
                    "phase_area",
                    vec![PhaseAreaPersistenceRowLikeCpp {
                        area_id: 7,
                        phase_id: 10,
                    }],
                )
            })
        }

        fn load_phase_name_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PhaseWorldCatalogLoadOutcomeLikeCpp<PhaseNamePersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.outcome(
                    "phase_name",
                    vec![PhaseNamePersistenceRowLikeCpp {
                        phase_id: 10,
                        name: "Phase Ten".into(),
                    }],
                )
            })
        }

        fn load_terrain_world_map_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PhaseWorldCatalogLoadOutcomeLikeCpp<TerrainWorldMapPersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.outcome(
                    "terrain_worldmap",
                    vec![TerrainWorldMapPersistenceRowLikeCpp {
                        terrain_swap_map: 609,
                        ui_map_phase_id: 42,
                    }],
                )
            })
        }

        fn load_terrain_swap_default_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PhaseWorldCatalogLoadOutcomeLikeCpp<TerrainSwapDefaultPersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.outcome(
                    "terrain_swap_defaults",
                    vec![TerrainSwapDefaultPersistenceRowLikeCpp {
                        map_id: 571,
                        terrain_swap_map: 609,
                    }],
                )
            })
        }
    }

    fn area_store() -> wow_data::AreaTableStore {
        wow_data::AreaTableStore::from_entries([wow_data::AreaTableEntry {
            id: 7,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        }])
    }

    fn phase_store() -> wow_data::PhaseStore {
        wow_data::PhaseStore::from_entries([wow_data::PhaseEntry { id: 10, flags: 0 }])
    }

    fn map_store() -> wow_data::MapStore {
        wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 571,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            wow_data::MapEntry {
                id: 609,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: 571,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])
    }

    #[tokio::test]
    async fn typed_rows_keep_current_startup_order_and_domain_validation() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
            empty: false,
        };
        let (phases, names, terrain) = load_phase_world_catalogs_like_cpp(
            &port,
            &area_store(),
            &phase_store(),
            &map_store(),
            |phase_id| phase_id == 42,
        )
        .await
        .unwrap();

        assert_eq!(phases.phase_area_count(), 1);
        assert_eq!(names.get_phase_name_like_cpp(10), "Phase Ten");
        assert_eq!(terrain.terrain_swaps_for_map(571), &[609]);
        assert_eq!(
            *port.calls.lock().unwrap(),
            [
                "phase_area",
                "phase_name",
                "terrain_worldmap",
                "terrain_swap_defaults"
            ]
        );
    }

    #[tokio::test]
    async fn empty_success_builds_only_seeded_domain_state() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
            empty: true,
        };
        let (phases, names, terrain) = load_phase_world_catalogs_like_cpp(
            &port,
            &area_store(),
            &phase_store(),
            &map_store(),
            |_| false,
        )
        .await
        .unwrap();

        assert_eq!(phases.phase_info_count(), 1);
        assert_eq!(phases.phase_area_count(), 0);
        assert!(names.is_empty());
        assert_eq!(terrain.terrain_swap_count(), 1);
    }

    #[tokio::test]
    async fn each_failure_stops_later_reads_before_any_catalog_tuple_is_returned() {
        let stages = [
            "phase_area",
            "phase_name",
            "terrain_worldmap",
            "terrain_swap_defaults",
        ];
        for (failed_index, failed_stage) in stages.into_iter().enumerate() {
            let port = RecordingPort {
                calls: Mutex::new(Vec::new()),
                fail_at: Some(failed_stage),
                empty: false,
            };
            let result = load_phase_world_catalogs_like_cpp(
                &port,
                &area_store(),
                &phase_store(),
                &map_store(),
                |_| true,
            )
            .await;
            assert!(result.is_err(), "{failed_stage} must fail startup");
            assert_eq!(
                *port.calls.lock().unwrap(),
                stages[..=failed_index],
                "later reads must not run after {failed_stage}"
            );
        }
    }
}
