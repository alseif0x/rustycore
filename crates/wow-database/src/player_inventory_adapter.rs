//! MariaDB adapter for Player-owned inventory mutations.

use std::sync::Arc;

use wow_persistence::{
    InventoryItemMutablePersistenceLikeCpp, InventoryLinkPersistenceLikeCpp,
    LootExistingStackPersistenceLikeCpp, LootNewStackPersistenceLikeCpp, PersistenceFutureLikeCpp,
    PersistenceOutcomeLikeCpp, PlayerInventoryPersistencePortLikeCpp,
    PlayerInventoryPersistenceRequestLikeCpp, QuestItemExistingStackPersistenceLikeCpp,
    QuestItemNewStackPersistenceLikeCpp, QuestTurnInItemPersistenceLikeCpp,
    StoredItemLootSourcePersistenceLikeCpp,
};

use crate::{
    CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction, SqlTransactionCommitError,
};

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

fn append_loot_existing_stack_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    stack: LootExistingStackPersistenceLikeCpp,
) {
    let mut count = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_COUNT);
    count.set_u32(0, stack.new_count);
    count.set_u64(1, stack.item_guid);
    transaction.append_expect_rows_affected(count, 1);

    if let Some(dynamic_flags) = stack.dynamic_flags {
        let mut flags = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
        flags.set_u32(0, dynamic_flags);
        flags.set_u64(1, stack.item_guid);
        transaction.append_expect_rows_affected(flags, 1);
    }
}

fn append_loot_new_stack_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    stack: LootNewStackPersistenceLikeCpp,
) {
    let mut item =
        PreparedStatement::for_statement(CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT);
    item.set_u64(0, stack.item_guid);
    item.set_u32(1, stack.entry_id);
    item.set_u64(2, stack.owner_guid);
    item.set_u32(3, stack.count);
    item.set_u32(4, stack.max_durability);
    item.set_u32(5, stack.dynamic_flags);
    item.set_i32(6, stack.random_properties_id);
    item.set_i32(7, stack.random_properties_seed);
    item.set_u8(8, stack.item_context);
    transaction.append_expect_rows_affected(item, 1);

    let mut link = PreparedStatement::for_statement(CharStatements::INS_CHAR_INVENTORY);
    link.set_u64(0, stack.owner_guid);
    link.set_u8(1, stack.slot);
    link.set_u64(2, stack.item_guid);
    transaction.append_expect_rows_affected(link, 1);
}

fn append_stored_item_loot_source_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    source: StoredItemLootSourcePersistenceLikeCpp,
) {
    let mut statement = PreparedStatement::for_statement(CharStatements::DEL_ITEMCONTAINER_ITEM);
    statement.set_u64(0, source.item_guid);
    statement.set_u32(1, source.item_id);
    statement.set_u32(2, source.count);
    statement.set_u32(3, source.loot_list_id);
    transaction.append_expect_rows_affected(statement, 1);
}

fn append_quest_status_projection_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    owner_guid: u64,
    status: &wow_persistence::QuestStatusPersistenceLikeCpp,
) {
    for statement in
        crate::player_quest_adapter::player_quest_status_statements_like_cpp(owner_guid, status)
    {
        transaction.append(statement);
    }
}

fn append_quest_item_existing_stack_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    stack: QuestItemExistingStackPersistenceLikeCpp,
) {
    let mut count = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_COUNT);
    count.set_u32(0, stack.new_count);
    count.set_u64(1, stack.item_guid);
    transaction.append(count);

    if let Some(dynamic_flags) = stack.dynamic_flags {
        let mut flags = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
        flags.set_u32(0, dynamic_flags);
        flags.set_u64(1, stack.item_guid);
        transaction.append(flags);
    }
}

fn append_quest_item_new_stack_like_cpp(
    transaction: &mut InventoryTransactionBuilderLikeCpp,
    stack: QuestItemNewStackPersistenceLikeCpp,
) {
    let mut item = PreparedStatement::for_statement(CharStatements::INS_ITEM_INSTANCE);
    item.set_u64(0, stack.item_guid);
    item.set_u32(1, stack.entry_id);
    item.set_u64(2, stack.owner_guid);
    item.set_u32(3, stack.count);
    item.set_u32(4, stack.max_durability);
    transaction.append(item);

    if stack.dynamic_flags != 0 {
        let mut flags = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
        flags.set_u32(0, stack.dynamic_flags);
        flags.set_u64(1, stack.item_guid);
        transaction.append(flags);
    }

    let mut link = PreparedStatement::for_statement(CharStatements::REP_CHAR_INVENTORY_ITEM);
    link.set_u64(0, stack.owner_guid);
    link.set_u64(1, stack.bag_guid);
    link.set_u8(2, stack.slot);
    link.set_u64(3, stack.item_guid);
    transaction.append(link);
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
                append_quest_status_projection_like_cpp(
                    &mut transaction,
                    request.owner_guid,
                    status,
                );
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
                append_quest_status_projection_like_cpp(
                    &mut transaction,
                    request.owner_guid,
                    status,
                );
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
                append_quest_status_projection_like_cpp(
                    &mut transaction,
                    request.owner_guid,
                    status,
                );
            }
        }
        PlayerInventoryPersistenceRequestLikeCpp::LootDisenchantBatch(request) => {
            for stack in &request.existing_stacks {
                append_loot_existing_stack_like_cpp(&mut transaction, *stack);
            }
            for stack in &request.new_stacks {
                append_loot_new_stack_like_cpp(&mut transaction, *stack);
            }
        }
        PlayerInventoryPersistenceRequestLikeCpp::LootQuestBoundProgress(request) => {
            for status in &request.quest_statuses {
                append_quest_status_projection_like_cpp(
                    &mut transaction,
                    request.owner_guid,
                    status,
                );
            }
            if let Some(source) = request.stored_item_source {
                append_stored_item_loot_source_like_cpp(&mut transaction, source);
            }
        }
        PlayerInventoryPersistenceRequestLikeCpp::LootDirectItemGrant(request) => {
            for stack in &request.existing_stacks {
                append_loot_existing_stack_like_cpp(&mut transaction, *stack);
            }
            for stack in &request.new_stacks {
                append_loot_new_stack_like_cpp(&mut transaction, *stack);
            }
            if let Some(source) = request.stored_item_source {
                append_stored_item_loot_source_like_cpp(&mut transaction, source);
            }
        }
        PlayerInventoryPersistenceRequestLikeCpp::QuestItemGrant(request) => {
            for stack in &request.existing_stacks {
                append_quest_item_existing_stack_like_cpp(&mut transaction, *stack);
            }
            for stack in &request.new_stacks {
                append_quest_item_new_stack_like_cpp(&mut transaction, *stack);
            }
        }
        PlayerInventoryPersistenceRequestLikeCpp::QuestTurnIn(request) => {
            for item in &request.items {
                match item {
                    QuestTurnInItemPersistenceLikeCpp::Update {
                        item_guid,
                        new_count,
                    } => {
                        let mut statement = PreparedStatement::for_statement(
                            CharStatements::UPD_ITEM_INSTANCE_COUNT,
                        );
                        statement.set_u32(0, *new_count);
                        statement.set_u64(1, *item_guid);
                        transaction.append(statement);
                    }
                    QuestTurnInItemPersistenceLikeCpp::Delete { item_guid } => {
                        append_delete_inventory_link_like_cpp(
                            &mut transaction,
                            request.owner_guid,
                            *item_guid,
                        );
                        append_delete_item_like_cpp(&mut transaction, *item_guid);
                    }
                }
            }
            for statement in
                crate::player_lifecycle_adapter::player_currency_save_statements_like_cpp(
                    &request.currency_save,
                )
            {
                transaction.append(statement);
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
        InventorySwapPersistenceLikeCpp, LootDirectItemGrantPersistenceLikeCpp,
        LootDisenchantBatchPersistenceLikeCpp, LootQuestBoundProgressPersistenceLikeCpp,
        PlayerCurrencySaveKindLikeCpp, PlayerCurrencySaveRequestLikeCpp,
        PlayerCurrencySaveRowLikeCpp, QuestItemGrantPersistenceLikeCpp,
        QuestObjectiveCountPersistenceLikeCpp, QuestStatusPersistenceLikeCpp,
        QuestTurnInPersistenceLikeCpp,
    };

    const QUEST_STATUS_REWARDED_LIKE_CPP: u8 = 6;

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

    fn loot_existing(dynamic_flags: Option<u32>) -> LootExistingStackPersistenceLikeCpp {
        LootExistingStackPersistenceLikeCpp {
            item_guid: 20,
            new_count: 3,
            dynamic_flags,
        }
    }

    fn loot_new() -> LootNewStackPersistenceLikeCpp {
        LootNewStackPersistenceLikeCpp {
            item_guid: 21,
            entry_id: 22,
            owner_guid: 10,
            count: 4,
            max_durability: 5,
            dynamic_flags: 6,
            random_properties_id: 7,
            random_properties_seed: 8,
            item_context: 9,
            slot: 10,
        }
    }

    fn loot_source() -> StoredItemLootSourcePersistenceLikeCpp {
        StoredItemLootSourcePersistenceLikeCpp {
            item_guid: 30,
            item_id: 31,
            count: 2,
            loot_list_id: 3,
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

    #[test]
    fn disenchant_batch_preserves_existing_then_new_stack_order_like_cpp() {
        let insert_sql = CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT.sql();
        assert!(insert_sql.contains("randomPropertiesId"));
        assert!(insert_sql.contains("randomPropertiesSeed"));
        assert!(insert_sql.contains("context"));
        assert!(
            insert_sql.contains("'', '', ?, ?, ?, ?"),
            "stored-new-item flags must remain a bound parameter"
        );
        let request = PlayerInventoryPersistenceRequestLikeCpp::LootDisenchantBatch(
            LootDisenchantBatchPersistenceLikeCpp {
                existing_stacks: vec![loot_existing(Some(6))],
                new_stacks: vec![loot_new()],
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
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
                CharStatements::UPD_ITEM_INSTANCE_FLAGS.sql(),
                CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT.sql(),
                CharStatements::INS_CHAR_INVENTORY.sql(),
            ]
        );
        assert!(
            builder
                .statement_sqls
                .iter()
                .all(|(_, expected)| *expected == Some(1))
        );
    }

    #[test]
    fn quest_bound_loot_preserves_quest_then_stored_source_order_like_cpp() {
        let request = PlayerInventoryPersistenceRequestLikeCpp::LootQuestBoundProgress(
            LootQuestBoundProgressPersistenceLikeCpp {
                owner_guid: 10,
                quest_statuses: vec![quest(3)],
                stored_item_source: Some(loot_source()),
            },
        );
        assert_eq!(
            sqls(request),
            [
                CharStatements::INS_CHAR_QUEST_STATUS.sql(),
                CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql(),
                CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES.sql(),
                CharStatements::DEL_ITEMCONTAINER_ITEM.sql(),
            ]
        );
    }

    #[test]
    fn direct_loot_grant_preserves_stacks_then_stored_source_order_like_cpp() {
        let request = PlayerInventoryPersistenceRequestLikeCpp::LootDirectItemGrant(
            LootDirectItemGrantPersistenceLikeCpp {
                existing_stacks: vec![loot_existing(None)],
                new_stacks: vec![loot_new()],
                stored_item_source: Some(loot_source()),
            },
        );
        assert_eq!(
            sqls(request),
            [
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
                CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT.sql(),
                CharStatements::INS_CHAR_INVENTORY.sql(),
                CharStatements::DEL_ITEMCONTAINER_ITEM.sql(),
            ]
        );
    }

    #[test]
    fn quest_item_grant_preserves_existing_then_new_stack_order_like_cpp() {
        let request = PlayerInventoryPersistenceRequestLikeCpp::QuestItemGrant(
            QuestItemGrantPersistenceLikeCpp {
                existing_stacks: vec![QuestItemExistingStackPersistenceLikeCpp {
                    item_guid: 20,
                    new_count: 3,
                    dynamic_flags: Some(4),
                }],
                new_stacks: vec![QuestItemNewStackPersistenceLikeCpp {
                    item_guid: 21,
                    entry_id: 22,
                    owner_guid: 10,
                    count: 5,
                    max_durability: 6,
                    dynamic_flags: 7,
                    bag_guid: 30,
                    slot: 8,
                }],
            },
        );
        assert_eq!(
            sqls(request),
            [
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
                CharStatements::UPD_ITEM_INSTANCE_FLAGS.sql(),
                CharStatements::INS_ITEM_INSTANCE.sql(),
                CharStatements::UPD_ITEM_INSTANCE_FLAGS.sql(),
                CharStatements::REP_CHAR_INVENTORY_ITEM.sql(),
            ]
        );
    }

    #[test]
    fn quest_turnin_preserves_item_then_currency_order_like_cpp() {
        let request =
            PlayerInventoryPersistenceRequestLikeCpp::QuestTurnIn(QuestTurnInPersistenceLikeCpp {
                owner_guid: 10,
                items: vec![
                    QuestTurnInItemPersistenceLikeCpp::Update {
                        item_guid: 20,
                        new_count: 2,
                    },
                    QuestTurnInItemPersistenceLikeCpp::Delete { item_guid: 21 },
                ],
                currency_save: PlayerCurrencySaveRequestLikeCpp {
                    player_guid: 10,
                    rows: vec![PlayerCurrencySaveRowLikeCpp {
                        kind: PlayerCurrencySaveKindLikeCpp::Changed,
                        currency_id: 22,
                        quantity: 23,
                        weekly_quantity: 24,
                        tracked_quantity: 25,
                        increased_cap_quantity: 26,
                        earned_quantity: 27,
                        flags: 28,
                    }],
                },
            });
        assert_eq!(
            sqls(request),
            [
                CharStatements::UPD_ITEM_INSTANCE_COUNT.sql(),
                CharStatements::DEL_CHAR_INVENTORY_ITEM.sql(),
                CharStatements::DEL_ITEM_INSTANCE.sql(),
                CharStatements::UPD_PLAYER_CURRENCY.sql(),
            ]
        );
    }
}
