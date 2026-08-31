//! Composition boundary for late C++ LFG World catalogs.

use anyhow::{Result, bail};
use wow_persistence::{
    LfgDungeonRewardPersistenceRowLikeCpp, LfgDungeonTemplatePersistenceRowLikeCpp,
    LfgWorldCatalogLoadOutcomeLikeCpp, LfgWorldCatalogPersistencePortLikeCpp,
};

fn template_row_like_cpp(
    row: LfgDungeonTemplatePersistenceRowLikeCpp,
) -> wow_data::LfgDungeonTemplateRowLikeCpp {
    wow_data::LfgDungeonTemplateRowLikeCpp {
        dungeon_id: row.dungeon_id,
        position_x: row.position_x,
        position_y: row.position_y,
        position_z: row.position_z,
        orientation: row.orientation,
        required_item_level: row.required_item_level,
    }
}

fn reward_row_like_cpp(
    row: LfgDungeonRewardPersistenceRowLikeCpp,
) -> wow_data::LfgDungeonRewardRowLikeCpp {
    wow_data::LfgDungeonRewardRowLikeCpp {
        dungeon_id: row.dungeon_id,
        reward: wow_data::LfgDungeonRewardLikeCpp {
            max_level: row.max_level,
            first_quest_id: row.first_quest_id,
            other_quest_id: row.other_quest_id,
        },
    }
}

fn loaded_rows_like_cpp<T>(outcome: LfgWorldCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        LfgWorldCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        LfgWorldCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

async fn load_rows_like_cpp(
    persistence: &dyn LfgWorldCatalogPersistencePortLikeCpp,
) -> Result<(
    Vec<LfgDungeonTemplatePersistenceRowLikeCpp>,
    Vec<LfgDungeonRewardPersistenceRowLikeCpp>,
)> {
    let templates =
        loaded_rows_like_cpp(persistence.load_lfg_dungeon_template_rows_like_cpp().await)?;
    let rewards = loaded_rows_like_cpp(persistence.load_lfg_dungeon_reward_rows_like_cpp().await)?;
    Ok((templates, rewards))
}

pub(super) async fn load_lfg_dungeon_store_like_cpp(
    persistence: &dyn LfgWorldCatalogPersistencePortLikeCpp,
    db2_store: &wow_data::LfgDungeonsStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    quest_store: &wow_data::quest::QuestStore,
) -> Result<wow_data::LfgLoadOutcomeLikeCpp> {
    let (templates, rewards) = load_rows_like_cpp(persistence).await?;
    Ok(wow_data::LfgDungeonStoreLikeCpp::from_sources_like_cpp(
        db2_store,
        map_difficulty_store,
        templates.into_iter().map(template_row_like_cpp),
        rewards.into_iter().map(reward_row_like_cpp),
        quest_store,
    ))
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

    impl LfgWorldCatalogPersistencePortLikeCpp for RecordingPort {
        fn load_lfg_dungeon_template_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            LfgWorldCatalogLoadOutcomeLikeCpp<LfgDungeonTemplatePersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.calls.lock().unwrap().push("templates");
                if self.fail_at == Some("templates") {
                    LfgWorldCatalogLoadOutcomeLikeCpp::Failed {
                        reason: "template read failed".into(),
                    }
                } else {
                    LfgWorldCatalogLoadOutcomeLikeCpp::Loaded(Vec::new())
                }
            })
        }

        fn load_lfg_dungeon_reward_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            LfgWorldCatalogLoadOutcomeLikeCpp<LfgDungeonRewardPersistenceRowLikeCpp>,
        > {
            Box::pin(async move {
                self.calls.lock().unwrap().push("rewards");
                if self.fail_at == Some("rewards") {
                    LfgWorldCatalogLoadOutcomeLikeCpp::Failed {
                        reason: "reward read failed".into(),
                    }
                } else {
                    LfgWorldCatalogLoadOutcomeLikeCpp::Loaded(Vec::new())
                }
            })
        }
    }

    #[tokio::test]
    async fn empty_success_preserves_template_then_reward_order() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
        };
        let (templates, rewards) = load_rows_like_cpp(&port).await.unwrap();
        assert!(templates.is_empty());
        assert!(rewards.is_empty());
        assert_eq!(*port.calls.lock().unwrap(), ["templates", "rewards"]);
    }

    #[tokio::test]
    async fn template_failure_stops_before_reward_read() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: Some("templates"),
        };
        assert_eq!(
            load_rows_like_cpp(&port).await.unwrap_err().to_string(),
            "template read failed"
        );
        assert_eq!(*port.calls.lock().unwrap(), ["templates"]);
    }

    #[tokio::test]
    async fn reward_failure_exposes_no_partial_template_batch() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: Some("rewards"),
        };
        assert_eq!(
            load_rows_like_cpp(&port).await.unwrap_err().to_string(),
            "reward read failed"
        );
        assert_eq!(*port.calls.lock().unwrap(), ["templates", "rewards"]);
    }

    #[test]
    fn typed_rows_preserve_every_domain_field() {
        assert_eq!(
            template_row_like_cpp(LfgDungeonTemplatePersistenceRowLikeCpp {
                dungeon_id: 1,
                position_x: 2.0,
                position_y: 3.0,
                position_z: 4.0,
                orientation: 5.0,
                required_item_level: 6,
            }),
            wow_data::LfgDungeonTemplateRowLikeCpp {
                dungeon_id: 1,
                position_x: 2.0,
                position_y: 3.0,
                position_z: 4.0,
                orientation: 5.0,
                required_item_level: 6,
            }
        );
        assert_eq!(
            reward_row_like_cpp(LfgDungeonRewardPersistenceRowLikeCpp {
                dungeon_id: 7,
                max_level: 8,
                first_quest_id: 9,
                other_quest_id: 10,
            }),
            wow_data::LfgDungeonRewardRowLikeCpp {
                dungeon_id: 7,
                reward: wow_data::LfgDungeonRewardLikeCpp {
                    max_level: 8,
                    first_quest_id: 9,
                    other_quest_id: 10,
                },
            }
        );
    }
}
