//! MariaDB adapter for C++ `InstanceLockMgr` persistence.

use std::sync::Arc;

use wow_persistence::{
    CharacterInstanceLockPersistenceRowLikeCpp, InstanceLockPersistenceLoadOutcomeLikeCpp,
    InstanceLockPersistenceMutationLikeCpp, InstanceLockPersistenceOutcomeLikeCpp,
    InstanceLockPersistencePlanLikeCpp, InstanceLockPersistencePortLikeCpp,
    PersistenceFutureLikeCpp, SharedInstanceLockPersistenceRowLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, PreparedStatement, SqlTransaction};

fn statement_for_mutation_like_cpp(
    mutation: InstanceLockPersistenceMutationLikeCpp,
) -> PreparedStatement {
    match mutation {
        InstanceLockPersistenceMutationLikeCpp::DeleteCharacterLock {
            player_guid_counter,
            map_id,
            lock_id,
        } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::DEL_CHARACTER_INSTANCE_LOCK);
            statement.set_u64(0, player_guid_counter);
            statement.set_u32(1, map_id);
            statement.set_u32(2, lock_id);
            statement
        }
        InstanceLockPersistenceMutationLikeCpp::InsertCharacterLock {
            player_guid_counter,
            map_id,
            lock_id,
            instance_id,
            difficulty_id,
            data,
            completed_encounters_mask,
            entrance_world_safe_loc_id,
            expiry_time,
            extended,
        } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::INS_CHARACTER_INSTANCE_LOCK);
            statement.set_u64(0, player_guid_counter);
            statement.set_u32(1, map_id);
            statement.set_u32(2, lock_id);
            statement.set_u32(3, instance_id);
            statement.set_u8(4, difficulty_id);
            statement.set_string(5, data);
            statement.set_u32(6, completed_encounters_mask);
            statement.set_u32(7, entrance_world_safe_loc_id);
            statement.set_u64(8, expiry_time);
            statement.set_u8(9, u8::from(extended));
            statement
        }
        InstanceLockPersistenceMutationLikeCpp::DeleteSharedInstance { instance_id } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::DEL_INSTANCE);
            statement.set_u32(0, instance_id);
            statement
        }
        InstanceLockPersistenceMutationLikeCpp::InsertSharedInstance {
            instance_id,
            data,
            completed_encounters_mask,
            entrance_world_safe_loc_id,
        } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::INS_INSTANCE);
            statement.set_u32(0, instance_id);
            statement.set_string(1, data);
            statement.set_u32(2, completed_encounters_mask);
            statement.set_u32(3, entrance_world_safe_loc_id);
            statement
        }
        InstanceLockPersistenceMutationLikeCpp::UpdateCharacterLockExtension {
            extended,
            player_guid_counter,
            map_id,
            lock_id,
        } => {
            let mut statement = PreparedStatement::for_statement(
                CharStatements::UPD_CHARACTER_INSTANCE_LOCK_EXTENSION,
            );
            statement.set_u8(0, u8::from(extended));
            statement.set_u64(1, player_guid_counter);
            statement.set_u32(2, map_id);
            statement.set_u32(3, lock_id);
            statement
        }
        InstanceLockPersistenceMutationLikeCpp::ForceExpireCharacterLock {
            expiry_time,
            player_guid_counter,
            map_id,
            lock_id,
        } => {
            let mut statement = PreparedStatement::for_statement(
                CharStatements::UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE,
            );
            statement.set_u64(0, expiry_time);
            statement.set_u64(1, player_guid_counter);
            statement.set_u32(2, map_id);
            statement.set_u32(3, lock_id);
            statement
        }
    }
}

fn decode_shared_rows_like_cpp(
    mut result: crate::SqlResult,
) -> Vec<SharedInstanceLockPersistenceRowLikeCpp> {
    let mut rows = Vec::new();
    if result.is_empty() {
        return rows;
    }

    loop {
        rows.push(SharedInstanceLockPersistenceRowLikeCpp {
            instance_id: result.try_read::<u32>(0).unwrap_or(0),
            data: result.read_string(1),
            completed_encounters_mask: result.try_read::<u32>(2).unwrap_or(0),
            entrance_world_safe_loc_id: result.try_read::<u32>(3).unwrap_or(0),
        });
        if !result.next_row() {
            break;
        }
    }
    rows
}

fn decode_character_rows_like_cpp(
    mut result: crate::SqlResult,
) -> Vec<CharacterInstanceLockPersistenceRowLikeCpp> {
    let mut rows = Vec::new();
    if result.is_empty() {
        return rows;
    }

    loop {
        rows.push(CharacterInstanceLockPersistenceRowLikeCpp {
            player_guid_counter: result
                .try_read::<u64>(0)
                .or_else(|| result.try_read::<i64>(0).map(|value| value as u64))
                .unwrap_or(0),
            map_id: result.try_read::<u32>(1).unwrap_or(0),
            lock_id: result.try_read::<u32>(2).unwrap_or(0),
            instance_id: result.try_read::<u32>(3).unwrap_or(0),
            difficulty_id: result.try_read::<u8>(4).unwrap_or(0),
            data: result.read_string(5),
            completed_encounters_mask: result.try_read::<u32>(6).unwrap_or(0),
            entrance_world_safe_loc_id: result.try_read::<u32>(7).unwrap_or(0),
            expiry_time: result
                .try_read::<u64>(8)
                .or_else(|| result.try_read::<i64>(8).map(|value| value as u64))
                .unwrap_or(0),
            extended: result.try_read::<u8>(9).unwrap_or(0) != 0,
        });
        if !result.next_row() {
            break;
        }
    }
    rows
}

pub struct MariaDbInstanceLockPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbInstanceLockPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl InstanceLockPersistencePortLikeCpp for MariaDbInstanceLockPersistenceAdapterLikeCpp {
    fn load_all_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, InstanceLockPersistenceLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let shared_result = match self
                .character_db
                .query(&self.character_db.prepare(CharStatements::SEL_INSTANCE))
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return InstanceLockPersistenceLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            let shared_rows = decode_shared_rows_like_cpp(shared_result);

            let character_result = match self
                .character_db
                .query(
                    &self
                        .character_db
                        .prepare(CharStatements::SEL_CHARACTER_INSTANCE_LOCK),
                )
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    return InstanceLockPersistenceLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            InstanceLockPersistenceLoadOutcomeLikeCpp::Loaded {
                shared_rows,
                character_rows: decode_character_rows_like_cpp(character_result),
            }
        })
    }

    fn commit_plan_like_cpp<'a>(
        &'a self,
        plan: InstanceLockPersistencePlanLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, InstanceLockPersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            if plan.is_empty() {
                return InstanceLockPersistenceOutcomeLikeCpp::Committed;
            }

            let mut transaction = SqlTransaction::new();
            for mutation in plan.mutations {
                transaction.append(statement_for_mutation_like_cpp(mutation));
            }
            match self.character_db.commit_transaction(transaction).await {
                Ok(()) => InstanceLockPersistenceOutcomeLikeCpp::Committed,
                Err(error) => InstanceLockPersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn mutation_renderer_preserves_cpp_statement_identity_and_bind_order() {
        let delete = statement_for_mutation_like_cpp(
            InstanceLockPersistenceMutationLikeCpp::DeleteCharacterLock {
                player_guid_counter: 77,
                map_id: 631,
                lock_id: 7,
            },
        );
        assert_eq!(
            delete.sql(),
            CharStatements::DEL_CHARACTER_INSTANCE_LOCK.sql()
        );
        assert_eq!(
            delete.params(),
            [SqlParam::U64(77), SqlParam::U32(631), SqlParam::U32(7)]
        );

        let insert = statement_for_mutation_like_cpp(
            InstanceLockPersistenceMutationLikeCpp::InsertCharacterLock {
                player_guid_counter: 77,
                map_id: 631,
                lock_id: 7,
                instance_id: 9001,
                difficulty_id: 4,
                data: "player".to_string(),
                completed_encounters_mask: 3,
                entrance_world_safe_loc_id: 42,
                expiry_time: 500,
                extended: true,
            },
        );
        assert_eq!(
            insert.sql(),
            CharStatements::INS_CHARACTER_INSTANCE_LOCK.sql()
        );
        assert_eq!(
            insert.params(),
            [
                SqlParam::U64(77),
                SqlParam::U32(631),
                SqlParam::U32(7),
                SqlParam::U32(9001),
                SqlParam::U8(4),
                SqlParam::String("player".to_string()),
                SqlParam::U32(3),
                SqlParam::U32(42),
                SqlParam::U64(500),
                SqlParam::U8(1),
            ]
        );

        let delete_shared = statement_for_mutation_like_cpp(
            InstanceLockPersistenceMutationLikeCpp::DeleteSharedInstance { instance_id: 9001 },
        );
        assert_eq!(delete_shared.sql(), CharStatements::DEL_INSTANCE.sql());
        assert_eq!(delete_shared.params(), [SqlParam::U32(9001)]);

        let insert_shared = statement_for_mutation_like_cpp(
            InstanceLockPersistenceMutationLikeCpp::InsertSharedInstance {
                instance_id: 9001,
                data: "shared".to_string(),
                completed_encounters_mask: 5,
                entrance_world_safe_loc_id: 99,
            },
        );
        assert_eq!(insert_shared.sql(), CharStatements::INS_INSTANCE.sql());
        assert_eq!(
            insert_shared.params(),
            [
                SqlParam::U32(9001),
                SqlParam::String("shared".to_string()),
                SqlParam::U32(5),
                SqlParam::U32(99),
            ]
        );

        let extension = statement_for_mutation_like_cpp(
            InstanceLockPersistenceMutationLikeCpp::UpdateCharacterLockExtension {
                extended: true,
                player_guid_counter: 77,
                map_id: 631,
                lock_id: 7,
            },
        );
        assert_eq!(
            extension.sql(),
            CharStatements::UPD_CHARACTER_INSTANCE_LOCK_EXTENSION.sql()
        );
        assert_eq!(
            extension.params(),
            [
                SqlParam::U8(1),
                SqlParam::U64(77),
                SqlParam::U32(631),
                SqlParam::U32(7),
            ]
        );

        let force_expire = statement_for_mutation_like_cpp(
            InstanceLockPersistenceMutationLikeCpp::ForceExpireCharacterLock {
                expiry_time: 1234,
                player_guid_counter: 77,
                map_id: 631,
                lock_id: 7,
            },
        );
        assert_eq!(
            force_expire.sql(),
            CharStatements::UPD_CHARACTER_INSTANCE_LOCK_FORCE_EXPIRE.sql()
        );
        assert_eq!(
            force_expire.params(),
            [
                SqlParam::U64(1234),
                SqlParam::U64(77),
                SqlParam::U32(631),
                SqlParam::U32(7),
            ]
        );
    }
}
