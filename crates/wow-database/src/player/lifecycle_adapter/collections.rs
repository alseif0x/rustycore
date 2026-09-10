//! Account collection statement selection and persistence-result translation.
//! Private MariaDB implementation; the port remains independent of driver errors.

use crate::statements::StatementDef;

use crate::params::PreparedStatement;
use crate::statements::LoginStatements;
use wow_persistence::AccountCollectionLoadRequestLikeCpp;

pub(super) fn account_collection_commit_outcome_like_cpp(
    result: Result<(), crate::SqlTransactionCommitError>,
    rows: u64,
) -> wow_persistence::PersistenceOutcomeLikeCpp {
    use crate::SqlTransactionCommitError;
    use wow_persistence::PersistenceOutcomeLikeCpp;

    match result {
        Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows },
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

#[cfg(test)]
mod tests {
    use super::account_collection_commit_outcome_like_cpp;
    use crate::{DatabaseError, SqlTransactionCommitError};
    use wow_persistence::PersistenceOutcomeLikeCpp;

    #[test]
    fn collection_commit_preserves_confirmation_and_row_count() {
        for rows in [0, 1, 19] {
            assert_eq!(
                account_collection_commit_outcome_like_cpp(Ok(()), rows),
                PersistenceOutcomeLikeCpp::Applied { rows }
            );
        }
    }

    #[test]
    fn collection_commit_preserves_known_rollback() {
        let error = DatabaseError::Transaction("statement rejected".into());
        let reason = error.to_string();
        assert_eq!(
            account_collection_commit_outcome_like_cpp(
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)),
                19
            ),
            PersistenceOutcomeLikeCpp::Failed { reason }
        );
    }

    #[test]
    fn collection_commit_never_relabels_unknown_as_rollback() {
        let error = DatabaseError::Transaction("COMMIT reply lost".into());
        let reason = error.to_string();
        assert_eq!(
            account_collection_commit_outcome_like_cpp(
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)),
                19
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason }
        );
    }
}

pub(super) fn account_collection_load_statements_like_cpp(
    request: AccountCollectionLoadRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let (bnet_account_id, statements) = match request {
        AccountCollectionLoadRequestLikeCpp::Mounts { bnet_account_id } => {
            (bnet_account_id, vec![LoginStatements::SEL_ACCOUNT_MOUNTS])
        }
        AccountCollectionLoadRequestLikeCpp::Toys { bnet_account_id } => {
            (bnet_account_id, vec![LoginStatements::SEL_ACCOUNT_TOYS])
        }
        AccountCollectionLoadRequestLikeCpp::Heirlooms { bnet_account_id } => (
            bnet_account_id,
            vec![LoginStatements::SEL_ACCOUNT_HEIRLOOMS],
        ),
        AccountCollectionLoadRequestLikeCpp::ItemAppearances { bnet_account_id } => (
            bnet_account_id,
            vec![
                LoginStatements::SEL_BNET_ITEM_APPEARANCES,
                LoginStatements::SEL_BNET_ITEM_FAVORITE_APPEARANCES,
            ],
        ),
        AccountCollectionLoadRequestLikeCpp::TransmogIllusions { bnet_account_id } => (
            bnet_account_id,
            vec![LoginStatements::SEL_BNET_TRANSMOG_ILLUSIONS],
        ),
    };

    statements
        .into_iter()
        .map(|statement| {
            let mut prepared = PreparedStatement::new(statement.sql());
            prepared.set_u32(0, bnet_account_id);
            prepared
        })
        .collect()
}
