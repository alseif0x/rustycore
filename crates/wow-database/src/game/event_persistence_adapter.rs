//! MariaDB adapter for represented game-event durability.

use std::sync::Arc;

use wow_persistence::{
    GameEventConditionSaveLoadOutcomeLikeCpp, GameEventConditionSavePersistenceRowLikeCpp,
    GameEventPersistenceMutationLikeCpp, GameEventPersistenceMutationOutcomeLikeCpp,
    GameEventPersistencePortLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction};

fn condition_saves_statement_like_cpp() -> PreparedStatement {
    PreparedStatement::for_statement(CharStatements::SEL_GAME_EVENT_CONDITION_SAVES)
}

fn read_unsigned_u32_like_cpp(
    result: &crate::SqlResult,
    column: usize,
    field: &str,
) -> Result<u32, String> {
    if result.is_null(column) {
        return Ok(0);
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(value);
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return u32::try_from(value).map_err(|_| format!("{field} value {value} exceeds u32"));
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return Ok(u32::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return Ok(u32::from(value));
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return u32::try_from(value).map_err(|_| format!("{field} value {value} is outside u32"));
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return u32::try_from(value).map_err(|_| format!("{field} value {value} is outside u32"));
    }
    Err(format!("could not decode {field} as an unsigned DB field"))
}

fn decode_condition_save_rows_like_cpp(
    mut result: crate::SqlResult,
) -> Result<Vec<GameEventConditionSavePersistenceRowLikeCpp>, String> {
    let mut rows = Vec::new();
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        let raw_event_id =
            read_unsigned_u32_like_cpp(&result, 0, "game_event_condition_save.eventEntry")?;
        let event_id = u8::try_from(raw_event_id).map_err(|_| {
            format!("game_event_condition_save.eventEntry value {raw_event_id} exceeds u8")
        })?;
        let condition_id =
            read_unsigned_u32_like_cpp(&result, 1, "game_event_condition_save.condition_id")?;
        let done = if result.is_null(2) {
            0.0
        } else {
            result
                .try_read::<f32>(2)
                .or_else(|| result.try_read::<f64>(2).map(|value| value as f32))
                .ok_or_else(|| {
                    "could not decode game_event_condition_save.done as a float DB field"
                        .to_string()
                })?
        };
        rows.push(GameEventConditionSavePersistenceRowLikeCpp {
            event_id,
            condition_id,
            done,
        });
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn mutation_statements_like_cpp(
    mutation: &GameEventPersistenceMutationLikeCpp,
) -> (Vec<PreparedStatement>, bool) {
    match *mutation {
        GameEventPersistenceMutationLikeCpp::ReplaceConditionSave {
            event_id,
            condition_id,
            done,
        } => {
            let mut delete =
                PreparedStatement::for_statement(CharStatements::DEL_GAME_EVENT_CONDITION_SAVE);
            delete.set_u8(0, event_id);
            delete.set_u32(1, condition_id);
            let mut insert =
                PreparedStatement::for_statement(CharStatements::INS_GAME_EVENT_CONDITION_SAVE);
            insert.set_u8(0, event_id);
            insert.set_u32(1, condition_id);
            insert.set_f32(2, done);
            (vec![delete, insert], true)
        }
        GameEventPersistenceMutationLikeCpp::SaveWorldEventState {
            event_id,
            state,
            next_start,
        } => {
            let mut delete = PreparedStatement::for_statement(CharStatements::DEL_GAME_EVENT_SAVE);
            delete.set_u8(0, event_id);
            let mut insert = PreparedStatement::for_statement(CharStatements::INS_GAME_EVENT_SAVE);
            insert.set_u8(0, event_id);
            insert.set_u8(1, state);
            insert.set_i64(2, next_start);
            (vec![delete, insert], true)
        }
        GameEventPersistenceMutationLikeCpp::DeleteWorldEventState {
            event_id,
            delete_condition_saves,
            delete_world_event_state,
        } => {
            let mut statements = Vec::new();
            if delete_condition_saves {
                let mut statement = PreparedStatement::for_statement(
                    CharStatements::DEL_ALL_GAME_EVENT_CONDITION_SAVE,
                );
                statement.set_u8(0, event_id);
                statements.push(statement);
            }
            if delete_world_event_state {
                let mut statement =
                    PreparedStatement::for_statement(CharStatements::DEL_GAME_EVENT_SAVE);
                statement.set_u8(0, event_id);
                statements.push(statement);
            }
            (statements, true)
        }
        GameEventPersistenceMutationLikeCpp::ResetSeasonalQuests {
            event_id,
            event_start_time,
        } => {
            let mut statement = PreparedStatement::for_statement(
                CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT,
            );
            statement.set_u16(0, event_id);
            statement.set_i64(1, event_start_time);
            (vec![statement], false)
        }
    }
}

pub struct MariaDbGameEventPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbGameEventPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl GameEventPersistencePortLikeCpp for MariaDbGameEventPersistenceAdapterLikeCpp {
    fn load_condition_saves_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, GameEventConditionSaveLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match self
                .character_db
                .query(&condition_saves_statement_like_cpp())
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return GameEventConditionSaveLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            match decode_condition_save_rows_like_cpp(result) {
                Ok(rows) => GameEventConditionSaveLoadOutcomeLikeCpp::Loaded(rows),
                Err(reason) => GameEventConditionSaveLoadOutcomeLikeCpp::Failed { reason },
            }
        })
    }

    fn execute_mutation_like_cpp<'a>(
        &'a self,
        mutation: GameEventPersistenceMutationLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, GameEventPersistenceMutationOutcomeLikeCpp> {
        Box::pin(async move {
            let (statements, transactional) = mutation_statements_like_cpp(&mutation);
            if statements.is_empty() {
                return GameEventPersistenceMutationOutcomeLikeCpp::Failed {
                    reason: "game-event mutation selected no durable rows".to_string(),
                };
            }
            let result = if transactional {
                let mut transaction = SqlTransaction::new();
                for statement in statements {
                    transaction.append(statement);
                }
                transaction
                    .commit(self.character_db.pool())
                    .await
                    .map(|_| ())
            } else {
                self.character_db.execute(&statements[0]).await.map(|_| ())
            };
            match result {
                Ok(()) => GameEventPersistenceMutationOutcomeLikeCpp::Applied,
                Err(error) => GameEventPersistenceMutationOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn game_event_mutations_preserve_cpp_statement_identity_order_and_binds() {
        let (statements, transactional) = mutation_statements_like_cpp(
            &GameEventPersistenceMutationLikeCpp::ReplaceConditionSave {
                event_id: 7,
                condition_id: 44,
                done: 5.25,
            },
        );
        assert!(transactional);
        assert_eq!(statements.len(), 2);
        assert_eq!(
            statements[0].sql(),
            CharStatements::DEL_GAME_EVENT_CONDITION_SAVE.sql()
        );
        assert_eq!(statements[0].params(), [SqlParam::U8(7), SqlParam::U32(44)]);
        assert_eq!(
            statements[1].sql(),
            CharStatements::INS_GAME_EVENT_CONDITION_SAVE.sql()
        );
        assert_eq!(
            statements[1].params(),
            [SqlParam::U8(7), SqlParam::U32(44), SqlParam::F32(5.25)]
        );

        let (statements, transactional) = mutation_statements_like_cpp(
            &GameEventPersistenceMutationLikeCpp::SaveWorldEventState {
                event_id: 2,
                state: 4,
                next_start: 20,
            },
        );
        assert!(transactional);
        assert_eq!(
            statements[0].sql(),
            CharStatements::DEL_GAME_EVENT_SAVE.sql()
        );
        assert_eq!(
            statements[1].sql(),
            CharStatements::INS_GAME_EVENT_SAVE.sql()
        );
        assert_eq!(
            statements[1].params(),
            [SqlParam::U8(2), SqlParam::U8(4), SqlParam::I64(20)]
        );

        let (statements, transactional) = mutation_statements_like_cpp(
            &GameEventPersistenceMutationLikeCpp::DeleteWorldEventState {
                event_id: 3,
                delete_condition_saves: true,
                delete_world_event_state: true,
            },
        );
        assert!(transactional);
        assert_eq!(
            statements[0].sql(),
            CharStatements::DEL_ALL_GAME_EVENT_CONDITION_SAVE.sql()
        );
        assert_eq!(
            statements[1].sql(),
            CharStatements::DEL_GAME_EVENT_SAVE.sql()
        );

        let (statements, transactional) = mutation_statements_like_cpp(
            &GameEventPersistenceMutationLikeCpp::ResetSeasonalQuests {
                event_id: 9,
                event_start_time: 1234,
            },
        );
        assert!(!transactional);
        assert_eq!(
            statements[0].sql(),
            CharStatements::DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT.sql()
        );
        assert_eq!(
            statements[0].params(),
            [SqlParam::U16(9), SqlParam::I64(1234)]
        );
        assert_eq!(
            condition_saves_statement_like_cpp().sql(),
            CharStatements::SEL_GAME_EVENT_CONDITION_SAVES.sql()
        );

        let (statements, transactional) = mutation_statements_like_cpp(
            &GameEventPersistenceMutationLikeCpp::DeleteWorldEventState {
                event_id: 3,
                delete_condition_saves: false,
                delete_world_event_state: false,
            },
        );
        assert!(transactional);
        assert!(statements.is_empty());
    }
}
