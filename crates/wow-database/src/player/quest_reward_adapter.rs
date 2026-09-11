// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! MariaDB adapter committing one quest-reward operation as a single
//! character transaction.
//!
//! C++ closes `Player::RewardQuest` with `SaveToDB(false)` (Player.cpp:14867),
//! whose character half is one `CharacterDatabase` transaction
//! (Player.cpp:19312) containing `_SaveInventory`, `_SaveQuestStatus`, the
//! daily/weekly/seasonal/monthly groups and `_SaveCurrency`
//! (Player.cpp:19630..19655). This adapter builds that batch in the same
//! group order and commits it once, so no reward is durable on its own.
//!
//! The statement builders are the ones the per-capability adapters already
//! use, so statement identity, bind order and expected-row contracts are
//! unchanged; only the transaction boundary is wider.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerQuestRewardCommitOutcomeLikeCpp,
    PlayerQuestRewardCommitWitnessLikeCpp, PlayerQuestRewardDurableRequestLikeCpp,
    PlayerQuestRewardPersistencePortLikeCpp, PlayerQuestStatusPersistenceRequestLikeCpp,
};

use crate::player::inventory_adapter::{
    InventoryTransactionBuilderLikeCpp, append_inventory_request_like_cpp,
};
use crate::player::lifecycle_adapter::economy::player_money_write_statement_like_cpp;
use crate::player::lifecycle_adapter::player_currency_save_statements_like_cpp;
use crate::player::quest_adapter::{
    player_quest_status_statements_like_cpp, quest_lockout_statements_like_cpp,
};
use crate::{CharStatements, CharacterDatabase, SqlTransaction, SqlTransactionCommitError};

/// Build the operation's single character batch.
///
/// The group order mirrors the C++ save: inventory, quest status, the quest
/// lockout groups, then currency. Money is the character row's own column and
/// is appended last so it is the transaction's final observable write, which
/// makes it a sound commit witness for the whole batch.
fn quest_reward_transaction_like_cpp(
    request: &PlayerQuestRewardDurableRequestLikeCpp,
) -> SqlTransaction {
    let mut transaction = InventoryTransactionBuilderLikeCpp::new();

    for mutation in &request.inventory_mutations {
        append_inventory_request_like_cpp(&mut transaction, mutation);
    }

    let quest_status_statements = match &request.quest_status {
        PlayerQuestStatusPersistenceRequestLikeCpp::Save { owner_guid, status } => {
            player_quest_status_statements_like_cpp(*owner_guid, status)
        }
        PlayerQuestStatusPersistenceRequestLikeCpp::Delete {
            owner_guid,
            quest_id,
        } => crate::player::quest_adapter::delete_quest_status_statements_like_cpp(
            *owner_guid,
            *quest_id,
        ),
    };
    for statement in quest_status_statements {
        transaction.append(statement);
    }

    for lockout in &request.lockouts {
        for statement in quest_lockout_statements_like_cpp(lockout) {
            transaction.append(statement);
        }
    }

    if let Some(currencies) = &request.currencies {
        for statement in player_currency_save_statements_like_cpp(currencies) {
            transaction.append(statement);
        }
    }

    if let Some(money) = &request.money {
        transaction.append(player_money_write_statement_like_cpp(
            &wow_persistence::PlayerMoneyWriteRequestLikeCpp {
                player_guid: request.owner_guid,
                money: money.money_after,
            },
        ));
    }

    transaction.finish()
}

/// Read back whether the rewarded quest still has an active status row.
///
/// Both durable shapes of the operation delete that row: the rewarded save
/// deletes it after inserting the rewarded record, and the repeatable delete
/// removes it outright. Its absence therefore proves the batch committed, and
/// its presence proves it rolled back.
async fn observe_quest_status_witness_like_cpp(
    character_db: &CharacterDatabase,
    owner_guid: u64,
    quest_id: u32,
) -> Option<bool> {
    let mut statement = character_db.prepare(CharStatements::SEL_CHAR_QUEST_STATUS);
    statement.set_u64(0, owner_guid);
    let mut result = character_db.query(&statement).await.ok()?;
    if result.is_empty() {
        return Some(true);
    }
    loop {
        if result.try_read::<u32>(0) == Some(quest_id) {
            return Some(false);
        }
        if !result.next_row() {
            break;
        }
    }
    Some(true)
}

pub struct MariaDbPlayerQuestRewardPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbPlayerQuestRewardPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl PlayerQuestRewardPersistencePortLikeCpp for MariaDbPlayerQuestRewardPersistenceAdapterLikeCpp {
    fn persist_quest_reward_like_cpp(
        &self,
        request: PlayerQuestRewardDurableRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerQuestRewardCommitOutcomeLikeCpp> {
        Box::pin(async move {
            let transaction = quest_reward_transaction_like_cpp(&request);
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PlayerQuestRewardCommitOutcomeLikeCpp::Committed,
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    PlayerQuestRewardCommitOutcomeLikeCpp::DefinitelyRolledBack {
                        reason: error.to_string(),
                    }
                }
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
                    let witness = match &request.money {
                        Some(_) => {
                            let mut observed =
                                self.character_db.prepare(CharStatements::SEL_CHAR_MONEY);
                            observed.set_u64(0, request.owner_guid);
                            PlayerQuestRewardCommitWitnessLikeCpp::Money {
                                observed_money: self
                                    .character_db
                                    .query(&observed)
                                    .await
                                    .ok()
                                    .filter(|result| !result.is_empty())
                                    .and_then(|result| result.try_read::<u64>(0)),
                            }
                        }
                        None => PlayerQuestRewardCommitWitnessLikeCpp::QuestStatus {
                            observed_matches_request: observe_quest_status_witness_like_cpp(
                                self.character_db.as_ref(),
                                request.owner_guid,
                                request.quest_id,
                            )
                            .await,
                        },
                    };
                    PlayerQuestRewardCommitOutcomeLikeCpp::CommitOutcomeUnknown {
                        reason: error.to_string(),
                        witness,
                    }
                }
            }
        })
    }
}
