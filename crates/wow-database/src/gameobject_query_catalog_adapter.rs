//! MariaDB adapter for Rust's transitional on-demand gameobject query catalog.

use std::sync::Arc;

use wow_persistence::{
    GAMEOBJECT_USE_TEMPLATE_DATA_COUNT_LIKE_CPP, GameObjectQueryCatalogOutcomeLikeCpp,
    GameObjectQueryCatalogPersistencePortLikeCpp, GameObjectQueryCatalogRequestLikeCpp,
    GameObjectQueryCatalogRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{PreparedStatement, WorldDatabase, WorldStatements};

fn statement_like_cpp(statement: WorldStatements, entry: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(statement);
    statement.set_u32(0, entry);
    statement
}

pub struct MariaDbGameObjectQueryCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbGameObjectQueryCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl GameObjectQueryCatalogPersistencePortLikeCpp
    for MariaDbGameObjectQueryCatalogPersistenceAdapterLikeCpp
{
    fn load_gameobject_query_catalog_like_cpp<'a>(
        &'a self,
        request: GameObjectQueryCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GameObjectQueryCatalogOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match self
                .world_db
                .query(&statement_like_cpp(
                    WorldStatements::SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY,
                    request.entry,
                ))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return GameObjectQueryCatalogOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            if result.is_empty() {
                return GameObjectQueryCatalogOutcomeLikeCpp::Missing;
            }

            let mut data = [0_i32; GAMEOBJECT_USE_TEMPLATE_DATA_COUNT_LIKE_CPP];
            for (index, value) in data.iter_mut().enumerate() {
                *value = result.try_read(8 + index).unwrap_or(0);
            }
            let mut row = GameObjectQueryCatalogRowLikeCpp {
                go_type: result.try_read(1).unwrap_or(0),
                display_id: result.try_read(2).unwrap_or(0),
                name: result.read_string(3),
                icon_name: result.read_string(4),
                cast_bar_caption: result.read_string(5),
                unk_string: result.read_string(6),
                size: result.try_read(7).unwrap_or(1.0),
                data,
                content_tuning_id: result.try_read(43).unwrap_or(0),
                quest_items: Vec::new(),
            };

            let mut locale_error = None;
            if !request.locale.is_empty() && request.locale != "enUS" {
                let mut statement = statement_like_cpp(
                    WorldStatements::SEL_GAMEOBJECT_TEMPLATE_LOCALE,
                    request.entry,
                );
                statement.set_string(1, &request.locale);
                match self.world_db.query(&statement).await {
                    Ok(locale) if !locale.is_empty() => {
                        let name = locale.read_string(0);
                        let cast_bar_caption = locale.read_string(1);
                        let unk_string = locale.read_string(2);
                        if !name.is_empty() {
                            row.name = name;
                        }
                        if !cast_bar_caption.is_empty() {
                            row.cast_bar_caption = cast_bar_caption;
                        }
                        if !unk_string.is_empty() {
                            row.unk_string = unk_string;
                        }
                    }
                    Ok(_) => {}
                    Err(error) => locale_error = Some(error.to_string()),
                }
            }

            let mut quest_items_error = None;
            match self
                .world_db
                .query(&statement_like_cpp(
                    WorldStatements::SEL_GAMEOBJECT_QUEST_ITEMS,
                    request.entry,
                ))
                .await
            {
                Ok(mut quest_items) if !quest_items.is_empty() => loop {
                    let item_id = quest_items.try_read::<i32>(0).unwrap_or(0);
                    if item_id > 0 {
                        row.quest_items.push(item_id);
                    }
                    if !quest_items.next_row() {
                        break;
                    }
                },
                Ok(_) => {}
                Err(error) => quest_items_error = Some(error.to_string()),
            }

            GameObjectQueryCatalogOutcomeLikeCpp::Found {
                row,
                locale_error,
                quest_items_error,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn gameobject_query_statements_preserve_identity_and_entry_bind() {
        for identity in [
            WorldStatements::SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY,
            WorldStatements::SEL_GAMEOBJECT_QUEST_ITEMS,
        ] {
            let statement = statement_like_cpp(identity, 0xA1B2_C3D4);
            assert_eq!(statement.sql(), identity.sql());
            assert_eq!(statement.params(), [SqlParam::U32(0xA1B2_C3D4)]);
        }
    }
}
