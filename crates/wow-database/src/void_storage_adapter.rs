//! MariaDB adapter for bounded void-storage unlock and slot-swap writes.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerMoneyTransactionOutcomeLikeCpp, VoidStorageItemWriteLikeCpp,
    VoidStoragePersistencePortLikeCpp, VoidStorageSwapWriteRequestLikeCpp,
    VoidStorageUnlockWriteRequestLikeCpp,
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
}
