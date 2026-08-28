//! MariaDB adapter for the represented gameobject-use template read.

use std::sync::Arc;

use wow_persistence::{
    GAMEOBJECT_USE_TEMPLATE_DATA_COUNT_LIKE_CPP, GameObjectUseTemplateLoadOutcomeLikeCpp,
    GameObjectUseTemplateLoadRequestLikeCpp, GameObjectUseTemplateLoadRowLikeCpp,
    GameObjectUseTemplatePersistencePortLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{PreparedStatement, WorldDatabase, WorldStatements};

const TYPE_COLUMN_LIKE_CPP: usize = 1;
const ICON_NAME_COLUMN_LIKE_CPP: usize = 4;
const SIZE_COLUMN_LIKE_CPP: usize = 7;
const DATA_START_COLUMN_LIKE_CPP: usize = 8;
const CONTENT_TUNING_ID_COLUMN_LIKE_CPP: usize = 43;

fn gameobject_use_template_statement_like_cpp(
    request: GameObjectUseTemplateLoadRequestLikeCpp,
) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(WorldStatements::SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY);
    statement.set_u32(0, request.entry);
    statement
}

pub struct MariaDbGameObjectUseTemplatePersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbGameObjectUseTemplatePersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl GameObjectUseTemplatePersistencePortLikeCpp
    for MariaDbGameObjectUseTemplatePersistenceAdapterLikeCpp
{
    fn load_gameobject_use_template_like_cpp<'a>(
        &'a self,
        request: GameObjectUseTemplateLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GameObjectUseTemplateLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = gameobject_use_template_statement_like_cpp(request);
            let result = match self.world_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return GameObjectUseTemplateLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            if result.is_empty() {
                return GameObjectUseTemplateLoadOutcomeLikeCpp::Missing;
            }

            let mut data = [0_u32; GAMEOBJECT_USE_TEMPLATE_DATA_COUNT_LIKE_CPP];
            for (index, value) in data.iter_mut().enumerate() {
                *value = result
                    .try_read::<i32>(DATA_START_COLUMN_LIKE_CPP + index)
                    .and_then(|raw| u32::try_from(raw).ok())
                    .unwrap_or(0);
            }

            GameObjectUseTemplateLoadOutcomeLikeCpp::Found(GameObjectUseTemplateLoadRowLikeCpp {
                go_type: result.try_read::<u32>(TYPE_COLUMN_LIKE_CPP).unwrap_or(0),
                icon_name: result.read_string(ICON_NAME_COLUMN_LIKE_CPP),
                size: result.try_read::<f32>(SIZE_COLUMN_LIKE_CPP).unwrap_or(1.0),
                data,
                content_tuning_id: result
                    .try_read::<u32>(CONTENT_TUNING_ID_COLUMN_LIKE_CPP)
                    .unwrap_or(0),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn gameobject_use_template_statement_preserves_identity_bind_and_projection() {
        let statement =
            gameobject_use_template_statement_like_cpp(GameObjectUseTemplateLoadRequestLikeCpp {
                entry: 0xA1B2_C3D4,
            });

        assert_eq!(
            statement.sql(),
            WorldStatements::SEL_GAMEOBJECT_TEMPLATE_BY_ENTRY.sql()
        );
        assert_eq!(statement.params(), [SqlParam::U32(0xA1B2_C3D4)]);
        assert_eq!(TYPE_COLUMN_LIKE_CPP, 1);
        assert_eq!(ICON_NAME_COLUMN_LIKE_CPP, 4);
        assert_eq!(SIZE_COLUMN_LIKE_CPP, 7);
        assert_eq!(DATA_START_COLUMN_LIKE_CPP, 8);
        assert_eq!(GAMEOBJECT_USE_TEMPLATE_DATA_COUNT_LIKE_CPP, 35);
        assert_eq!(
            DATA_START_COLUMN_LIKE_CPP + GAMEOBJECT_USE_TEMPLATE_DATA_COUNT_LIKE_CPP - 1,
            42
        );
        assert_eq!(CONTENT_TUNING_ID_COLUMN_LIKE_CPP, 43);
    }
}
