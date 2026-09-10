//! Money, inventory cleanup, currency, XP and talent-reset statement plans.
//! Private MariaDB implementation; no semantic port or transaction changes.

use super::player_currency_save_statements_like_cpp;
use crate::params::PreparedStatement;
use crate::statements::{CharStatements, LoginStatements};
use crate::transaction::SqlTransaction;
use wow_persistence::{
    PlayerBankSlotPurchaseRequestLikeCpp, PlayerBuybackClearRequestLikeCpp,
    PlayerCurrencySaveRequestLikeCpp, PlayerDurabilityRepairSaveLikeCpp,
    PlayerMoneyTransactionRequestLikeCpp, PlayerMoneyWriteRequestLikeCpp,
    PlayerRealmCharacterCountRefreshRequestLikeCpp, PlayerTalentResetPersistenceRequestLikeCpp,
    PlayerUncageItemStateRequestLikeCpp, PlayerXpPersistenceRequestLikeCpp,
};

pub(super) fn player_money_write_statement_like_cpp(
    request: &PlayerMoneyWriteRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::UPD_CHAR_MONEY);
    statement.set_u64(0, request.money);
    statement.set_u64(1, request.player_guid);
    statement
}

pub(super) fn player_durability_repair_statement_like_cpp(
    repair: &PlayerDurabilityRepairSaveLikeCpp,
) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(CharStatements::UPD_ITEM_INSTANCE_DURABILITY);
    statement.set_u32(0, repair.durability);
    statement.set_u64(1, repair.item_db_guid);
    statement
}

pub(super) fn player_money_transaction_statements_like_cpp(
    request: &PlayerMoneyTransactionRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = vec![player_money_write_statement_like_cpp(
        &PlayerMoneyWriteRequestLikeCpp {
            player_guid: request.player_guid,
            money: request.money_after,
        },
    )];
    for repair in &request.durability_repairs {
        statements.push(player_durability_repair_statement_like_cpp(repair));
    }
    statements
}

pub(super) const UPD_CHARACTER_MONEY_AND_BANK_SLOTS_LIKE_CPP: &str =
    "UPDATE characters SET money = ?, bankSlots = ? WHERE guid = ?";

pub(super) fn player_bank_slot_purchase_statement_like_cpp(
    request: &PlayerBankSlotPurchaseRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::new(UPD_CHARACTER_MONEY_AND_BANK_SLOTS_LIKE_CPP);
    statement.set_u64(0, request.money_after);
    statement.set_u8(1, request.bank_slot_count);
    statement.set_u64(2, request.player_guid);
    statement
}

pub(super) fn player_uncage_item_state_statement_like_cpp(
    request: PlayerUncageItemStateRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::SEL_UNCAGE_ITEM_STATE);
    statement.set_u64(0, request.item_guid);
    statement.set_u64(1, request.player_guid);
    statement.set_u64(2, request.item_guid);
    statement
}

pub(super) fn append_player_currency_save_request_like_cpp(
    transaction: &mut SqlTransaction,
    request: &PlayerCurrencySaveRequestLikeCpp,
) {
    for statement in player_currency_save_statements_like_cpp(request) {
        transaction.append(statement);
    }
}

pub(super) fn player_buyback_clear_statements_like_cpp(
    request: &PlayerBuybackClearRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(request.item_db_guids.len().saturating_mul(2));
    for &item_db_guid in &request.item_db_guids {
        let mut delete_inventory =
            PreparedStatement::for_statement(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        delete_inventory.set_u64(0, request.player_guid);
        delete_inventory.set_u64(1, item_db_guid);
        statements.push(delete_inventory);

        let mut delete_item = PreparedStatement::for_statement(CharStatements::DEL_ITEM_INSTANCE);
        delete_item.set_u64(0, item_db_guid);
        statements.push(delete_item);
    }
    statements
}

pub(super) fn player_talent_reset_statements_like_cpp(
    request: &PlayerTalentResetPersistenceRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(3 + request.retained_talents.len());

    let mut money = PreparedStatement::for_statement(CharStatements::UPD_CHAR_MONEY);
    money.set_u64(0, request.money_after);
    money.set_u64(1, request.player_guid);
    statements.push(money);

    let mut metadata =
        PreparedStatement::for_statement(CharStatements::UPD_CHAR_TALENT_RESET_STATE);
    metadata.set_u32(0, request.reset_cost);
    metadata.set_u64(1, request.reset_time_secs);
    metadata.set_u64(2, request.player_guid);
    statements.push(metadata);

    let mut delete = PreparedStatement::for_statement(CharStatements::DEL_CHAR_TALENT);
    delete.set_u64(0, request.player_guid);
    statements.push(delete);

    for row in &request.retained_talents {
        let mut insert = PreparedStatement::for_statement(CharStatements::INS_CHAR_TALENT);
        insert.set_u64(0, request.player_guid);
        insert.set_u32(1, row.talent_id);
        insert.set_u8(2, row.rank);
        insert.set_u8(3, row.talent_group);
        statements.push(insert);
    }
    statements
}

pub(super) fn player_xp_persistence_statements_like_cpp(
    request: &PlayerXpPersistenceRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(if request.rest.is_some() { 2 } else { 1 });
    if request.level_changed {
        let mut statement = PreparedStatement::for_statement(CharStatements::UPD_CHAR_LEVEL);
        statement.set_u8(0, request.level);
        statement.set_u32(1, request.xp);
        statement.set_u64(2, request.player_guid);
        statements.push(statement);
    } else {
        let mut statement = PreparedStatement::for_statement(CharStatements::UPD_CHAR_XP);
        statement.set_u32(0, request.xp);
        statement.set_u64(1, request.player_guid);
        statements.push(statement);
    }

    if let Some(rest) = request.rest {
        let mut statement =
            PreparedStatement::for_statement(CharStatements::UPD_CHAR_ONLINE_REST_STATE);
        statement.set_u8(0, rest.rest_state);
        statement.set_u32(1, rest.player_flags);
        statement.set_f32(2, rest.rest_bonus);
        statement.set_u64(3, request.player_guid);
        statements.push(statement);
    }
    statements
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlayerTalentResetCommitReconciliationLikeCpp {
    Applied,
    Failed,
    Unknown,
}

pub(super) fn reconcile_player_talent_reset_commit_like_cpp(
    money_before: u64,
    money_after: u64,
    observed_money: Option<u64>,
) -> PlayerTalentResetCommitReconciliationLikeCpp {
    if money_before == money_after {
        return PlayerTalentResetCommitReconciliationLikeCpp::Unknown;
    }
    match observed_money {
        Some(observed) if observed == money_after => {
            PlayerTalentResetCommitReconciliationLikeCpp::Applied
        }
        Some(observed) if observed == money_before => {
            PlayerTalentResetCommitReconciliationLikeCpp::Failed
        }
        Some(_) | None => PlayerTalentResetCommitReconciliationLikeCpp::Unknown,
    }
}

pub(super) fn player_realm_character_count_statements_like_cpp(
    request: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    num_chars: u8,
) -> (PreparedStatement, PreparedStatement) {
    let mut count = PreparedStatement::for_statement(CharStatements::SEL_SUM_CHARS);
    count.set_u32(0, request.account_id);

    let mut replace = PreparedStatement::for_statement(LoginStatements::REP_REALM_CHARACTERS);
    replace.set_u8(0, num_chars);
    replace.set_u32(1, request.account_id);
    replace.set_u32(2, request.realm_id);
    (count, replace)
}
