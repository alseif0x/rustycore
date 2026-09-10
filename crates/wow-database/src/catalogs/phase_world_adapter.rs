//! MariaDB adapter for C++ `ObjectMgr` immutable World phasing metadata.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    PersistenceFutureLikeCpp, PhaseAreaPersistenceRowLikeCpp, PhaseNamePersistenceRowLikeCpp,
    PhaseWorldCatalogLoadOutcomeLikeCpp, PhaseWorldCatalogPersistencePortLikeCpp,
    TerrainSwapDefaultPersistenceRowLikeCpp, TerrainWorldMapPersistenceRowLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

const PHASE_AREA_STATEMENT_LIKE_CPP: WorldStatements = WorldStatements::SEL_PHASE_AREAS;
const PHASE_NAME_STATEMENT_LIKE_CPP: WorldStatements = WorldStatements::SEL_PHASE_NAMES;
const TERRAIN_WORLD_MAP_STATEMENT_LIKE_CPP: WorldStatements =
    WorldStatements::SEL_TERRAIN_WORLD_MAPS;
const TERRAIN_SWAP_DEFAULT_STATEMENT_LIKE_CPP: WorldStatements =
    WorldStatements::SEL_TERRAIN_SWAP_DEFAULTS;

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

fn u32_checked_like_cpp(value: i128, field: &'static str) -> Result<u32> {
    u32::try_from(value).with_context(|| format!("{field} SQL value {value} is not u32"))
}

fn phase_area_values_like_cpp(values: [i128; 2]) -> Result<PhaseAreaPersistenceRowLikeCpp> {
    Ok(PhaseAreaPersistenceRowLikeCpp {
        area_id: u32_checked_like_cpp(values[0], "PhaseArea.AreaId")?,
        phase_id: u32_checked_like_cpp(values[1], "PhaseArea.PhaseId")?,
    })
}

fn phase_area_row_like_cpp(result: &SqlResult) -> Result<PhaseAreaPersistenceRowLikeCpp> {
    phase_area_values_like_cpp([
        read_integer_checked_like_cpp(result, 0, "PhaseArea.AreaId")?,
        read_integer_checked_like_cpp(result, 1, "PhaseArea.PhaseId")?,
    ])
}

fn phase_name_values_like_cpp(
    phase_id: i128,
    name: String,
) -> Result<PhaseNamePersistenceRowLikeCpp> {
    Ok(PhaseNamePersistenceRowLikeCpp {
        phase_id: u32_checked_like_cpp(phase_id, "PhaseName.ID")?,
        name,
    })
}

fn phase_name_row_like_cpp(result: &SqlResult) -> Result<PhaseNamePersistenceRowLikeCpp> {
    phase_name_values_like_cpp(
        read_integer_checked_like_cpp(result, 0, "PhaseName.ID")?,
        result.read_string(1),
    )
}

fn terrain_world_map_values_like_cpp(
    values: [i128; 2],
) -> Result<TerrainWorldMapPersistenceRowLikeCpp> {
    Ok(TerrainWorldMapPersistenceRowLikeCpp {
        terrain_swap_map: u32_checked_like_cpp(values[0], "TerrainWorldMap.TerrainSwapMap")?,
        ui_map_phase_id: u32_checked_like_cpp(values[1], "TerrainWorldMap.UiMapPhaseId")?,
    })
}

fn terrain_world_map_row_like_cpp(
    result: &SqlResult,
) -> Result<TerrainWorldMapPersistenceRowLikeCpp> {
    terrain_world_map_values_like_cpp([
        read_integer_checked_like_cpp(result, 0, "TerrainWorldMap.TerrainSwapMap")?,
        read_integer_checked_like_cpp(result, 1, "TerrainWorldMap.UiMapPhaseId")?,
    ])
}

fn terrain_swap_default_values_like_cpp(
    values: [i128; 2],
) -> Result<TerrainSwapDefaultPersistenceRowLikeCpp> {
    Ok(TerrainSwapDefaultPersistenceRowLikeCpp {
        map_id: u32_checked_like_cpp(values[0], "TerrainSwapDefault.MapId")?,
        terrain_swap_map: u32_checked_like_cpp(values[1], "TerrainSwapDefault.TerrainSwapMap")?,
    })
}

fn terrain_swap_default_row_like_cpp(
    result: &SqlResult,
) -> Result<TerrainSwapDefaultPersistenceRowLikeCpp> {
    terrain_swap_default_values_like_cpp([
        read_integer_checked_like_cpp(result, 0, "TerrainSwapDefault.MapId")?,
        read_integer_checked_like_cpp(result, 1, "TerrainSwapDefault.TerrainSwapMap")?,
    ])
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

fn classify_rows_like_cpp<T>(result: Result<Vec<T>>) -> PhaseWorldCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => PhaseWorldCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => PhaseWorldCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbPhaseWorldCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbPhaseWorldCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl PhaseWorldCatalogPersistencePortLikeCpp for MariaDbPhaseWorldCatalogPersistenceAdapterLikeCpp {
    fn load_phase_area_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PhaseWorldCatalogLoadOutcomeLikeCpp<PhaseAreaPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(&self.world_db, PHASE_AREA_STATEMENT_LIKE_CPP, |row| {
                    phase_area_row_like_cpp(row)
                })
                .await,
            )
        })
    }

    fn load_phase_name_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PhaseWorldCatalogLoadOutcomeLikeCpp<PhaseNamePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(&self.world_db, PHASE_NAME_STATEMENT_LIKE_CPP, |row| {
                    phase_name_row_like_cpp(row)
                })
                .await,
            )
        })
    }

    fn load_terrain_world_map_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PhaseWorldCatalogLoadOutcomeLikeCpp<TerrainWorldMapPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    TERRAIN_WORLD_MAP_STATEMENT_LIKE_CPP,
                    |row| terrain_world_map_row_like_cpp(row),
                )
                .await,
            )
        })
    }

    fn load_terrain_swap_default_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PhaseWorldCatalogLoadOutcomeLikeCpp<TerrainSwapDefaultPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    TERRAIN_SWAP_DEFAULT_STATEMENT_LIKE_CPP,
                    |row| terrain_swap_default_row_like_cpp(row),
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
    fn phase_world_statements_match_cpp_exactly() {
        assert_eq!(
            PHASE_AREA_STATEMENT_LIKE_CPP.sql(),
            "SELECT AreaId, PhaseId FROM `phase_area`"
        );
        assert_eq!(
            PHASE_NAME_STATEMENT_LIKE_CPP.sql(),
            "SELECT `ID`, `Name` FROM `phase_name`"
        );
        assert_eq!(
            TERRAIN_WORLD_MAP_STATEMENT_LIKE_CPP.sql(),
            "SELECT TerrainSwapMap, UiMapPhaseId FROM `terrain_worldmap`"
        );
        assert_eq!(
            TERRAIN_SWAP_DEFAULT_STATEMENT_LIKE_CPP.sql(),
            "SELECT MapId, TerrainSwapMap FROM `terrain_swap_defaults`"
        );
    }

    #[test]
    fn checked_rows_preserve_every_field_and_reject_invalid_unsigned_values() {
        assert_eq!(
            phase_area_values_like_cpp([1, 2]).unwrap(),
            PhaseAreaPersistenceRowLikeCpp {
                area_id: 1,
                phase_id: 2,
            }
        );
        assert_eq!(
            phase_name_values_like_cpp(3, "name".into()).unwrap(),
            PhaseNamePersistenceRowLikeCpp {
                phase_id: 3,
                name: "name".into(),
            }
        );
        assert_eq!(
            terrain_world_map_values_like_cpp([4, 5]).unwrap(),
            TerrainWorldMapPersistenceRowLikeCpp {
                terrain_swap_map: 4,
                ui_map_phase_id: 5,
            }
        );
        assert_eq!(
            terrain_swap_default_values_like_cpp([6, 7]).unwrap(),
            TerrainSwapDefaultPersistenceRowLikeCpp {
                map_id: 6,
                terrain_swap_map: 7,
            }
        );

        assert!(phase_area_values_like_cpp([-1, 2]).is_err());
        assert!(phase_name_values_like_cpp(i128::from(u32::MAX) + 1, String::new()).is_err());
        assert!(terrain_world_map_values_like_cpp([4, -1]).is_err());
        assert!(terrain_swap_default_values_like_cpp([-1, 7]).is_err());
    }
}
