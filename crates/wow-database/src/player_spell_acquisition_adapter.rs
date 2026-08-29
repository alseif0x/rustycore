//! MariaDB adapter for represented Player spell-acquisition persistence.

use std::sync::Arc;

use sqlx::{MySql, Transaction};
use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerSpellAcquisitionAuthorityLikeCpp,
    PlayerSpellAcquisitionDurableOperationLikeCpp,
    PlayerSpellAcquisitionMoneyReconciliationLikeCpp,
    PlayerSpellAcquisitionPersistenceAttemptLikeCpp, PlayerSpellAcquisitionPersistencePortLikeCpp,
    PlayerSpellAcquisitionPersistenceRequestLikeCpp, PlayerSpellAcquisitionSkillRowLikeCpp,
    PlayerSpellAcquisitionSpellRowLikeCpp,
    classify_player_spell_acquisition_money_reconciliation_like_cpp,
};

use crate::{CharacterDatabase, DatabaseError, is_database_deadlock_like_cpp};

pub struct MariaDbPlayerSpellAcquisitionPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbPlayerSpellAcquisitionPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

pub fn spell_acquisition_port(
    character_db: Arc<CharacterDatabase>,
) -> Arc<dyn PlayerSpellAcquisitionPersistencePortLikeCpp> {
    Arc::new(MariaDbPlayerSpellAcquisitionPersistenceAdapterLikeCpp::new(
        character_db,
    ))
}

fn definitely_rolled_back_like_cpp(
    error: impl Into<DatabaseError>,
) -> PlayerSpellAcquisitionPersistenceAttemptLikeCpp {
    let error = error.into();
    PlayerSpellAcquisitionPersistenceAttemptLikeCpp::DefinitelyRolledBack {
        retryable_deadlock: is_database_deadlock_like_cpp(&error),
        reason: error.to_string(),
    }
}

fn commit_error_like_cpp(
    error: impl Into<DatabaseError>,
) -> PlayerSpellAcquisitionPersistenceAttemptLikeCpp {
    let error = error.into();
    if is_database_deadlock_like_cpp(&error) {
        definitely_rolled_back_like_cpp(error)
    } else {
        PlayerSpellAcquisitionPersistenceAttemptLikeCpp::CommitOutcomeUnknown {
            reason: error.to_string(),
        }
    }
}

async fn rollback_like_cpp(transaction: Transaction<'_, MySql>) {
    if let Err(error) = transaction.rollback().await {
        tracing::error!(
            error = %error,
            "Failed to roll back represented Player spell-acquisition transaction"
        );
    }
}

async fn read_durable_authority_in_transaction_like_cpp(
    transaction: &mut Transaction<'_, MySql>,
    player_guid: u64,
    lock_rows: bool,
) -> Result<PlayerSpellAcquisitionAuthorityLikeCpp, DatabaseError> {
    let spell_sql = if lock_rows {
        "SELECT spell, active, disabled FROM character_spell WHERE guid = ? ORDER BY spell FOR UPDATE"
    } else {
        "SELECT spell, active, disabled FROM character_spell WHERE guid = ? ORDER BY spell"
    };
    let favorite_sql = if lock_rows {
        "SELECT spell FROM character_spell_favorite WHERE guid = ? ORDER BY spell FOR UPDATE"
    } else {
        "SELECT spell FROM character_spell_favorite WHERE guid = ? ORDER BY spell"
    };
    let skill_sql = if lock_rows {
        "SELECT skill, value, max, professionSlot FROM character_skills WHERE guid = ? ORDER BY skill FOR UPDATE"
    } else {
        "SELECT skill, value, max, professionSlot FROM character_skills WHERE guid = ? ORDER BY skill"
    };

    let spells = sqlx::query_as::<_, (i32, bool, bool)>(spell_sql)
        .bind(player_guid)
        .fetch_all(&mut **transaction)
        .await
        .map_err(DatabaseError::from)?
        .into_iter()
        .map(
            |(spell_id, active, disabled)| PlayerSpellAcquisitionSpellRowLikeCpp {
                spell_id,
                active,
                disabled,
            },
        )
        .collect();
    let favorite_spell_ids = sqlx::query_scalar::<_, i32>(favorite_sql)
        .bind(player_guid)
        .fetch_all(&mut **transaction)
        .await
        .map_err(DatabaseError::from)?;
    let skills = sqlx::query_as::<_, (u16, u16, u16, i8)>(skill_sql)
        .bind(player_guid)
        .fetch_all(&mut **transaction)
        .await
        .map_err(DatabaseError::from)?
        .into_iter()
        .map(
            |(skill_id, value, maximum, profession_slot)| PlayerSpellAcquisitionSkillRowLikeCpp {
                skill_id,
                value,
                maximum,
                profession_slot,
            },
        )
        .collect();
    Ok(PlayerSpellAcquisitionAuthorityLikeCpp {
        spells,
        favorite_spell_ids,
        skills,
    })
}

fn insert_affected_exactly_one_like_cpp(rows_affected: u64) -> Result<(), DatabaseError> {
    if rows_affected == 1 {
        Ok(())
    } else {
        Err(DatabaseError::Transaction(format!(
            "prepared player spell acquisition insert affected {rows_affected} rows; expected exactly 1"
        )))
    }
}

fn expected_replacement_operations_like_cpp(
    authority: &PlayerSpellAcquisitionAuthorityLikeCpp,
) -> Vec<PlayerSpellAcquisitionDurableOperationLikeCpp> {
    let mut operations = vec![
        PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills,
    ];
    operations.extend(
        authority
            .spells
            .iter()
            .copied()
            .map(PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell),
    );
    operations.extend(
        authority
            .favorite_spell_ids
            .iter()
            .copied()
            .map(PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell),
    );
    operations.extend(
        authority
            .skills
            .iter()
            .copied()
            .map(PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill),
    );
    operations
}

fn request_has_exact_replacement_order_like_cpp(
    request: &PlayerSpellAcquisitionPersistenceRequestLikeCpp,
) -> bool {
    request.operations == expected_replacement_operations_like_cpp(&request.resulting_authority)
}

fn source_authority_matches_like_cpp(
    observed: &PlayerSpellAcquisitionAuthorityLikeCpp,
    expected: &PlayerSpellAcquisitionAuthorityLikeCpp,
) -> bool {
    observed == expected
}

async fn execute_operation_like_cpp(
    transaction: &mut Transaction<'_, MySql>,
    player_guid: u64,
    operation: PlayerSpellAcquisitionDurableOperationLikeCpp,
) -> Result<(), DatabaseError> {
    let result = match operation {
        PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter => {
            let row = sqlx::query_scalar::<_, u64>(
                "SELECT guid FROM characters WHERE guid = ? FOR UPDATE",
            )
            .bind(player_guid)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(DatabaseError::from)?;
            if row != Some(player_guid) {
                return Err(DatabaseError::Transaction(
                    "prepared player spell acquisition character vanished".to_owned(),
                ));
            }
            return Ok(());
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells => {
            sqlx::query("DELETE FROM character_spell WHERE guid = ?")
                .bind(player_guid)
                .execute(&mut **transaction)
                .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells => {
            sqlx::query("DELETE FROM character_spell_favorite WHERE guid = ?")
                .bind(player_guid)
                .execute(&mut **transaction)
                .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills => {
            sqlx::query("DELETE FROM character_skills WHERE guid = ?")
                .bind(player_guid)
                .execute(&mut **transaction)
                .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(spell) => {
            sqlx::query(
                "INSERT INTO character_spell (guid, spell, active, disabled) VALUES (?, ?, ?, ?)",
            )
            .bind(player_guid)
            .bind(spell.spell_id)
            .bind(spell.active)
            .bind(spell.disabled)
            .execute(&mut **transaction)
            .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(spell_id) => {
            sqlx::query("INSERT INTO character_spell_favorite (guid, spell) VALUES (?, ?)")
                .bind(player_guid)
                .bind(spell_id)
                .execute(&mut **transaction)
                .await
        }
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill(skill) => {
            sqlx::query(
                "INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(player_guid)
            .bind(skill.skill_id)
            .bind(skill.value)
            .bind(skill.maximum)
            .bind(skill.profession_slot)
            .execute(&mut **transaction)
            .await
        }
    };

    let result = result.map_err(DatabaseError::from)?;
    if matches!(
        operation,
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(_)
            | PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(_)
            | PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill(_)
    ) {
        insert_affected_exactly_one_like_cpp(result.rows_affected())?;
    }
    Ok(())
}

impl PlayerSpellAcquisitionPersistencePortLikeCpp
    for MariaDbPlayerSpellAcquisitionPersistenceAdapterLikeCpp
{
    fn attempt_player_spell_acquisition_like_cpp(
        &self,
        request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerSpellAcquisitionPersistenceAttemptLikeCpp> {
        Box::pin(async move {
            if !request_has_exact_replacement_order_like_cpp(&request) {
                return definitely_rolled_back_like_cpp(DatabaseError::Transaction(
                    "trainer acquisition replacement operations do not match the resulting authority"
                        .to_owned(),
                ));
            }
            let mut transaction = match self.character_db.pool().begin().await {
                Ok(transaction) => transaction,
                Err(error) => return definitely_rolled_back_like_cpp(error),
            };

            let locked_money = match sqlx::query_scalar::<_, u64>(
                "SELECT money FROM characters WHERE guid = ? FOR UPDATE",
            )
            .bind(request.player_guid)
            .fetch_optional(&mut *transaction)
            .await
            {
                Ok(money) => money,
                Err(error) => {
                    rollback_like_cpp(transaction).await;
                    return definitely_rolled_back_like_cpp(error);
                }
            };
            if locked_money != Some(request.money_before) {
                rollback_like_cpp(transaction).await;
                return definitely_rolled_back_like_cpp(DatabaseError::Transaction(
                    "trainer acquisition durable money no longer matches the prepared balance"
                        .to_owned(),
                ));
            }

            let observed_source = match read_durable_authority_in_transaction_like_cpp(
                &mut transaction,
                request.player_guid,
                true,
            )
            .await
            {
                Ok(authority) => authority,
                Err(error) => {
                    rollback_like_cpp(transaction).await;
                    return definitely_rolled_back_like_cpp(error);
                }
            };
            if !source_authority_matches_like_cpp(&observed_source, &request.source_authority) {
                rollback_like_cpp(transaction).await;
                return definitely_rolled_back_like_cpp(DatabaseError::Transaction(
                    "trainer acquisition durable source no longer matches the prepared snapshot"
                        .to_owned(),
                ));
            }

            for operation in request.operations.iter().copied() {
                // The money lock above already locks and proves the character row,
                // matching the pre-extraction combined path.
                if operation == PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter {
                    continue;
                }
                if let Err(error) =
                    execute_operation_like_cpp(&mut transaction, request.player_guid, operation)
                        .await
                {
                    rollback_like_cpp(transaction).await;
                    return definitely_rolled_back_like_cpp(error);
                }
            }

            if request.money_before != request.money_after {
                let update = match sqlx::query(
                    "UPDATE characters SET money = ? WHERE guid = ? AND money = ?",
                )
                .bind(request.money_after)
                .bind(request.player_guid)
                .bind(request.money_before)
                .execute(&mut *transaction)
                .await
                {
                    Ok(update) => update,
                    Err(error) => {
                        rollback_like_cpp(transaction).await;
                        return definitely_rolled_back_like_cpp(error);
                    }
                };
                if update.rows_affected() != 1 {
                    rollback_like_cpp(transaction).await;
                    return definitely_rolled_back_like_cpp(DatabaseError::Transaction(format!(
                        "trainer acquisition money update affected {} rows; expected exactly 1",
                        update.rows_affected()
                    )));
                }
            }

            if let Err(error) = sqlx::query(
                "INSERT INTO character_spell_acquisition_operation (guid, operation_token) \
                 VALUES (?, ?) ON DUPLICATE KEY UPDATE operation_token = VALUES(operation_token)",
            )
            .bind(request.player_guid)
            .bind(request.operation_token.as_slice())
            .execute(&mut *transaction)
            .await
            {
                rollback_like_cpp(transaction).await;
                return definitely_rolled_back_like_cpp(error);
            }

            match transaction.commit().await {
                Ok(()) => PlayerSpellAcquisitionPersistenceAttemptLikeCpp::Applied,
                Err(error) => commit_error_like_cpp(error),
            }
        })
    }

    fn reconcile_player_spell_acquisition_like_cpp(
        &self,
        request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerSpellAcquisitionMoneyReconciliationLikeCpp> {
        Box::pin(async move {
            let indeterminate = || PlayerSpellAcquisitionMoneyReconciliationLikeCpp::Indeterminate;
            let mut transaction = match self.character_db.pool().begin().await {
                Ok(transaction) => transaction,
                Err(_) => return indeterminate(),
            };
            let observed_money = match sqlx::query_scalar::<_, u64>(
                "SELECT money FROM characters WHERE guid = ? FOR UPDATE",
            )
            .bind(request.player_guid)
            .fetch_optional(&mut *transaction)
            .await
            {
                Ok(Some(money)) => money,
                Ok(None) | Err(_) => {
                    rollback_like_cpp(transaction).await;
                    return indeterminate();
                }
            };
            let observed_operation_token = match sqlx::query_scalar::<_, Vec<u8>>(
                "SELECT operation_token FROM character_spell_acquisition_operation \
                 WHERE guid = ? FOR UPDATE",
            )
            .bind(request.player_guid)
            .fetch_optional(&mut *transaction)
            .await
            {
                Ok(token) => token,
                Err(_) => {
                    rollback_like_cpp(transaction).await;
                    return indeterminate();
                }
            };
            let observed_authority = match read_durable_authority_in_transaction_like_cpp(
                &mut transaction,
                request.player_guid,
                false,
            )
            .await
            {
                Ok(authority) => authority,
                Err(_) => {
                    rollback_like_cpp(transaction).await;
                    return indeterminate();
                }
            };
            if transaction.commit().await.is_err() {
                return indeterminate();
            }

            classify_player_spell_acquisition_money_reconciliation_like_cpp(
                request.money_after,
                observed_money,
                observed_authority == request.resulting_authority,
                observed_operation_token
                    .as_deref()
                    .is_some_and(|observed| observed == request.operation_token),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_row_count_is_exact_like_cpp() {
        assert!(insert_affected_exactly_one_like_cpp(1).is_ok());
        assert!(insert_affected_exactly_one_like_cpp(0).is_err());
        assert!(insert_affected_exactly_one_like_cpp(2).is_err());
    }

    #[test]
    fn non_deadlock_commit_error_is_unknown_but_statement_error_is_rollback() {
        let error = || DatabaseError::Transaction("fixture".to_owned());
        assert!(matches!(
            definitely_rolled_back_like_cpp(error()),
            PlayerSpellAcquisitionPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                retryable_deadlock: false,
                ..
            }
        ));
        assert!(matches!(
            commit_error_like_cpp(error()),
            PlayerSpellAcquisitionPersistenceAttemptLikeCpp::CommitOutcomeUnknown { .. }
        ));
    }

    #[test]
    fn strict_reconciliation_requires_money_authority_and_exact_token() {
        use PlayerSpellAcquisitionMoneyReconciliationLikeCpp::{Committed, Indeterminate};

        assert_eq!(
            classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 80, true, true),
            Committed
        );
        for evidence in [
            (80, 100, true, true),
            (80, 80, false, true),
            (80, 80, true, false),
        ] {
            assert_eq!(
                classify_player_spell_acquisition_money_reconciliation_like_cpp(
                    80, evidence.1, evidence.2, evidence.3
                ),
                Indeterminate
            );
        }
    }

    #[test]
    fn deterministic_replacement_order_is_preserved_by_the_request() {
        let authority = PlayerSpellAcquisitionAuthorityLikeCpp {
            spells: vec![PlayerSpellAcquisitionSpellRowLikeCpp {
                spell_id: 100,
                active: true,
                disabled: false,
            }],
            favorite_spell_ids: vec![100],
            skills: vec![PlayerSpellAcquisitionSkillRowLikeCpp {
                skill_id: 164,
                value: 1,
                maximum: 75,
                profession_slot: 0,
            }],
        };
        let operations = expected_replacement_operations_like_cpp(&authority);
        assert!(matches!(
            operations.as_slice(),
            [
                PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter,
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells,
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells,
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills,
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(_),
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(_),
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill(_),
            ]
        ));
        let request = PlayerSpellAcquisitionPersistenceRequestLikeCpp {
            player_guid: 42,
            money_before: 100,
            money_after: 80,
            operation_token: [1; 16],
            source_authority: PlayerSpellAcquisitionAuthorityLikeCpp {
                spells: Vec::new(),
                favorite_spell_ids: Vec::new(),
                skills: Vec::new(),
            },
            resulting_authority: authority,
            operations,
        };
        assert!(request_has_exact_replacement_order_like_cpp(&request));
        let mut malformed = request;
        malformed.operations.swap(1, 2);
        assert!(!request_has_exact_replacement_order_like_cpp(&malformed));
    }

    #[test]
    fn source_authority_mismatch_fails_closed_for_every_durable_collection() {
        let expected = PlayerSpellAcquisitionAuthorityLikeCpp {
            spells: vec![PlayerSpellAcquisitionSpellRowLikeCpp {
                spell_id: 100,
                active: true,
                disabled: false,
            }],
            favorite_spell_ids: vec![100],
            skills: vec![PlayerSpellAcquisitionSkillRowLikeCpp {
                skill_id: 164,
                value: 1,
                maximum: 75,
                profession_slot: 0,
            }],
        };
        assert!(source_authority_matches_like_cpp(&expected, &expected));
        for observed in [
            PlayerSpellAcquisitionAuthorityLikeCpp {
                spells: Vec::new(),
                ..expected.clone()
            },
            PlayerSpellAcquisitionAuthorityLikeCpp {
                favorite_spell_ids: Vec::new(),
                ..expected.clone()
            },
            PlayerSpellAcquisitionAuthorityLikeCpp {
                skills: Vec::new(),
                ..expected.clone()
            },
        ] {
            assert!(!source_authority_matches_like_cpp(&observed, &expected));
        }
    }
}
