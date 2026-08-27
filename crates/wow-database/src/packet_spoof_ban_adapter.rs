//! MariaDB adapter for PacketSpoof admission bans.

use std::sync::Arc;

use wow_persistence::{
    PacketSpoofAffectedAccountsLoadOutcomeLikeCpp, PacketSpoofBanPersistencePortLikeCpp,
    PacketSpoofBanTargetLikeCpp, PacketSpoofBanWriteRequestLikeCpp, PersistenceFutureLikeCpp,
    PersistenceOutcomeLikeCpp,
};

use crate::{LoginDatabase, LoginStatements, PreparedStatement, SqlTransaction};

fn accounts_by_ip_statement_like_cpp(address: &str) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(LoginStatements::SEL_ACCOUNT_BY_IP);
    statement.set_string(0, address);
    statement
}

fn account_ban_statements_like_cpp(
    account_id: u32,
    duration_secs: u32,
    author: &str,
    reason: &str,
) -> [PreparedStatement; 2] {
    let mut clear_active =
        PreparedStatement::for_statement(LoginStatements::UPD_ACCOUNT_NOT_BANNED);
    clear_active.set_u32(0, account_id);
    let mut insert = PreparedStatement::for_statement(LoginStatements::INS_ACCOUNT_BANNED);
    insert.set_u32(0, account_id);
    insert.set_u32(1, duration_secs);
    insert.set_string(2, author);
    insert.set_string(3, reason);
    [clear_active, insert]
}

fn ip_ban_statement_like_cpp(
    address: &str,
    duration_secs: u32,
    author: &str,
    reason: &str,
) -> PreparedStatement {
    let mut insert = PreparedStatement::for_statement(LoginStatements::INS_IP_BANNED);
    insert.set_string(0, address);
    insert.set_u32(1, duration_secs);
    insert.set_string(2, author);
    insert.set_string(3, reason);
    insert
}

pub struct MariaDbPacketSpoofBanPersistenceAdapterLikeCpp {
    login_db: Arc<LoginDatabase>,
}

impl MariaDbPacketSpoofBanPersistenceAdapterLikeCpp {
    pub fn new(login_db: Arc<LoginDatabase>) -> Self {
        Self { login_db }
    }
}

impl PacketSpoofBanPersistencePortLikeCpp for MariaDbPacketSpoofBanPersistenceAdapterLikeCpp {
    fn load_accounts_by_ip_like_cpp<'a>(
        &'a self,
        address: &'a str,
    ) -> PersistenceFutureLikeCpp<'a, PacketSpoofAffectedAccountsLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = accounts_by_ip_statement_like_cpp(address);
            let mut result = match self.login_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return PacketSpoofAffectedAccountsLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            if result.is_empty() {
                return PacketSpoofAffectedAccountsLoadOutcomeLikeCpp::Loaded(Vec::new());
            }

            let mut account_ids = Vec::with_capacity(result.count());
            loop {
                account_ids.push(result.read(0));
                if !result.next_row() {
                    break;
                }
            }
            account_ids.sort_unstable();
            account_ids.dedup();
            PacketSpoofAffectedAccountsLoadOutcomeLikeCpp::Loaded(account_ids)
        })
    }

    fn persist_packet_spoof_ban_like_cpp<'a>(
        &'a self,
        request: PacketSpoofBanWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match request.target {
                PacketSpoofBanTargetLikeCpp::Account { account_id } => {
                    let statements = account_ban_statements_like_cpp(
                        account_id,
                        request.duration_secs,
                        &request.author,
                        &request.reason,
                    );
                    let mut transaction = SqlTransaction::new();
                    for statement in statements {
                        transaction.append(statement);
                    }
                    self.login_db
                        .commit_transaction(transaction)
                        .await
                        .map(|_| 0)
                }
                PacketSpoofBanTargetLikeCpp::Ip { address } => {
                    let statement = ip_ban_statement_like_cpp(
                        &address,
                        request.duration_secs,
                        &request.author,
                        &request.reason,
                    );
                    self.login_db.execute(&statement).await
                }
            };

            match result {
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
    use crate::{SqlParam, StatementDef};

    #[test]
    fn packet_spoof_statements_preserve_cpp_identity_order_and_binds() {
        let lookup = accounts_by_ip_statement_like_cpp("127.0.0.1");
        assert_eq!(lookup.sql(), LoginStatements::SEL_ACCOUNT_BY_IP.sql());
        assert_eq!(lookup.params(), vec![SqlParam::String("127.0.0.1".into())]);

        let statements = account_ban_statements_like_cpp(7, 60, "author", "reason");
        assert_eq!(statements.len(), 2);
        assert_eq!(
            statements[0].sql(),
            LoginStatements::UPD_ACCOUNT_NOT_BANNED.sql()
        );
        assert_eq!(statements[0].params(), vec![SqlParam::U32(7)]);
        assert_eq!(
            statements[1].sql(),
            LoginStatements::INS_ACCOUNT_BANNED.sql()
        );
        assert_eq!(
            statements[1].params(),
            vec![
                SqlParam::U32(7),
                SqlParam::U32(60),
                SqlParam::String("author".into()),
                SqlParam::String("reason".into()),
            ]
        );

        let ip = ip_ban_statement_like_cpp("127.0.0.1", 60, "author", "reason");
        assert_eq!(ip.sql(), LoginStatements::INS_IP_BANNED.sql());
        assert_eq!(
            ip.params(),
            vec![
                SqlParam::String("127.0.0.1".into()),
                SqlParam::U32(60),
                SqlParam::String("author".into()),
                SqlParam::String("reason".into()),
            ]
        );
    }
}
