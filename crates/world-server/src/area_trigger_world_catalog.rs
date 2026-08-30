//! Composition boundary for represented AreaTrigger World-table catalogs.

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use wow_persistence::{
    AreaTriggerWorldCatalogPersistencePortLikeCpp, AreaTriggerWorldLoadOutcomeLikeCpp,
};

pub(super) struct AreaTriggerWorldCatalogsLikeCpp {
    pub area_trigger_store: Arc<wow_data::AreaTriggerStore>,
    pub script_outcome: wow_data::AreaTriggerScriptLoadOutcomeLikeCpp,
    pub tavern_outcome: wow_data::TavernAreaTriggerLoadOutcomeLikeCpp,
}

pub(super) async fn load_area_trigger_world_catalogs_like_cpp(
    persistence: &dyn AreaTriggerWorldCatalogPersistencePortLikeCpp,
    area_trigger_db2_store: &wow_data::AreaTriggerDb2Store,
    script_names: &mut wow_data::ScriptNameInternerLikeCpp,
) -> Result<AreaTriggerWorldCatalogsLikeCpp> {
    // Preserve the existing production sequence. The teleport-relation and
    // quest-relation operations remain dormant until their owners compose them.
    let destination_rows = loaded_rows_like_cpp(persistence.load_destination_rows_like_cpp().await)
        .context("Failed to load area triggers")?;
    let area_trigger_store = Arc::new(wow_data::load_area_triggers(
        destination_rows
            .into_iter()
            .map(|row| wow_data::AreaTriggerDestinationRowLikeCpp {
                trigger_id: row.trigger_id,
                target_map: row.target_map,
                target_x: row.target_x,
                target_y: row.target_y,
                target_z: row.target_z,
                target_orientation: row.target_orientation,
            }),
    ));

    let script_rows = loaded_rows_like_cpp(persistence.load_script_rows_like_cpp().await)
        .context("Failed to load C++ area trigger scripts")?;
    let script_outcome = wow_data::AreaTriggerScriptStoreLikeCpp::from_rows_like_cpp(
        script_rows
            .into_iter()
            .map(|row| wow_data::AreaTriggerScriptRowLikeCpp {
                entry: row.trigger_id,
                script_name: row.script_name,
            }),
        |entry| area_trigger_db2_store.get(entry).is_some(),
        script_names,
    );

    let tavern_rows = loaded_rows_like_cpp(persistence.load_tavern_rows_like_cpp().await)
        .context("Failed to load C++ tavern area triggers")?;
    let tavern_outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp(
        tavern_rows.into_iter().map(|row| row.trigger_id),
        |trigger_id| area_trigger_db2_store.get(trigger_id).is_some(),
    );

    Ok(AreaTriggerWorldCatalogsLikeCpp {
        area_trigger_store,
        script_outcome,
        tavern_outcome,
    })
}

fn loaded_rows_like_cpp<T>(outcome: AreaTriggerWorldLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        AreaTriggerWorldLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        AreaTriggerWorldLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use wow_persistence::{
        AreaTriggerDestinationPersistenceRowLikeCpp, AreaTriggerScriptPersistenceRowLikeCpp,
        AreaTriggerTeleportPersistenceRowLikeCpp, AreaTriggerWorldLoadOutcomeLikeCpp,
        PersistenceFutureLikeCpp, QuestAreaTriggerPersistenceRowLikeCpp,
        TavernAreaTriggerPersistenceRowLikeCpp,
    };

    struct RecordingPort {
        calls: Mutex<Vec<&'static str>>,
    }

    impl AreaTriggerWorldCatalogPersistencePortLikeCpp for RecordingPort {
        fn load_destination_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerDestinationPersistenceRowLikeCpp>,
        > {
            self.calls.lock().unwrap().push("destination");
            Box::pin(async { AreaTriggerWorldLoadOutcomeLikeCpp::Loaded(Vec::new()) })
        }

        fn load_script_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerScriptPersistenceRowLikeCpp>,
        > {
            self.calls.lock().unwrap().push("script");
            Box::pin(async { AreaTriggerWorldLoadOutcomeLikeCpp::Loaded(Vec::new()) })
        }

        fn load_teleport_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerTeleportPersistenceRowLikeCpp>,
        > {
            self.calls.lock().unwrap().push("dormant-teleport");
            Box::pin(async { AreaTriggerWorldLoadOutcomeLikeCpp::Loaded(Vec::new()) })
        }

        fn load_quest_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<QuestAreaTriggerPersistenceRowLikeCpp>,
        > {
            self.calls.lock().unwrap().push("dormant-quest");
            Box::pin(async { AreaTriggerWorldLoadOutcomeLikeCpp::Loaded(Vec::new()) })
        }

        fn load_tavern_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<TavernAreaTriggerPersistenceRowLikeCpp>,
        > {
            self.calls.lock().unwrap().push("tavern");
            Box::pin(async { AreaTriggerWorldLoadOutcomeLikeCpp::Loaded(Vec::new()) })
        }
    }

    #[tokio::test]
    async fn production_operations_keep_order_without_activating_dormant_reads() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
        };
        let db2 = wow_data::AreaTriggerDb2Store::from_entries([]);
        let mut scripts = wow_data::ScriptNameInternerLikeCpp::default();

        let loaded = load_area_trigger_world_catalogs_like_cpp(&port, &db2, &mut scripts)
            .await
            .unwrap();

        assert_eq!(
            *port.calls.lock().unwrap(),
            ["destination", "script", "tavern"]
        );
        assert_eq!(loaded.area_trigger_store.len(), 0);
        assert!(loaded.script_outcome.store.is_empty());
        assert!(loaded.tavern_outcome.store.is_empty());
    }

    struct FailingDestinationPort(RecordingPort);

    impl AreaTriggerWorldCatalogPersistencePortLikeCpp for FailingDestinationPort {
        fn load_destination_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerDestinationPersistenceRowLikeCpp>,
        > {
            self.0.calls.lock().unwrap().push("destination");
            Box::pin(async {
                AreaTriggerWorldLoadOutcomeLikeCpp::Failed {
                    reason: "destination decode failed".into(),
                }
            })
        }

        fn load_script_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerScriptPersistenceRowLikeCpp>,
        > {
            self.0.load_script_rows_like_cpp()
        }

        fn load_teleport_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerTeleportPersistenceRowLikeCpp>,
        > {
            self.0.load_teleport_rows_like_cpp()
        }

        fn load_quest_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<QuestAreaTriggerPersistenceRowLikeCpp>,
        > {
            self.0.load_quest_rows_like_cpp()
        }

        fn load_tavern_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            AreaTriggerWorldLoadOutcomeLikeCpp<TavernAreaTriggerPersistenceRowLikeCpp>,
        > {
            self.0.load_tavern_rows_like_cpp()
        }
    }

    #[tokio::test]
    async fn first_failure_stops_before_later_reads_or_publication() {
        let port = FailingDestinationPort(RecordingPort {
            calls: Mutex::new(Vec::new()),
        });
        let db2 = wow_data::AreaTriggerDb2Store::from_entries([]);
        let mut scripts = wow_data::ScriptNameInternerLikeCpp::default();

        let error = load_area_trigger_world_catalogs_like_cpp(&port, &db2, &mut scripts)
            .await
            .err()
            .unwrap();

        assert_eq!(*port.0.calls.lock().unwrap(), ["destination"]);
        assert!(error.to_string().contains("Failed to load area triggers"));
    }
}
