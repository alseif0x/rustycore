//! MariaDB adapter for bounded void-storage unlock and slot-swap writes.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerMoneyTransactionOutcomeLikeCpp, VoidStorageItemWriteLikeCpp,
    VoidStorageMergedInventoryItemWriteLikeCpp, VoidStorageNewInventoryItemWriteLikeCpp,
    VoidStoragePersistencePortLikeCpp, VoidStorageQuestStatusWriteLikeCpp,
    VoidStorageSwapWriteRequestLikeCpp, VoidStorageTransferWriteRequestLikeCpp,
    VoidStorageUnlockWriteRequestLikeCpp, VoidStorageWithdrawalInventoryWriteLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction};

fn void_storage_replace_statement_like_cpp(
    player_guid: u64,
    slot: u8,
    item: &VoidStorageItemWriteLikeCpp,
) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(CharStatements::REP_CHAR_VOID_STORAGE_ITEM);
    statement.set_u64(0, item.item_id);
    statement.set_u64(1, player_guid);
    statement.set_u32(2, item.item_entry);
    statement.set_u8(3, slot);
    statement.set_u64(4, item.creator_guid);
    statement.set_u32(5, item.fixed_scaling_level);
    statement.set_i32(6, item.random_properties_id);
    statement.set_i32(7, item.random_properties_seed);
    statement.set_u8(8, item.context);
    statement
}

fn void_storage_delete_slot_statement_like_cpp(player_guid: u64, slot: u8) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT);
    statement.set_u8(0, slot);
    statement.set_u64(1, player_guid);
    statement
}

fn void_storage_delete_all_statement_like_cpp(player_guid: u64) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_CHAR_GUID);
    statement.set_u64(0, player_guid);
    statement
}

fn void_storage_unlock_statements_like_cpp(
    request: &VoidStorageUnlockWriteRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut update_money = PreparedStatement::for_statement(CharStatements::UPD_CHAR_MONEY);
    update_money.set_u64(0, request.money_after);
    update_money.set_u64(1, request.player_guid);

    let mut update_flags = PreparedStatement::for_statement(CharStatements::UPD_CHAR_PLAYER_FLAGS);
    update_flags.set_u32(0, request.player_flags_after);
    update_flags.set_u64(1, request.player_guid);

    vec![
        update_money,
        update_flags,
        void_storage_delete_all_statement_like_cpp(request.player_guid),
    ]
}

fn void_storage_swap_statements_like_cpp(
    request: &VoidStorageSwapWriteRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = vec![void_storage_replace_statement_like_cpp(
        request.player_guid,
        request.new_slot,
        &request.source_item,
    )];
    statements.push(match &request.destination_item {
        Some(item) => {
            void_storage_replace_statement_like_cpp(request.player_guid, request.old_slot, item)
        }
        None => void_storage_delete_slot_statement_like_cpp(request.player_guid, request.old_slot),
    });
    statements
}

fn void_storage_destroy_item_statements_like_cpp(
    player_guid: u64,
    item_db_guid: u64,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(9);
    let mut delete_inventory =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_INVENTORY_ITEM);
    delete_inventory.set_u64(0, player_guid);
    delete_inventory.set_u64(1, item_db_guid);
    statements.push(delete_inventory);
    for cleanup_kind in [
        CharStatements::DEL_ITEM_REFUND_INSTANCE,
        CharStatements::DEL_ITEM_BOP_TRADE,
        CharStatements::DEL_ITEM_INSTANCE_GEMS,
        CharStatements::DEL_ITEM_INSTANCE_TRANSMOG,
        CharStatements::DEL_GIFT,
        CharStatements::DEL_ITEMCONTAINER_ITEMS,
        CharStatements::DEL_ITEMCONTAINER_MONEY,
    ] {
        let mut cleanup = PreparedStatement::for_statement(cleanup_kind);
        cleanup.set_u64(0, item_db_guid);
        statements.push(cleanup);
    }
    let mut delete_item = PreparedStatement::for_statement(CharStatements::DEL_ITEM_INSTANCE);
    delete_item.set_u64(0, item_db_guid);
    statements.push(delete_item);
    statements
}

fn void_storage_new_inventory_item_statements_like_cpp(
    player_guid: u64,
    item: &VoidStorageNewInventoryItemWriteLikeCpp,
) -> [PreparedStatement; 2] {
    let mut insert = PreparedStatement::for_statement(CharStatements::INS_ITEM_INSTANCE_CLONE);
    insert.set_u64(0, item.item_db_guid);
    insert.set_u32(1, item.item_entry);
    insert.set_u64(2, player_guid);
    insert.set_u64(3, item.creator_guid);
    insert.set_u64(4, 0);
    insert.set_u32(5, item.count);
    insert.set_u32(6, 0);
    insert.set_string(7, "");
    insert.set_string(8, &item.enchantments);
    insert.set_u32(9, item.item_flags);
    insert.set_u32(10, item.max_durability);
    insert.set_u32(11, item.total_played_time);
    insert.set_i32(12, item.random_properties_id);
    insert.set_i32(13, item.random_properties_seed);
    insert.set_u8(14, item.context);

    let mut link = PreparedStatement::for_statement(CharStatements::REP_CHAR_INVENTORY_ITEM);
    link.set_u64(0, player_guid);
    link.set_u64(1, item.container_db_guid);
    link.set_u8(2, item.inventory_slot);
    link.set_u64(3, item.item_db_guid);
    [insert, link]
}

fn void_storage_merged_inventory_item_statement_like_cpp(
    item: &VoidStorageMergedInventoryItemWriteLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE);
    statement.set_u32(0, item.item_entry);
    statement.set_u64(1, item.owner_guid);
    statement.set_u64(2, item.creator_guid);
    statement.set_u64(3, item.gift_creator_guid);
    statement.set_u32(4, item.count);
    statement.set_u32(5, item.expiration);
    statement.set_string(6, &item.charges);
    statement.set_u32(7, item.dynamic_flags);
    statement.set_string(8, &item.enchantments);
    statement.set_u32(9, item.durability);
    statement.set_u32(10, item.create_played_time);
    statement.set_string(11, &item.text);
    statement.set_u32(12, item.battle_pet_species_id);
    statement.set_u32(13, item.battle_pet_breed_data);
    statement.set_u32(14, item.battle_pet_level);
    statement.set_u32(15, item.battle_pet_display_id);
    statement.set_i32(16, item.random_properties_id);
    statement.set_i32(17, item.property_seed);
    statement.set_i32(18, item.context);
    statement.set_u64(19, item.item_db_guid);
    statement
}

fn append_void_storage_quest_status_statements_like_cpp(
    statements: &mut Vec<PreparedStatement>,
    player_guid: u64,
    status: &VoidStorageQuestStatusWriteLikeCpp,
) {
    const QUEST_STATUS_REWARDED_LIKE_CPP: u8 = 6;
    if status.status == QUEST_STATUS_REWARDED_LIKE_CPP {
        let mut rewarded =
            PreparedStatement::for_statement(CharStatements::INS_CHAR_QUESTSTATUS_REWARDED);
        rewarded.set_u64(0, player_guid);
        rewarded.set_u32(1, status.quest_id);
        statements.push(rewarded);

        let mut delete_status =
            PreparedStatement::for_statement(CharStatements::DEL_CHAR_QUEST_STATUS);
        delete_status.set_u64(0, player_guid);
        delete_status.set_u32(1, status.quest_id);
        statements.push(delete_status);
    } else {
        let mut save = PreparedStatement::for_statement(CharStatements::INS_CHAR_QUEST_STATUS);
        save.set_u64(0, player_guid);
        save.set_u32(1, status.quest_id);
        save.set_u8(2, status.status);
        save.set_u8(3, u8::from(status.explored));
        save.set_i64(4, status.accept_time_secs);
        save.set_i64(5, status.end_time_secs);
        statements.push(save);
    }

    let mut delete_objectives =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST);
    delete_objectives.set_u64(0, player_guid);
    delete_objectives.set_u32(1, status.quest_id);
    statements.push(delete_objectives);

    if status.status != QUEST_STATUS_REWARDED_LIKE_CPP {
        for objective in &status.objectives {
            let mut replace =
                PreparedStatement::for_statement(CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES);
            replace.set_u64(0, player_guid);
            replace.set_u32(1, status.quest_id);
            replace.set_u8(2, objective.storage_index);
            replace.set_i32(3, objective.count);
            statements.push(replace);
        }
    }
}

fn void_storage_transfer_statements_like_cpp(
    request: &VoidStorageTransferWriteRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::new();
    let mut update_money = PreparedStatement::for_statement(CharStatements::UPD_CHAR_MONEY);
    update_money.set_u64(0, request.money_after);
    update_money.set_u64(1, request.player_guid);
    statements.push(update_money);

    for deposit in &request.deposits {
        for destroyed in &deposit.destroyed_items {
            statements.extend(void_storage_destroy_item_statements_like_cpp(
                request.player_guid,
                destroyed.item_db_guid,
            ));
        }
        statements.push(void_storage_replace_statement_like_cpp(
            request.player_guid,
            deposit.void_slot,
            &deposit.void_item,
        ));
    }

    for withdrawal in &request.withdrawals {
        match &withdrawal.inventory_write {
            VoidStorageWithdrawalInventoryWriteLikeCpp::None => {}
            VoidStorageWithdrawalInventoryWriteLikeCpp::New(item) => {
                statements.extend(void_storage_new_inventory_item_statements_like_cpp(
                    request.player_guid,
                    item,
                ));
            }
            VoidStorageWithdrawalInventoryWriteLikeCpp::MergeExisting(item) => {
                statements.push(void_storage_merged_inventory_item_statement_like_cpp(item))
            }
        }
        statements.push(void_storage_delete_slot_statement_like_cpp(
            request.player_guid,
            withdrawal.old_void_slot,
        ));
    }

    for status in &request.quest_statuses {
        append_void_storage_quest_status_statements_like_cpp(
            &mut statements,
            request.player_guid,
            status,
        );
    }
    statements
}

pub struct MariaDbVoidStoragePersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbVoidStoragePersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }

    async fn commit_and_observe_money_like_cpp(
        &self,
        statements: Vec<PreparedStatement>,
        player_guid: u64,
    ) -> PlayerMoneyTransactionOutcomeLikeCpp {
        let mut transaction = SqlTransaction::new();
        for statement in statements {
            transaction.append(statement);
        }
        crate::player_money_transaction_adapter::commit_player_money_transaction_and_observe_like_cpp(
            self.character_db.as_ref(),
            transaction,
            Some(player_guid),
        )
        .await
    }
}

impl VoidStoragePersistencePortLikeCpp for MariaDbVoidStoragePersistenceAdapterLikeCpp {
    fn persist_void_storage_unlock_like_cpp<'a>(
        &'a self,
        request: VoidStorageUnlockWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async move {
            self.commit_and_observe_money_like_cpp(
                void_storage_unlock_statements_like_cpp(&request),
                request.player_guid,
            )
            .await
        })
    }

    fn persist_void_storage_swap_like_cpp<'a>(
        &'a self,
        request: VoidStorageSwapWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async move {
            self.commit_and_observe_money_like_cpp(
                void_storage_swap_statements_like_cpp(&request),
                request.player_guid,
            )
            .await
        })
    }

    fn persist_void_storage_transfer_like_cpp<'a>(
        &'a self,
        request: VoidStorageTransferWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async move {
            self.commit_and_observe_money_like_cpp(
                void_storage_transfer_statements_like_cpp(&request),
                request.player_guid,
            )
            .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    fn item(item_id: u64) -> VoidStorageItemWriteLikeCpp {
        VoidStorageItemWriteLikeCpp {
            item_id,
            item_entry: 19_019,
            creator_guid: 7,
            fixed_scaling_level: 80,
            random_properties_id: -13,
            random_properties_seed: 29,
            context: 6,
        }
    }

    #[test]
    fn unlock_preserves_money_flags_delete_all_order_and_binds() {
        let statements =
            void_storage_unlock_statements_like_cpp(&VoidStorageUnlockWriteRequestLikeCpp {
                player_guid: 42,
                money_before: 2_000_000,
                money_after: 1_000_000,
                player_flags_after: 0x0080_0000,
            });
        assert_eq!(statements.len(), 3);
        assert_eq!(statements[0].sql(), CharStatements::UPD_CHAR_MONEY.sql());
        assert_eq!(
            statements[0].params(),
            &[SqlParam::U64(1_000_000), SqlParam::U64(42)]
        );
        assert_eq!(
            statements[1].sql(),
            CharStatements::UPD_CHAR_PLAYER_FLAGS.sql()
        );
        assert_eq!(
            statements[1].params(),
            &[SqlParam::U32(0x0080_0000), SqlParam::U64(42)]
        );
        assert_eq!(
            statements[2].sql(),
            CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_CHAR_GUID.sql()
        );
        assert_eq!(statements[2].params(), &[SqlParam::U64(42)]);
    }

    #[test]
    fn swap_preserves_destination_then_source_order_for_used_and_empty_slots() {
        let occupied = VoidStorageSwapWriteRequestLikeCpp {
            player_guid: 42,
            money_before: 100,
            money_after: 100,
            old_slot: 3,
            new_slot: 9,
            source_item: item(77),
            destination_item: Some(item(88)),
        };
        let statements = void_storage_swap_statements_like_cpp(&occupied);
        assert_eq!(statements.len(), 2);
        assert_eq!(
            statements[0].sql(),
            CharStatements::REP_CHAR_VOID_STORAGE_ITEM.sql()
        );
        assert_eq!(statements[0].params()[0], SqlParam::U64(77));
        assert_eq!(statements[0].params()[3], SqlParam::U8(9));
        assert_eq!(statements[1].params()[0], SqlParam::U64(88));
        assert_eq!(statements[1].params()[3], SqlParam::U8(3));

        let empty = VoidStorageSwapWriteRequestLikeCpp {
            destination_item: None,
            ..occupied
        };
        let statements = void_storage_swap_statements_like_cpp(&empty);
        assert_eq!(
            statements[1].sql(),
            CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT.sql()
        );
        assert_eq!(
            statements[1].params(),
            &[SqlParam::U8(3), SqlParam::U64(42)]
        );
    }

    #[test]
    fn transfer_preserves_mixed_money_inventory_void_and_quest_order() {
        let request = VoidStorageTransferWriteRequestLikeCpp {
            player_guid: 42,
            money_before: 500_000,
            money_after: 400_000,
            deposits: vec![wow_persistence::VoidStorageDepositWriteLikeCpp {
                destroyed_items: vec![wow_persistence::VoidStorageDestroyedItemWriteLikeCpp {
                    item_db_guid: 501,
                }],
                void_slot: 3,
                void_item: item(77),
            }],
            withdrawals: vec![
                wow_persistence::VoidStorageWithdrawalWriteLikeCpp {
                    old_void_slot: 4,
                    inventory_write: VoidStorageWithdrawalInventoryWriteLikeCpp::New(
                        VoidStorageNewInventoryItemWriteLikeCpp {
                            item_db_guid: 601,
                            item_entry: 19_020,
                            creator_guid: 8,
                            count: 1,
                            enchantments: "1 0 0".to_string(),
                            item_flags: 5,
                            max_durability: 90,
                            total_played_time: 900,
                            random_properties_id: -14,
                            random_properties_seed: 30,
                            context: 7,
                            container_db_guid: 700,
                            inventory_slot: 25,
                        },
                    ),
                },
                wow_persistence::VoidStorageWithdrawalWriteLikeCpp {
                    old_void_slot: 5,
                    inventory_write: VoidStorageWithdrawalInventoryWriteLikeCpp::MergeExisting(
                        VoidStorageMergedInventoryItemWriteLikeCpp {
                            item_db_guid: 602,
                            item_entry: 19_021,
                            owner_guid: 42,
                            creator_guid: 9,
                            gift_creator_guid: 10,
                            count: 2,
                            expiration: 11,
                            charges: "3 ".to_string(),
                            dynamic_flags: 12,
                            enchantments: "2 0 0".to_string(),
                            durability: 80,
                            create_played_time: 901,
                            text: "merged".to_string(),
                            battle_pet_species_id: 13,
                            battle_pet_breed_data: 14,
                            battle_pet_level: 15,
                            battle_pet_display_id: 16,
                            random_properties_id: 17,
                            property_seed: 18,
                            context: 19,
                        },
                    ),
                },
                wow_persistence::VoidStorageWithdrawalWriteLikeCpp {
                    old_void_slot: 6,
                    inventory_write: VoidStorageWithdrawalInventoryWriteLikeCpp::None,
                },
            ],
            quest_statuses: vec![
                VoidStorageQuestStatusWriteLikeCpp {
                    quest_id: 100,
                    status: 3,
                    explored: true,
                    accept_time_secs: 20,
                    end_time_secs: 21,
                    objectives: vec![wow_persistence::VoidStorageQuestObjectiveWriteLikeCpp {
                        storage_index: 2,
                        count: 4,
                    }],
                },
                VoidStorageQuestStatusWriteLikeCpp {
                    quest_id: 101,
                    status: 6,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objectives: Vec::new(),
                },
            ],
        };

        let statements = void_storage_transfer_statements_like_cpp(&request);
        assert_eq!(statements.len(), 23);
        let identities = statements
            .iter()
            .map(PreparedStatement::sql)
            .collect::<Vec<_>>();
        assert_eq!(identities[0], CharStatements::UPD_CHAR_MONEY.sql());
        assert_eq!(identities[1], CharStatements::DEL_CHAR_INVENTORY_ITEM.sql());
        assert_eq!(identities[9], CharStatements::DEL_ITEM_INSTANCE.sql());
        assert_eq!(
            identities[10],
            CharStatements::REP_CHAR_VOID_STORAGE_ITEM.sql()
        );
        assert_eq!(
            identities[11],
            CharStatements::INS_ITEM_INSTANCE_CLONE.sql()
        );
        assert_eq!(
            identities[12],
            CharStatements::REP_CHAR_INVENTORY_ITEM.sql()
        );
        assert_eq!(
            identities[13],
            CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT.sql()
        );
        assert_eq!(identities[14], CharStatements::UPD_ITEM_INSTANCE.sql());
        assert_eq!(
            identities[15],
            CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT.sql()
        );
        assert_eq!(
            identities[16],
            CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT.sql()
        );
        assert_eq!(identities[17], CharStatements::INS_CHAR_QUEST_STATUS.sql());
        assert_eq!(
            identities[18],
            CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql()
        );
        assert_eq!(
            identities[19],
            CharStatements::REP_CHAR_QUEST_STATUS_OBJECTIVES.sql()
        );
        assert_eq!(
            identities[20],
            CharStatements::INS_CHAR_QUESTSTATUS_REWARDED.sql()
        );
        assert_eq!(identities[21], CharStatements::DEL_CHAR_QUEST_STATUS.sql());
        assert_eq!(
            identities[22],
            CharStatements::DEL_CHAR_QUEST_STATUS_OBJECTIVES_BY_QUEST.sql()
        );
        assert_eq!(
            statements[0].params(),
            &[SqlParam::U64(400_000), SqlParam::U64(42)]
        );
        assert_eq!(statements[11].params()[0], SqlParam::U64(601));
        assert_eq!(statements[12].params()[1], SqlParam::U64(700));
        assert_eq!(statements[14].params()[19], SqlParam::U64(602));
        assert_eq!(statements[19].params()[2], SqlParam::U8(2));
    }

    #[test]
    fn empty_transfer_still_persists_the_money_fence_statement() {
        let statements =
            void_storage_transfer_statements_like_cpp(&VoidStorageTransferWriteRequestLikeCpp {
                player_guid: 42,
                money_before: 100,
                money_after: 100,
                deposits: Vec::new(),
                withdrawals: Vec::new(),
                quest_statuses: Vec::new(),
            });
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].sql(), CharStatements::UPD_CHAR_MONEY.sql());
        assert_eq!(
            statements[0].params(),
            &[SqlParam::U64(100), SqlParam::U64(42)]
        );
    }
}
