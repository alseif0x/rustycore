//! MariaDB adapter for the SQLx-free quest POI load capability.

use std::sync::Arc;

use sqlx::Row;
use wow_persistence::{
    PersistenceFutureLikeCpp, QuestPoiBlobLoadRowLikeCpp, QuestPoiLoadOutcomeLikeCpp,
    QuestPoiLoadStageLikeCpp, QuestPoiPersistencePortLikeCpp, QuestPoiPointLoadRowLikeCpp,
};

use crate::WorldDatabase;

const LOAD_QUEST_POI_POINTS_SQL: &str =
    "SELECT QuestID, Idx1, X, Y, Z FROM quest_poi_points ORDER BY QuestID DESC, Idx1, Idx2";
const LOAD_QUEST_POI_BLOBS_SQL: &str = "SELECT QuestID, BlobIndex, Idx1, ObjectiveIndex, QuestObjectiveID, QuestObjectID, MapID, UiMapID, Priority, Flags, WorldEffectID, PlayerConditionID, NavigationPlayerConditionID, SpawnTrackingID, AlwaysAllowMergingBlobs FROM quest_poi ORDER BY QuestID, Idx1";

pub struct MariaDbQuestPoiPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbQuestPoiPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl QuestPoiPersistencePortLikeCpp for MariaDbQuestPoiPersistenceAdapterLikeCpp {
    fn load_quest_poi_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestPoiLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let point_rows = match sqlx::query(LOAD_QUEST_POI_POINTS_SQL)
                .fetch_all(self.world_db.pool())
                .await
            {
                Ok(rows) => rows,
                Err(error) => {
                    return QuestPoiLoadOutcomeLikeCpp::Failed {
                        stage: QuestPoiLoadStageLikeCpp::Points,
                        reason: error.to_string(),
                    };
                }
            };

            let mut points = Vec::with_capacity(point_rows.len());
            for row in point_rows {
                let decoded = (|| {
                    Ok::<_, sqlx::Error>(QuestPoiPointLoadRowLikeCpp {
                        quest_id: row.try_get(0)?,
                        idx1: row.try_get(1)?,
                        x: row.try_get(2)?,
                        y: row.try_get(3)?,
                        z: row.try_get(4)?,
                    })
                })();
                match decoded {
                    Ok(row) => points.push(row),
                    Err(error) => {
                        return QuestPoiLoadOutcomeLikeCpp::Failed {
                            stage: QuestPoiLoadStageLikeCpp::Points,
                            reason: error.to_string(),
                        };
                    }
                }
            }

            let blob_rows = match sqlx::query(LOAD_QUEST_POI_BLOBS_SQL)
                .fetch_all(self.world_db.pool())
                .await
            {
                Ok(rows) => rows,
                Err(error) => {
                    return QuestPoiLoadOutcomeLikeCpp::Failed {
                        stage: QuestPoiLoadStageLikeCpp::Blobs,
                        reason: error.to_string(),
                    };
                }
            };

            let mut blobs = Vec::with_capacity(blob_rows.len());
            for row in blob_rows {
                let decoded = (|| {
                    Ok::<_, sqlx::Error>(QuestPoiBlobLoadRowLikeCpp {
                        quest_id: row.try_get(0)?,
                        blob_index: row.try_get(1)?,
                        idx1: row.try_get(2)?,
                        objective_index: row.try_get(3)?,
                        quest_objective_id: row.try_get(4)?,
                        quest_object_id: row.try_get(5)?,
                        map_id: row.try_get(6)?,
                        ui_map_id: row.try_get(7)?,
                        priority: row.try_get(8)?,
                        flags: row.try_get(9)?,
                        world_effect_id: row.try_get(10)?,
                        player_condition_id: row.try_get(11)?,
                        navigation_player_condition_id: row.try_get(12)?,
                        spawn_tracking_id: row.try_get(13)?,
                        always_allow_merging_blobs: row.try_get::<u8, _>(14)? != 0,
                    })
                })();
                match decoded {
                    Ok(row) => blobs.push(row),
                    Err(error) => {
                        return QuestPoiLoadOutcomeLikeCpp::Failed {
                            stage: QuestPoiLoadStageLikeCpp::Blobs,
                            reason: error.to_string(),
                        };
                    }
                }
            }

            QuestPoiLoadOutcomeLikeCpp::Loaded { points, blobs }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quest_poi_queries_preserve_represented_shapes_and_ordering() {
        assert_eq!(LOAD_QUEST_POI_POINTS_SQL.matches('?').count(), 0);
        assert!(LOAD_QUEST_POI_POINTS_SQL.ends_with("ORDER BY QuestID DESC, Idx1, Idx2"));
        assert_eq!(LOAD_QUEST_POI_BLOBS_SQL.matches('?').count(), 0);
        assert!(LOAD_QUEST_POI_BLOBS_SQL.ends_with("ORDER BY QuestID, Idx1"));
    }
}
