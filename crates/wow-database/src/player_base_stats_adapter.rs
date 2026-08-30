//! MariaDB adapter for C++ `ObjectMgr::LoadPlayerInfo` base-stat sources.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP, PersistenceFutureLikeCpp,
    PlayerBaseStatsLoadOutcomeLikeCpp, PlayerBaseStatsPersistencePortLikeCpp,
    PlayerClassLevelStatsPersistenceRowLikeCpp, PlayerRaceStatsPersistenceRowLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

const STARTUP_STATEMENTS_LIKE_CPP: [WorldStatements; 2] = [
    WorldStatements::SEL_PLAYER_RACESTATS,
    WorldStatements::SEL_PLAYER_CLASSLEVELSTATS,
];

fn read_integer_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<i128> {
    result
        .try_read::<i64>(column)
        .map(i128::from)
        .or_else(|| result.try_read::<u64>(column).map(i128::from))
        .or_else(|| result.try_read::<i32>(column).map(i128::from))
        .or_else(|| result.try_read::<u32>(column).map(i128::from))
        .or_else(|| result.try_read::<i16>(column).map(i128::from))
        .or_else(|| result.try_read::<u16>(column).map(i128::from))
        .or_else(|| result.try_read::<i8>(column).map(i128::from))
        .or_else(|| result.try_read::<u8>(column).map(i128::from))
        .with_context(|| format!("missing or non-integer {field} SQL column {column}"))
}

fn u8_checked_like_cpp(value: i128, field: &'static str) -> Result<u8> {
    u8::try_from(value).with_context(|| format!("{field} SQL value {value} is not u8"))
}

fn i16_checked_like_cpp(value: i128, field: &'static str) -> Result<i16> {
    i16::try_from(value).with_context(|| format!("{field} SQL value {value} is not i16"))
}

fn u16_field_like_cpp(value: i128, field: &'static str) -> Result<u16> {
    if let Ok(value) = u16::try_from(value) {
        return Ok(value);
    }
    i16::try_from(value)
        .map(|value| value as u16)
        .with_context(|| format!("{field} SQL value {value} is outside the C++ uint16 field range"))
}

fn race_values_like_cpp(
    race: i128,
    values: [i128; PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP],
) -> Result<PlayerRaceStatsPersistenceRowLikeCpp> {
    let mut stat_modifiers = [0; PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP];
    for (index, value) in values.into_iter().enumerate() {
        stat_modifiers[index] = i16_checked_like_cpp(value, "PlayerRaceStats.StatModifier")?;
    }
    Ok(PlayerRaceStatsPersistenceRowLikeCpp {
        race: u8_checked_like_cpp(race, "PlayerRaceStats.Race")?,
        stat_modifiers,
    })
}

fn class_level_values_like_cpp(
    class: i128,
    level: i128,
    values: [i128; PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP],
) -> Result<PlayerClassLevelStatsPersistenceRowLikeCpp> {
    let mut primary_stats = [0; PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP];
    for (index, value) in values.into_iter().enumerate() {
        primary_stats[index] = u16_field_like_cpp(value, "PlayerClassLevelStats.PrimaryStat")?;
    }
    Ok(PlayerClassLevelStatsPersistenceRowLikeCpp {
        class: u8_checked_like_cpp(class, "PlayerClassLevelStats.Class")?,
        level: u8_checked_like_cpp(level, "PlayerClassLevelStats.Level")?,
        primary_stats,
    })
}

fn race_row_like_cpp(result: &SqlResult) -> Result<PlayerRaceStatsPersistenceRowLikeCpp> {
    let mut values = [0; PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_integer_checked_like_cpp(result, index + 1, "PlayerRaceStats.StatModifier")?;
    }
    race_values_like_cpp(
        read_integer_checked_like_cpp(result, 0, "PlayerRaceStats.Race")?,
        values,
    )
}

fn class_level_row_like_cpp(
    result: &SqlResult,
) -> Result<PlayerClassLevelStatsPersistenceRowLikeCpp> {
    let mut values = [0; PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP];
    for (index, value) in values.iter_mut().enumerate() {
        *value =
            read_integer_checked_like_cpp(result, index + 2, "PlayerClassLevelStats.PrimaryStat")?;
    }
    class_level_values_like_cpp(
        read_integer_checked_like_cpp(result, 0, "PlayerClassLevelStats.Class")?,
        read_integer_checked_like_cpp(result, 1, "PlayerClassLevelStats.Level")?,
        values,
    )
}

async fn query_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> Result<T>,
) -> Result<Vec<T>> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::new();
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(decode(&result)?);
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn classify_rows_like_cpp<T>(result: Result<Vec<T>>) -> PlayerBaseStatsLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => PlayerBaseStatsLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => PlayerBaseStatsLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbPlayerBaseStatsPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbPlayerBaseStatsPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl PlayerBaseStatsPersistencePortLikeCpp for MariaDbPlayerBaseStatsPersistenceAdapterLikeCpp {
    fn load_player_race_stats_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerBaseStatsLoadOutcomeLikeCpp<PlayerRaceStatsPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    STARTUP_STATEMENTS_LIKE_CPP[0],
                    race_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_player_class_level_stats_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerBaseStatsLoadOutcomeLikeCpp<PlayerClassLevelStatsPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    STARTUP_STATEMENTS_LIKE_CPP[1],
                    class_level_row_like_cpp,
                )
                .await,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn player_base_stat_statements_match_cpp_order_and_sql() {
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_PLAYER_RACESTATS,
                WorldStatements::SEL_PLAYER_CLASSLEVELSTATS,
            ]
        );
        assert_eq!(
            WorldStatements::SEL_PLAYER_RACESTATS.sql(),
            "SELECT race, str, agi, sta, inte, spi FROM player_racestats"
        );
        assert_eq!(
            WorldStatements::SEL_PLAYER_CLASSLEVELSTATS.sql(),
            "SELECT class, level, str, agi, sta, inte, spi FROM player_classlevelstats"
        );
    }

    #[test]
    fn checked_rows_preserve_all_signed_and_cpp_uint16_fields() {
        let race = race_values_like_cpp(7, [-1, 2, -3, 4, -5]).unwrap();
        assert_eq!(race.race, 7);
        assert_eq!(race.stat_modifiers, [-1, 2, -3, 4, -5]);
        assert!(race_values_like_cpp(256, [0; 5]).is_err());
        assert!(race_values_like_cpp(1, [i128::from(i16::MAX) + 1; 5]).is_err());

        let class = class_level_values_like_cpp(5, 80, [1, 2, 3, 4, -1]).unwrap();
        assert_eq!(class.class, 5);
        assert_eq!(class.level, 80);
        assert_eq!(class.primary_stats, [1, 2, 3, 4, u16::MAX]);
        assert!(class_level_values_like_cpp(-1, 1, [0; 5]).is_err());
        assert!(class_level_values_like_cpp(1, 256, [0; 5]).is_err());
        assert!(class_level_values_like_cpp(1, 1, [i128::from(u16::MAX) + 1; 5]).is_err());
    }
}
