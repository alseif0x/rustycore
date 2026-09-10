// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! MariaDB adapter for the recoverable battle-pet trainer-purchase saga.
//!
//! The application state machine remains in `wow-world`; this module owns
//! Character DB statements, transaction boundaries, row decoding and
//! reconciliation after a lost COMMIT reply.

use std::sync::Arc;

use tracing::info;
use wow_persistence::{
    BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseCommandLikeCpp,
    BattlePetPurchaseCommitFenceLikeCpp, BattlePetPurchaseCompensationOutcomeLikeCpp,
    BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchasePersistencePortLikeCpp,
    BattlePetPurchaseStatusLikeCpp, BattlePetPurchaseStoreErrorLikeCpp, PersistenceFutureLikeCpp,
    reconcile_battle_pet_purchase_charge_like_cpp, reconcile_battle_pet_purchase_mark_like_cpp,
};

use crate::{CharStatements, CharacterDatabase, SqlResult, SqlTransaction};

pub struct CharacterBattlePetPurchasePersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl CharacterBattlePetPurchasePersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }

    fn read_command_row_like_cpp(
        result: &SqlResult,
    ) -> Result<BattlePetPurchaseCommandLikeCpp, BattlePetPurchaseStoreErrorLikeCpp> {
        macro_rules! required {
            ($index:expr, $ty:ty, $name:literal) => {
                result.try_read::<$ty>($index).ok_or_else(|| {
                    BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        concat!("battle-pet purchase row cannot decode ", $name).to_string(),
                    )
                })?
            };
        }

        let key_bytes: Vec<u8> = required!(0, Vec<u8>, "request_key");
        let request_key: [u8; 16] = key_bytes.try_into().map_err(|_| {
            BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                "battle-pet purchase request_key is not 16 bytes".to_string(),
            )
        })?;
        let status_raw: u8 = required!(13, u8, "status");
        let status =
            BattlePetPurchaseStatusLikeCpp::from_u8_like_cpp(status_raw).ok_or_else(|| {
                BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                    "battle-pet purchase row has unknown status {status_raw}"
                ))
            })?;
        Ok(BattlePetPurchaseCommandLikeCpp {
            request_key,
            character_guid: required!(1, u64, "guid"),
            account_id: required!(2, u32, "account_id"),
            trainer_id: required!(3, u32, "trainer_id"),
            spell_id: required!(4, u32, "spell_id"),
            species: required!(5, u32, "species"),
            breed: required!(6, u16, "breed"),
            quality: required!(7, u8, "quality"),
            display_id: required!(8, u32, "display_id"),
            level: required!(9, u16, "level"),
            price: required!(10, u32, "price"),
            money_before: required!(11, u64, "money_before"),
            money_after: required!(12, u64, "money_after"),
            status,
            published: required!(15, u8, "published") != 0,
            failure_reason: result.try_read(14),
        })
    }

    async fn load_command_impl(
        &self,
        request_key: [u8; 16],
    ) -> Result<Option<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp> {
        let mut statement = self
            .character_db
            .prepare(CharStatements::SEL_BATTLE_PET_PURCHASE_BY_KEY);
        statement.set_bytes(0, request_key.to_vec());
        let result = self.character_db.query(&statement).await.map_err(|error| {
            BattlePetPurchaseStoreErrorLikeCpp::Retryable(format!(
                "battle-pet purchase read failed: {error}"
            ))
        })?;
        if result.is_empty() {
            return Ok(None);
        }
        Self::read_command_row_like_cpp(&result).map(Some)
    }

    async fn load_character_money_impl(
        &self,
        character_guid: u64,
    ) -> Result<Option<u64>, BattlePetPurchaseStoreErrorLikeCpp> {
        let mut statement = self.character_db.prepare(CharStatements::cpp(
            "CHAR_SEL_CHARACTER_MONEY",
            "SELECT money FROM characters WHERE guid = ?",
        ));
        statement.set_u64(0, character_guid);
        let result = self.character_db.query(&statement).await.map_err(|error| {
            BattlePetPurchaseStoreErrorLikeCpp::Retryable(format!(
                "battle-pet purchase character money read failed: {error}"
            ))
        })?;
        if result.is_empty() {
            return Ok(None);
        }
        result.try_read::<u64>(0).map(Some).ok_or_else(|| {
            BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                "battle-pet purchase character row cannot decode money".to_string(),
            )
        })
    }

    fn mark_statement_like_cpp(
        &self,
        statement_def: CharStatements,
        request_key: [u8; 16],
        reason: Option<&'static str>,
    ) -> SqlTransaction {
        let mut statement = self.character_db.prepare(statement_def);
        let mut index = 0;
        if let Some(reason) = reason {
            statement.set_string(index, reason);
            index += 1;
        }
        statement.set_bytes(index, request_key.to_vec());
        let mut transaction = SqlTransaction::new();
        transaction.append_expect_rows_affected(statement, 1);
        transaction
    }
}

impl BattlePetPurchasePersistencePortLikeCpp
    for CharacterBattlePetPurchasePersistenceAdapterLikeCpp
{
    fn charge_and_insert_command<'a>(
        &'a self,
        command: BattlePetPurchaseCommandLikeCpp,
        mut cancellation_fence: Box<dyn BattlePetPurchaseCommitFenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let money = (command.money_after != command.money_before).then(|| {
                let mut money = self
                    .character_db
                    .prepare(CharStatements::UPD_CHARACTER_MONEY_GUARDED);
                money.set_u64(0, command.money_after);
                money.set_u64(1, command.character_guid);
                money.set_u64(2, command.money_before);
                money
            });
            let mut insert = self
                .character_db
                .prepare(CharStatements::INS_BATTLE_PET_PURCHASE);
            insert.set_bytes(0, command.request_key.to_vec());
            insert.set_u64(1, command.character_guid);
            insert.set_u32(2, command.account_id);
            insert.set_u32(3, command.trainer_id);
            insert.set_u32(4, command.spell_id);
            insert.set_u32(5, command.species);
            insert.set_u16(6, command.breed);
            insert.set_u8(7, command.quality);
            insert.set_u32(8, command.display_id);
            insert.set_u16(9, command.level);
            insert.set_u32(10, command.price);
            insert.set_u64(11, command.money_before);
            insert.set_u64(12, command.money_after);
            insert.set_u8(
                13,
                BattlePetPurchaseStatusLikeCpp::PendingApplication.as_u8_like_cpp(),
            );
            let mut transaction = SqlTransaction::new();
            if let Some(money) = money {
                transaction.append_expect_rows_affected(money, 1);
            }
            transaction.append_expect_rows_affected(insert, 1);
            cancellation_fence.arm_like_cpp();
            let outcome = match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => Ok(BattlePetPurchaseChargeOutcomeLikeCpp::Charged),
                Err(error) => match self.load_command_impl(command.request_key).await {
                    Ok(row) => Ok(reconcile_battle_pet_purchase_charge_like_cpp(
                        row.as_ref(),
                        &command,
                    )),
                    Err(_) => Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(format!(
                        "battle-pet purchase charge COMMIT outcome is unknown and the command row cannot be read: {error}"
                    ))),
                },
            };
            cancellation_fence.disarm_like_cpp();
            outcome
        })
    }

    fn load_pending_commands<'a>(
        &'a self,
        character_guid: u64,
        limit: u32,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<Vec<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let mut statement = self
                .character_db
                .prepare(CharStatements::SEL_BATTLE_PET_PURCHASE_PENDING);
            statement.set_u64(0, character_guid);
            statement.set_u32(1, limit);
            let mut result = self.character_db.query(&statement).await.map_err(|error| {
                BattlePetPurchaseStoreErrorLikeCpp::Retryable(format!(
                    "battle-pet purchase recovery scan failed: {error}"
                ))
            })?;
            let mut commands = Vec::new();
            if !result.is_empty() {
                loop {
                    commands.push(Self::read_command_row_like_cpp(&result)?);
                    if !result.next_row() {
                        break;
                    }
                }
            }
            Ok(commands)
        })
    }

    fn mark_published<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let mut statement = self
                .character_db
                .prepare(CharStatements::UPD_BATTLE_PET_PURCHASE_PUBLISHED);
            statement.set_bytes(0, request_key.to_vec());
            let mut transaction = SqlTransaction::new();
            transaction.append_expect_rows_affected(statement, 1);
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => Ok(BattlePetPurchaseMarkOutcomeLikeCpp::Applied),
                Err(_) => match self.load_command_impl(request_key).await {
                    Ok(Some(row)) if row.published => {
                        Ok(BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied)
                    }
                    Ok(Some(row))
                        if row.status.is_terminal_like_cpp()
                            && row.status != BattlePetPurchaseStatusLikeCpp::Completed =>
                    {
                        Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                            "battle-pet purchase publication mark on terminal {:?}",
                            row.status
                        )))
                    }
                    Ok(Some(_)) => Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                        "battle-pet purchase publication mark did not commit".to_string(),
                    )),
                    Ok(None) => Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "battle-pet purchase command disappeared during publication mark"
                            .to_string(),
                    )),
                    Err(error) => Err(error),
                },
            }
        })
    }

    fn mark_completed<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let transaction = self.mark_statement_like_cpp(
                CharStatements::UPD_BATTLE_PET_PURCHASE_COMPLETED,
                request_key,
                None,
            );
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => Ok(BattlePetPurchaseMarkOutcomeLikeCpp::Applied),
                Err(_) => match self.load_command_impl(request_key).await {
                    Ok(row) => reconcile_battle_pet_purchase_mark_like_cpp(
                        row.as_ref(),
                        BattlePetPurchaseStatusLikeCpp::PendingApplication,
                        BattlePetPurchaseStatusLikeCpp::Completed,
                    ),
                    Err(error) => Err(error),
                },
            }
        })
    }

    fn mark_compensation_pending<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let transaction = self.mark_statement_like_cpp(
                CharStatements::UPD_BATTLE_PET_PURCHASE_COMPENSATION_PENDING,
                request_key,
                Some(reason),
            );
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => Ok(BattlePetPurchaseMarkOutcomeLikeCpp::Applied),
                Err(_) => match self.load_command_impl(request_key).await {
                    Ok(row) => reconcile_battle_pet_purchase_mark_like_cpp(
                        row.as_ref(),
                        BattlePetPurchaseStatusLikeCpp::PendingApplication,
                        BattlePetPurchaseStatusLikeCpp::CompensationPending,
                    ),
                    Err(error) => Err(error),
                },
            }
        })
    }

    fn compensate<'a>(
        &'a self,
        request_key: [u8; 16],
        max_money: u64,
        mut cancellation_fence: Box<dyn BattlePetPurchaseCommitFenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<
        'a,
        Result<BattlePetPurchaseCompensationOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let command = match self.load_command_impl(request_key).await? {
                Some(command) => command,
                None => {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "battle-pet purchase command to compensate does not exist".to_string(),
                    ));
                }
            };
            match command.status {
                BattlePetPurchaseStatusLikeCpp::Compensated => {
                    let durable_money = self
                        .load_character_money_impl(command.character_guid)
                        .await?
                        .ok_or_else(|| {
                            BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                                "compensated battle-pet purchase has no character row".to_string(),
                            )
                        })?;
                    return Ok(
                        BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
                            durable_money,
                        },
                    );
                }
                BattlePetPurchaseStatusLikeCpp::Completed => {
                    return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted);
                }
                BattlePetPurchaseStatusLikeCpp::CompensationPending => {}
                status => {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                        "battle-pet purchase compensation from {status:?} is not owned here"
                    )));
                }
            }

            let refund = (command.price != 0).then(|| {
                let mut refund = self
                    .character_db
                    .prepare(CharStatements::UPD_CHARACTER_MONEY_REFUND);
                refund.set_u32(0, command.price);
                refund.set_u64(1, max_money);
                refund.set_u64(2, command.character_guid);
                refund
            });
            let mut flip = self
                .character_db
                .prepare(CharStatements::UPD_BATTLE_PET_PURCHASE_COMPENSATED);
            flip.set_bytes(0, request_key.to_vec());
            let mut transaction = SqlTransaction::new();
            if let Some(refund) = refund {
                transaction.append_expect_rows_affected(refund, 1);
            }
            transaction.append_expect_rows_affected(flip, 1);
            cancellation_fence.arm_like_cpp();
            let commit_result = transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await;
            let outcome = match commit_result {
                Ok(()) => match self.load_character_money_impl(command.character_guid).await {
                    Ok(Some(durable_money)) => {
                        Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated {
                            durable_money,
                        })
                    }
                    Ok(None) => Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                        "battle-pet purchase refund committed but character row disappeared"
                            .to_string(),
                    )),
                    Err(error) => Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(format!(
                        "battle-pet purchase refund committed but durable money cannot be read: {error:?}"
                    ))),
                },
                Err(error) => match self.load_command_impl(request_key).await {
                    Ok(Some(row)) => match row.status {
                        BattlePetPurchaseStatusLikeCpp::Compensated => {
                            match self.load_character_money_impl(command.character_guid).await {
                                Ok(Some(durable_money)) => Ok(
                                    BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
                                        durable_money,
                                    },
                                ),
                                Ok(None) => Err(
                                    BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                                        "compensated battle-pet purchase has no character row"
                                            .to_string(),
                                    ),
                                ),
                                Err(read_error) => Err(
                                    BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(format!(
                                        "battle-pet purchase is durably compensated but durable money cannot be read: {read_error:?}"
                                    )),
                                ),
                            }
                        }
                        BattlePetPurchaseStatusLikeCpp::Completed => {
                            Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted)
                        }
                        BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                            match self.load_character_money_impl(command.character_guid).await {
                                Ok(None) => Ok(
                                    BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing,
                                ),
                                Ok(Some(_)) => Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                                    "battle-pet purchase compensation did not commit".to_string(),
                                )),
                                Err(read_error) => {
                                    Err(if error.is_commit_outcome_unknown_like_cpp() {
                                        BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(format!(
                                            "battle-pet purchase refund COMMIT is unknown and cannot be reconciled: {read_error:?}"
                                        ))
                                    } else {
                                        read_error
                                    })
                                }
                            }
                        }
                        status => Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                            "battle-pet purchase compensation observed unexpected {status:?}"
                        ))),
                    },
                    Ok(None) => Err(if error.is_commit_outcome_unknown_like_cpp() {
                        BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                            "battle-pet purchase refund COMMIT is unknown and command disappeared"
                                .to_string(),
                        )
                    } else {
                        BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                            "battle-pet purchase command disappeared during compensation"
                                .to_string(),
                        )
                    }),
                    Err(read_error) => Err(if error.is_commit_outcome_unknown_like_cpp() {
                        BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(format!(
                            "battle-pet purchase refund COMMIT is unknown and command cannot be read: {read_error:?}"
                        ))
                    } else {
                        read_error
                    }),
                },
            };
            if !matches!(
                outcome,
                Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(_))
            ) {
                cancellation_fence.disarm_like_cpp();
            }
            outcome
        })
    }

    fn mark_terminal_failure<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> PersistenceFutureLikeCpp<'a, Result<(), BattlePetPurchaseStoreErrorLikeCpp>> {
        Box::pin(async move {
            let transaction = self.mark_statement_like_cpp(
                CharStatements::UPD_BATTLE_PET_PURCHASE_TERMINAL_FAILURE,
                request_key,
                Some(reason),
            );
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => {
                    info!(
                        target: "battle_pet_purchase",
                        reason,
                        "Battle-pet purchase recorded as terminal failure"
                    );
                    Ok(())
                }
                Err(error) => match self.load_command_impl(request_key).await {
                    Ok(Some(row))
                        if row.status == BattlePetPurchaseStatusLikeCpp::TerminalFailure =>
                    {
                        Ok(())
                    }
                    _ => Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(format!(
                        "battle-pet purchase terminal-failure mark did not commit: {error}"
                    ))),
                },
            }
        })
    }
}
