//! MariaDB adapter for C++ trainer startup catalogs.

use std::sync::Arc;

use wow_persistence::{
    CreatureTrainerPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
    TrainerCatalogLoadOutcomeLikeCpp, TrainerCatalogPersistencePortLikeCpp,
    TrainerCatalogPersistenceRowsLikeCpp, TrainerLocalePersistenceRowLikeCpp,
    TrainerPersistenceRowLikeCpp, TrainerSpellPersistenceRowLikeCpp,
};

use crate::{DatabaseError, SqlResult, WorldDatabase, WorldStatements};

const TRAINER_CATALOG_STATEMENTS_LIKE_CPP: [WorldStatements; 4] = [
    WorldStatements::SEL_TRAINER_SPELLS_ALL,
    WorldStatements::SEL_TRAINERS_ALL,
    WorldStatements::SEL_TRAINER_LOCALES,
    WorldStatements::SEL_CREATURE_TRAINERS_ALL,
];

async fn query_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> T,
) -> Result<Vec<T>, DatabaseError> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::new();
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(decode(&result));
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn trainer_spell_row_like_cpp(
    values: (u32, u32, u32, u32, u32, [u32; 3], u8),
) -> TrainerSpellPersistenceRowLikeCpp {
    TrainerSpellPersistenceRowLikeCpp {
        trainer_id: values.0,
        spell_id: values.1,
        money_cost: values.2,
        req_skill_line: values.3,
        req_skill_rank: values.4,
        req_ability: values.5,
        req_level: values.6,
    }
}

fn trainer_row_like_cpp(values: (u32, u8, String)) -> TrainerPersistenceRowLikeCpp {
    TrainerPersistenceRowLikeCpp {
        id: values.0,
        trainer_type: values.1,
        greeting: values.2,
    }
}

fn trainer_locale_row_like_cpp(
    values: (u32, String, String),
) -> TrainerLocalePersistenceRowLikeCpp {
    TrainerLocalePersistenceRowLikeCpp {
        id: values.0,
        locale: values.1,
        greeting: values.2,
    }
}

fn creature_trainer_row_like_cpp(
    values: (u32, u32, u32, u32),
) -> CreatureTrainerPersistenceRowLikeCpp {
    CreatureTrainerPersistenceRowLikeCpp {
        creature_id: values.0,
        trainer_id: values.1,
        menu_id: values.2,
        option_id: values.3,
    }
}

pub struct MariaDbTrainerCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbTrainerCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl TrainerCatalogPersistencePortLikeCpp for MariaDbTrainerCatalogPersistenceAdapterLikeCpp {
    fn load_trainer_catalog_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, TrainerCatalogLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = async {
                let trainer_spells = query_rows_like_cpp(
                    &self.world_db,
                    TRAINER_CATALOG_STATEMENTS_LIKE_CPP[0],
                    |row| {
                        trainer_spell_row_like_cpp((
                            row.read(0),
                            row.read(1),
                            row.read(2),
                            row.read(3),
                            row.read(4),
                            [row.read(5), row.read(6), row.read(7)],
                            row.read(8),
                        ))
                    },
                )
                .await?;
                let trainers = query_rows_like_cpp(
                    &self.world_db,
                    TRAINER_CATALOG_STATEMENTS_LIKE_CPP[1],
                    |row| trainer_row_like_cpp((row.read(0), row.read(1), row.read_string(2))),
                )
                .await?;
                let trainer_locales = query_rows_like_cpp(
                    &self.world_db,
                    TRAINER_CATALOG_STATEMENTS_LIKE_CPP[2],
                    |row| {
                        trainer_locale_row_like_cpp((
                            row.read(0),
                            row.read_string(1),
                            row.read_string(2),
                        ))
                    },
                )
                .await?;
                let creature_trainers = query_rows_like_cpp(
                    &self.world_db,
                    TRAINER_CATALOG_STATEMENTS_LIKE_CPP[3],
                    |row| {
                        creature_trainer_row_like_cpp((
                            row.read(0),
                            row.read(1),
                            row.read(2),
                            row.read(3),
                        ))
                    },
                )
                .await?;
                Ok::<_, DatabaseError>(TrainerCatalogPersistenceRowsLikeCpp {
                    trainer_spells,
                    trainers,
                    trainer_locales,
                    creature_trainers,
                })
            }
            .await;

            match result {
                Ok(rows) => TrainerCatalogLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => TrainerCatalogLoadOutcomeLikeCpp::Failed {
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
    fn trainer_statement_order_and_sql_match_cpp() {
        assert_eq!(
            TRAINER_CATALOG_STATEMENTS_LIKE_CPP.map(WorldStatements::sql),
            [
                "SELECT TrainerId, SpellId, MoneyCost, ReqSkillLine, ReqSkillRank, ReqAbility1, ReqAbility2, ReqAbility3, ReqLevel FROM trainer_spell",
                "SELECT Id, Type, Greeting FROM trainer",
                "SELECT Id, locale, Greeting_lang FROM trainer_locale",
                "SELECT CreatureID, TrainerID, MenuID, OptionID FROM creature_trainer",
            ]
        );
    }

    #[test]
    fn every_trainer_boundary_row_preserves_fields() {
        assert_eq!(
            trainer_spell_row_like_cpp((1, 2, 3, 4, 5, [6, 7, 8], 9)),
            TrainerSpellPersistenceRowLikeCpp {
                trainer_id: 1,
                spell_id: 2,
                money_cost: 3,
                req_skill_line: 4,
                req_skill_rank: 5,
                req_ability: [6, 7, 8],
                req_level: 9,
            }
        );
        assert_eq!(
            trainer_row_like_cpp((10, 2, "hello".into())),
            TrainerPersistenceRowLikeCpp {
                id: 10,
                trainer_type: 2,
                greeting: "hello".into(),
            }
        );
        assert_eq!(
            trainer_locale_row_like_cpp((11, "frFR".into(), "bonjour".into())),
            TrainerLocalePersistenceRowLikeCpp {
                id: 11,
                locale: "frFR".into(),
                greeting: "bonjour".into(),
            }
        );
        assert_eq!(
            creature_trainer_row_like_cpp((12, 13, 14, 15)),
            CreatureTrainerPersistenceRowLikeCpp {
                creature_id: 12,
                trainer_id: 13,
                menu_id: 14,
                option_id: 15,
            }
        );
    }
}
