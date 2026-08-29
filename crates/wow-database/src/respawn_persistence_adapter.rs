//! MariaDB adapter for map-owned respawn durability.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, RespawnPersistenceLoadOutcomeLikeCpp,
    RespawnPersistenceMutationLikeCpp, RespawnPersistenceMutationOutcomeLikeCpp,
    RespawnPersistencePortLikeCpp, RespawnPersistenceRowLikeCpp,
};

use crate::{CharStatements, CharacterDatabase, PreparedStatement};

fn load_for_map_statement_like_cpp(map_id: u16, instance_id: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::SEL_RESPAWNS);
    statement.set_u16(0, map_id);
    statement.set_u32(1, instance_id);
    statement
}

fn load_all_statement_like_cpp() -> PreparedStatement {
    PreparedStatement::for_statement(CharStatements::SEL_ALL_RESPAWNS)
}

fn mutation_statement_like_cpp(mutation: RespawnPersistenceMutationLikeCpp) -> PreparedStatement {
    match mutation {
        RespawnPersistenceMutationLikeCpp::Save { key, respawn_time } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::REP_RESPAWN);
            statement.set_u16(0, key.object_type_raw);
            statement.set_u64(1, key.spawn_id);
            statement.set_i64(2, respawn_time);
            statement.set_u16(3, key.map_id);
            statement.set_u32(4, key.instance_id);
            statement
        }
        RespawnPersistenceMutationLikeCpp::Delete { key } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::DEL_RESPAWN);
            statement.set_u16(0, key.object_type_raw);
            statement.set_u64(1, key.spawn_id);
            statement.set_u16(2, key.map_id);
            statement.set_u32(3, key.instance_id);
            statement
        }
    }
}

fn decode_rows_like_cpp(
    mut result: crate::SqlResult,
    fixed_map: Option<(u16, u32)>,
) -> Vec<RespawnPersistenceRowLikeCpp> {
    let mut rows = Vec::new();
    if result.is_empty() {
        return rows;
    }

    loop {
        let (map_id, instance_id) = fixed_map.map_or_else(
            || {
                (
                    result
                        .try_read::<u32>(3)
                        .or_else(|| result.try_read::<u16>(3).map(u32::from))
                        .unwrap_or(0),
                    result.try_read::<u32>(4).unwrap_or(0),
                )
            },
            |(map_id, instance_id)| (u32::from(map_id), instance_id),
        );
        rows.push(RespawnPersistenceRowLikeCpp {
            object_type_raw: result
                .try_read::<u16>(0)
                .or_else(|| result.try_read::<u8>(0).map(u16::from))
                .unwrap_or(u16::MAX),
            spawn_id: result
                .try_read::<u64>(1)
                .or_else(|| result.try_read::<i64>(1).map(|value| value as u64))
                .unwrap_or(0),
            respawn_time: result.try_read::<i64>(2).unwrap_or(0),
            map_id,
            instance_id,
        });
        if !result.next_row() {
            break;
        }
    }
    rows
}

pub struct MariaDbRespawnPersistenceAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbRespawnPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

impl RespawnPersistencePortLikeCpp for MariaDbRespawnPersistenceAdapterLikeCpp {
    fn load_for_map_like_cpp<'a>(
        &'a self,
        map_id: u16,
        instance_id: u32,
    ) -> PersistenceFutureLikeCpp<'a, RespawnPersistenceLoadOutcomeLikeCpp> {
        Box::pin(async move {
            match self
                .character_db
                .query(&load_for_map_statement_like_cpp(map_id, instance_id))
                .await
            {
                Ok(result) => RespawnPersistenceLoadOutcomeLikeCpp::Loaded(decode_rows_like_cpp(
                    result,
                    Some((map_id, instance_id)),
                )),
                Err(error) => RespawnPersistenceLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_all_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, RespawnPersistenceLoadOutcomeLikeCpp> {
        Box::pin(async move {
            match self
                .character_db
                .query(&load_all_statement_like_cpp())
                .await
            {
                Ok(result) => {
                    RespawnPersistenceLoadOutcomeLikeCpp::Loaded(decode_rows_like_cpp(result, None))
                }
                Err(error) => RespawnPersistenceLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn execute_mutation_like_cpp<'a>(
        &'a self,
        mutation: RespawnPersistenceMutationLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, RespawnPersistenceMutationOutcomeLikeCpp> {
        Box::pin(async move {
            match self
                .character_db
                .execute(&mutation_statement_like_cpp(mutation))
                .await
            {
                Ok(affected_rows) => {
                    RespawnPersistenceMutationOutcomeLikeCpp::Applied { affected_rows }
                }
                Err(error) => RespawnPersistenceMutationOutcomeLikeCpp::Failed {
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
    use wow_persistence::RespawnPersistenceKeyLikeCpp;

    const KEY: RespawnPersistenceKeyLikeCpp = RespawnPersistenceKeyLikeCpp {
        object_type_raw: 1,
        spawn_id: 0x0102_0304_0506_0708,
        map_id: 530,
        instance_id: 0x1122_3344,
    };

    #[test]
    fn respawn_statements_preserve_cpp_identity_and_parameter_order() {
        let load = load_for_map_statement_like_cpp(KEY.map_id, KEY.instance_id);
        assert_eq!(load.sql(), CharStatements::SEL_RESPAWNS.sql());
        assert_eq!(
            load.params(),
            [SqlParam::U16(KEY.map_id), SqlParam::U32(KEY.instance_id)]
        );
        assert_eq!(
            load_all_statement_like_cpp().sql(),
            CharStatements::SEL_ALL_RESPAWNS.sql()
        );

        let save = mutation_statement_like_cpp(RespawnPersistenceMutationLikeCpp::Save {
            key: KEY,
            respawn_time: 0x1020_3040_5060_7080,
        });
        assert_eq!(save.sql(), CharStatements::REP_RESPAWN.sql());
        assert_eq!(
            save.params(),
            [
                SqlParam::U16(KEY.object_type_raw),
                SqlParam::U64(KEY.spawn_id),
                SqlParam::I64(0x1020_3040_5060_7080),
                SqlParam::U16(KEY.map_id),
                SqlParam::U32(KEY.instance_id),
            ]
        );

        let delete =
            mutation_statement_like_cpp(RespawnPersistenceMutationLikeCpp::Delete { key: KEY });
        assert_eq!(delete.sql(), CharStatements::DEL_RESPAWN.sql());
        assert_eq!(
            delete.params(),
            [
                SqlParam::U16(KEY.object_type_raw),
                SqlParam::U64(KEY.spawn_id),
                SqlParam::U16(KEY.map_id),
                SqlParam::U32(KEY.instance_id),
            ]
        );
    }
}
