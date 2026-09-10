//! Compatibility adapter for money-marked CharacterDB transactions.
//!
//! Callers that still construct concrete transactions use this while their
//! whole business capability is migrated. New capability adapters may reuse
//! the same exact commit classification and durable-money observation.

use wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp;

use crate::{CharStatements, CharacterDatabase, SqlTransaction, SqlTransactionCommitError};

pub async fn commit_player_money_transaction_and_observe_like_cpp(
    character_db: &CharacterDatabase,
    transaction: SqlTransaction,
    player_guid: Option<u64>,
) -> PlayerMoneyTransactionOutcomeLikeCpp {
    match transaction
        .commit_with_outcome_like_cpp(character_db.pool())
        .await
    {
        Ok(()) => PlayerMoneyTransactionOutcomeLikeCpp::Committed,
        Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
            PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
                reason: error.to_string(),
            }
        }
        Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
            let observed_money = match player_guid {
                Some(player_guid) => {
                    let mut observed = character_db.prepare(CharStatements::SEL_CHAR_MONEY);
                    observed.set_u64(0, player_guid);
                    character_db
                        .query(&observed)
                        .await
                        .ok()
                        .filter(|result| !result.is_empty())
                        .and_then(|result| result.try_read::<u64>(0))
                }
                None => None,
            };
            PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
                reason: error.to_string(),
                observed_money,
            }
        }
    }
}
