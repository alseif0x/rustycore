//! MariaDB adapter for the represented next-mail-time read.

use std::sync::Arc;

use wow_persistence::{
    NextMailTimeLoadOutcomeLikeCpp, NextMailTimeLoadRequestLikeCpp, NextMailTimeLoadRowLikeCpp,
    NextMailTimePersistencePortLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, PreparedStatement};

const MESSAGE_TYPE_COLUMN_LIKE_CPP: usize = 1;
const SENDER_COLUMN_LIKE_CPP: usize = 2;
const DELIVER_TIME_COLUMN_LIKE_CPP: usize = 7;
const CHECKED_COLUMN_LIKE_CPP: usize = 10;
const STATIONERY_COLUMN_LIKE_CPP: usize = 11;

fn next_mail_time_load_statement_like_cpp(
    request: NextMailTimeLoadRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::SEL_MAIL);
    statement.set_u64(0, request.player_guid);
    statement
}

pub struct MariaDbNextMailTimePersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbNextMailTimePersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl NextMailTimePersistencePortLikeCpp for MariaDbNextMailTimePersistenceAdapterLikeCpp {
    fn load_next_mail_time_rows_like_cpp<'a>(
        &'a self,
        request: NextMailTimeLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, NextMailTimeLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = next_mail_time_load_statement_like_cpp(request);
            let mut result = match self.character_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return NextMailTimeLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            let mut rows = Vec::new();
            if !result.is_empty() {
                loop {
                    // Preserve the represented handler's tolerant decode: a
                    // malformed column becomes the same zero value it used
                    // before this structural cut.
                    rows.push(NextMailTimeLoadRowLikeCpp {
                        message_type: result
                            .try_read::<u8>(MESSAGE_TYPE_COLUMN_LIKE_CPP)
                            .unwrap_or(0),
                        sender: result.try_read::<u64>(SENDER_COLUMN_LIKE_CPP).unwrap_or(0),
                        deliver_time: result
                            .try_read::<i64>(DELIVER_TIME_COLUMN_LIKE_CPP)
                            .unwrap_or(0),
                        checked: result.try_read::<u8>(CHECKED_COLUMN_LIKE_CPP).unwrap_or(0),
                        stationery: result
                            .try_read::<i32>(STATIONERY_COLUMN_LIKE_CPP)
                            .unwrap_or(0),
                    });
                    if !result.next_row() {
                        break;
                    }
                }
            }

            NextMailTimeLoadOutcomeLikeCpp::Loaded(rows)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn next_mail_time_statement_preserves_cpp_identity_bind_and_projection() {
        let statement = next_mail_time_load_statement_like_cpp(NextMailTimeLoadRequestLikeCpp {
            player_guid: 0x0102_0304_0506_0708,
        });

        assert_eq!(statement.sql(), CharStatements::SEL_MAIL.sql());
        assert_eq!(statement.params(), [SqlParam::U64(0x0102_0304_0506_0708)]);
        assert_eq!(
            [
                MESSAGE_TYPE_COLUMN_LIKE_CPP,
                SENDER_COLUMN_LIKE_CPP,
                DELIVER_TIME_COLUMN_LIKE_CPP,
                CHECKED_COLUMN_LIKE_CPP,
                STATIONERY_COLUMN_LIKE_CPP,
            ],
            [1, 2, 7, 10, 11]
        );
    }
}
