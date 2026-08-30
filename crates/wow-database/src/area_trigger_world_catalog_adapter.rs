//! MariaDB adapter for represented AreaTrigger World-table rows.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    AreaTriggerDestinationPersistenceRowLikeCpp, AreaTriggerScriptPersistenceRowLikeCpp,
    AreaTriggerTeleportPersistenceRowLikeCpp, AreaTriggerWorldCatalogPersistencePortLikeCpp,
    AreaTriggerWorldLoadOutcomeLikeCpp, PersistenceFutureLikeCpp,
    QuestAreaTriggerPersistenceRowLikeCpp, TavernAreaTriggerPersistenceRowLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

const AREA_TRIGGER_STATEMENTS_LIKE_CPP: [WorldStatements; 5] = [
    WorldStatements::SEL_AREA_TRIGGER_TELEPORT,
    WorldStatements::SEL_AREA_TRIGGER_SCRIPTS,
    WorldStatements::SEL_AREA_TRIGGER_TELEPORT_RELATIONS,
    WorldStatements::SEL_QUEST_AREA_TRIGGER_RELATIONS,
    WorldStatements::SEL_TAVERN_AREA_TRIGGERS,
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

fn u32_field_like_cpp(value: i128, field: &'static str) -> Result<u32> {
    if let Ok(value) = u32::try_from(value) {
        return Ok(value);
    }
    i32::try_from(value)
        .map(|value| value as u32)
        .with_context(|| format!("{field} SQL value {value} is outside the C++ uint32 field range"))
}

fn read_u32_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<u32> {
    u32_field_like_cpp(read_integer_checked_like_cpp(result, column, field)?, field)
}

fn read_f32_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<f32> {
    result
        .try_read::<f32>(column)
        .or_else(|| result.try_read::<f64>(column).map(|value| value as f32))
        .with_context(|| format!("missing or non-floating {field} SQL column {column}"))
}

fn read_string_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<String> {
    result
        .try_read::<String>(column)
        .with_context(|| format!("missing or non-string {field} SQL column {column}"))
}

fn destination_values_like_cpp(
    values: (i128, i128, f32, f32, f32, f32),
) -> Result<AreaTriggerDestinationPersistenceRowLikeCpp> {
    Ok(AreaTriggerDestinationPersistenceRowLikeCpp {
        trigger_id: u32_field_like_cpp(values.0, "AreaTriggerDestination.ID")?,
        target_map: u32_field_like_cpp(values.1, "AreaTriggerDestination.MapID")?,
        target_x: values.2,
        target_y: values.3,
        target_z: values.4,
        target_orientation: values.5,
    })
}

fn destination_row_like_cpp(
    row: &SqlResult,
) -> Result<AreaTriggerDestinationPersistenceRowLikeCpp> {
    destination_values_like_cpp((
        read_integer_checked_like_cpp(row, 0, "AreaTriggerDestination.ID")?,
        read_integer_checked_like_cpp(row, 1, "AreaTriggerDestination.MapID")?,
        read_f32_checked_like_cpp(row, 2, "AreaTriggerDestination.LocX")?,
        read_f32_checked_like_cpp(row, 3, "AreaTriggerDestination.LocY")?,
        read_f32_checked_like_cpp(row, 4, "AreaTriggerDestination.LocZ")?,
        read_f32_checked_like_cpp(row, 5, "AreaTriggerDestination.Facing")?,
    ))
}

fn script_row_like_cpp(row: &SqlResult) -> Result<AreaTriggerScriptPersistenceRowLikeCpp> {
    Ok(AreaTriggerScriptPersistenceRowLikeCpp {
        trigger_id: read_u32_checked_like_cpp(row, 0, "AreaTriggerScript.Entry")?,
        script_name: read_string_checked_like_cpp(row, 1, "AreaTriggerScript.ScriptName")?,
    })
}

fn teleport_row_like_cpp(row: &SqlResult) -> Result<AreaTriggerTeleportPersistenceRowLikeCpp> {
    Ok(AreaTriggerTeleportPersistenceRowLikeCpp {
        trigger_id: read_u32_checked_like_cpp(row, 0, "AreaTriggerTeleport.ID")?,
        port_loc_id: read_u32_checked_like_cpp(row, 1, "AreaTriggerTeleport.PortLocID")?,
    })
}

fn quest_row_like_cpp(row: &SqlResult) -> Result<QuestAreaTriggerPersistenceRowLikeCpp> {
    Ok(QuestAreaTriggerPersistenceRowLikeCpp {
        trigger_id: read_u32_checked_like_cpp(row, 0, "QuestAreaTrigger.ID")?,
        quest_id: read_u32_checked_like_cpp(row, 1, "QuestAreaTrigger.Quest")?,
    })
}

fn tavern_row_like_cpp(row: &SqlResult) -> Result<TavernAreaTriggerPersistenceRowLikeCpp> {
    Ok(TavernAreaTriggerPersistenceRowLikeCpp {
        trigger_id: read_u32_checked_like_cpp(row, 0, "TavernAreaTrigger.ID")?,
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

fn classify_rows_like_cpp<T>(result: Result<Vec<T>>) -> AreaTriggerWorldLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => AreaTriggerWorldLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => AreaTriggerWorldLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbAreaTriggerWorldCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbAreaTriggerWorldCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl AreaTriggerWorldCatalogPersistencePortLikeCpp
    for MariaDbAreaTriggerWorldCatalogPersistenceAdapterLikeCpp
{
    fn load_destination_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerDestinationPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    AREA_TRIGGER_STATEMENTS_LIKE_CPP[0],
                    destination_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_script_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerScriptPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    AREA_TRIGGER_STATEMENTS_LIKE_CPP[1],
                    script_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_teleport_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerTeleportPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    AREA_TRIGGER_STATEMENTS_LIKE_CPP[2],
                    teleport_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_quest_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<QuestAreaTriggerPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    AREA_TRIGGER_STATEMENTS_LIKE_CPP[3],
                    quest_row_like_cpp,
                )
                .await,
            )
        })
    }

    fn load_tavern_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<TavernAreaTriggerPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    AREA_TRIGGER_STATEMENTS_LIKE_CPP[4],
                    tavern_row_like_cpp,
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
    fn statements_keep_each_area_trigger_operation_exact_and_independent() {
        assert_eq!(
            AREA_TRIGGER_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_AREA_TRIGGER_TELEPORT,
                WorldStatements::SEL_AREA_TRIGGER_SCRIPTS,
                WorldStatements::SEL_AREA_TRIGGER_TELEPORT_RELATIONS,
                WorldStatements::SEL_QUEST_AREA_TRIGGER_RELATIONS,
                WorldStatements::SEL_TAVERN_AREA_TRIGGERS,
            ]
        );
        assert_eq!(
            AREA_TRIGGER_STATEMENTS_LIKE_CPP[1].sql(),
            "SELECT entry, ScriptName FROM areatrigger_scripts"
        );
        assert_eq!(
            AREA_TRIGGER_STATEMENTS_LIKE_CPP[2].sql(),
            "SELECT ID, PortLocID FROM areatrigger_teleport"
        );
        assert_eq!(
            AREA_TRIGGER_STATEMENTS_LIKE_CPP[3].sql(),
            "SELECT id, quest FROM areatrigger_involvedrelation"
        );
        assert_eq!(
            AREA_TRIGGER_STATEMENTS_LIKE_CPP[4].sql(),
            "SELECT id FROM areatrigger_tavern"
        );
    }

    #[test]
    fn typed_destination_preserves_fields_and_rejects_out_of_width_values() {
        assert_eq!(
            destination_values_like_cpp((7, 530, 1.0, 2.0, 3.0, 4.0)).unwrap(),
            AreaTriggerDestinationPersistenceRowLikeCpp {
                trigger_id: 7,
                target_map: 530,
                target_x: 1.0,
                target_y: 2.0,
                target_z: 3.0,
                target_orientation: 4.0,
            }
        );
        assert_eq!(u32_field_like_cpp(-1, "field").unwrap(), u32::MAX);
        assert!(u32_field_like_cpp(i128::from(u32::MAX) + 1, "field").is_err());
    }
}
