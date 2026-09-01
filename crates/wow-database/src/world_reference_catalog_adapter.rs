//! MariaDB adapter for immutable world identifiers and safe locations.

use std::sync::Arc;

use anyhow::{Result, bail};
use wow_persistence::{
    PersistenceFutureLikeCpp, WorldObjectIdCatalogKindLikeCpp,
    WorldReferenceCatalogPersistencePortLikeCpp, WorldReferenceRowsLoadOutcomeLikeCpp,
    WorldSafeLocPersistenceRowLikeCpp, WorldSpawnCatalogKindLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

pub struct MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn object_ids(&self, kind: WorldObjectIdCatalogKindLikeCpp) -> Result<Vec<u32>> {
        let statement = match kind {
            WorldObjectIdCatalogKindLikeCpp::CreatureTemplate => {
                WorldStatements::SEL_CREATURE_TEMPLATE_IDS
            }
            WorldObjectIdCatalogKindLikeCpp::GameObjectTemplate => {
                WorldStatements::SEL_GAMEOBJECT_TEMPLATE_IDS
            }
            WorldObjectIdCatalogKindLikeCpp::GameEvent => WorldStatements::SEL_VALID_GAME_EVENT_IDS,
            WorldObjectIdCatalogKindLikeCpp::WorldState => WorldStatements::SEL_WORLD_STATE_IDS,
            WorldObjectIdCatalogKindLikeCpp::Trainer => WorldStatements::SEL_TRAINER_IDS,
            WorldObjectIdCatalogKindLikeCpp::ConversationLineTemplate => {
                WorldStatements::SEL_CONVERSATION_LINE_TEMPLATE_IDS
            }
        };
        let stmt = self.world_db.prepare(statement);
        let mut result = self.world_db.query(&stmt).await?;
        let mut ids = Vec::new();
        if !result.is_empty() {
            loop {
                if let Some(id) = read_world_id_like_cpp(&result, 0)? {
                    ids.push(id);
                }
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(ids)
    }

    async fn spawn_ids(&self, kind: WorldSpawnCatalogKindLikeCpp) -> Result<Vec<(u32, u32)>> {
        let statement = match kind {
            WorldSpawnCatalogKindLikeCpp::Creature => WorldStatements::SEL_CREATURE_SPAWN_IDS,
            WorldSpawnCatalogKindLikeCpp::GameObject => WorldStatements::SEL_GAMEOBJECT_SPAWN_IDS,
        };
        let stmt = self.world_db.prepare(statement);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                let guid = result.read::<u64>(0);
                if let Ok(guid) = u32::try_from(guid) {
                    rows.push((guid, result.read(1)));
                }
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn safe_locs(&self) -> Result<Vec<WorldSafeLocPersistenceRowLikeCpp>> {
        let stmt = self.world_db.prepare(WorldStatements::SEL_WORLD_SAFE_LOCS);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(WorldSafeLocPersistenceRowLikeCpp {
                    id: result.read(0),
                    map_id: result.read(1),
                    x: result.read(2),
                    y: result.read(3),
                    z: result.read(4),
                    facing_degrees: result.read(5),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

fn read_world_id_like_cpp(result: &SqlResult, column: usize) -> Result<Option<u32>> {
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(Some(value));
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return Ok(u32::try_from(value).ok());
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return Ok(Some(u32::from(value)));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return Ok(Some(u32::from(value)));
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return Ok(u32::try_from(value).ok());
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return Ok(u32::try_from(value).ok());
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return Ok(u32::try_from(value).ok());
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return Ok(u32::try_from(value).ok());
    }
    bail!("unsupported ID column type while loading world reference catalog")
}

fn outcome<T>(result: Result<T>) -> WorldReferenceRowsLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => WorldReferenceRowsLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => WorldReferenceRowsLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

impl WorldReferenceCatalogPersistencePortLikeCpp
    for MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp
{
    fn load_world_object_ids_like_cpp(
        &self,
        kind: WorldObjectIdCatalogKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, WorldReferenceRowsLoadOutcomeLikeCpp<Vec<u32>>> {
        Box::pin(async move { outcome(self.object_ids(kind).await) })
    }

    fn load_world_spawn_ids_like_cpp(
        &self,
        kind: WorldSpawnCatalogKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, WorldReferenceRowsLoadOutcomeLikeCpp<Vec<(u32, u32)>>> {
        Box::pin(async move { outcome(self.spawn_ids(kind).await) })
    }

    fn load_world_safe_locs_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldReferenceRowsLoadOutcomeLikeCpp<Vec<WorldSafeLocPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.safe_locs().await) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn bounded_kinds_map_to_expected_statement_families() {
        assert!(
            WorldStatements::SEL_CREATURE_TEMPLATE_IDS
                .sql()
                .contains("creature_template")
        );
        assert!(
            WorldStatements::SEL_GAMEOBJECT_TEMPLATE_IDS
                .sql()
                .contains("gameobject_template")
        );
        assert!(
            WorldStatements::SEL_CREATURE_SPAWN_IDS
                .sql()
                .contains("creature")
        );
        assert!(
            WorldStatements::SEL_GAMEOBJECT_SPAWN_IDS
                .sql()
                .contains("gameobject")
        );
        assert!(
            WorldStatements::SEL_WORLD_SAFE_LOCS
                .sql()
                .contains("world_safe_locs")
        );
    }
}
