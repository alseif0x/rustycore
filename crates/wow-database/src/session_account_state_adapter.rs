//! MariaDB adapter for session-owned account data and tutorial state.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, SessionAccountDataLoadOutcomeLikeCpp,
    SessionAccountDataRowLikeCpp, SessionAccountDataSaveLikeCpp, SessionAccountDataScopeLikeCpp,
    SessionAccountStatePortLikeCpp, SessionTutorialsLoadOutcomeLikeCpp,
};

use crate::CharacterDatabase;
use crate::params::PreparedStatement;
use crate::statements::CharStatements;

fn account_data_load_statement_like_cpp(
    scope: SessionAccountDataScopeLikeCpp,
) -> PreparedStatement {
    match scope {
        SessionAccountDataScopeLikeCpp::Global { account_id } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::SEL_ACCOUNT_DATA);
            stmt.set_u32(0, account_id);
            stmt
        }
        SessionAccountDataScopeLikeCpp::Character { guid_low } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::SEL_PLAYER_ACCOUNT_DATA);
            stmt.set_u64(0, guid_low);
            stmt
        }
    }
}

fn tutorials_load_statement_like_cpp(account_id: u32) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::SEL_TUTORIALS);
    stmt.set_u32(0, account_id);
    stmt
}

fn account_data_save_statement_like_cpp(save: &SessionAccountDataSaveLikeCpp) -> PreparedStatement {
    let mut stmt = match save.scope {
        SessionAccountDataScopeLikeCpp::Global { account_id } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::REP_ACCOUNT_DATA);
            stmt.set_u32(0, account_id);
            stmt
        }
        SessionAccountDataScopeLikeCpp::Character { guid_low } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::REP_PLAYER_ACCOUNT_DATA);
            stmt.set_u64(0, guid_low);
            stmt
        }
    };
    stmt.set_u8(1, save.data_type);
    stmt.set_i64(2, save.time);
    stmt.set_string(3, save.data.clone());
    stmt
}

pub struct MariaDbSessionAccountStateAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbSessionAccountStateAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl SessionAccountStatePortLikeCpp for MariaDbSessionAccountStateAdapterLikeCpp {
    fn load_account_data_like_cpp<'a>(
        &'a self,
        scope: SessionAccountDataScopeLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SessionAccountDataLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let stmt = account_data_load_statement_like_cpp(scope);
            match self.character_db.query(&stmt).await {
                Ok(mut result) => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(SessionAccountDataRowLikeCpp {
                                data_type: result.try_read::<u8>(0).unwrap_or(u8::MAX),
                                time: result.try_read::<i64>(1).unwrap_or(0),
                                data: result.read_string(2),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    SessionAccountDataLoadOutcomeLikeCpp::Loaded(rows)
                }
                Err(error) => SessionAccountDataLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_tutorials_like_cpp<'a>(
        &'a self,
        account_id: u32,
    ) -> PersistenceFutureLikeCpp<'a, SessionTutorialsLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let stmt = tutorials_load_statement_like_cpp(account_id);
            match self.character_db.query(&stmt).await {
                Ok(result) if result.is_empty() => SessionTutorialsLoadOutcomeLikeCpp::Loaded(None),
                Ok(result) => {
                    let mut tutorials = [0u32; 8];
                    for (index, value) in tutorials.iter_mut().enumerate() {
                        *value = result.try_read::<u32>(index).unwrap_or(0);
                    }
                    SessionTutorialsLoadOutcomeLikeCpp::Loaded(Some(tutorials))
                }
                Err(error) => SessionTutorialsLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn save_account_data_like_cpp<'a>(
        &'a self,
        save: SessionAccountDataSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let stmt = account_data_save_statement_like_cpp(&save);
            match self.character_db.execute(&stmt).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::SqlParam;
    use crate::statements::StatementDef;

    #[test]
    fn load_statements_preserve_scope_identity_and_bind_order_like_cpp() {
        let global = account_data_load_statement_like_cpp(SessionAccountDataScopeLikeCpp::Global {
            account_id: 17,
        });
        assert_eq!(global.sql(), CharStatements::SEL_ACCOUNT_DATA.sql());
        assert_eq!(global.params(), [SqlParam::U32(17)]);

        let character =
            account_data_load_statement_like_cpp(SessionAccountDataScopeLikeCpp::Character {
                guid_low: 29,
            });
        assert_eq!(
            character.sql(),
            CharStatements::SEL_PLAYER_ACCOUNT_DATA.sql()
        );
        assert_eq!(character.params(), [SqlParam::U64(29)]);

        let tutorials = tutorials_load_statement_like_cpp(31);
        assert_eq!(tutorials.sql(), CharStatements::SEL_TUTORIALS.sql());
        assert_eq!(tutorials.params(), [SqlParam::U32(31)]);
    }

    #[test]
    fn save_statements_preserve_scope_identity_and_bind_order_like_cpp() {
        let global = account_data_save_statement_like_cpp(&SessionAccountDataSaveLikeCpp {
            scope: SessionAccountDataScopeLikeCpp::Global { account_id: 17 },
            data_type: 2,
            time: 41,
            data: "global".to_owned(),
        });
        assert_eq!(global.sql(), CharStatements::REP_ACCOUNT_DATA.sql());
        assert_eq!(
            global.params(),
            [
                SqlParam::U32(17),
                SqlParam::U8(2),
                SqlParam::I64(41),
                SqlParam::String("global".to_owned()),
            ]
        );

        let character = account_data_save_statement_like_cpp(&SessionAccountDataSaveLikeCpp {
            scope: SessionAccountDataScopeLikeCpp::Character { guid_low: 29 },
            data_type: 3,
            time: 43,
            data: "character".to_owned(),
        });
        assert_eq!(
            character.sql(),
            CharStatements::REP_PLAYER_ACCOUNT_DATA.sql()
        );
        assert_eq!(character.params()[0], SqlParam::U64(29));
    }
}
