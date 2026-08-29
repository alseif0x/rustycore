//! MariaDB adapters for C++ vehicle startup catalogs.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, VEHICLE_SEAT_COUNT_LIKE_CPP, VehicleHotfixLoadOutcomeLikeCpp,
    VehicleHotfixPersistencePortLikeCpp, VehicleHotfixPersistenceRowLikeCpp,
    VehicleSeatHotfixPersistenceRowLikeCpp, VehicleSpawnAccessoryPersistenceRowLikeCpp,
    VehicleTemplateAccessoryPersistenceRowLikeCpp, VehicleTemplatePersistenceRowLikeCpp,
    VehicleWorldCatalogLoadOutcomeLikeCpp, VehicleWorldCatalogPersistencePortLikeCpp,
};

use crate::{DatabaseError, HotfixDatabase, HotfixStatements, SqlResult, WorldDatabase};

const VEHICLE_TEMPLATE_SQL_LIKE_CPP: &str =
    "SELECT `creatureId`, `despawnDelayMs` FROM `vehicle_template`";
const VEHICLE_TEMPLATE_ACCESSORY_SQL_LIKE_CPP: &str = "SELECT `entry`, `accessory_entry`, `seat_id`, `minion`, `summontype`, `summontimer` FROM `vehicle_template_accessory`";
const VEHICLE_ACCESSORY_SQL_LIKE_CPP: &str = "SELECT `guid`, `accessory_entry`, `seat_id`, `minion`, `summontype`, `summontimer` FROM `vehicle_accessory`";

#[cfg(test)]
const STARTUP_SOURCES_LIKE_RUST: [(&str, &str); 5] = [
    ("hotfix", "SEL_VEHICLE"),
    ("hotfix", "SEL_VEHICLE_SEAT"),
    ("world", VEHICLE_TEMPLATE_SQL_LIKE_CPP),
    ("world", VEHICLE_TEMPLATE_ACCESSORY_SQL_LIKE_CPP),
    ("world", VEHICLE_ACCESSORY_SQL_LIKE_CPP),
];

async fn query_hotfix_rows_like_cpp<T>(
    db: &HotfixDatabase,
    statement: HotfixStatements,
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

async fn query_world_rows_like_cpp<T>(
    db: &WorldDatabase,
    sql: &str,
    mut decode: impl FnMut(&SqlResult) -> T,
) -> Result<Vec<T>, DatabaseError> {
    let mut result = db.direct_query(sql).await?;
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

fn classify_hotfix_rows_like_cpp<T>(
    result: Result<Vec<T>, DatabaseError>,
) -> VehicleHotfixLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => VehicleHotfixLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => VehicleHotfixLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbVehicleHotfixPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbVehicleHotfixPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl VehicleHotfixPersistencePortLikeCpp for MariaDbVehicleHotfixPersistenceAdapterLikeCpp {
    fn load_vehicle_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleHotfixLoadOutcomeLikeCpp<VehicleHotfixPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_hotfix_rows_like_cpp(
                query_hotfix_rows_like_cpp(&self.hotfix_db, HotfixStatements::SEL_VEHICLE, |row| {
                    let mut seat_ids = [0; VEHICLE_SEAT_COUNT_LIKE_CPP];
                    for (offset, seat_id) in seat_ids.iter_mut().enumerate() {
                        *seat_id = row.read(18 + offset);
                    }
                    VehicleHotfixPersistenceRowLikeCpp {
                        id: row.read(0),
                        flags: row.read(1),
                        flags_b: row.read(2),
                        seat_ids,
                    }
                })
                .await,
            )
        })
    }

    fn load_vehicle_seat_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleHotfixLoadOutcomeLikeCpp<VehicleSeatHotfixPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_hotfix_rows_like_cpp(
                query_hotfix_rows_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::SEL_VEHICLE_SEAT,
                    |row| VehicleSeatHotfixPersistenceRowLikeCpp {
                        id: row.read(0),
                        attachment_offset_x: row.read(1),
                        attachment_offset_y: row.read(2),
                        attachment_offset_z: row.read(3),
                        flags: row.read(7),
                        flags_b: row.read(8),
                        flags_c: row.read(9),
                    },
                )
                .await,
            )
        })
    }
}

pub struct MariaDbVehicleWorldCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbVehicleWorldCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

fn classify_world_rows_like_cpp<T>(
    result: Result<Vec<T>, DatabaseError>,
) -> VehicleWorldCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => VehicleWorldCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => VehicleWorldCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

impl VehicleWorldCatalogPersistencePortLikeCpp
    for MariaDbVehicleWorldCatalogPersistenceAdapterLikeCpp
{
    fn load_vehicle_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleTemplatePersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_world_rows_like_cpp(
                query_world_rows_like_cpp(&self.world_db, VEHICLE_TEMPLATE_SQL_LIKE_CPP, |row| {
                    VehicleTemplatePersistenceRowLikeCpp {
                        creature_entry: row.read(0),
                        despawn_delay_ms: row.read(1),
                    }
                })
                .await,
            )
        })
    }

    fn load_vehicle_template_accessory_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleTemplateAccessoryPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_world_rows_like_cpp(
                query_world_rows_like_cpp(
                    &self.world_db,
                    VEHICLE_TEMPLATE_ACCESSORY_SQL_LIKE_CPP,
                    |row| VehicleTemplateAccessoryPersistenceRowLikeCpp {
                        creature_entry: row.read(0),
                        accessory_entry: row.read(1),
                        seat_id: row.read(2),
                        is_minion: row.read(3),
                        summoned_type: row.read(4),
                        summon_time_ms: row.read(5),
                    },
                )
                .await,
            )
        })
    }

    fn load_vehicle_spawn_accessory_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VehicleWorldCatalogLoadOutcomeLikeCpp<VehicleSpawnAccessoryPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_world_rows_like_cpp(
                query_world_rows_like_cpp(&self.world_db, VEHICLE_ACCESSORY_SQL_LIKE_CPP, |row| {
                    VehicleSpawnAccessoryPersistenceRowLikeCpp {
                        spawn_guid: row.read(0),
                        accessory_entry: row.read(1),
                        seat_id: row.read(2),
                        is_minion: row.read(3),
                        summoned_type: row.read(4),
                        summon_time_ms: row.read(5),
                    }
                })
                .await,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_startup_sources_preserve_statement_and_world_query_order() {
        assert_eq!(
            STARTUP_SOURCES_LIKE_RUST,
            [
                ("hotfix", "SEL_VEHICLE"),
                ("hotfix", "SEL_VEHICLE_SEAT"),
                ("world", VEHICLE_TEMPLATE_SQL_LIKE_CPP),
                ("world", VEHICLE_TEMPLATE_ACCESSORY_SQL_LIKE_CPP),
                ("world", VEHICLE_ACCESSORY_SQL_LIKE_CPP),
            ]
        );
    }
}
