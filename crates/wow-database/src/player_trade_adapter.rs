//! MariaDB adapter for one completed player trade (#38).
//!
//! C++ `HandleAcceptTradeOpcode` commits both participants'
//! `SaveInventoryAndGoldToDB` in one Character transaction
//! (`TradeHandler.cpp:503-507`). Every row this trade changes is written in
//! that one transaction; links and owners are guarded by affected-row counts,
//! so a row another writer moved makes the whole trade roll back.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerTradeItemPersistenceLikeCpp,
    PlayerTradePersistenceOutcomeLikeCpp, PlayerTradePersistencePortLikeCpp,
    PlayerTradePersistenceRequestLikeCpp,
};

use crate::{
    CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction, SqlTransactionCommitError,
};

fn statement(kind: CharStatements, bind: impl FnOnce(&mut PreparedStatement)) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(kind);
    bind(&mut statement);
    statement
}

/// The statements of one trade, with the affected-row count each must report
/// (`None` when any count is acceptable).
pub(crate) fn player_trade_statements_like_cpp(
    request: &PlayerTradePersistenceRequestLikeCpp,
) -> Vec<(PreparedStatement, Option<u64>)> {
    let mut statements = Vec::new();
    for item in &request.items {
        match item {
            PlayerTradeItemPersistenceLikeCpp::Move {
                item_guid,
                from_owner_guid,
                to_owner_guid,
                bag_guid,
                slot,
                new_count,
                wrapped,
            } => {
                statements.push((
                    statement(CharStatements::DEL_CHAR_INVENTORY_ITEM, |s| {
                        s.set_u64(0, *from_owner_guid);
                        s.set_u64(1, *item_guid);
                    }),
                    Some(1),
                ));
                statements.push((
                    statement(CharStatements::UPD_ITEM_OWNER, |s| {
                        s.set_u64(0, *to_owner_guid);
                        s.set_u64(1, *item_guid);
                    }),
                    Some(1),
                ));
                if let Some(new_count) = new_count {
                    statements.push((
                        statement(CharStatements::UPD_ITEM_INSTANCE_COUNT, |s| {
                            s.set_u32(0, *new_count);
                            s.set_u64(1, *item_guid);
                        }),
                        Some(1),
                    ));
                }
                if *wrapped {
                    statements.push((
                        statement(CharStatements::UPD_GIFT_OWNER, |s| {
                            s.set_u64(0, *to_owner_guid);
                            s.set_u64(1, *item_guid);
                        }),
                        None,
                    ));
                }
                statements.push((
                    statement(CharStatements::REP_CHAR_INVENTORY_ITEM, |s| {
                        s.set_u64(0, *to_owner_guid);
                        s.set_u64(1, *bag_guid);
                        s.set_u8(2, *slot);
                        s.set_u64(3, *item_guid);
                    }),
                    None,
                ));
            }
            PlayerTradeItemPersistenceLikeCpp::MergeInto {
                item_guid,
                new_count,
            } => statements.push((
                statement(CharStatements::UPD_ITEM_INSTANCE_COUNT, |s| {
                    s.set_u32(0, *new_count);
                    s.set_u64(1, *item_guid);
                }),
                Some(1),
            )),
            PlayerTradeItemPersistenceLikeCpp::FullyMerged {
                item_guid,
                from_owner_guid,
            } => {
                statements.push((
                    statement(CharStatements::DEL_CHAR_INVENTORY_ITEM, |s| {
                        s.set_u64(0, *from_owner_guid);
                        s.set_u64(1, *item_guid);
                    }),
                    Some(1),
                ));
                for kind in [
                    CharStatements::DEL_ITEM_REFUND_INSTANCE,
                    CharStatements::DEL_ITEM_BOP_TRADE,
                    CharStatements::DEL_ITEM_INSTANCE_GEMS,
                    CharStatements::DEL_ITEM_INSTANCE_TRANSMOG,
                    CharStatements::DEL_GIFT,
                    CharStatements::DEL_ITEMCONTAINER_ITEMS,
                    CharStatements::DEL_ITEMCONTAINER_MONEY,
                ] {
                    statements.push((statement(kind, |s| s.set_u64(0, *item_guid)), None));
                }
                statements.push((
                    statement(CharStatements::DEL_ITEM_INSTANCE, |s| {
                        s.set_u64(0, *item_guid)
                    }),
                    Some(1),
                ));
            }
        }
    }
    for money in &request.money {
        if money.money_before == money.money_after {
            continue;
        }
        statements.push((
            statement(CharStatements::UPD_CHAR_MONEY, |s| {
                s.set_u64(0, money.money_after);
                s.set_u64(1, money.player_guid);
            }),
            Some(1),
        ));
    }
    statements
}

pub struct MariaDbPlayerTradePersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbPlayerTradePersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }

    async fn observe_money_like_cpp(
        &self,
        request: &PlayerTradePersistenceRequestLikeCpp,
    ) -> Vec<(u64, Option<u64>)> {
        let mut observed = Vec::new();
        for money in request
            .money
            .iter()
            .filter(|money| money.money_before != money.money_after)
        {
            let select = statement(CharStatements::SEL_CHAR_MONEY, |s| {
                s.set_u64(0, money.player_guid)
            });
            let value = match self.character_db.query(&select).await {
                Ok(result) if result.is_empty() => None,
                Ok(result) => result.try_read::<u64>(0),
                Err(_) => None,
            };
            observed.push((money.player_guid, value));
        }
        observed
    }
}

impl PlayerTradePersistencePortLikeCpp for MariaDbPlayerTradePersistenceAdapterLikeCpp {
    fn persist_player_trade_like_cpp(
        &self,
        request: PlayerTradePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerTradePersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            for (statement, expected) in player_trade_statements_like_cpp(&request) {
                match expected {
                    Some(expected) => transaction.append_expect_rows_affected(statement, expected),
                    None => transaction.append(statement),
                }
            }
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PlayerTradePersistenceOutcomeLikeCpp::Committed,
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    PlayerTradePersistenceOutcomeLikeCpp::DefinitelyRolledBack {
                        reason: error.to_string(),
                    }
                }
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
                    PlayerTradePersistenceOutcomeLikeCpp::CommitOutcomeUnknown {
                        reason: error.to_string(),
                        observed_money: self.observe_money_like_cpp(&request).await,
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::PlayerTradeMoneyPersistenceLikeCpp;

    #[test]
    fn trade_statements_move_merge_and_money_in_one_batch_like_cpp() {
        let request = PlayerTradePersistenceRequestLikeCpp {
            money: vec![
                PlayerTradeMoneyPersistenceLikeCpp {
                    player_guid: 1,
                    money_before: 100,
                    money_after: 40,
                },
                PlayerTradeMoneyPersistenceLikeCpp {
                    player_guid: 2,
                    money_before: 7,
                    money_after: 7,
                },
            ],
            items: vec![
                PlayerTradeItemPersistenceLikeCpp::Move {
                    item_guid: 10,
                    from_owner_guid: 1,
                    to_owner_guid: 2,
                    bag_guid: 0,
                    slot: 23,
                    new_count: None,
                    wrapped: true,
                },
                PlayerTradeItemPersistenceLikeCpp::MergeInto {
                    item_guid: 20,
                    new_count: 5,
                },
                PlayerTradeItemPersistenceLikeCpp::FullyMerged {
                    item_guid: 11,
                    from_owner_guid: 2,
                },
            ],
        };
        let statements = player_trade_statements_like_cpp(&request);
        let sqls: Vec<(&str, Option<u64>)> = statements
            .iter()
            .map(|(statement, expected)| (statement.sql(), *expected))
            .collect();
        assert_eq!(
            sqls[0],
            (
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
                Some(1)
            )
        );
        assert_eq!(
            sqls[1],
            (
                "UPDATE item_instance SET owner_guid = ? WHERE guid = ?",
                Some(1)
            )
        );
        assert_eq!(
            sqls[2].0,
            "UPDATE character_gifts SET guid = ? WHERE item_guid = ?"
        );
        assert!(sqls[3].0.starts_with("REPLACE INTO character_inventory"));
        assert_eq!(
            sqls[4],
            ("UPDATE item_instance SET count = ? WHERE guid = ?", Some(1))
        );
        assert_eq!(sqls[5].1, Some(1), "the merged item's link must exist");
        assert_eq!(
            sqls.iter()
                .filter(|(sql, _)| sql.starts_with("UPDATE characters SET money"))
                .count(),
            1,
            "only a changed money row is written"
        );
        assert_eq!(
            *sqls.last().unwrap(),
            ("UPDATE characters SET money = ? WHERE guid = ?", Some(1))
        );
    }
}
