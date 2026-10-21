//! MariaDB adapter for bounded ObjectMgr auxiliary startup catalogs.

use std::sync::Arc;

use anyhow::Result;
use wow_persistence::{
    AccessRequirementPersistenceRowLikeCpp, GraveyardZonePersistenceRowLikeCpp,
    PersistenceFutureLikeCpp, SceneTemplatePersistenceRowLikeCpp,
    SpawnGroupTemplatePersistenceRowLikeCpp, TrinityStringPersistenceRowLikeCpp,
    WorldAuxiliaryCatalogPersistencePortLikeCpp, WorldAuxiliaryRowsLoadOutcomeLikeCpp,
};

use crate::field_like_cpp::FieldRowLikeCpp;
use crate::result::collect_rows_like_cpp;
use crate::{WorldDatabase, WorldStatements};

pub struct MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn access_requirement_rows(&self) -> Result<Vec<AccessRequirementPersistenceRowLikeCpp>> {
        let result = self.world_db.direct_query(
            "SELECT mapid, difficulty, level_min, level_max, item, item2, quest_done_A, quest_done_H, completed_achievement, quest_failed_text FROM access_requirement",
        ).await?;
        // ObjectMgr::LoadAccessRequirements (ObjectMgr.cpp:7187-7225) getters.
        collect_rows_like_cpp(result, "access_requirement", |row| {
            Ok(AccessRequirementPersistenceRowLikeCpp {
                map_id: row.get_like_cpp(0, "access_requirement.mapid")?,
                difficulty: row.get_like_cpp(1, "access_requirement.difficulty")?,
                level_min: row.get_like_cpp(2, "access_requirement.level_min")?,
                level_max: row.get_like_cpp(3, "access_requirement.level_max")?,
                item: row.get_like_cpp(4, "access_requirement.item")?,
                item2: row.get_like_cpp(5, "access_requirement.item2")?,
                quest_done_a: row.get_like_cpp(6, "access_requirement.quest_done_A")?,
                quest_done_h: row.get_like_cpp(7, "access_requirement.quest_done_H")?,
                completed_achievement: row
                    .get_like_cpp(8, "access_requirement.completed_achievement")?,
                quest_failed_text: row
                    .get_string_like_cpp(9, "access_requirement.quest_failed_text")?,
            })
        })
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
        let result = self.world_db.direct_query(
            "SELECT entry, content_default, content_loc1, content_loc2, content_loc3, content_loc4, content_loc5, content_loc6, content_loc7, content_loc8 FROM trinity_string",
        ).await?;
        // ObjectMgr::LoadTrinityStrings (ObjectMgr.cpp:8835-8860) getters.
        collect_rows_like_cpp(result, "trinity_string", |row| {
            let mut content: [String; 9] = Default::default();
            for (index, text) in content.iter_mut().enumerate() {
                *text = row.get_string_like_cpp(index + 1, "trinity_string.content")?;
            }
            Ok(TrinityStringPersistenceRowLikeCpp {
                entry: row.get_like_cpp(0, "trinity_string.entry")?,
                content,
            })
        })
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
