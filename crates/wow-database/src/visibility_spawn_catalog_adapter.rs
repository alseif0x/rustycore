//! MariaDB adapter for transitional visibility spawn reads.

use std::sync::Arc;

use wow_persistence::{
    CreatureVisibilityPersistenceRowLikeCpp, GameObjectVisibilityPersistenceRowLikeCpp,
    PersistenceFutureLikeCpp, VisibilitySpawnCatalogOutcomeLikeCpp,
    VisibilitySpawnCatalogPersistencePortLikeCpp, VisibilitySpawnCatalogRequestLikeCpp,
};

use crate::{PreparedStatement, SqlResult, WorldDatabase, WorldStatements};

fn bounds_statement_like_cpp(
    statement: WorldStatements,
    request: VisibilitySpawnCatalogRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(statement);
    statement.set_u16(0, request.map_id);
    statement.set_f32(1, request.x_min);
    statement.set_f32(2, request.x_max);
    statement.set_f32(3, request.y_min);
    statement.set_f32(4, request.y_max);
    statement
}

fn optional_u32(result: &SqlResult, column: usize) -> Option<u32> {
    result
        .try_read::<Option<u32>>(column)
        .flatten()
        .or_else(|| {
            result
                .try_read::<Option<i64>>(column)
                .flatten()
                .map(|value| value.max(0) as u32)
        })
        .or_else(|| result.try_read(column))
        .or_else(|| {
            result
                .try_read::<i64>(column)
                .map(|value| value.max(0) as u32)
        })
}

fn optional_u64(result: &SqlResult, column: usize) -> Option<u64> {
    result
        .try_read::<Option<i64>>(column)
        .flatten()
        .map(|value| value as u64)
        .or_else(|| result.try_read::<Option<u64>>(column).flatten())
        .or_else(|| result.try_read::<i64>(column).map(|value| value as u64))
        .or_else(|| result.try_read(column))
}

fn decode_creature(result: &SqlResult) -> CreatureVisibilityPersistenceRowLikeCpp {
    CreatureVisibilityPersistenceRowLikeCpp {
        spawn_guid: result
            .try_read::<i64>(0)
            .map(|v| v as u64)
            .or_else(|| result.try_read(0))
            .unwrap_or(0),
        entry: result.try_read(1).unwrap_or(0),
        position: std::array::from_fn(|index| result.try_read(2 + index).unwrap_or(0.0)),
        current_health: result.try_read(6).unwrap_or(100),
        current_mana: result.try_read(7).unwrap_or(0),
        model_id: result.try_read(8).unwrap_or(0),
        min_level: result.try_read::<Option<u8>>(9).flatten().unwrap_or(1),
        faction: i32::from(result.try_read::<u16>(11).unwrap_or(35)),
        template_npc_flags: result
            .try_read::<i64>(12)
            .map(|v| v as u64)
            .or_else(|| result.try_read(12))
            .unwrap_or(0),
        template_unit_flags: std::array::from_fn(|index| result.try_read(13 + index).unwrap_or(0)),
        speed_walk: result.try_read(16).unwrap_or(1.0),
        speed_run: result.try_read(17).unwrap_or(1.14286),
        scale: result.try_read(18).unwrap_or(1.0),
        unit_class: result.try_read(19).unwrap_or(1),
        flags_extra: result.try_read(20).unwrap_or(0),
        attack_time: [
            result.try_read(21).unwrap_or(2000),
            result
                .try_read(22)
                .unwrap_or_else(|| result.try_read(21).unwrap_or(2000)),
        ],
        template_display_id: result.try_read::<Option<u32>>(23).flatten().unwrap_or(0),
        template_display_scale: result
            .try_read::<Option<f32>>(42)
            .flatten()
            .or_else(|| result.try_read(42))
            .unwrap_or(1.0),
        loot_id: result.try_read::<Option<u32>>(24).flatten().unwrap_or(0),
        skin_loot_id: result.try_read::<Option<u32>>(25).flatten().unwrap_or(0),
        gold: [
            result.try_read::<Option<u32>>(26).flatten().unwrap_or(0),
            result.try_read::<Option<u32>>(27).flatten().unwrap_or(0),
        ],
        phase_use_flags: result
            .try_read::<u8>(28)
            .or_else(|| result.try_read::<i16>(28).map(|v| v.max(0) as u8))
            .unwrap_or(0),
        phase_id: result
            .try_read::<u16>(29)
            .or_else(|| result.try_read::<i32>(29).map(|v| v.max(0) as u16))
            .unwrap_or(0),
        phase_group_id: result
            .try_read::<u32>(30)
            .or_else(|| result.try_read::<i32>(30).map(|v| v.max(0) as u32))
            .unwrap_or(0),
        terrain_swap_map: result.try_read(31).unwrap_or(-1),
        ground_movement_type: result
            .try_read::<Option<u8>>(32)
            .flatten()
            .or_else(|| result.try_read(32))
            .or_else(|| result.try_read::<i16>(32).map(|v| v.max(0) as u8))
            .unwrap_or(1),
        swim_allowed: result
            .try_read::<Option<u8>>(33)
            .flatten()
            .or_else(|| result.try_read(33))
            .or_else(|| result.try_read::<i16>(33).map(|v| v.max(0) as u8))
            .unwrap_or(1)
            != 0,
        flight_movement_type: result
            .try_read::<Option<u8>>(34)
            .flatten()
            .or_else(|| result.try_read(34))
            .or_else(|| result.try_read::<i16>(34).map(|v| v.max(0) as u8))
            .unwrap_or(0),
        rooted: result
            .try_read::<Option<u8>>(35)
            .flatten()
            .or_else(|| result.try_read(35))
            .or_else(|| result.try_read::<i16>(35).map(|v| v.max(0) as u8))
            .unwrap_or(0)
            != 0,
        chase_movement_type: result
            .try_read::<Option<u8>>(36)
            .flatten()
            .or_else(|| result.try_read(36))
            .or_else(|| result.try_read::<i16>(36).map(|v| v.max(0) as u8))
            .unwrap_or(1),
        random_movement_type: result
            .try_read::<Option<u8>>(37)
            .flatten()
            .or_else(|| result.try_read(37))
            .or_else(|| result.try_read::<i16>(37).map(|v| v.max(0) as u8))
            .unwrap_or(0),
        interaction_pause_timer_ms: optional_u32(result, 38).unwrap_or(180_000),
        wander_distance: result
            .try_read::<Option<f32>>(39)
            .flatten()
            .or_else(|| result.try_read(39))
            .unwrap_or(0.0)
            .max(0.0),
        effective_movement_type: result
            .try_read::<Option<u8>>(40)
            .flatten()
            .or_else(|| result.try_read(40))
            .or_else(|| result.try_read::<i16>(40).map(|v| v.max(0) as u8))
            .unwrap_or(0),
        waypoint_path_id: optional_u32(result, 41)
            .or_else(|| result.try_read::<i64>(41).map(|v| v.max(0) as u32))
            .unwrap_or(0),
        classification: result.try_read(43).unwrap_or(0),
        regen_health: result
            .try_read::<u8>(44)
            .map(|v| v != 0)
            .or_else(|| result.try_read::<i8>(44).map(|v| v != 0))
            .unwrap_or(true),
        spawn_npc_flags_override: optional_u64(result, 45),
        spawn_unit_flags_override: [
            optional_u32(result, 46),
            optional_u32(result, 47),
            optional_u32(result, 48),
        ],
        equipment_id: result
            .try_read::<i8>(49)
            .map(i16::from)
            .or_else(|| result.try_read(49))
            .unwrap_or(0),
        respawn_delay_secs: optional_u32(result, 50).unwrap_or(300),
        spawn_difficulties: result
            .try_read::<Option<String>>(51)
            .flatten()
            .or_else(|| result.try_read(51))
            .unwrap_or_default(),
        script_name: result
            .try_read::<Option<String>>(52)
            .flatten()
            .or_else(|| result.try_read(52))
            .unwrap_or_default(),
        string_id: result
            .try_read::<Option<String>>(53)
            .flatten()
            .or_else(|| result.try_read::<String>(53))
            .filter(|v| !v.is_empty()),
        vehicle_id: optional_u32(result, 54).unwrap_or(0),
    }
}

fn decode_gameobject(result: &SqlResult) -> GameObjectVisibilityPersistenceRowLikeCpp {
    GameObjectVisibilityPersistenceRowLikeCpp {
        spawn_guid: result
            .try_read::<i64>(0)
            .map(|v| v as u64)
            .or_else(|| result.try_read(0))
            .unwrap_or(0),
        entry: result.try_read(1).unwrap_or(0),
        position: std::array::from_fn(|index| result.try_read(2 + index).unwrap_or(0.0)),
        rotation: std::array::from_fn(|index| result.try_read(6 + index).unwrap_or(0.0)),
        anim_progress: result.try_read(10).unwrap_or(255),
        state: result.try_read::<u8>(11).unwrap_or(1) as i8,
        go_type: result.try_read(12).unwrap_or(0),
        display_id: result.try_read(13).unwrap_or(0),
        scale: result.try_read(15).unwrap_or(1.0),
        template_data: std::array::from_fn(|index| result.try_read::<i32>(16 + index).unwrap_or(0)),
        phase_use_flags: result
            .try_read::<u8>(51)
            .or_else(|| result.try_read::<i16>(51).map(|v| v.max(0) as u8))
            .unwrap_or(0),
        phase_id: result
            .try_read::<u16>(52)
            .or_else(|| result.try_read::<i32>(52).map(|v| v.max(0) as u16))
            .unwrap_or(0),
        phase_group_id: result
            .try_read::<u32>(53)
            .or_else(|| result.try_read::<i32>(53).map(|v| v.max(0) as u32))
            .unwrap_or(0),
        terrain_swap_map: result.try_read(54).unwrap_or(-1),
        effective_flags: result
            .try_read::<u32>(55)
            .or_else(|| {
                result
                    .try_read::<i64>(55)
                    .and_then(|value| u32::try_from(value).ok())
            })
            .unwrap_or(0),
        effective_faction: result
            .try_read::<u32>(56)
            .or_else(|| {
                result
                    .try_read::<i64>(56)
                    .and_then(|value| u32::try_from(value).ok())
            })
            .unwrap_or(0),
        override_source_known: result
            .try_read::<u8>(57)
            .map(|v| v != 0)
            .or_else(|| result.try_read::<i64>(57).map(|v| v != 0))
            .unwrap_or(false),
        parent_rotation: [
            result.try_read(58).unwrap_or(0.0),
            result.try_read(59).unwrap_or(0.0),
            result.try_read(60).unwrap_or(0.0),
            result.try_read(61).unwrap_or(1.0),
        ],
    }
}

pub struct MariaDbVisibilitySpawnCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}
impl MariaDbVisibilitySpawnCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl VisibilitySpawnCatalogPersistencePortLikeCpp
    for MariaDbVisibilitySpawnCatalogPersistenceAdapterLikeCpp
{
    fn load_creatures_in_bounds_like_cpp(
        &self,
        request: VisibilitySpawnCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VisibilitySpawnCatalogOutcomeLikeCpp<CreatureVisibilityPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            load_rows(
                &self.world_db,
                bounds_statement_like_cpp(WorldStatements::SEL_CREATURES_IN_RANGE, request),
                decode_creature,
            )
            .await
        })
    }
    fn load_gameobjects_in_bounds_like_cpp(
        &self,
        request: VisibilitySpawnCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        '_,
        VisibilitySpawnCatalogOutcomeLikeCpp<GameObjectVisibilityPersistenceRowLikeCpp>,
    > {
        Box::pin(async move {
            load_rows(
                &self.world_db,
                bounds_statement_like_cpp(WorldStatements::SEL_GAMEOBJECTS_IN_RANGE, request),
                decode_gameobject,
            )
            .await
        })
    }
}

async fn load_rows<T>(
    db: &WorldDatabase,
    statement: PreparedStatement,
    decode: fn(&SqlResult) -> T,
) -> VisibilitySpawnCatalogOutcomeLikeCpp<T> {
    let mut result = match db.query(&statement).await {
        Ok(result) => result,
        Err(error) => {
            return VisibilitySpawnCatalogOutcomeLikeCpp::Failed {
                reason: error.to_string(),
            };
        }
    };
    let mut rows = Vec::new();
    if !result.is_empty() {
        loop {
            rows.push(decode(&result));
            if !result.next_row() {
                break;
            }
        }
    }
    VisibilitySpawnCatalogOutcomeLikeCpp::Loaded(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqlParam, StatementDef};

    #[test]
    fn visibility_catalog_preserves_statement_identity_and_bounds_bind_order_like_cpp() {
        let request = VisibilitySpawnCatalogRequestLikeCpp {
            map_id: 530,
            x_min: -42.5,
            x_max: 15.25,
            y_min: 7.0,
            y_max: 99.75,
        };

        for statement_id in [
            WorldStatements::SEL_CREATURES_IN_RANGE,
            WorldStatements::SEL_GAMEOBJECTS_IN_RANGE,
        ] {
            let statement = bounds_statement_like_cpp(statement_id, request);
            assert_eq!(statement.sql(), statement_id.sql());
            assert_eq!(
                statement.params(),
                [
                    SqlParam::U16(530),
                    SqlParam::F32(-42.5),
                    SqlParam::F32(15.25),
                    SqlParam::F32(7.0),
                    SqlParam::F32(99.75),
                ]
            );
        }
    }
}
