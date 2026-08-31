//! Composition boundary for immutable C++ World quest-item metadata.

use anyhow::{Context, Result, bail};
use tracing::info;
use wow_persistence::{
    CreatureQuestItemPersistenceRowLikeCpp, GameObjectQuestItemPersistenceRowLikeCpp,
    QuestItemCatalogLoadOutcomeLikeCpp, QuestItemCatalogPersistencePortLikeCpp,
};

fn loaded_rows_like_cpp<T>(outcome: QuestItemCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        QuestItemCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        QuestItemCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

fn gameobject_quest_item_like_cpp(
    row: GameObjectQuestItemPersistenceRowLikeCpp,
) -> (u32, u32, u32) {
    (row.gameobject_entry, row.item_id, row.idx)
}

fn creature_quest_item_like_cpp(
    row: CreatureQuestItemPersistenceRowLikeCpp,
) -> (u32, u8, u32, u32) {
    (row.creature_entry, row.difficulty_id, row.item_id, row.idx)
}

pub(super) async fn load_quest_item_catalogs_like_cpp(
    persistence: &dyn QuestItemCatalogPersistencePortLikeCpp,
    gameobject_exists: impl Fn(u32) -> bool,
    creature_exists: impl Fn(u32) -> bool,
    item_exists: impl Fn(u32) -> bool,
) -> Result<(
    wow_data::GameObjectQuestItemStoreLikeCpp,
    wow_data::CreatureQuestItemStoreLikeCpp,
)> {
    // C++ `World::SetInitialWorldSettings` and current Rust both publish the
    // gameobject store before reading and publishing the creature store.
    let gameobject_rows =
        loaded_rows_like_cpp(persistence.load_gameobject_quest_item_rows_like_cpp().await)
            .context("Failed to load C++ gameobject_questitem rows")?;
    let gameobject_outcome = wow_data::GameObjectQuestItemStoreLikeCpp::from_rows_like_cpp(
        gameobject_rows
            .into_iter()
            .map(gameobject_quest_item_like_cpp),
        gameobject_exists,
        &item_exists,
    );
    for (entry, idx) in &gameobject_outcome.report.skipped_missing_gameobject {
        tracing::error!(
            target: "sql.sql",
            "Table `gameobject_questitem` has data for nonexistent gameobject (entry: {}, idx: {}), skipped",
            entry,
            idx
        );
    }
    for (entry, item_id, idx) in &gameobject_outcome.report.skipped_missing_item {
        tracing::error!(
            target: "sql.sql",
            "Table `gameobject_questitem` has nonexistent item (ID: {}) in gameobject (entry: {}, idx: {}), skipped",
            item_id,
            entry,
            idx
        );
    }
    info!(
        "Loaded {} C++ gameobject quest items from {} rows ({} skipped)",
        gameobject_outcome.report.loaded_items,
        gameobject_outcome.report.rows_seen,
        gameobject_outcome.report.skipped_missing_gameobject.len()
            + gameobject_outcome.report.skipped_missing_item.len()
    );

    let creature_rows =
        loaded_rows_like_cpp(persistence.load_creature_quest_item_rows_like_cpp().await)
            .context("Failed to load C++ creature_questitem rows")?;
    let creature_outcome = wow_data::CreatureQuestItemStoreLikeCpp::from_rows_like_cpp(
        creature_rows.into_iter().map(creature_quest_item_like_cpp),
        creature_exists,
        &item_exists,
    );
    for (entry, difficulty, idx) in &creature_outcome.report.skipped_missing_creature {
        tracing::error!(
            target: "sql.sql",
            "Table `creature_questitem` has data for nonexistent creature (entry: {}, difficulty: {}, idx: {}), skipped",
            entry,
            difficulty,
            idx
        );
    }
    for (entry, difficulty, item_id, idx) in &creature_outcome.report.skipped_missing_item {
        tracing::error!(
            target: "sql.sql",
            "Table `creature_questitem` has nonexistent item (ID: {}) in creature (entry: {}, difficulty: {}, idx: {}), skipped",
            item_id,
            entry,
            difficulty,
            idx
        );
    }
    info!(
        "Loaded {} C++ creature quest items from {} rows ({} skipped; difficulty fallback lookup represented)",
        creature_outcome.report.loaded_items,
        creature_outcome.report.rows_seen,
        creature_outcome.report.skipped_missing_creature.len()
            + creature_outcome.report.skipped_missing_item.len()
    );

    Ok((gameobject_outcome.store, creature_outcome.store))
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
        ) -> QuestItemCatalogLoadOutcomeLikeCpp<T> {
            self.calls.lock().unwrap().push(stage);
            if self.fail_at == Some(stage) {
                QuestItemCatalogLoadOutcomeLikeCpp::Failed {
                    reason: format!("{stage} read failed"),
                }
            } else if self.empty {
                QuestItemCatalogLoadOutcomeLikeCpp::Loaded(Vec::new())
            } else {
                QuestItemCatalogLoadOutcomeLikeCpp::Loaded(rows)
            }
        }
    }

    impl QuestItemCatalogPersistencePortLikeCpp for RecordingPort {
        fn load_gameobject_quest_item_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            QuestItemCatalogLoadOutcomeLikeCpp<GameObjectQuestItemPersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.outcome(
                    "gameobject",
                    vec![
                        GameObjectQuestItemPersistenceRowLikeCpp {
                            gameobject_entry: 11,
                            item_id: 101,
                            idx: 0,
                        },
                        GameObjectQuestItemPersistenceRowLikeCpp {
                            gameobject_entry: 12,
                            item_id: 101,
                            idx: 1,
                        },
                        GameObjectQuestItemPersistenceRowLikeCpp {
                            gameobject_entry: 11,
                            item_id: 999,
                            idx: 2,
                        },
                    ],
                )
            })
        }

        fn load_creature_quest_item_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            QuestItemCatalogLoadOutcomeLikeCpp<CreatureQuestItemPersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.outcome(
                    "creature",
                    vec![
                        CreatureQuestItemPersistenceRowLikeCpp {
                            creature_entry: 44,
                            difficulty_id: 5,
                            item_id: 202,
                            idx: 0,
                        },
                        CreatureQuestItemPersistenceRowLikeCpp {
                            creature_entry: 45,
                            difficulty_id: 5,
                            item_id: 202,
                            idx: 1,
                        },
                        CreatureQuestItemPersistenceRowLikeCpp {
                            creature_entry: 44,
                            difficulty_id: 5,
                            item_id: 999,
                            idx: 2,
                        },
                    ],
                )
            })
        }
    }

    #[tokio::test]
    async fn typed_rows_keep_cpp_order_and_domain_validation() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
            empty: false,
        };
        let (gameobjects, creatures) = load_quest_item_catalogs_like_cpp(
            &port,
            |entry| entry == 11,
            |entry| entry == 44,
            |item| matches!(item, 101 | 202),
        )
        .await
        .unwrap();

        assert_eq!(
            gameobjects.get_gameobject_quest_item_list_like_cpp(11),
            Some([101].as_slice())
        );
        assert_eq!(
            creatures.get_creature_quest_item_list_like_cpp(
                44,
                5,
                &wow_data::DifficultyStore::from_entries([]),
            ),
            Some([202].as_slice())
        );
        assert_eq!(*port.calls.lock().unwrap(), ["gameobject", "creature"]);
    }

    #[tokio::test]
    async fn empty_success_publishes_two_empty_stores() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
            empty: true,
        };
        let (gameobjects, creatures) =
            load_quest_item_catalogs_like_cpp(&port, |_| true, |_| true, |_| true)
                .await
                .unwrap();

        assert!(gameobjects.is_empty());
        assert!(creatures.is_empty());
        assert_eq!(*port.calls.lock().unwrap(), ["gameobject", "creature"]);
    }

    #[tokio::test]
    async fn each_failure_stops_later_reads_and_returns_no_store_pair() {
        let stages = ["gameobject", "creature"];
        for (failed_index, failed_stage) in stages.into_iter().enumerate() {
            let port = RecordingPort {
                calls: Mutex::new(Vec::new()),
                fail_at: Some(failed_stage),
                empty: false,
            };
            let result =
                load_quest_item_catalogs_like_cpp(&port, |_| true, |_| true, |_| true).await;

            assert!(result.is_err(), "{failed_stage} must fail startup");
            assert_eq!(*port.calls.lock().unwrap(), stages[..=failed_index]);
        }
    }
}
