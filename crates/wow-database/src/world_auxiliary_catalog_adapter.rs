//! MariaDB adapter for bounded ObjectMgr auxiliary startup catalogs.

use std::sync::Arc;

use anyhow::Result;
use wow_persistence::{
    AccessRequirementPersistenceRowLikeCpp, GraveyardZonePersistenceRowLikeCpp,
    PersistenceFutureLikeCpp, SceneTemplatePersistenceRowLikeCpp,
    SpawnGroupTemplatePersistenceRowLikeCpp, TrinityStringPersistenceRowLikeCpp,
    WorldAuxiliaryCatalogPersistencePortLikeCpp, WorldAuxiliaryRowsLoadOutcomeLikeCpp,
};

use crate::{WorldDatabase, WorldStatements};

pub struct MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn access_requirement_rows(&self) -> Result<Vec<AccessRequirementPersistenceRowLikeCpp>> {
        let mut result = self.world_db.direct_query(
            "SELECT mapid, difficulty, level_min, level_max, item, item2, quest_done_A, quest_done_H, completed_achievement, quest_failed_text FROM access_requirement",
        ).await?;
        let mut rows = Vec::with_capacity(result.row_count_like_cpp());
        if !result.is_empty() {
            loop {
                let fields = result.fields();
                rows.push(AccessRequirementPersistenceRowLikeCpp {
                    map_id: fields.try_read(0).unwrap_or(0),
                    difficulty: fields.try_read(1).unwrap_or(0),
                    level_min: fields.try_read(2).unwrap_or(0),
                    level_max: fields.try_read(3).unwrap_or(0),
                    item: fields.try_read(4).unwrap_or(0),
                    item2: fields.try_read(5).unwrap_or(0),
                    quest_done_a: fields.try_read(6).unwrap_or(0),
                    quest_done_h: fields.try_read(7).unwrap_or(0),
                    completed_achievement: fields.try_read(8).unwrap_or(0),
                    quest_failed_text: fields.read_string(9),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn graveyard_rows(&self) -> Result<Vec<GraveyardZonePersistenceRowLikeCpp>> {
        let stmt = self.world_db.prepare(WorldStatements::SEL_GRAVEYARD_ZONE);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(GraveyardZonePersistenceRowLikeCpp {
                    safe_loc_id: result.read(0),
                    ghost_zone_id: result.read(1),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn scene_rows(&self) -> Result<Vec<SceneTemplatePersistenceRowLikeCpp>> {
        let stmt = self.world_db.prepare(WorldStatements::SEL_SCENE_TEMPLATES);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(SceneTemplatePersistenceRowLikeCpp {
                    scene_id: result.read(0),
                    flags: result.read(1),
                    script_package_id: result.read(2),
                    encrypted: result.read(3),
                    script_name: result.read(4),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn spawn_group_rows(&self) -> Result<Vec<SpawnGroupTemplatePersistenceRowLikeCpp>> {
        let stmt = self
            .world_db
            .prepare(WorldStatements::SEL_SPAWN_GROUP_TEMPLATES);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(SpawnGroupTemplatePersistenceRowLikeCpp {
                    group_id: result.read(0),
                    name: result.read(1),
                    flags: result.read(2),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn trinity_string_rows(&self) -> Result<Vec<TrinityStringPersistenceRowLikeCpp>> {
        let mut result = self.world_db.direct_query(
            "SELECT entry, content_default, content_loc1, content_loc2, content_loc3, content_loc4, content_loc5, content_loc6, content_loc7, content_loc8 FROM trinity_string",
        ).await?;
        let mut rows = Vec::with_capacity(result.row_count_like_cpp());
        if !result.is_empty() {
            loop {
                let fields = result.fields();
                rows.push(TrinityStringPersistenceRowLikeCpp {
                    entry: fields.try_read(0).unwrap_or(0),
                    content: std::array::from_fn(|index| fields.read_string(index + 1)),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

fn outcome<T>(result: Result<T>) -> WorldAuxiliaryRowsLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => WorldAuxiliaryRowsLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => WorldAuxiliaryRowsLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

impl WorldAuxiliaryCatalogPersistencePortLikeCpp
    for MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp
{
    fn load_access_requirement_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<AccessRequirementPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.access_requirement_rows().await) })
    }
    fn load_graveyard_zone_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<GraveyardZonePersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.graveyard_rows().await) })
    }
    fn load_scene_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<SceneTemplatePersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.scene_rows().await) })
    }
    fn load_spawn_group_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<SpawnGroupTemplatePersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.spawn_group_rows().await) })
    }
    fn load_trinity_string_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldAuxiliaryRowsLoadOutcomeLikeCpp<Vec<TrinityStringPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.trinity_string_rows().await) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn prepared_statement_sources_keep_cpp_table_identity() {
        assert!(
            WorldStatements::SEL_GRAVEYARD_ZONE
                .sql()
                .contains("graveyard_zone")
        );
        assert!(
            WorldStatements::SEL_SCENE_TEMPLATES
                .sql()
                .contains("scene_template")
        );
        assert!(
            WorldStatements::SEL_SPAWN_GROUP_TEMPLATES
                .sql()
                .contains("spawn_group_template")
        );
    }
}
