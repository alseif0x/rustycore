//! MariaDB adapter for C++ `ObjectMgr` reputation startup catalogs.

use std::sync::Arc;

use wow_persistence::{
    CreatureOnKillReputationPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
    ReputationCatalogLoadOutcomeLikeCpp, ReputationCatalogPersistencePortLikeCpp,
    ReputationRewardRatePersistenceRowLikeCpp, ReputationSpilloverTemplatePersistenceRowLikeCpp,
};

use crate::{DatabaseError, SqlResult, WorldDatabase, WorldStatements};

const STARTUP_STATEMENTS_LIKE_CPP: [WorldStatements; 3] = [
    WorldStatements::SEL_REPUTATION_REWARD_RATE,
    WorldStatements::SEL_CREATURE_ONKILL_REPUTATION,
    WorldStatements::SEL_REPUTATION_SPILLOVER_TEMPLATE,
];

async fn query_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> T,
) -> Result<Vec<T>, DatabaseError> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::new();
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(decode(&result));
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn classify_rows_like_cpp<T>(
    result: Result<Vec<T>, DatabaseError>,
) -> ReputationCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => ReputationCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => ReputationCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbReputationCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbReputationCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl ReputationCatalogPersistencePortLikeCpp for MariaDbReputationCatalogPersistenceAdapterLikeCpp {
    fn load_reward_rate_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ReputationCatalogLoadOutcomeLikeCpp<ReputationRewardRatePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(&self.world_db, STARTUP_STATEMENTS_LIKE_CPP[0], |row| {
                    ReputationRewardRatePersistenceRowLikeCpp {
                        faction_id: read_db_u32_like_cpp(row, 0),
                        quest_rate: row.read(1),
                        quest_daily_rate: row.read(2),
                        quest_weekly_rate: row.read(3),
                        quest_monthly_rate: row.read(4),
                        quest_repeatable_rate: row.read(5),
                        creature_rate: row.read(6),
                        spell_rate: row.read(7),
                    }
                })
                .await,
            )
        })
    }

    fn load_creature_onkill_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ReputationCatalogLoadOutcomeLikeCpp<CreatureOnKillReputationPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(&self.world_db, STARTUP_STATEMENTS_LIKE_CPP[1], |row| {
                    CreatureOnKillReputationPersistenceRowLikeCpp {
                        creature_id: read_db_u32_like_cpp(row, 0),
                        rep_faction_1: read_db_i16_like_cpp(row, 1) as u32,
                        rep_faction_2: read_db_i16_like_cpp(row, 2) as u32,
                        is_team_award_1: read_db_bool_like_cpp(row, 3),
                        reputation_max_cap_1: read_db_u8_like_cpp(row, 4),
                        rep_value_1: row.read(5),
                        is_team_award_2: read_db_bool_like_cpp(row, 6),
                        reputation_max_cap_2: read_db_u8_like_cpp(row, 7),
                        rep_value_2: row.read(8),
                        team_dependent: read_db_bool_like_cpp(row, 9),
                    }
                })
                .await,
            )
        })
    }

    fn load_spillover_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ReputationCatalogLoadOutcomeLikeCpp<ReputationSpilloverTemplatePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(&self.world_db, STARTUP_STATEMENTS_LIKE_CPP[2], |row| {
                    ReputationSpilloverTemplatePersistenceRowLikeCpp {
                        faction_id: u32::from(read_db_u16_like_cpp(row, 0)),
                        faction: [
                            u32::from(read_db_u16_like_cpp(row, 1)),
                            u32::from(read_db_u16_like_cpp(row, 4)),
                            u32::from(read_db_u16_like_cpp(row, 7)),
                            u32::from(read_db_u16_like_cpp(row, 10)),
                            u32::from(read_db_u16_like_cpp(row, 13)),
                        ],
                        faction_rate: [
                            row.read(2),
                            row.read(5),
                            row.read(8),
                            row.read(11),
                            row.read(14),
                        ],
                        faction_rank: [
                            read_db_u8_like_cpp(row, 3),
                            read_db_u8_like_cpp(row, 6),
                            read_db_u8_like_cpp(row, 9),
                            read_db_u8_like_cpp(row, 12),
                            read_db_u8_like_cpp(row, 15),
                        ],
                    }
                })
                .await,
            )
        })
    }
}

fn read_db_u32_like_cpp(result: &SqlResult, column: usize) -> u32 {
    if let Some(value) = result.try_read::<u32>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u32_like_cpp(value);
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return u32::from(value);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u32_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return u32::from(value);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_db_u32_like_cpp(i32::from(value));
    }
    0
}

fn read_db_u16_like_cpp(result: &SqlResult, column: usize) -> u16 {
    if let Some(value) = result.try_read::<u16>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u16_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return u16::from(value);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return u16::from(normalize_signed_db_u8_like_cpp(i32::from(value)));
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return u16::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u16_like_cpp(value);
    }
    0
}

fn read_db_u8_like_cpp(result: &SqlResult, column: usize) -> u8 {
    if let Some(value) = result.try_read::<u8>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_db_u8_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return u8::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u8_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return u8::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u8_like_cpp(value);
    }
    0
}

fn read_db_i16_like_cpp(result: &SqlResult, column: usize) -> i16 {
    if let Some(value) = result.try_read::<i16>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return normalize_unsigned_db_i16_like_cpp(u32::from(value));
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return i16::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return normalize_unsigned_db_i16_like_cpp(value);
    }
    0
}

fn read_db_bool_like_cpp(result: &SqlResult, column: usize) -> bool {
    read_db_u8_like_cpp(result, column) == 1
}

fn normalize_signed_db_u32_like_cpp(value: i32) -> u32 {
    value as u32
}

fn normalize_signed_db_u16_like_cpp(value: i32) -> u16 {
    let converted = value as u16;
    if i32::from(converted) == value || (converted as i16) as i32 == value {
        converted
    } else {
        0
    }
}

fn normalize_signed_db_u8_like_cpp(value: i32) -> u8 {
    let converted = value as u8;
    if i32::from(converted) == value || (converted as i8) as i32 == value {
        converted
    } else {
        0
    }
}

fn normalize_unsigned_db_i16_like_cpp(value: u32) -> i16 {
    if value <= u32::from(u16::MAX) {
        value as i16
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reputation_statement_order_matches_cpp_startup_order() {
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_REPUTATION_REWARD_RATE,
                WorldStatements::SEL_CREATURE_ONKILL_REPUTATION,
                WorldStatements::SEL_REPUTATION_SPILLOVER_TEMPLATE,
            ]
        );
    }

    #[test]
    fn signed_reputation_columns_preserve_existing_cpp_accessor_normalization() {
        assert_eq!(normalize_signed_db_u32_like_cpp(-1), u32::MAX);
        assert_eq!(normalize_signed_db_u16_like_cpp(-1), u16::MAX);
        assert_eq!(normalize_signed_db_u16_like_cpp(0x1_0000), 0);
        assert_eq!(normalize_signed_db_u8_like_cpp(-1), u8::MAX);
        assert_eq!(normalize_signed_db_u8_like_cpp(0x100), 0);
        assert_eq!(normalize_unsigned_db_i16_like_cpp(u32::from(u16::MAX)), -1);
        assert_eq!(
            normalize_unsigned_db_i16_like_cpp(u32::from(u16::MAX) + 1),
            0
        );
    }
}
