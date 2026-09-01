//! MariaDB adapter for Player-owned inventory mutations.

use std::sync::Arc;

use wow_persistence::{
    InventoryItemMutablePersistenceLikeCpp, InventoryLinkPersistenceLikeCpp,
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerInventoryPersistencePortLikeCpp,
    PlayerInventoryPersistenceRequestLikeCpp, QuestStatusPersistenceLikeCpp,
};

use crate::{
    CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction, SqlTransactionCommitError,
};

const QUEST_STATUS_REWARDED_LIKE_CPP: u8 = 6;

struct InventoryTransactionBuilderLikeCpp {
    transaction: SqlTransaction,
    #[cfg(test)]
    statement_sqls: Vec<(String, Option<u64>)>,
}

impl InventoryTransactionBuilderLikeCpp {
    fn new() -> Self {
        Self {
            transaction: SqlTransaction::new(),
            #[cfg(test)]
            statement_sqls: Vec::new(),
        }
    }

    fn append(&mut self, statement: PreparedStatement) {
        #[cfg(test)]
        self.statement_sqls.push((statement.sql().to_owned(), None));
        self.transaction.append(statement);
    }

    fn append_expect_rows_affected(&mut self, statement: PreparedStatement, expected: u64) {
        #[cfg(test)]
        self.statement_sqls
            .push((statement.sql().to_owned(), Some(expected)));
        self.transaction
            .append_expect_rows_affected(statement, expected);
    }

    fn finish(self) -> SqlTransaction {
        self.transaction
    }
}

fn append_mutable_item_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    item: &InventoryItemMutablePersistenceLikeCpp,
) {
    let mut statement =
        PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_STORAGE_MUTABLE);
    statement.set_u32(0, item.count);
    statement.set_u32(1, item.expiration);
    statement.set_string(2, &item.charges);
    statement.set_u32(3, item.flags);
    statement.set_string(4, &item.enchantments);
    statement.set_u32(5, item.durability);
    statement.set_u32(6, item.played_time);
    statement.set_u64(7, item.item_guid);
    transaction.append(statement);
}

fn append_delete_inventory_link_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    owner_guid: u64,
    item_guid: u64,
) {
    let mut statement = PreparedStatement::for_statement(CharStatements::DEL_CHAR_INVENTORY_ITEM);
    statement.set_u64(0, owner_guid);
    statement.set_u64(1, item_guid);
    transaction.append(statement);
}

fn append_replace_inventory_link_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    link: InventoryLinkPersistenceLikeCpp,
) {
    let mut statement = PreparedStatement::for_statement(CharStatements::REP_CHAR_INVENTORY_ITEM);
    statement.set_u64(0, link.owner_guid);
    statement.set_u64(1, link.bag_guid);
    statement.set_u8(2, link.slot);
    statement.set_u64(3, link.item_guid);
    transaction.append(statement);
}

fn append_full_item_cleanup_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    item_guid: u64,
) {
    for statement_kind in [
        CharStatements::DEL_ITEM_REFUND_INSTANCE,
        CharStatements::DEL_ITEM_BOP_TRADE,
        CharStatements::DEL_ITEM_INSTANCE_GEMS,
        CharStatements::DEL_ITEM_INSTANCE_TRANSMOG,
        CharStatements::DEL_GIFT,
        CharStatements::DEL_ITEMCONTAINER_ITEMS,
        CharStatements::DEL_ITEMCONTAINER_MONEY,
    ] {
        let mut statement = PreparedStatement::for_statement(statement_kind);
        statement.set_u64(0, item_guid);
        transaction.append(statement);
    }
}

fn append_delete_item_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    item_guid: u64,
) {
    let mut statement = PreparedStatement::for_statement(CharStatements::DEL_ITEM_INSTANCE);
    statement.set_u64(0, item_guid);
    transaction.append(statement);
}

fn append_quest_status_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    owner_guid: u64,
    status: &QuestStatusPersistenceLikeCpp,
) {
    if status.status == QUEST_STATUS_REWARDED_LIKE_CPP {
        let mut rewarded =
            PreparedStatement::for_statement(CharStatements::INS_CHAR_QUESTSTATUS_REWARDED);
        rewarded.set_u64(0, owner_guid);
        rewarded.set_u32(1, status.quest_id);
        transaction.append(rewarded);
        let mut delete = PreparedStatement::for_statement(CharStatements::DEL_CHAR_QUEST_STATUS);
        delete.set_u64(0, owner_guid);
        delete.set_u32(1, status.quest_id);
        transaction.append(delete);
    } else {
        let mut save = PreparedStatement::for_statement(CharStatements::INS_CHAR_QUEST_STATUS);
        save.set_u64(0, owner_guid);
        save.set_u32(1, status.quest_id);
        save.set_u8(2, status.status);
        save.set_u8(3, u8::from(status.explored));
        save.set_i64(4, status.accept_time_secs);
        save.set_i64(5, status.end_time_secs);
        transaction.append(save);
    }
    let mut delete_objectives =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
    delete_objectives.set_u64(0, owner_guid);
    delete_objectives.set_u32(1, status.quest_id);
    transaction.append(delete_objectives);
    if status.status == QUEST_STATUS_REWARDED_LIKE_CPP {
        return;
    }
    for objective in &status.objectives {
        let mut replace =
            PreparedStatement::for_statement(CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES);
        replace.set_u64(0, owner_guid);
        replace.set_u32(1, status.quest_id);
        replace.set_u8(2, objective.objective_index);
        replace.set_i32(3, objective.count);
        transaction.append(replace);
    }
}

fn inventory_transaction_like_cpp(
    request: &PlayerInventoryPersistenceRequestLikeCpp,
) -> InventoryTransactionBuilderLikeCpp {
    let mut transaction = InventoryTransactionBuilderLikeCpp::new();
    match request {
        PlayerInventoryPersistenceRequestLikeCpp::StorageMove(request) => {
            for item in &request.mutable_items {
                append_mutable_item_like_cpp(&mut transaction, item);
            }
            if let Some(item_guid) = request.delete_source_link_item_guid {
                append_delete_inventory_link_like_cpp(
                    &mut transaction,
                    request.owner_guid,
                    item_guid,
                );
            }
            if let Some(link) = request.destination_link {
                append_replace_inventory_link_like_cpp(&mut transaction, link);
            } else if let Some(item_guid) = request.fully_merged_source_item_guid {
                append_full_item_cleanup_like_cpp(&mut transaction, item_guid);
                append_delete_item_like_cpp(&mut transaction, item_guid);
            }
            for status in &request.quest_statuses {
                append_quest_status_like_cpp(&mut transaction, request.owner_guid, status);
            }
        }
        PlayerInventoryPersistenceRequestLikeCpp::Equip(request) => {
            append_mutable_item_like_cpp(&mut transaction, &request.mutable_item);
            append_delete_inventory_link_like_cpp(
                &mut transaction,
                request.delete_source_link_owner_guid,
                request.delete_source_link_item_guid,
            );
            append_replace_inventory_link_like_cpp(&mut transaction, request.destination_link);
        }
        PlayerInventoryPersistenceRequestLikeCpp::StackMerge(request) => {
            append_mutable_item_like_cpp(&mut transaction, &request.destination_item);
            match &request.source {
                wow_persistence::InventoryStackMergeSourcePersistenceLikeCpp::Retained(item) => {
                    append_mutable_item_like_cpp(&mut transaction, item);
                }
                wow_persistence::InventoryStackMergeSourcePersistenceLikeCpp::FullyMerged {
                    item_guid,
                } => {
                    append_delete_inventory_link_like_cpp(
                        &mut transaction,
                        request.owner_guid,
                        *item_guid,
                    );
                    append_full_item_cleanup_like_cpp(&mut transaction, *item_guid);
                    append_delete_item_like_cpp(&mut transaction, *item_guid);
                }
            }
        }
        PlayerInventoryPersistenceRequestLikeCpp::Swap(request) => {
            append_mutable_item_like_cpp(&mut transaction, &request.source_item);
            append_mutable_item_like_cpp(&mut transaction, &request.destination_item);
            for link in &request.child_links {
                append_replace_inventory_link_like_cpp(&mut transaction, *link);
            }
            append_replace_inventory_link_like_cpp(&mut transaction, request.source_link);
            append_replace_inventory_link_like_cpp(&mut transaction, request.destination_link);
        }
        PlayerInventoryPersistenceRequestLikeCpp::PartialDestroy(request) => {
            let mut update =
                PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_COUNT);
            update.set_u32(0, request.new_count);
            update.set_u64(1, request.item_guid);
            transaction.append(update);
            for status in &request.quest_statuses {
                append_quest_status_like_cpp(&mut transaction, request.owner_guid, status);
            }
        }
        PlayerInventoryPersistenceRequestLikeCpp::GraphDestroy(request) => {
            for node in &request.nodes {
                if let Some(expected_owner_db_guid) = node.expected_owner_db_guid {
                    let mut delete_inventory = PreparedStatement::for_statement(
                        CharStatements::DEL_CHAR_INVENTORY_ITEM_BY_OWNER,
                    );
                    delete_inventory.set_u64(0, request.owner_guid);
                    delete_inventory.set_u64(1, node.item_guid);
                    delete_inventory.set_u64(2, expected_owner_db_guid);
                    transaction.append_expect_rows_affected(delete_inventory, 1);
                } else {
                    append_delete_inventory_link_like_cpp(
                        &mut transaction,
                        request.owner_guid,
                        node.item_guid,
                    );
                }
                append_full_item_cleanup_like_cpp(&mut transaction, node.item_guid);
                if let Some(expected_owner_db_guid) = node.expected_owner_db_guid {
                    let mut delete_item = PreparedStatement::for_statement(
                        CharStatements::DEL_ITEM_INSTANCE_BY_GUID_AND_OWNER,
                    );
                    delete_item.set_u64(0, node.item_guid);
                    delete_item.set_u64(1, expected_owner_db_guid);
                    transaction.append_expect_rows_affected(delete_item, 1);
                } else {
                    append_delete_item_like_cpp(&mut transaction, node.item_guid);
                }
            }
            for status in &request.quest_statuses {
                append_quest_status_like_cpp(&mut transaction, request.owner_guid, status);
            }
        }
    }
    transaction
}

pub struct MariaDbPlayerInventoryPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbPlayerInventoryPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl PlayerInventoryPersistencePortLikeCpp for MariaDbPlayerInventoryPersistenceAdapterLikeCpp {
    fn persist_inventory_mutation_like_cpp(
        &self,
        request: PlayerInventoryPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            match inventory_transaction_like_cpp(&request)
                .finish()
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
    use crate::StatementDef;
    use wow_persistence::{
        InventoryDestroyNodePersistenceLikeCpp, InventoryEquipPersistenceLikeCpp,
        InventoryGraphDestroyPersistenceLikeCpp, InventoryStackMergePersistenceLikeCpp,
        InventoryStackMergeSourcePersistenceLikeCpp, InventoryStorageMovePersistenceLikeCpp,
        InventorySwapPersistenceLikeCpp, QuestObjectiveCountPersistenceLikeCpp,
    };

    fn mutable(item_guid: u64) -> InventoryItemMutablePersistenceLikeCpp {
        InventoryItemMutablePersistenceLikeCpp {
            item_guid,
            count: 2,
            expiration: 3,
            charges: "4 ".into(),
            flags: 5,
            enchantments: "6 ".into(),
            durability: 7,
            played_time: 8,
        }
    }

    fn link(item_guid: u64, bag_guid: u64, slot: u8) -> InventoryLinkPersistenceLikeCpp {
        InventoryLinkPersistenceLikeCpp {
            owner_guid: 10,
            bag_guid,
            slot,
            item_guid,
        }
    }

    fn quest(status: u8) -> QuestStatusPersistenceLikeCpp {
        QuestStatusPersistenceLikeCpp {
            quest_id: 91,
            status,
            explored: true,
            accept_time_secs: 92,
            end_time_secs: 93,
            objectives: vec![QuestObjectiveCountPersistenceLikeCpp {
                objective_index: 2,
                count: 4,
            }],
        }
    }

    fn sqls(request: PlayerInventoryPersistenceRequestLikeCpp) -> Vec<String> {
        inventory_transaction_like_cpp(&request)
            .statement_sqls
            .into_iter()
            .map(|(sql, _)| sql)
            .collect()
    }

    #[test]
    fn fully_merged_storage_move_preserves_cleanup_then_quest_order_like_cpp() {
        let request = PlayerInventoryPersistenceRequestLikeCpp::StorageMove(
            InventoryStorageMovePersistenceLikeCpp {
                owner_guid: 10,
                mutable_items: vec![mutable(20)],
                delete_source_link_item_guid: Some(20),
                destination_link: None,
                fully_merged_source_item_guid: Some(20),
                quest_statuses: vec![quest(3)],
            },
        );
        assert_eq!(
            sqls(request),
            [
                CharStatements::UPD_ITEM_INSTANCE_STORAGE_MUTABLE.sql(),
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::DEL_ITEM_REFUND_INSTANCE.sql(),
                CharStatements::DEL_ITEM_BOP_TRADE.sql(),
                CharStatements::DEL_ITEM_INSTANCE_GEMS.sql(),
                CharStatements::DEL_ITEM_INSTANCE_TRANSMOG.sql(),
                CharStatements::DEL_GIFT.sql(),
                CharStatements::DEL_ITEMCONTAINER_ITEMS.sql(),
                CharStatements::DEL_ITEMCONTAINER_MONEY.sql(),
                CharStatements::DEL_ITEM_INSTANCE.sql(),
                CharStatements::INS_CHAR_QUEST_STATUS.sql(),
                CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql(),
                CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES.sql(),
            ]
        );
    }

    #[test]
    fn equip_merge_and_swap_keep_their_distinct_cplusplus_orders() {
        let equip =
            PlayerInventoryPersistenceRequestLikeCpp::Equip(InventoryEquipPersistenceLikeCpp {
                mutable_item: mutable(20),
                delete_source_link_owner_guid: 10,
                delete_source_link_item_guid: 20,
                destination_link: link(20, 30, 4),
            });
        assert_eq!(
            sqls(equip),
            [
                CharStatements::UPD_ITEM_INSTANCE_STORAGE_MUTABLE.sql(),
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::REP_CHAR_INVENTORY_ITEM.sql(),
            ]
        );

        let retained_merge = PlayerInventoryPersistenceRequestLikeCpp::StackMerge(
            InventoryStackMergePersistenceLikeCpp {
                owner_guid: 10,
                destination_item: mutable(21),
                source: InventoryStackMergeSourcePersistenceLikeCpp::Retained(mutable(20)),
            },
        );
        assert_eq!(
            sqls(retained_merge),
            [
                CharStatements::UPD_ITEM_INSTANCE_STORAGE_MUTABLE.sql(),
                CharStatements::UPD_ITEM_INSTANCE_STORAGE_MUTABLE.sql(),
            ]
        );

        let swap =
            PlayerInventoryPersistenceRequestLikeCpp::Swap(InventorySwapPersistenceLikeCpp {
                source_item: mutable(20),
                destination_item: mutable(21),
                child_links: vec![link(22, 20, 1)],
                source_link: link(20, 31, 2),
                destination_link: link(21, 30, 3),
            });
        assert_eq!(
            sqls(swap),
            [
                CharStatements::UPD_ITEM_INSTANCE_STORAGE_MUTABLE.sql(),
                CharStatements::UPD_ITEM_INSTANCE_STORAGE_MUTABLE.sql(),
                CharStatements::REP_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::REP_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::REP_CHAR_INVENTORY_ITEM.sql(),
            ]
        );
    }

    #[test]
    fn guarded_graph_destroy_keeps_both_affected_row_guards_and_rewarded_order() {
        let request = PlayerInventoryPersistenceRequestLikeCpp::GraphDestroy(
            InventoryGraphDestroyPersistenceLikeCpp {
                owner_guid: 10,
                nodes: vec![InventoryDestroyNodePersistenceLikeCpp {
                    item_guid: 20,
                    expected_owner_db_guid: Some(30),
                }],
                quest_statuses: vec![quest(QUEST_STATUS_REWARDED_LIKE_CPP)],
            },
        );
        let builder = inventory_transaction_like_cpp(&request);
        assert_eq!(
            builder
                .statement_sqls
                .iter()
                .map(|(sql, _)| sql.as_str())
                .collect::<Vec<_>>(),
            [
                CharStatements::DEL_CHAR_INVENTORY_ITEM_BY_OWNER.sql(),
                CharStatements::DEL_ITEM_REFUND_INSTANCE.sql(),
                CharStatements::DEL_ITEM_BOP_TRADE.sql(),
                CharStatements::DEL_ITEM_INSTANCE_GEMS.sql(),
                CharStatements::DEL_ITEM_INSTANCE_TRANSMOG.sql(),
                CharStatements::DEL_GIFT.sql(),
                CharStatements::DEL_ITEMCONTAINER_ITEMS.sql(),
                CharStatements::DEL_ITEMCONTAINER_MONEY.sql(),
                CharStatements::DEL_ITEM_INSTANCE_BY_GUID_AND_OWNER.sql(),
                CharStatements::INS_CHAR_QUESTSTATUS_REWARDED.sql(),
                CharStatements::DEL_CHAR_QUEST_STATUS.sql(),
                CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql(),
            ]
        );
        assert_eq!(builder.statement_sqls[0].1, Some(1));
        assert_eq!(builder.statement_sqls[8].1, Some(1));
    }
}
