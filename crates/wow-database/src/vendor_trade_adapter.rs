//! MariaDB adapter for Player/vendor commerce.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerMoneyTransactionOutcomeLikeCpp,
    VendorBuybackDestinationPersistenceLikeCpp, VendorRefundCleanupPersistenceLikeCpp,
    VendorSaleItemPersistenceLikeCpp, VendorTradePersistencePortLikeCpp,
    VendorTradePersistenceRequestLikeCpp,
};

use crate::{
    CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction, SqlTransactionCommitError,
};

fn update_money_like_cpp(player_guid: u64, money: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::UPD_CHAR_MONEY);
    statement.set_u64(0, money);
    statement.set_u64(1, player_guid);
    statement
}

fn update_count_like_cpp(item_guid: u64, count: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_COUNT);
    statement.set_u32(0, count);
    statement.set_u64(1, item_guid);
    statement
}

fn delete_inventory_link_like_cpp(owner_guid: u64, item_guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::DEL_CHAR_INVENTORY_ITEM);
    statement.set_u64(0, owner_guid);
    statement.set_u64(1, item_guid);
    statement
}

fn delete_item_like_cpp(item_guid: u64) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::DEL_ITEM_INSTANCE);
    statement.set_u64(0, item_guid);
    statement
}

fn update_inventory_slot_like_cpp(
    player_guid: u64,
    inventory_slot: u8,
    item_guid: u64,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::UPD_CHAR_INVENTORY_SLOT);
    statement.set_u8(0, inventory_slot);
    statement.set_u64(1, player_guid);
    statement.set_u64(2, item_guid);
    statement
}

fn insert_inventory_link_like_cpp(
    player_guid: u64,
    inventory_slot: u8,
    item_guid: u64,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::INS_CHAR_INVENTORY);
    statement.set_u64(0, player_guid);
    statement.set_u8(1, inventory_slot);
    statement.set_u64(2, item_guid);
    statement
}

fn append_item_turnins_like_cpp(
    statements: &mut Vec<PreparedStatement>,
    turnins: &[wow_persistence::VendorItemTurninPersistenceLikeCpp],
) {
    for turnin in turnins {
        match *turnin {
            wow_persistence::VendorItemTurninPersistenceLikeCpp::Update {
                item_guid,
                new_count,
            } => statements.push(update_count_like_cpp(item_guid, new_count)),
            wow_persistence::VendorItemTurninPersistenceLikeCpp::Delete {
                owner_guid,
                item_guid,
            } => {
                statements.push(delete_inventory_link_like_cpp(owner_guid, item_guid));
                statements.push(delete_item_like_cpp(item_guid));
            }
        }
    }
}

fn vendor_trade_statements_like_cpp(
    request: &VendorTradePersistenceRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::new();
    match request {
        VendorTradePersistenceRequestLikeCpp::CurrencyPurchase(request) => {
            append_item_turnins_like_cpp(&mut statements, &request.item_turnins);
            statements.extend(
                crate::player_lifecycle_adapter::player_currency_save_statements_like_cpp(
                    &request.currency_save,
                ),
            );
        }
        VendorTradePersistenceRequestLikeCpp::ItemPurchase(request) => {
            statements.push(update_money_like_cpp(
                request.player_guid,
                request.money_after,
            ));
            for stack in &request.existing_stacks {
                statements.push(update_count_like_cpp(stack.item_guid, stack.new_count));
            }
            for stack in &request.new_stacks {
                let mut insert = PreparedStatement::for_statement(
                    CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT,
                );
                insert.set_u64(0, stack.item_guid);
                insert.set_u32(1, stack.item_entry);
                insert.set_u64(2, stack.owner_guid);
                insert.set_u32(3, stack.count);
                insert.set_u32(4, stack.durability);
                insert.set_u32(5, stack.flags);
                insert.set_i32(6, stack.random_properties_id);
                insert.set_i32(7, stack.property_seed);
                insert.set_u8(8, stack.context);
                statements.push(insert);
                statements.push(insert_inventory_link_like_cpp(
                    stack.owner_guid,
                    stack.inventory_slot,
                    stack.item_guid,
                ));
            }
            if let Some(refund) = request.refund_metadata {
                let mut flags =
                    PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
                flags.set_u32(0, refund.flags_after);
                flags.set_u64(1, refund.item_guid);
                statements.push(flags);
                let mut delete =
                    PreparedStatement::for_statement(CharStatements::DEL_ITEM_REFUND_INSTANCE);
                delete.set_u64(0, refund.item_guid);
                statements.push(delete);
                let mut insert =
                    PreparedStatement::for_statement(CharStatements::INS_ITEM_REFUND_INSTANCE);
                insert.set_u64(0, refund.item_guid);
                insert.set_u64(1, refund.player_guid);
                insert.set_u64(2, refund.paid_money);
                insert.set_u16(3, refund.paid_extended_cost);
                statements.push(insert);
            }
            append_item_turnins_like_cpp(&mut statements, &request.item_turnins);
            statements.extend(
                crate::player_lifecycle_adapter::player_currency_save_statements_like_cpp(
                    &request.currency_save,
                ),
            );
        }
        VendorTradePersistenceRequestLikeCpp::Buyback(request) => {
            statements.push(update_money_like_cpp(
                request.player_guid,
                request.money_after,
            ));
            for destination in &request.destinations {
                match *destination {
                    VendorBuybackDestinationPersistenceLikeCpp::Merge {
                        item_guid,
                        new_count,
                    } => statements.push(update_count_like_cpp(item_guid, new_count)),
                    VendorBuybackDestinationPersistenceLikeCpp::Move {
                        inventory_slot,
                        item_guid,
                        new_count,
                    } => {
                        statements.push(update_inventory_slot_like_cpp(
                            request.player_guid,
                            inventory_slot,
                            item_guid,
                        ));
                        if let Some(new_count) = new_count {
                            statements.push(update_count_like_cpp(item_guid, new_count));
                        }
                    }
                }
            }
            if let Some(item_guid) = request.delete_source_item_guid {
                statements.push(delete_inventory_link_like_cpp(
                    request.player_guid,
                    item_guid,
                ));
                statements.push(delete_item_like_cpp(item_guid));
            }
        }
        VendorTradePersistenceRequestLikeCpp::Sale(request) => {
            if let Some(item_guid) = request.evicted_buyback_item_guid {
                statements.push(delete_inventory_link_like_cpp(
                    request.player_guid,
                    item_guid,
                ));
                statements.push(delete_item_like_cpp(item_guid));
            }
            match &request.sold_item {
                VendorSaleItemPersistenceLikeCpp::FullStack {
                    item_guid,
                    buyback_slot,
                } => statements.push(update_inventory_slot_like_cpp(
                    request.player_guid,
                    *buyback_slot,
                    *item_guid,
                )),
                VendorSaleItemPersistenceLikeCpp::PartialStack {
                    source_item_guid,
                    source_count_after,
                    sold_clone,
                } => {
                    statements.push(update_count_like_cpp(
                        *source_item_guid,
                        *source_count_after,
                    ));
                    let mut insert =
                        PreparedStatement::for_statement(CharStatements::INS_ITEM_INSTANCE_CLONE);
                    insert.set_u64(0, sold_clone.item_guid);
                    insert.set_u32(1, sold_clone.item_entry);
                    insert.set_u64(2, sold_clone.owner_guid);
                    insert.set_u64(3, sold_clone.creator_guid);
                    insert.set_u64(4, sold_clone.gift_creator_guid);
                    insert.set_u32(5, sold_clone.count);
                    insert.set_u32(6, sold_clone.expiration);
                    insert.set_string(7, &sold_clone.charges);
                    insert.set_string(8, &sold_clone.enchantments);
                    insert.set_u32(9, sold_clone.flags);
                    insert.set_u32(10, sold_clone.durability);
                    insert.set_u32(11, sold_clone.create_played_time);
                    insert.set_i32(12, sold_clone.random_properties_id);
                    insert.set_i32(13, sold_clone.property_seed);
                    insert.set_u8(14, sold_clone.context);
                    statements.push(insert);
                    statements.push(insert_inventory_link_like_cpp(
                        sold_clone.owner_guid,
                        sold_clone.buyback_slot,
                        sold_clone.item_guid,
                    ));
                }
            }
            statements.push(update_money_like_cpp(
                request.player_guid,
                request.money_after,
            ));
        }
        VendorTradePersistenceRequestLikeCpp::Refund(request) => {
            let mut delete_refund =
                PreparedStatement::for_statement(CharStatements::DEL_ITEM_REFUND_INSTANCE);
            delete_refund.set_u64(0, request.refunded_item_guid);
            statements.push(delete_refund);
            statements.push(delete_inventory_link_like_cpp(
                request.player_guid,
                request.refunded_item_guid,
            ));
            statements.push(delete_item_like_cpp(request.refunded_item_guid));
            statements.push(update_money_like_cpp(
                request.player_guid,
                request.money_after,
            ));
            for stack in &request.existing_stacks {
                statements.push(update_count_like_cpp(stack.item_guid, stack.new_count));
            }
            for stack in &request.new_stacks {
                let mut insert =
                    PreparedStatement::for_statement(CharStatements::INS_ITEM_INSTANCE);
                insert.set_u64(0, stack.item_guid);
                insert.set_u32(1, stack.item_entry);
                insert.set_u64(2, stack.owner_guid);
                insert.set_u32(3, stack.count);
                insert.set_u32(4, stack.durability);
                statements.push(insert);
                statements.push(insert_inventory_link_like_cpp(
                    stack.owner_guid,
                    stack.inventory_slot,
                    stack.item_guid,
                ));
            }
            statements.extend(
                crate::player_lifecycle_adapter::player_currency_save_statements_like_cpp(
                    &request.currency_save,
                ),
            );
        }
    }
    statements
}

fn refund_cleanup_statements_like_cpp(
    request: VendorRefundCleanupPersistenceLikeCpp,
) -> [PreparedStatement; 2] {
    let mut delete = PreparedStatement::for_statement(CharStatements::DEL_ITEM_REFUND_INSTANCE);
    delete.set_u64(0, request.item_guid);
    let mut flags = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
    flags.set_u32(0, request.flags_after);
    flags.set_u64(1, request.item_guid);
    [delete, flags]
}

pub struct MariaDbVendorTradePersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbVendorTradePersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl VendorTradePersistencePortLikeCpp for MariaDbVendorTradePersistenceAdapterLikeCpp {
    fn persist_vendor_trade_like_cpp(
        &self,
        request: VendorTradePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async move {
            let player_guid = request.player_guid();
            let mut transaction = SqlTransaction::new();
            for statement in vendor_trade_statements_like_cpp(&request) {
                transaction.append(statement);
            }
            crate::player_money_transaction_adapter::commit_player_money_transaction_and_observe_like_cpp(
                self.character_db.as_ref(),
                transaction,
                Some(player_guid),
            )
            .await
        })
    }

    fn clear_refund_metadata_like_cpp(
        &self,
        request: VendorRefundCleanupPersistenceLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            for statement in refund_cleanup_statements_like_cpp(request) {
                transaction.append(statement);
            }
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};
    use wow_persistence::{
        PlayerCurrencySaveKindLikeCpp, PlayerCurrencySaveRequestLikeCpp,
        PlayerCurrencySaveRowLikeCpp, VendorBuybackPersistenceLikeCpp,
        VendorCurrencyPurchasePersistenceLikeCpp, VendorExistingStackPersistenceLikeCpp,
        VendorItemPurchasePersistenceLikeCpp, VendorItemTurninPersistenceLikeCpp,
        VendorPurchasedStackPersistenceLikeCpp, VendorRefundMetadataPersistenceLikeCpp,
        VendorRefundPersistenceLikeCpp, VendorRefundReturnedStackPersistenceLikeCpp,
        VendorSalePersistenceLikeCpp, VendorSoldClonePersistenceLikeCpp,
    };

    fn currency_save(player_guid: u64) -> PlayerCurrencySaveRequestLikeCpp {
        PlayerCurrencySaveRequestLikeCpp {
            player_guid,
            rows: vec![PlayerCurrencySaveRowLikeCpp {
                currency_id: 81,
                quantity: 10,
                weekly_quantity: 2,
                tracked_quantity: 3,
                increased_cap_quantity: 4,
                earned_quantity: 5,
                flags: 6,
                kind: PlayerCurrencySaveKindLikeCpp::Changed,
            }],
        }
    }

    fn sqls(request: VendorTradePersistenceRequestLikeCpp) -> Vec<String> {
        vendor_trade_statements_like_cpp(&request)
            .into_iter()
            .map(|statement| statement.sql().to_owned())
            .collect()
    }

    #[test]
    fn currency_purchase_preserves_turnin_then_currency_order_like_cpp() {
        let request = VendorTradePersistenceRequestLikeCpp::CurrencyPurchase(
            VendorCurrencyPurchasePersistenceLikeCpp {
                player_guid: 7,
                money_before: 100,
                money_after: 100,
                item_turnins: vec![
                    VendorItemTurninPersistenceLikeCpp::Update {
                        item_guid: 11,
                        new_count: 2,
                    },
                    VendorItemTurninPersistenceLikeCpp::Delete {
                        owner_guid: 7,
                        item_guid: 12,
                    },
                ],
                currency_save: currency_save(7),
            },
        );
        assert_eq!(
            sqls(request),
            vec![
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::DEL_ITEM_INSTANCE.sql(),
                CharStatements::UPD_PLAYER_CURRENCY.sql(),
            ]
        );
    }

    #[test]
    fn item_purchase_preserves_money_stacks_refund_turnins_currency_order_like_cpp() {
        let request = VendorTradePersistenceRequestLikeCpp::ItemPurchase(
            VendorItemPurchasePersistenceLikeCpp {
                player_guid: 7,
                money_before: 500,
                money_after: 400,
                existing_stacks: vec![VendorExistingStackPersistenceLikeCpp {
                    item_guid: 21,
                    new_count: 3,
                }],
                new_stacks: vec![VendorPurchasedStackPersistenceLikeCpp {
                    item_guid: 22,
                    item_entry: 1000,
                    owner_guid: 7,
                    count: 1,
                    durability: 50,
                    flags: 0,
                    random_properties_id: 0,
                    property_seed: 0,
                    context: 5,
                    inventory_slot: 24,
                }],
                refund_metadata: Some(VendorRefundMetadataPersistenceLikeCpp {
                    item_guid: 22,
                    player_guid: 7,
                    paid_money: 100,
                    paid_extended_cost: 9,
                    flags_after: 0x1000,
                }),
                item_turnins: vec![VendorItemTurninPersistenceLikeCpp::Delete {
                    owner_guid: 7,
                    item_guid: 23,
                }],
                currency_save: currency_save(7),
            },
        );
        let statements = vendor_trade_statements_like_cpp(&request);
        assert_eq!(
            statements
                .iter()
                .map(|statement| statement.sql().to_owned())
                .collect::<Vec<_>>(),
            vec![
                CharStatements::UPD_CHAR_MONEY.sql(),
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
                CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT.sql(),
                CharStatements::INS_CHAR_INVENTORY.sql(),
                CharStatements::UPD_ITEM_INSTANCE_FLAGS.sql(),
                CharStatements::DEL_ITEM_REFUND_INSTANCE.sql(),
                CharStatements::INS_ITEM_REFUND_INSTANCE.sql(),
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::DEL_ITEM_INSTANCE.sql(),
                CharStatements::UPD_PLAYER_CURRENCY.sql(),
            ]
        );
        assert_eq!(
            statements[0].params(),
            &[SqlParam::U64(400), SqlParam::U64(7)]
        );
        assert_eq!(
            statements[2].params(),
            &[
                SqlParam::U64(22),
                SqlParam::U32(1000),
                SqlParam::U64(7),
                SqlParam::U32(1),
                SqlParam::U32(50),
                SqlParam::U32(0),
                SqlParam::I32(0),
                SqlParam::I32(0),
                SqlParam::U8(5),
            ]
        );
        assert_eq!(
            statements[6].params(),
            &[
                SqlParam::U64(22),
                SqlParam::U64(7),
                SqlParam::U64(100),
                SqlParam::U16(9),
            ]
        );
    }

    #[test]
    fn buyback_preserves_money_merge_move_order_and_source_cleanup_like_cpp() {
        let request =
            VendorTradePersistenceRequestLikeCpp::Buyback(VendorBuybackPersistenceLikeCpp {
                player_guid: 7,
                money_before: 500,
                money_after: 450,
                destinations: vec![
                    VendorBuybackDestinationPersistenceLikeCpp::Merge {
                        item_guid: 31,
                        new_count: 4,
                    },
                    VendorBuybackDestinationPersistenceLikeCpp::Move {
                        inventory_slot: 25,
                        item_guid: 32,
                        new_count: Some(1),
                    },
                ],
                delete_source_item_guid: None,
            });
        assert_eq!(
            sqls(request),
            vec![
                CharStatements::UPD_CHAR_MONEY.sql(),
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
                CharStatements::UPD_CHAR_INVENTORY_SLOT.sql(),
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
            ]
        );

        let fully_merged =
            VendorTradePersistenceRequestLikeCpp::Buyback(VendorBuybackPersistenceLikeCpp {
                player_guid: 7,
                money_before: 500,
                money_after: 450,
                destinations: Vec::new(),
                delete_source_item_guid: Some(32),
            });
        assert_eq!(
            sqls(fully_merged),
            vec![
                CharStatements::UPD_CHAR_MONEY.sql(),
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::DEL_ITEM_INSTANCE.sql(),
            ]
        );
    }

    #[test]
    fn sale_preserves_eviction_split_clone_link_then_money_order_like_cpp() {
        let request = VendorTradePersistenceRequestLikeCpp::Sale(VendorSalePersistenceLikeCpp {
            player_guid: 7,
            money_before: 100,
            money_after: 200,
            evicted_buyback_item_guid: Some(40),
            sold_item: VendorSaleItemPersistenceLikeCpp::PartialStack {
                source_item_guid: 41,
                source_count_after: 3,
                sold_clone: VendorSoldClonePersistenceLikeCpp {
                    item_guid: 42,
                    item_entry: 1001,
                    owner_guid: 7,
                    creator_guid: 0,
                    gift_creator_guid: 0,
                    count: 2,
                    expiration: 0,
                    charges: String::new(),
                    enchantments: String::new(),
                    flags: 0,
                    durability: 10,
                    create_played_time: 0,
                    random_properties_id: 0,
                    property_seed: 0,
                    context: 0,
                    buyback_slot: 75,
                },
            },
        });
        assert_eq!(
            sqls(request),
            vec![
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::DEL_ITEM_INSTANCE.sql(),
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
                CharStatements::INS_ITEM_INSTANCE_CLONE.sql(),
                CharStatements::INS_CHAR_INVENTORY.sql(),
                CharStatements::UPD_CHAR_MONEY.sql(),
            ]
        );
    }

    #[test]
    fn refund_preserves_destroy_money_returned_stacks_currency_and_cleanup_order_like_cpp() {
        let request =
            VendorTradePersistenceRequestLikeCpp::Refund(VendorRefundPersistenceLikeCpp {
                player_guid: 7,
                refunded_item_guid: 50,
                money_before: 100,
                money_after: 200,
                existing_stacks: vec![VendorExistingStackPersistenceLikeCpp {
                    item_guid: 51,
                    new_count: 3,
                }],
                new_stacks: vec![VendorRefundReturnedStackPersistenceLikeCpp {
                    item_guid: 52,
                    item_entry: 1002,
                    owner_guid: 7,
                    count: 2,
                    durability: 20,
                    inventory_slot: 26,
                }],
                currency_save: currency_save(7),
            });
        assert_eq!(
            sqls(request),
            vec![
                CharStatements::DEL_ITEM_REFUND_INSTANCE.sql(),
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::DEL_ITEM_INSTANCE.sql(),
                CharStatements::UPD_CHAR_MONEY.sql(),
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
                CharStatements::INS_ITEM_INSTANCE.sql(),
                CharStatements::INS_CHAR_INVENTORY.sql(),
                CharStatements::UPD_PLAYER_CURRENCY.sql(),
            ]
        );
        assert_eq!(
            refund_cleanup_statements_like_cpp(VendorRefundCleanupPersistenceLikeCpp {
                item_guid: 50,
                flags_after: 0,
            })
            .map(|statement| statement.sql().to_owned()),
            [
                CharStatements::DEL_ITEM_REFUND_INSTANCE.sql().to_owned(),
                CharStatements::UPD_ITEM_INSTANCE_FLAGS.sql().to_owned(),
            ]
        );
    }
}
