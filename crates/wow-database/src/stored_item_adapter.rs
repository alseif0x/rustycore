//! MariaDB adapter for durable Item-owned state.

use std::sync::Arc;

use wow_persistence::{
    InventoryItemCountPersistenceRequestLikeCpp, InventoryItemDestroyPersistenceRequestLikeCpp,
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, StoredItemLoadOutcomeLikeCpp,
    StoredItemLootPersistenceRowLikeCpp, StoredItemLootSaveRequestLikeCpp,
    StoredItemPersistencePortLikeCpp, WrappedGiftOpenPersistenceRequestLikeCpp,
    WrappedGiftPersistenceRowLikeCpp,
};

use crate::{
    CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction, SqlTransactionCommitError,
};

fn load_wrapped_gift_statement_like_cpp(item_guid: u64) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_GIFT_BY_ITEM);
    statement.set_u64(0, item_guid);
    statement
}

fn open_wrapped_gift_statements_like_cpp(
    request: WrappedGiftOpenPersistenceRequestLikeCpp,
) -> [PreparedStatement; 2] {
    let mut update = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_OPEN_GIFT);
    update.set_u32(0, request.entry);
    update.set_u32(1, request.flags);
    update.set_u32(2, request.durability);
    update.set_u64(3, request.item_guid);
    let mut delete = PreparedStatement::for_statement(CharStatements::DEL_GIFT);
    delete.set_u64(0, request.item_guid);
    [update, delete]
}

fn update_inventory_item_count_statement_like_cpp(
    request: InventoryItemCountPersistenceRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_COUNT);
    statement.set_u32(0, request.count);
    statement.set_u64(1, request.item_guid);
    statement
}

fn destroy_inventory_item_statements_like_cpp(
    request: InventoryItemDestroyPersistenceRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(if request.expire_refund { 3 } else { 2 });
    if request.expire_refund {
        let mut delete = PreparedStatement::for_statement(CharStatements::DEL_ITEM_REFUND_INSTANCE);
        delete.set_u64(0, request.item_guid);
        statements.push(delete);
    }
    let mut delete_inventory =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_INVENTORY_ITEM);
    delete_inventory.set_u64(0, request.owner_guid);
    delete_inventory.set_u64(1, request.item_guid);
    statements.push(delete_inventory);
    let mut delete_item = PreparedStatement::for_statement(CharStatements::DEL_ITEM_INSTANCE);
    delete_item.set_u64(0, request.item_guid);
    statements.push(delete_item);
    statements
}

fn load_stored_item_money_statement_like_cpp(item_guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::SEL_ITEMCONTAINER_MONEY);
    statement.set_u64(0, item_guid);
    statement
}

fn load_stored_item_loot_statement_like_cpp(item_guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::SEL_ITEMCONTAINER_ITEMS);
    statement.set_u64(0, item_guid);
    statement
}

fn save_stored_item_loot_statements_like_cpp(
    request: &StoredItemLootSaveRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(request.items.len() + 3);
    if request.money > 0 {
        let mut delete = PreparedStatement::for_statement(CharStatements::DEL_ITEMCONTAINER_MONEY);
        delete.set_u64(0, request.item_guid);
        statements.push(delete);
        let mut insert = PreparedStatement::for_statement(CharStatements::INS_ITEMCONTAINER_MONEY);
        insert.set_u64(0, request.item_guid);
        insert.set_u32(1, request.money);
        statements.push(insert);
    }
    let mut delete = PreparedStatement::for_statement(CharStatements::DEL_ITEMCONTAINER_ITEMS);
    delete.set_u64(0, request.item_guid);
    statements.push(delete);
    for item in &request.items {
        let mut insert = PreparedStatement::for_statement(CharStatements::INS_ITEMCONTAINER_ITEMS);
        insert.set_u64(0, request.item_guid);
        insert.set_u32(1, item.item_id);
        insert.set_u32(2, item.count);
        insert.set_u32(3, item.item_index);
        insert.set_bool(4, item.follow_loot_rules);
        insert.set_bool(5, item.free_for_all);
        insert.set_bool(6, item.blocked);
        insert.set_bool(7, item.counted);
        insert.set_bool(8, item.under_threshold);
        insert.set_bool(9, item.needs_quest);
        insert.set_i32(10, item.random_properties_id);
        insert.set_i32(11, item.random_properties_seed);
        insert.set_u8(12, item.context);
        statements.push(insert);
    }
    statements
}

pub struct MariaDbStoredItemPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbStoredItemPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }

    async fn commit_like_cpp(&self, transaction: SqlTransaction) -> PersistenceOutcomeLikeCpp {
        match transaction
            .commit_with_outcome_like_cpp(self.character_db.pool())
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
}

impl StoredItemPersistencePortLikeCpp for MariaDbStoredItemPersistenceAdapterLikeCpp {
    fn load_wrapped_gift_like_cpp(
        &self,
        item_guid: u64,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemLoadOutcomeLikeCpp<WrappedGiftPersistenceRowLikeCpp>>
    {
        Box::pin(async move {
            let statement = load_wrapped_gift_statement_like_cpp(item_guid);
            match self.character_db.query(&statement).await {
                Ok(result) if result.is_empty() => StoredItemLoadOutcomeLikeCpp::Missing,
                Ok(result) => {
                    StoredItemLoadOutcomeLikeCpp::Loaded(WrappedGiftPersistenceRowLikeCpp {
                        entry: result.try_read(0).unwrap_or(0),
                        flags: result.try_read(1).unwrap_or(0),
                    })
                }
                Err(error) => StoredItemLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn open_wrapped_gift_like_cpp(
        &self,
        request: WrappedGiftOpenPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            for statement in open_wrapped_gift_statements_like_cpp(request) {
                transaction.append(statement);
            }
            self.commit_like_cpp(transaction).await
        })
    }

    fn update_inventory_item_count_like_cpp(
        &self,
        request: InventoryItemCountPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = update_inventory_item_count_statement_like_cpp(request);
            match self.character_db.execute(&statement).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn destroy_inventory_item_like_cpp(
        &self,
        request: InventoryItemDestroyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            for statement in destroy_inventory_item_statements_like_cpp(request) {
                transaction.append(statement);
            }
            self.commit_like_cpp(transaction).await
        })
    }

    fn load_stored_item_money_like_cpp(
        &self,
        item_guid: u64,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemLoadOutcomeLikeCpp<u32>> {
        Box::pin(async move {
            let statement = load_stored_item_money_statement_like_cpp(item_guid);
            match self.character_db.query(&statement).await {
                Ok(result) if result.is_empty() => StoredItemLoadOutcomeLikeCpp::Missing,
                Ok(result) => match result.try_read(0) {
                    Some(money) => StoredItemLoadOutcomeLikeCpp::Loaded(money),
                    None => StoredItemLoadOutcomeLikeCpp::Missing,
                },
                Err(error) => StoredItemLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_stored_item_loot_like_cpp(
        &self,
        item_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StoredItemLoadOutcomeLikeCpp<Vec<StoredItemLootPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            let statement = load_stored_item_loot_statement_like_cpp(item_guid);
            let mut result = match self.character_db.query(&statement).await {
                Ok(result) if result.is_empty() => return StoredItemLoadOutcomeLikeCpp::Missing,
                Ok(result) => result,
                Err(error) => {
                    return StoredItemLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            let mut items = Vec::new();
            loop {
                items.push(StoredItemLootPersistenceRowLikeCpp {
                    item_id: result.try_read(0).unwrap_or(0),
                    count: result.try_read(1).unwrap_or(0),
                    item_index: result.try_read(2).unwrap_or(u32::MAX),
                    follow_loot_rules: result.try_read(3).unwrap_or(false),
                    free_for_all: result.try_read(4).unwrap_or(false),
                    blocked: result.try_read(5).unwrap_or(false),
                    counted: result.try_read(6).unwrap_or(false),
                    under_threshold: result.try_read(7).unwrap_or(false),
                    needs_quest: result.try_read(8).unwrap_or(false),
                    random_properties_id: result.try_read(9).unwrap_or(0),
                    random_properties_seed: result.try_read(10).unwrap_or(0),
                    context: result.try_read(11).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
            StoredItemLoadOutcomeLikeCpp::Loaded(items)
        })
    }

    fn save_stored_item_loot_like_cpp(
        &self,
        request: StoredItemLootSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            for statement in save_stored_item_loot_statements_like_cpp(&request) {
                transaction.append(statement);
            }
            self.commit_like_cpp(transaction).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    fn statement_sqls(statements: &[PreparedStatement]) -> Vec<&str> {
        statements.iter().map(PreparedStatement::sql).collect()
    }

    #[test]
    fn wrapped_gift_open_preserves_update_then_gift_delete_like_cpp() {
        let statements =
            open_wrapped_gift_statements_like_cpp(WrappedGiftOpenPersistenceRequestLikeCpp {
                item_guid: 91,
                entry: 42,
                flags: 7,
                durability: 123,
            });
        assert_eq!(
            statement_sqls(&statements),
            [
                CharStatements::UPD_ITEM_INSTANCE_OPEN_GIFT.sql(),
                CharStatements::DEL_GIFT.sql(),
            ]
        );
        assert_eq!(
            statements[0].params(),
            [
                SqlParam::U32(42),
                SqlParam::U32(7),
                SqlParam::U32(123),
                SqlParam::U64(91),
            ]
        );
        assert_eq!(statements[1].params(), [SqlParam::U64(91)]);
    }

    #[test]
    fn item_destroy_includes_refund_delete_only_when_required_like_cpp() {
        let request = InventoryItemDestroyPersistenceRequestLikeCpp {
            owner_guid: 11,
            item_guid: 22,
            expire_refund: true,
        };
        let with_refund = destroy_inventory_item_statements_like_cpp(request);
        assert_eq!(
            statement_sqls(&with_refund),
            [
                CharStatements::DEL_ITEM_REFUND_INSTANCE.sql(),
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::DEL_ITEM_INSTANCE.sql(),
            ]
        );
        let without_refund = destroy_inventory_item_statements_like_cpp(
            InventoryItemDestroyPersistenceRequestLikeCpp {
                expire_refund: false,
                ..request
            },
        );
        assert_eq!(
            statement_sqls(&without_refund),
            [
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::DEL_ITEM_INSTANCE.sql(),
            ]
        );
        assert_eq!(
            without_refund[0].params(),
            [SqlParam::U64(11), SqlParam::U64(22)]
        );
    }

    #[test]
    fn stored_loot_save_preserves_money_then_items_order_and_metadata_like_cpp() {
        let request = StoredItemLootSaveRequestLikeCpp {
            item_guid: 77,
            money: 99,
            items: vec![StoredItemLootPersistenceRowLikeCpp {
                item_id: 10,
                count: 2,
                item_index: 3,
                follow_loot_rules: true,
                free_for_all: false,
                blocked: true,
                counted: false,
                under_threshold: true,
                needs_quest: false,
                random_properties_id: -4,
                random_properties_seed: 5,
                context: 6,
            }],
        };
        let statements = save_stored_item_loot_statements_like_cpp(&request);
        assert_eq!(
            statement_sqls(&statements),
            [
                CharStatements::DEL_ITEMCONTAINER_MONEY.sql(),
                CharStatements::INS_ITEMCONTAINER_MONEY.sql(),
                CharStatements::DEL_ITEMCONTAINER_ITEMS.sql(),
                CharStatements::INS_ITEMCONTAINER_ITEMS.sql(),
            ]
        );
        assert_eq!(
            statements[3].params(),
            [
                SqlParam::U64(77),
                SqlParam::U32(10),
                SqlParam::U32(2),
                SqlParam::U32(3),
                SqlParam::Bool(true),
                SqlParam::Bool(false),
                SqlParam::Bool(true),
                SqlParam::Bool(false),
                SqlParam::Bool(true),
                SqlParam::Bool(false),
                SqlParam::I32(-4),
                SqlParam::I32(5),
                SqlParam::U8(6),
            ]
        );

        let no_money =
            save_stored_item_loot_statements_like_cpp(&StoredItemLootSaveRequestLikeCpp {
                money: 0,
                items: Vec::new(),
                ..request
            });
        assert_eq!(
            statement_sqls(&no_money),
            [CharStatements::DEL_ITEMCONTAINER_ITEMS.sql()]
        );
    }
}
