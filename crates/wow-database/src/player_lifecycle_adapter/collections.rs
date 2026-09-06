//! Account collection read statement selection.
//! Private MariaDB implementation; no semantic port or transaction changes.

use crate::statements::StatementDef;

use crate::params::PreparedStatement;
use crate::statements::LoginStatements;
use wow_persistence::AccountCollectionLoadRequestLikeCpp;

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
