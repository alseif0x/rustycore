//! MariaDB adapter for represented C++ `ObjectMgr::LoadPlayerInfo` sources.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerCreateCastSpellPersistenceRowLikeCpp,
    PlayerCreateCustomSpellPersistenceRowLikeCpp, PlayerCreateInfoPersistenceRowLikeCpp,
    PlayerCreationCatalogLoadOutcomeLikeCpp, PlayerCreationCatalogPersistencePortLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

const STARTUP_STATEMENTS_LIKE_CPP: [WorldStatements; 3] = [
    WorldStatements::SEL_PLAYER_CREATEINFO,
    WorldStatements::SEL_PLAYER_CREATEINFO_CAST_SPELL,
    WorldStatements::SEL_PLAYER_CREATEINFO_CUSTOM_SPELL,
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

fn integer_checked_like_cpp<T>(value: i128, field: &'static str) -> Result<T>
where
    T: TryFrom<i128>,
{
    T::try_from(value).map_err(|_| anyhow::anyhow!("{field} SQL value {value} is out of range"))
}

fn read_float_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<f32> {
    result
        .try_read::<f32>(column)
        .or_else(|| result.try_read::<f64>(column).map(|value| value as f32))
        .with_context(|| format!("missing or non-numeric {field} SQL column {column}"))
}

fn read_optional_integer_checked_like_cpp<T>(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<Option<T>>
where
    T: TryFrom<i128>,
{
    if result.is_null(column) {
        return Ok(None);
    }
    integer_checked_like_cpp(read_integer_checked_like_cpp(result, column, field)?, field).map(Some)
}

fn read_optional_float_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<Option<f32>> {
    if result.is_null(column) {
        return Ok(None);
    }
    read_float_checked_like_cpp(result, column, field).map(Some)
}

fn player_create_info_row_like_cpp(
    result: &SqlResult,
) -> Result<PlayerCreateInfoPersistenceRowLikeCpp> {
    Ok(PlayerCreateInfoPersistenceRowLikeCpp {
        race: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 0, "PlayerCreateInfo.Race")?,
            "PlayerCreateInfo.Race",
        )?,
        class: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 1, "PlayerCreateInfo.Class")?,
            "PlayerCreateInfo.Class",
        )?,
        map_id: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 2, "PlayerCreateInfo.Map")?,
            "PlayerCreateInfo.Map",
        )?,
        position_x: read_float_checked_like_cpp(result, 3, "PlayerCreateInfo.PositionX")?,
        position_y: read_float_checked_like_cpp(result, 4, "PlayerCreateInfo.PositionY")?,
        position_z: read_float_checked_like_cpp(result, 5, "PlayerCreateInfo.PositionZ")?,
        orientation: read_float_checked_like_cpp(result, 6, "PlayerCreateInfo.Orientation")?,
        npe_map_id: read_optional_integer_checked_like_cpp(result, 7, "PlayerCreateInfo.NpeMap")?,
        npe_position_x: read_optional_float_checked_like_cpp(
            result,
            8,
            "PlayerCreateInfo.NpePositionX",
        )?,
        npe_position_y: read_optional_float_checked_like_cpp(
            result,
            9,
            "PlayerCreateInfo.NpePositionY",
        )?,
        npe_position_z: read_optional_float_checked_like_cpp(
            result,
            10,
            "PlayerCreateInfo.NpePositionZ",
        )?,
        npe_orientation: read_optional_float_checked_like_cpp(
            result,
            11,
            "PlayerCreateInfo.NpeOrientation",
        )?,
        npe_transport_guid: read_optional_integer_checked_like_cpp(
            result,
            12,
            "PlayerCreateInfo.NpeTransportGuid",
        )?,
        npe_transport_entry: read_optional_integer_checked_like_cpp(
            result,
            13,
            "PlayerCreateInfo.NpeTransportEntry",
        )?,
    })
}

fn player_create_cast_spell_row_like_cpp(
    result: &SqlResult,
) -> Result<PlayerCreateCastSpellPersistenceRowLikeCpp> {
    Ok(PlayerCreateCastSpellPersistenceRowLikeCpp {
        race_mask: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 0, "PlayerCreateCastSpell.RaceMask")?,
            "PlayerCreateCastSpell.RaceMask",
        )?,
        class_mask: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 1, "PlayerCreateCastSpell.ClassMask")?,
            "PlayerCreateCastSpell.ClassMask",
        )?,
        spell_id: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 2, "PlayerCreateCastSpell.Spell")?,
            "PlayerCreateCastSpell.Spell",
        )?,
        create_mode: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 3, "PlayerCreateCastSpell.CreateMode")?,
            "PlayerCreateCastSpell.CreateMode",
        )?,
    })
}

fn player_create_custom_spell_row_like_cpp(
    result: &SqlResult,
) -> Result<PlayerCreateCustomSpellPersistenceRowLikeCpp> {
    Ok(PlayerCreateCustomSpellPersistenceRowLikeCpp {
        race_mask: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 0, "PlayerCreateCustomSpell.RaceMask")?,
            "PlayerCreateCustomSpell.RaceMask",
        )?,
        class_mask: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 1, "PlayerCreateCustomSpell.ClassMask")?,
            "PlayerCreateCustomSpell.ClassMask",
        )?,
        spell_id: integer_checked_like_cpp(
            read_integer_checked_like_cpp(result, 2, "PlayerCreateCustomSpell.Spell")?,
            "PlayerCreateCustomSpell.Spell",
        )?,
    })
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

fn classify_rows_like_cpp<T>(result: Result<Vec<T>>) -> PlayerCreationCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => PlayerCreationCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => PlayerCreationCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbPlayerCreationCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbPlayerCreationCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl PlayerCreationCatalogPersistencePortLikeCpp
    for MariaDbPlayerCreationCatalogPersistenceAdapterLikeCpp
{
    fn load_player_create_info_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerCreationCatalogLoadOutcomeLikeCpp<PlayerCreateInfoPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    STARTUP_STATEMENTS_LIKE_CPP[0],
                    player_create_info_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_player_create_cast_spell_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerCreationCatalogLoadOutcomeLikeCpp<PlayerCreateCastSpellPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    STARTUP_STATEMENTS_LIKE_CPP[1],
                    player_create_cast_spell_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_player_create_custom_spell_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerCreationCatalogLoadOutcomeLikeCpp<PlayerCreateCustomSpellPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    STARTUP_STATEMENTS_LIKE_CPP[2],
                    player_create_custom_spell_row_like_cpp,
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
    fn player_creation_statements_keep_existing_rust_startup_order_and_sql() {
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_PLAYER_CREATEINFO,
                WorldStatements::SEL_PLAYER_CREATEINFO_CAST_SPELL,
                WorldStatements::SEL_PLAYER_CREATEINFO_CUSTOM_SPELL,
            ]
        );
        assert!(
            WorldStatements::SEL_PLAYER_CREATEINFO
                .sql()
                .starts_with("SELECT p.race, p.class, p.map")
        );
        assert_eq!(
            WorldStatements::SEL_PLAYER_CREATEINFO_CAST_SPELL.sql(),
            "SELECT raceMask, classMask, spell, createMode FROM playercreateinfo_cast_spell"
        );
        assert_eq!(
            WorldStatements::SEL_PLAYER_CREATEINFO_CUSTOM_SPELL.sql(),
            "SELECT racemask, classmask, Spell FROM playercreateinfo_spell_custom"
        );
    }

    #[test]
    fn integer_conversion_rejects_out_of_range_instead_of_fabricating_values() {
        assert_eq!(integer_checked_like_cpp::<u8>(255, "field").unwrap(), 255);
        assert!(integer_checked_like_cpp::<u8>(256, "field").is_err());
        assert_eq!(integer_checked_like_cpp::<i8>(-1, "field").unwrap(), -1);
        assert!(integer_checked_like_cpp::<u64>(-1, "field").is_err());
    }
}
