//! MariaDB adapter for Player-owned quest state.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerQuestActivePersistenceRowLikeCpp,
    PlayerQuestDailyPersistenceRowLikeCpp, PlayerQuestIdPersistenceRowLikeCpp,
    PlayerQuestLoadOutcomeLikeCpp, PlayerQuestLockoutPersistenceRequestLikeCpp,
    PlayerQuestObjectivePersistenceRowLikeCpp, PlayerQuestPersistencePortLikeCpp,
    PlayerQuestSeasonalPersistenceRowLikeCpp, PlayerQuestStatusPersistenceRequestLikeCpp,
    QuestStatusPersistenceLikeCpp,
};

use crate::{
    CharStatements, CharacterDatabase, PreparedStatement, SqlResult, SqlTransaction,
    SqlTransactionCommitError,
};

const QUEST_STATUS_REWARDED_LIKE_CPP: u8 = 6;

pub(crate) fn player_quest_status_statements_like_cpp(
    owner_guid: u64,
    status: &QuestStatusPersistenceLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::new();
    if status.status == QUEST_STATUS_REWARDED_LIKE_CPP {
        let mut rewarded =
            PreparedStatement::for_statement(CharStatements::INS_CHAR_QUESTSTATUS_REWARDED);
        rewarded.set_u64(0, owner_guid);
        rewarded.set_u32(1, status.quest_id);
        statements.push(rewarded);

        let mut delete = PreparedStatement::for_statement(CharStatements::DEL_CHAR_QUEST_STATUS);
        delete.set_u64(0, owner_guid);
        delete.set_u32(1, status.quest_id);
        statements.push(delete);
    } else {
        let mut save = PreparedStatement::for_statement(CharStatements::INS_CHAR_QUEST_STATUS);
        save.set_u64(0, owner_guid);
        save.set_u32(1, status.quest_id);
        save.set_u8(2, status.status);
        save.set_u8(3, u8::from(status.explored));
        save.set_i64(4, status.accept_time_secs);
        save.set_i64(5, status.end_time_secs);
        statements.push(save);
    }

    let mut delete_objectives =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
    delete_objectives.set_u64(0, owner_guid);
    delete_objectives.set_u32(1, status.quest_id);
    statements.push(delete_objectives);

    if status.status != QUEST_STATUS_REWARDED_LIKE_CPP {
        for objective in &status.objectives {
            let mut replace =
                PreparedStatement::for_statement(CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES);
            replace.set_u64(0, owner_guid);
            replace.set_u32(1, status.quest_id);
            replace.set_u8(2, objective.objective_index);
            replace.set_i32(3, objective.count);
            statements.push(replace);
        }
    }
    statements
}

fn delete_quest_status_statements_like_cpp(
    owner_guid: u64,
    quest_id: u32,
) -> Vec<PreparedStatement> {
    let mut status = PreparedStatement::for_statement(CharStatements::DEL_CHAR_QUEST_STATUS);
    status.set_u64(0, owner_guid);
    status.set_u32(1, quest_id);

    let mut objectives =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
    objectives.set_u64(0, owner_guid);
    objectives.set_u32(1, quest_id);
    vec![status, objectives]
}

fn quest_lockout_statements_like_cpp(
    request: &PlayerQuestLockoutPersistenceRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::new();
    match request {
        PlayerQuestLockoutPersistenceRequestLikeCpp::Daily {
            owner_guid,
            completed_time,
            quest_ids,
        } => {
            let mut delete =
                PreparedStatement::for_statement(CharStatements::DEL_CHARACTER_QUESTSTATUS_DAILY);
            delete.set_u64(0, *owner_guid);
            statements.push(delete);
            for quest_id in quest_ids {
                let mut insert = PreparedStatement::for_statement(
                    CharStatements::INS_CHARACTER_QUESTSTATUS_DAILY,
                );
                insert.set_u64(0, *owner_guid);
                insert.set_u32(1, *quest_id);
                insert.set_i64(2, *completed_time);
                statements.push(insert);
            }
        }
        PlayerQuestLockoutPersistenceRequestLikeCpp::Weekly {
            owner_guid,
            quest_ids,
        } => {
            let mut delete =
                PreparedStatement::for_statement(CharStatements::DEL_CHARACTER_QUESTSTATUS_WEEKLY);
            delete.set_u64(0, *owner_guid);
            statements.push(delete);
            for quest_id in quest_ids {
                let mut insert = PreparedStatement::for_statement(
                    CharStatements::INS_CHARACTER_QUESTSTATUS_WEEKLY,
                );
                insert.set_u64(0, *owner_guid);
                insert.set_u32(1, *quest_id);
                statements.push(insert);
            }
        }
        PlayerQuestLockoutPersistenceRequestLikeCpp::Monthly {
            owner_guid,
            quest_ids,
        } => {
            let mut delete =
                PreparedStatement::for_statement(CharStatements::DEL_CHARACTER_QUESTSTATUS_MONTHLY);
            delete.set_u64(0, *owner_guid);
            statements.push(delete);
            for quest_id in quest_ids {
                let mut insert = PreparedStatement::for_statement(
                    CharStatements::INS_CHARACTER_QUESTSTATUS_MONTHLY,
                );
                insert.set_u64(0, *owner_guid);
                insert.set_u32(1, *quest_id);
                statements.push(insert);
            }
        }
        PlayerQuestLockoutPersistenceRequestLikeCpp::Seasonal {
            owner_guid,
            completions,
        } => {
            let mut delete = PreparedStatement::for_statement(
                CharStatements::DEL_CHARACTER_QUESTSTATUS_SEASONAL,
            );
            delete.set_u64(0, *owner_guid);
            statements.push(delete);
            for completion in completions {
                let mut insert = PreparedStatement::for_statement(
                    CharStatements::INS_CHARACTER_QUESTSTATUS_SEASONAL,
                );
                insert.set_u64(0, *owner_guid);
                insert.set_u32(1, completion.quest_id);
                insert.set_u32(2, u32::from(completion.event_id));
                insert.set_i64(3, completion.completed_time);
                statements.push(insert);
            }
        }
    }
    statements
}

fn owner_statement_like_cpp(statement: CharStatements, owner_guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(statement);
    statement.set_u64(0, owner_guid);
    statement
}

async fn load_rows_like_cpp<T>(
    character_db: &CharacterDatabase,
    statement: PreparedStatement,
    mut decode: impl FnMut(&SqlResult) -> T,
) -> PlayerQuestLoadOutcomeLikeCpp<T> {
    match character_db.query(&statement).await {
        Ok(mut result) => {
            let mut rows = Vec::new();
            if !result.is_empty() {
                loop {
                    rows.push(decode(&result));
                    if !result.next_row() {
                        break;
                    }
                }
            }
            PlayerQuestLoadOutcomeLikeCpp::Loaded(rows)
        }
        Err(error) => PlayerQuestLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

async fn commit_like_cpp(
    character_db: &CharacterDatabase,
    statements: Vec<PreparedStatement>,
) -> PersistenceOutcomeLikeCpp {
    let mut transaction = SqlTransaction::new();
    for statement in statements {
        transaction.append(statement);
    }
    match transaction
        .commit_with_outcome_like_cpp(character_db.pool())
        .await
    {
        Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows: 0 },
        Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
            PersistenceOutcomeLikeCpp::Failed {
                reason: error.to_string(),
            }
        }
        Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
            PersistenceOutcomeLikeCpp::Unknown {
                reason: error.to_string(),
            }
        }
    }
}

pub struct MariaDbPlayerQuestPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbPlayerQuestPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl PlayerQuestPersistencePortLikeCpp for MariaDbPlayerQuestPersistenceAdapterLikeCpp {
    fn load_active_statuses_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestActivePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            load_rows_like_cpp(
                self.character_db.as_ref(),
                owner_statement_like_cpp(CharStatements::SEL_CHAR_QUEST_STATUS, owner_guid),
                |row| PlayerQuestActivePersistenceRowLikeCpp {
                    quest_id: row.try_read(0),
                    status: row.try_read(1),
                    explored: row.try_read(2),
                    accept_time_secs: row.try_read(3),
                    end_time_secs: row.try_read(4),
                },
            )
            .await
        })
    }

    fn load_objectives_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestObjectivePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            load_rows_like_cpp(
                self.character_db.as_ref(),
                owner_statement_like_cpp(
                    CharStatements::SEL_CHAR_QUEST_STATUS_OBJECTIVES,
                    owner_guid,
                ),
                |row| PlayerQuestObjectivePersistenceRowLikeCpp {
                    quest_id: row.try_read(0),
                    storage_index: row.try_read(1),
                    count: row.try_read(2),
                },
            )
            .await
        })
    }

    fn load_rewarded_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestIdPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            load_rows_like_cpp(
                self.character_db.as_ref(),
                owner_statement_like_cpp(CharStatements::SEL_CHARACTER_QUESTSTATUSREW, owner_guid),
                |row| PlayerQuestIdPersistenceRowLikeCpp {
                    quest_id: row.try_read(0),
                },
            )
            .await
        })
    }

    fn load_daily_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestDailyPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            load_rows_like_cpp(
                self.character_db.as_ref(),
                owner_statement_like_cpp(
                    CharStatements::SEL_CHARACTER_QUESTSTATUS_DAILY,
                    owner_guid,
                ),
                |row| PlayerQuestDailyPersistenceRowLikeCpp {
                    quest_id: row.try_read(0),
                    completed_time: row.try_read(1),
                },
            )
            .await
        })
    }

    fn load_weekly_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestIdPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            load_rows_like_cpp(
                self.character_db.as_ref(),
                owner_statement_like_cpp(
                    CharStatements::SEL_CHARACTER_QUESTSTATUS_WEEKLY,
                    owner_guid,
                ),
                |row| PlayerQuestIdPersistenceRowLikeCpp {
                    quest_id: row.try_read(0),
                },
            )
            .await
        })
    }

    fn load_monthly_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestIdPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            load_rows_like_cpp(
                self.character_db.as_ref(),
                owner_statement_like_cpp(
                    CharStatements::SEL_CHARACTER_QUESTSTATUS_MONTHLY,
                    owner_guid,
                ),
                |row| PlayerQuestIdPersistenceRowLikeCpp {
                    quest_id: row.try_read(0),
                },
            )
            .await
        })
    }

    fn load_seasonal_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestSeasonalPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            load_rows_like_cpp(
                self.character_db.as_ref(),
                owner_statement_like_cpp(
                    CharStatements::SEL_CHAR_QUEST_STATUS_SEASONAL,
                    owner_guid,
                ),
                |row| PlayerQuestSeasonalPersistenceRowLikeCpp {
                    quest_id: row.try_read(0),
                    event_id: row.try_read(1),
                    completed_time: row.try_read(2),
                },
            )
            .await
        })
    }

    fn persist_status_like_cpp(
        &self,
        request: PlayerQuestStatusPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statements = match request {
                PlayerQuestStatusPersistenceRequestLikeCpp::Save { owner_guid, status } => {
                    player_quest_status_statements_like_cpp(owner_guid, &status)
                }
                PlayerQuestStatusPersistenceRequestLikeCpp::Delete {
                    owner_guid,
                    quest_id,
                } => delete_quest_status_statements_like_cpp(owner_guid, quest_id),
            };
            commit_like_cpp(self.character_db.as_ref(), statements).await
        })
    }

    fn persist_lockout_like_cpp(
        &self,
        request: PlayerQuestLockoutPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            commit_like_cpp(
                self.character_db.as_ref(),
                quest_lockout_statements_like_cpp(&request),
            )
            .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};
    use wow_persistence::{
        PlayerQuestSeasonalCompletionPersistenceLikeCpp, QuestObjectiveCountPersistenceLikeCpp,
    };

    #[test]
    fn quest_status_preserves_status_objectives_and_rewarded_migration_order_like_cpp() {
        let active = QuestStatusPersistenceLikeCpp {
            quest_id: 77,
            status: 3,
            explored: true,
            accept_time_secs: 12,
            end_time_secs: 34,
            objectives: vec![QuestObjectiveCountPersistenceLikeCpp {
                objective_index: 2,
                count: 5,
            }],
        };
        let active = player_quest_status_statements_like_cpp(44, &active);
        assert_eq!(
            active
                .iter()
                .map(PreparedStatement::sql)
                .collect::<Vec<_>>(),
            vec![
                CharStatements::INS_CHAR_QUEST_STATUS.sql(),
                CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql(),
                CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES.sql(),
            ]
        );
        assert_eq!(active[0].params()[0], SqlParam::U64(44));

        let rewarded = QuestStatusPersistenceLikeCpp {
            status: QUEST_STATUS_REWARDED_LIKE_CPP,
            objectives: Vec::new(),
            ..active_status(78)
        };
        assert_eq!(
            player_quest_status_statements_like_cpp(44, &rewarded)
                .iter()
                .map(PreparedStatement::sql)
                .collect::<Vec<_>>(),
            vec![
                CharStatements::INS_CHAR_QUESTSTATUS_REWARDED.sql(),
                CharStatements::DEL_CHAR_QUEST_STATUS.sql(),
                CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql(),
            ]
        );
    }

    fn active_status(quest_id: u32) -> QuestStatusPersistenceLikeCpp {
        QuestStatusPersistenceLikeCpp {
            quest_id,
            status: 3,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objectives: Vec::new(),
        }
    }

    #[test]
    fn quest_lockouts_preserve_delete_then_projection_order_like_cpp() {
        let daily = quest_lockout_statements_like_cpp(
            &PlayerQuestLockoutPersistenceRequestLikeCpp::Daily {
                owner_guid: 44,
                completed_time: 99,
                quest_ids: vec![7, 9],
            },
        );
        assert_eq!(
            daily.iter().map(PreparedStatement::sql).collect::<Vec<_>>(),
            vec![
                CharStatements::DEL_CHARACTER_QUESTSTATUS_DAILY.sql(),
                CharStatements::INS_CHARACTER_QUESTSTATUS_DAILY.sql(),
                CharStatements::INS_CHARACTER_QUESTSTATUS_DAILY.sql(),
            ]
        );
        assert_eq!(
            daily[1].params(),
            &[SqlParam::U64(44), SqlParam::U32(7), SqlParam::I64(99)]
        );

        let seasonal = quest_lockout_statements_like_cpp(
            &PlayerQuestLockoutPersistenceRequestLikeCpp::Seasonal {
                owner_guid: 44,
                completions: vec![PlayerQuestSeasonalCompletionPersistenceLikeCpp {
                    quest_id: 10,
                    event_id: 3,
                    completed_time: 100,
                }],
            },
        );
        assert_eq!(
            seasonal
                .iter()
                .map(PreparedStatement::sql)
                .collect::<Vec<_>>(),
            vec![
                CharStatements::DEL_CHARACTER_QUESTSTATUS_SEASONAL.sql(),
                CharStatements::INS_CHARACTER_QUESTSTATUS_SEASONAL.sql(),
            ]
        );
        assert_eq!(
            seasonal[1].params(),
            &[
                SqlParam::U64(44),
                SqlParam::U32(10),
                SqlParam::U32(3),
                SqlParam::I64(100),
            ]
        );
    }
}
