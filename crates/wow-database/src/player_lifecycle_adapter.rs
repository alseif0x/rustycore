// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! MariaDB adapter for the Player lifecycle port.
//!
//! `wow-persistence` says *what* to persist and how the outcome is classified;
//! this is the only place that knows the statement, the pool and the driver
//! error. Keeping the mapping here is the point of the split: the Session sees
//! `Applied` / `Failed` / `Unknown` and never a `sqlx::Error`.

use crate::params::PreparedStatement;
use crate::statements::{CharStatements, LoginStatements, WorldStatements};
use crate::transaction::SqlTransaction;
use crate::{CharacterDatabase, LoginDatabase, SqlTransactionCommitError, WorldDatabase};
use std::sync::Arc;
use wow_persistence::PlayerCurrencySaveKindLikeCpp;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp, AccountCollectionSaveLikeCpp,
    AccountHeirloomLoadRowLikeCpp, AccountMaskBlockLikeCpp, AccountMountLoadRowLikeCpp,
    AccountToyLoadRowLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp,
    PlayerActionButtonLoadRowLikeCpp, PlayerBagInventoryLoadRowLikeCpp,
    PlayerBankSlotPurchaseRequestLikeCpp, PlayerBattlegroundLocationLoadRowLikeCpp,
    PlayerBuybackClearRequestLikeCpp, PlayerCharacterAuraEffectLoadRowLikeCpp,
    PlayerCharacterAuraLoadRowLikeCpp, PlayerCharacterBaseLoadOutcomeLikeCpp,
    PlayerCharacterBaseLoadRequestLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSaveResultLikeCpp, PlayerCufProfileLoadRowLikeCpp, PlayerCurrencyLoadRowLikeCpp,
    PlayerCurrencySaveRequestLikeCpp, PlayerCustomizationLoadRowLikeCpp,
    PlayerDurabilityRepairSaveLikeCpp, PlayerEquipmentInventoryLoadRowLikeCpp,
    PlayerEquipmentSetLoadRowLikeCpp, PlayerGlyphLoadRowLikeCpp,
    PlayerGuildMembershipLoadRowLikeCpp, PlayerHomebindLocationLoadRowLikeCpp,
    PlayerHomebindPersistenceRequestLikeCpp, PlayerInitialWorldStateRowsLikeCpp,
    PlayerInitialWorldStateTemplateRowLikeCpp, PlayerInitialWorldStateValueRowLikeCpp,
    PlayerInitialWorldStatesLoadOutcomeLikeCpp, PlayerInstanceTimeRestrictionLoadRowLikeCpp,
    PlayerLifecyclePortLikeCpp, PlayerLoginAdmissionLoadOutcomeLikeCpp,
    PlayerLoginAdmissionLoadRequestLikeCpp, PlayerLoginAdmissionLoadedLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerLoginAuxiliaryLoadedLikeCpp, PlayerLoginItemRepairRequestLikeCpp,
    PlayerLoginPetTalentResetOutcomeLikeCpp, PlayerLoginTransportLoadOutcomeLikeCpp,
    PlayerLoginTransportLoadRequestLikeCpp, PlayerMailLoadRowLikeCpp,
    PlayerMoneyTransactionOutcomeLikeCpp, PlayerMoneyTransactionRequestLikeCpp,
    PlayerMoneyWriteRequestLikeCpp, PlayerOfflineMarkLikeCpp, PlayerOnlineMarkRequestLikeCpp,
    PlayerPetAuraEffectLoadRowLikeCpp, PlayerPetAuraLoadRowLikeCpp,
    PlayerPetDeclinedNamesLoadRowLikeCpp, PlayerPetSpellChargeLoadRowLikeCpp,
    PlayerPetSpellCooldownLoadRowLikeCpp, PlayerPetSpellLoadRowLikeCpp,
    PlayerPetStableLoadRowLikeCpp, PlayerRealmCharacterCountRefreshRequestLikeCpp,
    PlayerReputationLoadRowLikeCpp, PlayerSkillLoadRowLikeCpp, PlayerSpellChargeLoadRowLikeCpp,
    PlayerSpellCooldownLoadRowLikeCpp, PlayerSpellLoadRowLikeCpp, PlayerTalentLoadRowLikeCpp,
    PlayerTalentResetPersistenceRequestLikeCpp, PlayerTraitConfigLoadRowLikeCpp,
    PlayerTraitEntryLoadRowLikeCpp, PlayerTransmogOutfitLoadRowLikeCpp,
    PlayerUncageItemStateLikeCpp, PlayerUncageItemStateLoadOutcomeLikeCpp,
    PlayerUncageItemStateRequestLikeCpp, PlayerVoidStorageLoadRowLikeCpp,
    PlayerXpPersistenceRequestLikeCpp,
};

mod economy;
use economy::{
    PlayerTalentResetCommitReconciliationLikeCpp, append_player_currency_save_request_like_cpp,
    player_bank_slot_purchase_statement_like_cpp, player_buyback_clear_statements_like_cpp,
    player_durability_repair_statement_like_cpp, player_money_transaction_statements_like_cpp,
    player_money_write_statement_like_cpp, player_realm_character_count_statements_like_cpp,
    player_talent_reset_statements_like_cpp, player_uncage_item_state_statement_like_cpp,
    player_xp_persistence_statements_like_cpp, reconcile_player_talent_reset_commit_like_cpp,
};
mod login_writes;
use login_writes::{
    player_homebind_persistence_statement_like_cpp, player_login_item_repair_statements_like_cpp,
    player_login_pet_talent_reset_statements_like_cpp, player_online_mark_statement_like_cpp,
};
mod save_plan;
mod save_steps;
use save_plan::player_character_save_statements_like_cpp;
mod collections;
use collections::account_collection_load_statements_like_cpp;
mod login_reads;
use login_reads::{
    nonnegative_i32_to_u32_like_cpp, nonnegative_i64_to_u64_like_cpp,
    player_character_base_load_row_like_cpp, player_character_base_load_statement_like_cpp,
    player_inventory_item_load_row_like_cpp, player_login_admission_load_statement_like_cpp,
    player_login_auxiliary_load_statement_like_cpp,
};
mod transports;
pub(crate) fn player_currency_save_statements_like_cpp(
    request: &PlayerCurrencySaveRequestLikeCpp,
) -> Vec<PreparedStatement> {
    request
        .rows
        .iter()
        .map(|row| match row.kind {
            PlayerCurrencySaveKindLikeCpp::New => {
                let mut statement =
                    PreparedStatement::for_statement(CharStatements::REP_PLAYER_CURRENCY);
                statement.set_u64(0, request.player_guid);
                statement.set_u16(1, row.currency_id);
                statement.set_u32(2, row.quantity);
                statement.set_u32(3, row.weekly_quantity);
                statement.set_u32(4, row.tracked_quantity);
                statement.set_u32(5, row.increased_cap_quantity);
                statement.set_u32(6, row.earned_quantity);
                statement.set_u8(7, row.flags);
                statement
            }
            PlayerCurrencySaveKindLikeCpp::Changed => {
                let mut statement =
                    PreparedStatement::for_statement(CharStatements::UPD_PLAYER_CURRENCY);
                statement.set_u32(0, row.quantity);
                statement.set_u32(1, row.weekly_quantity);
                statement.set_u32(2, row.tracked_quantity);
                statement.set_u32(3, row.increased_cap_quantity);
                statement.set_u32(4, row.earned_quantity);
                statement.set_u8(5, row.flags);
                statement.set_u64(6, request.player_guid);
                statement.set_u16(7, row.currency_id);
                statement
            }
        })
        .collect()
}
pub use save_plan::build_tutorials_save_statement_like_cpp;
use transports::{
    player_login_transport_load_rows_like_cpp, player_login_transport_load_statement_like_cpp,
};

/// Binds the lifecycle port to the Characters, Login and World adapters its
/// semantic requests address.
pub struct MariaDbPlayerLifecycleAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
    login_db: Arc<LoginDatabase>,
    world_db: Arc<WorldDatabase>,
}

impl MariaDbPlayerLifecycleAdapterLikeCpp {
    pub fn new(
        character_db: Arc<CharacterDatabase>,
        login_db: Arc<LoginDatabase>,
        world_db: Arc<WorldDatabase>,
    ) -> Self {
        Self {
            character_db,
            login_db,
            world_db,
        }
    }
}

impl PlayerLifecyclePortLikeCpp for MariaDbPlayerLifecycleAdapterLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match mark {
                PlayerOfflineMarkLikeCpp::Character { guid_low } => {
                    let mut stmt = self.character_db.prepare(CharStatements::UPD_CHAR_OFFLINE);
                    stmt.set_u32(0, guid_low);
                    self.character_db.execute(&stmt).await
                }
                PlayerOfflineMarkLikeCpp::CharacterAccount { account_id } => {
                    let mut stmt = self
                        .character_db
                        .prepare(CharStatements::UPD_ACCOUNT_ONLINE);
                    stmt.set_u32(0, account_id);
                    self.character_db.execute(&stmt).await
                }
                PlayerOfflineMarkLikeCpp::LoginAccount { account_id } => {
                    let mut stmt = self.login_db.prepare(LoginStatements::UPD_ACCOUNT_OFFLINE);
                    stmt.set_u32(0, account_id);
                    self.login_db.execute(&stmt).await
                }
            };
            match result {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                // A single-statement write outside a transaction either applied
                // or it did not; there is no COMMIT whose outcome could be
                // indeterminate. `Unknown` is reserved for the transactional
                // paths #200 migrates next, so do not manufacture it here.
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_homebind_persistence_statement_like_cpp(request);
            match self.character_db.execute(&statement).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        request: PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let item_count = request.item_db_guids.len() as u64;
            let mut transaction = SqlTransaction::new();
            for statement in player_buyback_clear_statements_like_cpp(&request) {
                transaction.append(statement);
            }
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows: item_count },
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

    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        request: PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            for statement in player_money_transaction_statements_like_cpp(&request) {
                transaction.append(statement);
            }

            crate::player_money_transaction_adapter::commit_player_money_transaction_and_observe_like_cpp(
                self.character_db.as_ref(),
                transaction,
                Some(request.player_guid),
            )
            .await
        })
    }

    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        request: PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async move {
            let mut transaction = SqlTransaction::new();
            transaction.append_expect_rows_affected(
                player_bank_slot_purchase_statement_like_cpp(&request),
                1,
            );
            crate::player_money_transaction_adapter::commit_player_money_transaction_and_observe_like_cpp(
                self.character_db.as_ref(),
                transaction,
                Some(request.player_guid),
            )
            .await
        })
    }

    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        request: PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerUncageItemStateLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_uncage_item_state_statement_like_cpp(request);
            match self.character_db.query(&statement).await {
                Ok(result) => {
                    PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(PlayerUncageItemStateLikeCpp {
                        owner_guid: result.try_read::<Option<u64>>(0).flatten(),
                        inventory_linked: result.try_read::<u64>(1).unwrap_or_default() != 0,
                    })
                }
                Err(error) => PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        repair: PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_durability_repair_statement_like_cpp(&repair);
            match self.character_db.execute(&statement).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn persist_money_write_like_cpp<'a>(
        &'a self,
        request: PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_money_write_statement_like_cpp(&request);
            match self.character_db.execute(&statement).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        request: PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let rows = request.rows.len() as u64;
            let mut transaction = SqlTransaction::new();
            append_player_currency_save_request_like_cpp(&mut transaction, &request);
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows },
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

    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        request: PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statements = player_talent_reset_statements_like_cpp(&request);
            let rows = statements.len() as u64;
            let mut transaction = SqlTransaction::new();
            for statement in statements {
                transaction.append(statement);
            }

            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    PersistenceOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    }
                }
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
                    let observed_money = if request.money_before == request.money_after {
                        None
                    } else {
                        let mut observed =
                            self.character_db.prepare(CharStatements::SEL_CHAR_MONEY);
                        observed.set_u64(0, request.player_guid);
                        self.character_db
                            .query(&observed)
                            .await
                            .ok()
                            .filter(|result| !result.is_empty())
                            .and_then(|result| result.try_read::<u64>(0))
                    };

                    match reconcile_player_talent_reset_commit_like_cpp(
                        request.money_before,
                        request.money_after,
                        observed_money,
                    ) {
                        PlayerTalentResetCommitReconciliationLikeCpp::Applied => {
                            PersistenceOutcomeLikeCpp::Applied { rows }
                        }
                        PlayerTalentResetCommitReconciliationLikeCpp::Failed => {
                            PersistenceOutcomeLikeCpp::Failed {
                                reason: error.to_string(),
                            }
                        }
                        PlayerTalentResetCommitReconciliationLikeCpp::Unknown => {
                            PersistenceOutcomeLikeCpp::Unknown {
                                reason: error.to_string(),
                            }
                        }
                    }
                }
            }
        })
    }

    fn persist_xp_like_cpp<'a>(
        &'a self,
        request: PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statements = player_xp_persistence_statements_like_cpp(&request);
            let rows = statements.len() as u64;
            let mut transaction = SqlTransaction::new();
            for statement in statements {
                transaction.append(statement);
            }
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows },
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

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        request: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let (count_statement, _) = player_realm_character_count_statements_like_cpp(request, 0);
            let num_chars = match self.character_db.query(&count_statement).await {
                Ok(result) if !result.is_empty() => result.try_read::<i64>(0).unwrap_or(0) as u8,
                Ok(_) => 0,
                Err(error) => {
                    return PersistenceOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            let (_, replace_statement) =
                player_realm_character_count_statements_like_cpp(request, num_chars);
            match self.login_db.execute(&replace_statement).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, PlayerInitialWorldStatesLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let templates = {
                let statement = self.world_db.prepare(WorldStatements::SEL_WORLD_STATES);
                match self.world_db.query(&statement).await {
                    Ok(mut result) => {
                        let mut rows = Vec::new();
                        if !result.is_empty() {
                            loop {
                                rows.push(PlayerInitialWorldStateTemplateRowLikeCpp {
                                    id: result.read(0),
                                    default_value: result.read(1),
                                    map_ids_csv: result.try_read(2).unwrap_or_default(),
                                    area_ids_csv: result.try_read(3).unwrap_or_default(),
                                });
                                if !result.next_row() {
                                    break;
                                }
                            }
                        }
                        PlayerInitialWorldStateRowsLikeCpp::Loaded(rows)
                    }
                    Err(error) => PlayerInitialWorldStateRowsLikeCpp::Failed {
                        reason: error.to_string(),
                    },
                }
            };

            // Preserve C++ and the existing Rust order even when the template
            // read failed: this is a second logical database, not one ACID unit.
            let saved_values = {
                let statement = self
                    .character_db
                    .prepare(CharStatements::SEL_WORLD_STATE_VALUES);
                match self.character_db.query(&statement).await {
                    Ok(mut result) => {
                        let mut rows = Vec::new();
                        if !result.is_empty() {
                            loop {
                                rows.push(PlayerInitialWorldStateValueRowLikeCpp {
                                    id: result.read(0),
                                    value: result.read(1),
                                });
                                if !result.next_row() {
                                    break;
                                }
                            }
                        }
                        PlayerInitialWorldStateRowsLikeCpp::Loaded(rows)
                    }
                    Err(error) => PlayerInitialWorldStateRowsLikeCpp::Failed {
                        reason: error.to_string(),
                    },
                }
            };

            PlayerInitialWorldStatesLoadOutcomeLikeCpp {
                templates,
                saved_values,
            }
        })
    }

    fn load_login_transports_like_cpp<'a>(
        &'a self,
        request: PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginTransportLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_login_transport_load_statement_like_cpp(request);
            match self.world_db.query(&statement).await {
                Ok(result) => PlayerLoginTransportLoadOutcomeLikeCpp::Loaded(
                    player_login_transport_load_rows_like_cpp(result),
                ),
                Err(error) => PlayerLoginTransportLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterBaseLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_character_base_load_statement_like_cpp(request);
            match self.character_db.query(&statement).await {
                Ok(result) if result.is_empty() => {
                    PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(None)
                }
                Ok(result) => PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(Some(
                    player_character_base_load_row_like_cpp(&result),
                )),
                Err(error) => PlayerCharacterBaseLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statements = account_collection_load_statements_like_cpp(request);
            match request {
                AccountCollectionLoadRequestLikeCpp::Mounts { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountMountLoadRowLikeCpp {
                                        mount_spell_id: result.try_read::<i32>(0).unwrap_or(0),
                                        flags: result.try_read::<u8>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::Mounts(rows),
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
                AccountCollectionLoadRequestLikeCpp::Toys { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountToyLoadRowLikeCpp {
                                        item_id: result.try_read::<i32>(0).unwrap_or(0),
                                        is_favorite: result.try_read::<bool>(1).unwrap_or(false),
                                        has_fanfare: result.try_read::<bool>(2).unwrap_or(false),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::Toys(rows),
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
                AccountCollectionLoadRequestLikeCpp::Heirlooms { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountHeirloomLoadRowLikeCpp {
                                        item_id: result.try_read::<i32>(0).unwrap_or(0),
                                        flags: result.try_read::<u32>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::Heirlooms(rows),
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
                AccountCollectionLoadRequestLikeCpp::ItemAppearances { .. } => {
                    let appearance_blocks = match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountMaskBlockLikeCpp {
                                        block_index: result.try_read::<u32>(0).unwrap_or(0),
                                        mask: result.try_read::<u32>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionRowsLikeCpp::Loaded(rows)
                        }
                        Err(error) => AccountCollectionRowsLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    };
                    let favorite_appearance_ids = match self.login_db.query(&statements[1]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(result.try_read::<u32>(0).unwrap_or(0));
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionRowsLikeCpp::Loaded(rows)
                        }
                        Err(error) => AccountCollectionRowsLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    };
                    AccountCollectionLoadOutcomeLikeCpp::Loaded(
                        AccountCollectionLoadedLikeCpp::ItemAppearances {
                            appearance_blocks,
                            favorite_appearance_ids,
                        },
                    )
                }
                AccountCollectionLoadRequestLikeCpp::TransmogIllusions { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountMaskBlockLikeCpp {
                                        block_index: result.try_read::<u32>(0).unwrap_or(0),
                                        mask: result.try_read::<u32>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::TransmogIllusions {
                                    illusion_blocks: rows,
                                },
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
            }
        })
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_login_admission_load_statement_like_cpp(request);
            let mut result = match self.character_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            let loaded = match request {
                PlayerLoginAdmissionLoadRequestLikeCpp::BattlegroundLocation { .. } => {
                    let row =
                        (!result.is_empty()).then(|| PlayerBattlegroundLocationLoadRowLikeCpp {
                            x: result.try_read(2),
                            y: result.try_read(3),
                            z: result.try_read(4),
                            orientation: result.try_read(5),
                            map_id: result.try_read(6),
                        });
                    PlayerLoginAdmissionLoadedLikeCpp::BattlegroundLocation(row)
                }
                PlayerLoginAdmissionLoadRequestLikeCpp::HomebindLocation { .. } => {
                    let row = (!result.is_empty()).then(|| PlayerHomebindLocationLoadRowLikeCpp {
                        map_id: result.try_read(0),
                        area_id: result.try_read(1),
                        x: result.try_read(2),
                        y: result.try_read(3),
                        z: result.try_read(4),
                        orientation: result.try_read(5),
                    });
                    PlayerLoginAdmissionLoadedLikeCpp::HomebindLocation(row)
                }
                PlayerLoginAdmissionLoadRequestLikeCpp::GuildMembership { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerGuildMembershipLoadRowLikeCpp {
                                guild_id: result.try_read(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAdmissionLoadedLikeCpp::GuildMembership(rows)
                }
            };
            PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(loaded)
        })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_login_auxiliary_load_statement_like_cpp(request);
            let mut result = match self.character_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            let loaded = match request {
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Mail { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerMailLoadRowLikeCpp {
                                mail_id: result.try_read(0).unwrap_or(0),
                                message_type: result.try_read(1).unwrap_or(0),
                                sender: result.try_read(2).unwrap_or(0),
                                receiver: result.try_read(3).unwrap_or(0),
                                expire_time: result.try_read(6).unwrap_or(0),
                                deliver_time: result.try_read(7).unwrap_or(0),
                                checked_flags: result.try_read(10).unwrap_or(0),
                                stationery_id: result.try_read(11).unwrap_or(0),
                                template_id: result.try_read(12).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Mail(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCustomizationLoadRowLikeCpp {
                                option_id: result.try_read::<u32>(0).unwrap_or(0),
                                choice_id: result.try_read::<u32>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Customizations(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(result.try_read::<u32>(0).unwrap_or(0));
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::CompletedAchievements(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerInstanceTimeRestrictionLoadRowLikeCpp {
                                instance_id: result.try_read::<u32>(0).unwrap_or(0),
                                release_time: result.try_read::<u64>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::InstanceTimeRestrictions(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerSpellCooldownLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                item_id: result.try_read::<u32>(1).unwrap_or(0),
                                cooldown_end: result.try_read::<i64>(2).unwrap_or(0),
                                category_id: result.try_read::<u32>(3).unwrap_or(0),
                                category_end: result.try_read::<i64>(4).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::SpellCooldowns(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerSpellChargeLoadRowLikeCpp {
                                category_id: result.try_read::<u32>(0).unwrap_or(0),
                                recharge_start: result.try_read::<i64>(1).unwrap_or(0),
                                recharge_end: result.try_read::<i64>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::SpellCharges(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerTraitEntryLoadRowLikeCpp {
                                trait_config_id: result.try_read::<i32>(0),
                                trait_node_id: result.try_read::<i32>(1),
                                trait_node_entry_id: result.try_read::<i32>(2),
                                rank: result.try_read::<i32>(3),
                                granted_ranks: result.try_read::<i32>(4),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::TraitEntries(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerTraitConfigLoadRowLikeCpp {
                                id: result.try_read::<i32>(0),
                                config_type: result.try_read::<i32>(1),
                                chr_specialization_id: result.try_read::<i32>(2),
                                combat_config_flags: result.try_read::<i32>(3),
                                local_identifier: result.try_read::<i32>(4),
                                skill_line_id: result.try_read::<i32>(5),
                                trait_system_id: result.try_read::<i32>(6),
                                name: result.try_read::<String>(7),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetStableLoadRowLikeCpp {
                                pet_number: result.try_read::<u32>(0).unwrap_or(0),
                                creature_id: result.try_read::<u32>(1).unwrap_or(0),
                                display_id: result.try_read::<u32>(2).unwrap_or(0),
                                level: result.try_read::<u8>(3).unwrap_or(1),
                                experience: result.try_read::<u32>(4).unwrap_or(0),
                                react_state: result.try_read::<u8>(5).unwrap_or(0),
                                slot: result.try_read::<i16>(6).unwrap_or(-1),
                                name: result.read_string(7),
                                was_renamed: result.try_read::<bool>(8).unwrap_or(false),
                                health: result.try_read::<u32>(9).unwrap_or(1),
                                mana: result.try_read::<u32>(10).unwrap_or(0),
                                action_bar: result.try_read::<String>(11).unwrap_or_default(),
                                last_save_time: result.try_read::<u32>(12).unwrap_or(0),
                                created_by_spell_id: result.try_read::<u32>(13).unwrap_or(0),
                                pet_type: result.try_read::<u8>(14).unwrap_or(0),
                                specialization_id: result.try_read::<u16>(15).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetStable(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuras { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetAuraLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(1).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(2).unwrap_or(0),
                                recalculate_mask: result.try_read::<u32>(3).unwrap_or(0),
                                difficulty: result.try_read::<u8>(4).unwrap_or(0),
                                stack_count: result.try_read::<u8>(5).unwrap_or(0),
                                max_duration_ms: result.try_read::<i32>(6).unwrap_or(0),
                                remain_time_ms: result.try_read::<i32>(7).unwrap_or(0),
                                remain_charges: result.try_read::<u8>(8).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetAuras(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuraEffects { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetAuraEffectLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(1).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(2).unwrap_or(0),
                                effect_index: result.try_read::<u8>(3).unwrap_or(0),
                                amount: result.try_read::<i32>(4).unwrap_or(0),
                                base_amount: result.try_read::<i32>(5).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetAuraEffects(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpells { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetSpellLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                active: result.try_read::<u8>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetSpells(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCooldowns { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetSpellCooldownLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                cooldown_end_unix_secs: result.try_read::<i64>(1).unwrap_or(0),
                                category_id: result.try_read::<u32>(2).unwrap_or(0),
                                category_end_unix_secs: result.try_read::<i64>(3).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCooldowns(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCharges { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetSpellChargeLoadRowLikeCpp {
                                category_id: result.try_read::<u32>(0).unwrap_or(0),
                                recharge_start_unix_secs: result.try_read::<i64>(1).unwrap_or(0),
                                recharge_end_unix_secs: result.try_read::<i64>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCharges(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetDeclinedNames { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        rows.push(PlayerPetDeclinedNamesLoadRowLikeCpp {
                            names: [
                                result.read_string(0),
                                result.read_string(1),
                                result.read_string(2),
                                result.read_string(3),
                                result.read_string(4),
                            ],
                        });
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetDeclinedNames(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(result.try_read::<u32>(0).unwrap_or(0));
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::GroupMembership(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentSets { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerEquipmentSetLoadRowLikeCpp {
                                set_guid: result.try_read::<u64>(0).unwrap_or(0),
                                set_id: result.try_read::<u8>(1).unwrap_or(0),
                                name: result.try_read(2).unwrap_or_default(),
                                icon: result.try_read(3).unwrap_or_default(),
                                ignore_mask: result.try_read::<u32>(4).unwrap_or(0),
                                assigned_spec_index: result.try_read::<i32>(5).unwrap_or(-1),
                                item_low_guids: (0..19)
                                    .map(|slot| result.try_read::<u64>(6 + slot).unwrap_or(0))
                                    .collect(),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentSets(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TransmogOutfits { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            let set_guid = result
                                .try_read::<i64>(0)
                                .and_then(nonnegative_i64_to_u64_like_cpp)
                                .or_else(|| result.try_read::<u64>(0))
                                .unwrap_or(0);
                            let ignore_mask = result
                                .try_read::<i32>(4)
                                .and_then(nonnegative_i32_to_u32_like_cpp)
                                .or_else(|| result.try_read::<u32>(4))
                                .unwrap_or(0);
                            rows.push(PlayerTransmogOutfitLoadRowLikeCpp {
                                set_guid,
                                set_id: result.try_read::<u8>(1).unwrap_or(0),
                                name: result.try_read(2).unwrap_or_default(),
                                icon: result.try_read(3).unwrap_or_default(),
                                ignore_mask,
                                appearances: (0..19)
                                    .map(|slot| result.try_read::<i32>(5 + slot).unwrap_or(0))
                                    .collect(),
                                enchants: [
                                    result.try_read::<i32>(24).unwrap_or(0),
                                    result.try_read::<i32>(25).unwrap_or(0),
                                ],
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::TransmogOutfits(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CufProfiles { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCufProfileLoadRowLikeCpp {
                                id: result.try_read::<u8>(0).unwrap_or(0),
                                name: result.try_read(1).unwrap_or_default(),
                                frame_height: result.try_read::<u16>(2).unwrap_or(0),
                                frame_width: result.try_read::<u16>(3).unwrap_or(0),
                                sort_by: result.try_read::<u8>(4).unwrap_or(0),
                                health_text: result.try_read::<u8>(5).unwrap_or(0),
                                bool_options: result.try_read::<u32>(6).unwrap_or(0),
                                top_point: result.try_read::<u8>(7).unwrap_or(0),
                                bottom_point: result.try_read::<u8>(8).unwrap_or(0),
                                left_point: result.try_read::<u8>(9).unwrap_or(0),
                                top_offset: result.try_read::<u16>(10).unwrap_or(0),
                                bottom_offset: result.try_read::<u16>(11).unwrap_or(0),
                                left_offset: result.try_read::<u16>(12).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::CufProfiles(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCurrencyLoadRowLikeCpp {
                                currency_id: result.try_read::<u16>(0).unwrap_or(0),
                                quantity: result.try_read::<u32>(1).unwrap_or(0),
                                weekly_quantity: result.try_read::<u32>(2).unwrap_or(0),
                                tracked_quantity: result.try_read::<u32>(3).unwrap_or(0),
                                increased_cap_quantity: result.try_read::<u32>(4).unwrap_or(0),
                                earned_quantity: result.try_read::<u32>(5).unwrap_or(0),
                                flags: result.try_read::<u8>(6).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Currencies(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Spells { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerSpellLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                active: result.try_read::<u8>(1).unwrap_or(1),
                                disabled: result.try_read::<u8>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Spells(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellFavorites { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(result.try_read::<u32>(0).unwrap_or(0));
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::SpellFavorites(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Skills { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            let value = result.try_read::<u16>(1).unwrap_or(0);
                            rows.push(PlayerSkillLoadRowLikeCpp {
                                skill_id: result.try_read::<u16>(0).unwrap_or(0),
                                value,
                                max: result.try_read::<u16>(2).unwrap_or(value),
                                profession_slot: result.try_read::<i8>(3).unwrap_or(-1),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Skills(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Talents { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerTalentLoadRowLikeCpp {
                                talent_id: result.try_read::<u32>(0).unwrap_or(0),
                                rank: result.try_read::<u8>(1).unwrap_or(0),
                                talent_group: result.try_read::<u8>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Talents(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Glyphs { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerGlyphLoadRowLikeCpp {
                                talent_group: result.try_read::<u8>(0).unwrap_or(0),
                                glyph_slot: result.try_read::<u8>(1).unwrap_or(0),
                                glyph_id: result.try_read::<u16>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Glyphs(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::ActionButtons { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerActionButtonLoadRowLikeCpp {
                                button: result.read(0),
                                action: result.try_read::<u32>(1).unwrap_or(0),
                                button_type: result.try_read::<u8>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::ActionButtons(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Reputation { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerReputationLoadRowLikeCpp {
                                faction_id: result.try_read::<u16>(0).unwrap_or(0),
                                standing: result.try_read::<i32>(1).unwrap_or(0),
                                flags: result.try_read::<u16>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Reputation(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCharacterAuraLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(2).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(3).unwrap_or(0),
                                recalculate_mask: result.try_read::<u32>(4).unwrap_or(0),
                                difficulty: result.try_read::<u8>(5).unwrap_or(0),
                                stack_count: result.try_read::<u8>(6).unwrap_or(1),
                                max_duration_ms: result.try_read::<i32>(7).unwrap_or(0),
                                remain_time_ms: result.try_read::<i32>(8).unwrap_or(0),
                                remain_charges: result.try_read::<u8>(9).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuras(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCharacterAuraEffectLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(2).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(3).unwrap_or(0),
                                effect_index: result.try_read::<u8>(4).unwrap_or(0),
                                amount: result.try_read::<i32>(5).unwrap_or(0),
                                base_amount: result.try_read::<i32>(6).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuraEffects(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerEquipmentInventoryLoadRowLikeCpp {
                                slot: result.read(0),
                                item: player_inventory_item_load_row_like_cpp(&result, 1),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentInventory(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerBagInventoryLoadRowLikeCpp {
                                bag_slot: result.read(0),
                                inner_slot: result.read(1),
                                item: player_inventory_item_load_row_like_cpp(&result, 2),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::BagInventory(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerVoidStorageLoadRowLikeCpp {
                                item_id: result.try_read::<u64>(0).unwrap_or(0),
                                item_entry: result.try_read::<u32>(1).unwrap_or(0),
                                slot: result.try_read::<u8>(2).unwrap_or(u8::MAX),
                                creator_guid: result.try_read::<u64>(3).unwrap_or(0),
                                fixed_scaling_level: result.try_read::<u32>(4).unwrap_or(0),
                                random_properties_id: result.try_read::<i32>(5).unwrap_or(0),
                                random_properties_seed: result.try_read::<i32>(6).unwrap_or(0),
                                context: result.try_read::<u8>(7).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::VoidStorage(rows)
                }
            };
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(loaded)
        })
    }

    fn persist_login_item_repairs_like_cpp<'a>(
        &'a self,
        request: PlayerLoginItemRepairRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statements = player_login_item_repair_statements_like_cpp(&request);
            let rows = statements.len() as u64;
            if statements.is_empty() {
                return PersistenceOutcomeLikeCpp::Applied { rows: 0 };
            }

            let mut tx = SqlTransaction::new();
            for statement in statements {
                tx.append(statement);
            }
            match tx
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows },
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

    fn reset_login_pet_talents_like_cpp<'a>(
        &'a self,
        player_guid: u64,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginPetTalentResetOutcomeLikeCpp> {
        Box::pin(async move {
            let [delete_spells, reset_specializations] =
                player_login_pet_talent_reset_statements_like_cpp(player_guid);

            let spell_delete = match self.character_db.execute(&delete_spells).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            };
            // C++ always submits this second independent write after the first;
            // do not short-circuit when spell deletion failed.
            let specialization_reset = match self.character_db.execute(&reset_specializations).await
            {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            };

            PlayerLoginPetTalentResetOutcomeLikeCpp {
                spell_delete,
                specialization_reset,
            }
        })
    }

    fn mark_player_online_like_cpp<'a>(
        &'a self,
        request: PlayerOnlineMarkRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_online_mark_statement_like_cpp(request);
            match self.character_db.execute(&statement).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let mut tx = SqlTransaction::new();
            let rows = match &save {
                AccountCollectionSaveLikeCpp::Mounts(rows) => {
                    for row in rows {
                        let mut stmt = self.login_db.prepare(LoginStatements::REP_ACCOUNT_MOUNTS);
                        stmt.set_u32(0, row.bnet_account_id);
                        stmt.set_u32(1, row.mount_spell_id);
                        stmt.set_u8(2, row.flags);
                        tx.append(stmt);
                    }
                    rows.len()
                }
                AccountCollectionSaveLikeCpp::Toys(rows) => {
                    for row in rows {
                        let mut stmt = self.login_db.prepare(LoginStatements::REP_ACCOUNT_TOYS);
                        stmt.set_u32(0, row.bnet_account_id);
                        stmt.set_u32(1, row.item_id);
                        stmt.set_bool(2, row.is_favorite);
                        stmt.set_bool(3, row.has_fanfare);
                        tx.append(stmt);
                    }
                    rows.len()
                }
                AccountCollectionSaveLikeCpp::Heirlooms(rows) => {
                    for row in rows {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::REP_ACCOUNT_HEIRLOOMS);
                        stmt.set_u32(0, row.bnet_account_id);
                        stmt.set_u32(1, row.item_id);
                        stmt.set_u32(2, row.flags);
                        tx.append(stmt);
                    }
                    rows.len()
                }
                AccountCollectionSaveLikeCpp::ItemAppearances {
                    bnet_account_id,
                    appearance_blocks,
                    favorite_inserts,
                    favorite_deletes,
                } => {
                    for block in appearance_blocks {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::INS_BNET_ITEM_APPEARANCES);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, block.block_index);
                        stmt.set_u32(2, block.mask);
                        tx.append(stmt);
                    }
                    // Inserts before deletes, as the Session built them.
                    for id in favorite_inserts {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::INS_BNET_ITEM_FAVORITE_APPEARANCE);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, *id);
                        tx.append(stmt);
                    }
                    for id in favorite_deletes {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::DEL_BNET_ITEM_FAVORITE_APPEARANCE);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, *id);
                        tx.append(stmt);
                    }
                    appearance_blocks.len() + favorite_inserts.len() + favorite_deletes.len()
                }
                AccountCollectionSaveLikeCpp::TransmogIllusions {
                    bnet_account_id,
                    illusion_blocks,
                } => {
                    for block in illusion_blocks {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::INS_BNET_TRANSMOG_ILLUSIONS);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, block.block_index);
                        stmt.set_u32(2, block.mask);
                        tx.append(stmt);
                    }
                    illusion_blocks.len()
                }
            };
            // Existing Rust boundary, not C++ transaction parity: #187 tracks
            // the separate collection transactions (see AccountCollectionSaveLikeCpp).
            match self.login_db.commit_transaction(tx).await {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows: rows as u64 },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        Box::pin(async move {
            let committed = request.committed_groups_like_cpp();
            let statements = player_character_save_statements_like_cpp(&request);
            let rows = statements.len() as u64;
            let mut tx = SqlTransaction::new();
            for statement in statements {
                tx.append(statement);
            }
            let outcome = match tx
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows },
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
            };
            PlayerCharacterSaveResultLikeCpp { outcome, committed }
        })
    }
}

#[cfg(test)]
mod tests;
