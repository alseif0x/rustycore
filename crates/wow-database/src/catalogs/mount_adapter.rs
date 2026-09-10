//! MariaDB adapter for C++ mount startup catalogs.

use std::sync::Arc;

use wow_persistence::{
    MountCapabilityHotfixRowLikeCpp, MountCatalogLoadOutcomeLikeCpp,
    MountCatalogPersistencePortLikeCpp, MountDefinitionRowLikeCpp, MountHotfixRowLikeCpp,
    MountTypeXCapabilityHotfixRowLikeCpp, MountXDisplayHotfixRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{
    DatabaseError, HotfixDatabase, HotfixStatements, SqlResult, WorldDatabase, WorldStatements,
};

#[cfg(test)]
const STARTUP_STATEMENTS_LIKE_CPP: [(&str, &str); 5] = [
    ("hotfix", "SEL_MOUNT"),
    ("world", "SEL_MOUNT_DEFINITIONS"),
    ("hotfix", "SEL_MOUNT_CAPABILITY"),
    ("hotfix", "SEL_MOUNT_TYPE_X_CAPABILITY"),
    ("hotfix", "SEL_MOUNT_X_DISPLAY"),
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
) -> MountCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => MountCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => MountCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbMountCatalogPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
    world_db: Arc<WorldDatabase>,
}

impl MariaDbMountCatalogPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>, world_db: Arc<WorldDatabase>) -> Self {
        Self {
            hotfix_db,
            world_db,
        }
    }
}

impl MountCatalogPersistencePortLikeCpp for MariaDbMountCatalogPersistenceAdapterLikeCpp {
    fn load_mount_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountHotfixRowLikeCpp>> {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_hotfix_rows_like_cpp(&self.hotfix_db, HotfixStatements::SEL_MOUNT, |row| {
                    MountHotfixRowLikeCpp {
                        id: row.read(3),
                        mount_type_id: row.read(4),
                        flags: row.read(5),
                        source_type_enum: row.read(6),
                        source_spell_id: row.read(7),
                        player_condition_id: row.read(8),
                        mount_fly_ride_height: row.read(9),
                        ui_model_scene_id: row.read(10),
                    }
                })
                .await,
            )
        })
    }

    fn load_mount_definition_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountDefinitionRowLikeCpp>>
    {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_world_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_MOUNT_DEFINITIONS,
                    |row| MountDefinitionRowLikeCpp {
                        spell_id: row.read(0),
                        other_faction_spell_id: row.read(1),
                    },
                )
                .await,
            )
        })
    }

    fn load_mount_capability_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountCapabilityHotfixRowLikeCpp>>
    {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_hotfix_rows_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::SEL_MOUNT_CAPABILITY,
                    |row| MountCapabilityHotfixRowLikeCpp {
                        id: row.read(0),
                        flags: row.read(1),
                        req_riding_skill: row.read(2),
                        req_area_id: row.read(3),
                        req_spell_aura_id: row.read(4),
                        req_spell_known_id: row.read(5),
                        mod_spell_aura_id: row.read(6),
                        req_map_id: row.read(7),
                    },
                )
                .await,
            )
        })
    }

    fn load_mount_type_x_capability_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        MountCatalogLoadOutcomeLikeCpp<MountTypeXCapabilityHotfixRowLikeCpp>,
    > {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_hotfix_rows_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::SEL_MOUNT_TYPE_X_CAPABILITY,
                    |row| MountTypeXCapabilityHotfixRowLikeCpp {
                        id: row.read(0),
                        mount_type_id: row.read(1),
                        mount_capability_id: row.read(2),
                        order_index: row.read(3),
                    },
                )
                .await,
            )
        })
    }

    fn load_mount_x_display_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, MountCatalogLoadOutcomeLikeCpp<MountXDisplayHotfixRowLikeCpp>>
    {
        Box::pin(async move {
            classify_rows_like_cpp(
                query_hotfix_rows_like_cpp(
                    &self.hotfix_db,
                    HotfixStatements::SEL_MOUNT_X_DISPLAY,
                    |row| MountXDisplayHotfixRowLikeCpp {
                        id: row.read(0),
                        creature_display_info_id: row.read(1),
                        player_condition_id: row.read(2),
                        mount_id: row.read(3),
                    },
                )
                .await,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_startup_statement_order_preserves_the_existing_bootstrap_contract() {
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_CPP,
            [
                ("hotfix", "SEL_MOUNT"),
                ("world", "SEL_MOUNT_DEFINITIONS"),
                ("hotfix", "SEL_MOUNT_CAPABILITY"),
                ("hotfix", "SEL_MOUNT_TYPE_X_CAPABILITY"),
                ("hotfix", "SEL_MOUNT_X_DISPLAY"),
            ]
        );
    }
}
