//! MariaDB adapter for Rust's transitional on-demand player-name query.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerNameQueryOutcomeLikeCpp, PlayerNameQueryPersistencePortLikeCpp,
    PlayerNameQueryRequestLikeCpp, PlayerNameQueryRowLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, PreparedStatement};

fn statement_like_cpp(player_guid_counter: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::SEL_CHARACTER);
    statement.set_u64(0, player_guid_counter);
    statement
}

pub struct MariaDbPlayerNameQueryPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbPlayerNameQueryPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl PlayerNameQueryPersistencePortLikeCpp for MariaDbPlayerNameQueryPersistenceAdapterLikeCpp {
    fn load_player_name_like_cpp<'a>(
        &'a self,
        request: PlayerNameQueryRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerNameQueryOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match self
                .character_db
                .query(&statement_like_cpp(request.player_guid_counter))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return PlayerNameQueryOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            if result.is_empty() {
                return PlayerNameQueryOutcomeLikeCpp::Missing;
            }

            PlayerNameQueryOutcomeLikeCpp::Found(PlayerNameQueryRowLikeCpp {
                name: result.read_string(2),
                race: result.read(3),
                class: result.read(4),
                sex: result.read(5),
                level: result.read(6),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn player_name_statement_preserves_identity_and_guid_bind() {
        let statement = statement_like_cpp(0x0102_0304_0506_0708);
        assert_eq!(statement.sql(), CharStatements::SEL_CHARACTER.sql());
        assert_eq!(statement.params(), [SqlParam::U64(0x0102_0304_0506_0708)]);
    }
}
