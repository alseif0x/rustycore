//! MariaDB adapter for WorldState templates and saved values at startup.

use std::sync::Arc;

use anyhow::Result;
use wow_persistence::{
    PersistenceFutureLikeCpp, WorldStateSavedValuePersistenceRowLikeCpp,
    WorldStateStartupCatalogLikeCpp, WorldStateStartupLoadOutcomeLikeCpp,
    WorldStateStartupPersistencePortLikeCpp, WorldStateTemplatePersistenceRowLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, WorldDatabase, WorldStatements};

async fn load_world_then_character_rows_like_cpp(
    world_db: &WorldDatabase,
    character_db: &CharacterDatabase,
) -> Result<WorldStateStartupCatalogLikeCpp> {
    let mut templates = Vec::new();
    let mut result = world_db
        .query(&world_db.prepare(WorldStatements::SEL_WORLD_STATES))
        .await?;
    if !result.is_empty() {
        loop {
            templates.push(WorldStateTemplatePersistenceRowLikeCpp {
                id: result.read(0),
                default_value: result.read(1),
                map_ids_csv: result.try_read(2).unwrap_or_default(),
                area_ids_csv: result.try_read(3).unwrap_or_default(),
                script_name: result.try_read(4).unwrap_or_default(),
            });
            if !result.next_row() {
                break;
            }
        }
    }

    let mut saved_values = Vec::new();
    let mut result = character_db
        .query(&character_db.prepare(CharStatements::SEL_WORLD_STATE_VALUES))
        .await?;
    if !result.is_empty() {
        loop {
            saved_values.push(WorldStateSavedValuePersistenceRowLikeCpp {
                id: result.read(0),
                value: result.read(1),
            });
            if !result.next_row() {
                break;
            }
        }
    }

    Ok(WorldStateStartupCatalogLikeCpp {
        templates,
        saved_values,
    })
}

pub struct MariaDbWorldStateStartupPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbWorldStateStartupPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>, character_db: Arc<CharacterDatabase>) -> Self {
        Self {
            world_db,
            character_db,
        }
    }
}

impl WorldStateStartupPersistencePortLikeCpp for MariaDbWorldStateStartupPersistenceAdapterLikeCpp {
    fn load_world_then_character_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, WorldStateStartupLoadOutcomeLikeCpp> {
        Box::pin(async move {
            match load_world_then_character_rows_like_cpp(&self.world_db, &self.character_db).await
            {
                Ok(catalog) => WorldStateStartupLoadOutcomeLikeCpp::Loaded(catalog),
                Err(error) => WorldStateStartupLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn world_state_startup_keeps_world_then_characters_statement_order() {
        assert_eq!(
            [
                WorldStatements::SEL_WORLD_STATES.sql(),
                CharStatements::SEL_WORLD_STATE_VALUES.sql(),
            ],
            [
                "SELECT ID, DefaultValue, MapIDs, AreaIDs, ScriptName FROM world_state",
                "SELECT Id, Value FROM world_state_value",
            ]
        );
    }
}
