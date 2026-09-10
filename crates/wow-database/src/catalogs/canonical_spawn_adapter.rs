//! MariaDB adapter for the staged canonical spawn startup catalog.

use std::sync::Arc;

use anyhow::{Result, bail};
use wow_persistence::{
    AreaTriggerSpawnPersistenceRowLikeCpp, CanonicalSpawnCatalogLoadOutcomeLikeCpp,
    CanonicalSpawnCatalogPersistencePortLikeCpp, CreatureFormationPersistenceRowLikeCpp,
    CreatureSpawnPersistenceRowLikeCpp, GameObjectSpawnPersistenceRowLikeCpp,
    LinkedRespawnPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
    PoolAutospawnCandidatePersistenceRowLikeCpp, PoolMemberKindPersistenceLikeCpp,
    PoolMemberPersistenceRowLikeCpp, PoolTemplatePersistenceRowLikeCpp,
    SpawnGroupMemberPersistenceRowLikeCpp, WaypointPathCatalogLikeCpp,
    WaypointPathNodePersistenceRowLikeCpp, WaypointPathPersistenceRowLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

const CREATURE_GROUND_MOVEMENT_RUN_LIKE_CPP: u8 = 1;
const CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP: u32 = 180_000;

async fn query_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> Result<T>,
) -> Result<Vec<T>> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::with_capacity(result.count());
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

fn read_u32_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<u32> {
    if result.is_null(column) {
        return Ok(0);
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(value);
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return u32::try_from(value).map_err(|_| {
            anyhow::anyhow!("{field} value {value} exceeds the represented u32 domain")
        });
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return Ok(u32::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return Ok(u32::from(value));
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return u32::try_from(value).map_err(|_| {
            anyhow::anyhow!("{field} value {value} is outside the represented unsigned domain")
        });
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return u32::try_from(value).map_err(|_| {
            anyhow::anyhow!("{field} value {value} is outside the represented unsigned domain")
        });
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return u32::try_from(value).map_err(|_| {
            anyhow::anyhow!("{field} value {value} is outside the represented unsigned domain")
        });
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return u32::try_from(value).map_err(|_| {
            anyhow::anyhow!("{field} value {value} is outside the represented unsigned domain")
        });
    }
    bail!("could not decode {field} at column {column} as a C++ unsigned DB field")
}

fn read_u64_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<u64> {
    if result.is_null(column) {
        return Ok(0);
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return Ok(value);
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(u64::from(value));
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return Ok(u64::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return Ok(u64::from(value));
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return u64::try_from(value)
            .map_err(|_| anyhow::anyhow!("{field} value {value} is negative"));
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return u64::try_from(value)
            .map_err(|_| anyhow::anyhow!("{field} value {value} is negative"));
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return u64::try_from(value)
            .map_err(|_| anyhow::anyhow!("{field} value {value} is negative"));
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return u64::try_from(value)
            .map_err(|_| anyhow::anyhow!("{field} value {value} is negative"));
    }
    bail!("could not decode {field} at column {column} as a C++ unsigned 64-bit DB field")
}

fn read_u8_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<u8> {
    u8::try_from(read_u32_like_cpp(result, column, field)?)
        .map_err(|_| anyhow::anyhow!("{field} exceeds the represented u8 domain"))
}

fn read_u16_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<u16> {
    u16::try_from(read_u32_like_cpp(result, column, field)?)
        .map_err(|_| anyhow::anyhow!("{field} exceeds the represented u16 domain"))
}

fn creature_spawntimesecs_to_i32_like_cpp(value: u32) -> Result<i32> {
    i32::try_from(value).map_err(|_| {
        anyhow::anyhow!(
            "creature.spawntimesecs value {value} exceeds the represented int32 SpawnData domain"
        )
    })
}

fn pool_member_kind_raw_like_cpp(kind: PoolMemberKindPersistenceLikeCpp) -> u8 {
    match kind {
        PoolMemberKindPersistenceLikeCpp::Creature => 0,
        PoolMemberKindPersistenceLikeCpp::GameObject => 1,
        PoolMemberKindPersistenceLikeCpp::Pool => 2,
    }
}

async fn load_creature_spawns_like_cpp(
    db: &WorldDatabase,
) -> Result<Vec<CreatureSpawnPersistenceRowLikeCpp>> {
    query_rows_like_cpp(db, WorldStatements::SEL_CREATURE_SPAWNS, |result| {
        Ok(CreatureSpawnPersistenceRowLikeCpp {
            spawn_id: result.read(0),
            entry: result.read(1),
            map_id: result.read(2),
            x: result.read(3),
            y: result.read(4),
            z: result.read(5),
            orientation: result.read(6),
            model_id: result.try_read(7).unwrap_or(0),
            equipment_id: result.try_read(8).unwrap_or(0),
            spawn_time_secs: creature_spawntimesecs_to_i32_like_cpp(result.read(9))?,
            wander_distance: result.try_read(10).unwrap_or(0.0),
            curhealth: result.try_read(12).unwrap_or(0),
            curmana: result.try_read(13).unwrap_or(0),
            movement_type: result.try_read(14).unwrap_or(0),
            spawn_difficulties: result.read(15),
            event_entry: result.try_read(16).unwrap_or(0),
            pool_id: result.try_read(17).unwrap_or(0),
            npc_flags: result.try_read::<Option<u64>>(18).unwrap_or(None),
            unit_flags: result.try_read::<Option<u32>>(19).unwrap_or(None),
            unit_flags2: result.try_read::<Option<u32>>(20).unwrap_or(None),
            unit_flags3: result.try_read::<Option<u32>>(21).unwrap_or(None),
            phase_use_flags: result.read(22),
            phase_id: read_u32_like_cpp(result, 23, "creature.phaseid")?,
            phase_group: read_u32_like_cpp(result, 24, "creature.phasegroup")?,
            terrain_swap_map: result.read(25),
            script_name: result.try_read(26).unwrap_or_default(),
            string_id: result.try_read(27).unwrap_or_default(),
            ground_movement_type: result
                .try_read::<Option<u8>>(28)
                .flatten()
                .unwrap_or(CREATURE_GROUND_MOVEMENT_RUN_LIKE_CPP),
            swim_allowed: result.try_read::<Option<u8>>(29).flatten().unwrap_or(1) != 0,
            flight_movement_type: result.try_read::<Option<u8>>(30).flatten().unwrap_or(0),
            rooted: result.try_read::<Option<u8>>(31).flatten().unwrap_or(0) != 0,
            chase_movement_type: result.try_read::<Option<u8>>(32).flatten().unwrap_or(0),
            random_movement_type: result.try_read::<Option<u8>>(33).flatten().unwrap_or(0),
            interaction_pause_timer_ms: result
                .try_read::<Option<u32>>(34)
                .flatten()
                .unwrap_or(CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP),
        })
    })
    .await
}

async fn load_waypoints_like_cpp(db: &WorldDatabase) -> Result<WaypointPathCatalogLikeCpp> {
    let paths = query_rows_like_cpp(db, WorldStatements::SEL_WAYPOINT_PATHS, |result| {
        Ok(WaypointPathPersistenceRowLikeCpp {
            path_id: read_u32_like_cpp(result, 0, "waypoint_path.PathId")?,
            move_type: result.try_read(1).unwrap_or(0),
            flags: result.try_read(2).unwrap_or(0),
        })
    })
    .await?;
    let nodes = query_rows_like_cpp(db, WorldStatements::SEL_WAYPOINT_PATH_NODES, |result| {
        Ok(WaypointPathNodePersistenceRowLikeCpp {
            path_id: read_u32_like_cpp(result, 0, "waypoint_path_node.PathId")?,
            node_id: read_u32_like_cpp(result, 1, "waypoint_path_node.NodeId")?,
            x: result.read(2),
            y: result.read(3),
            z: result.read(4),
            orientation: result.try_read::<Option<f32>>(5).unwrap_or(None),
            delay: read_u32_like_cpp(result, 6, "waypoint_path_node.Delay")?,
        })
    })
    .await?;
    Ok(WaypointPathCatalogLikeCpp { paths, nodes })
}

async fn load_formations_like_cpp(
    db: &WorldDatabase,
) -> Result<Vec<CreatureFormationPersistenceRowLikeCpp>> {
    query_rows_like_cpp(db, WorldStatements::SEL_CREATURE_FORMATIONS, |result| {
        Ok(CreatureFormationPersistenceRowLikeCpp {
            leader_spawn_id: read_u64_like_cpp(result, 0, "creature_formations.leaderGUID")?,
            member_spawn_id: read_u64_like_cpp(result, 1, "creature_formations.memberGUID")?,
            dist: result.read(2),
            angle_degrees: result.read(3),
            group_ai: read_u32_like_cpp(result, 4, "creature_formations.groupAI")?,
            point_1: u32::from(read_u16_like_cpp(result, 5, "creature_formations.point_1")?),
            point_2: u32::from(read_u16_like_cpp(result, 6, "creature_formations.point_2")?),
        })
    })
    .await
}

async fn load_gameobjects_like_cpp(
    db: &WorldDatabase,
) -> Result<Vec<GameObjectSpawnPersistenceRowLikeCpp>> {
    query_rows_like_cpp(db, WorldStatements::SEL_GAMEOBJECT_SPAWNS, |result| {
        Ok(GameObjectSpawnPersistenceRowLikeCpp {
            spawn_id: result.read(0),
            entry: result.read(1),
            map_id: result.read(2),
            x: result.read(3),
            y: result.read(4),
            z: result.read(5),
            orientation: result.read(6),
            rotation: [
                result.read(7),
                result.read(8),
                result.read(9),
                result.read(10),
            ],
            spawn_time_secs: result.read(11),
            anim_progress: result.read(12),
            state: result.read(13),
            spawn_difficulties: result.read(14),
            event_entry: result.try_read(15).unwrap_or(0),
            pool_id: result.try_read(16).unwrap_or(0),
            phase_use_flags: result.read(17),
            phase_id: read_u32_like_cpp(result, 18, "gameobject.phaseid")?,
            phase_group: read_u32_like_cpp(result, 19, "gameobject.phasegroup")?,
            terrain_swap_map: result.read(20),
            script_name: result.try_read(21).unwrap_or_default(),
            string_id: result.try_read(22).unwrap_or_default(),
        })
    })
    .await
}

async fn load_area_triggers_like_cpp(
    db: &WorldDatabase,
) -> Result<Vec<AreaTriggerSpawnPersistenceRowLikeCpp>> {
    query_rows_like_cpp(db, WorldStatements::SEL_AREATRIGGER_SPAWNS, |result| {
        Ok(AreaTriggerSpawnPersistenceRowLikeCpp {
            spawn_id: result.read(0),
            create_properties_id: result.read(1),
            is_custom: result.read(2),
            map_id: result.read(3),
            spawn_difficulties: result.read(4),
            x: result.read(5),
            y: result.read(6),
            z: result.read(7),
            orientation: result.read(8),
            phase_use_flags: result.read(9),
            phase_id: read_u32_like_cpp(result, 10, "areatrigger.phaseid")?,
            phase_group: read_u32_like_cpp(result, 11, "areatrigger.phasegroup")?,
            spell_for_visuals: result.try_read(12).unwrap_or(None),
            script_name: result.try_read(13).unwrap_or_default(),
        })
    })
    .await
}

async fn load_linked_like_cpp(
    db: &WorldDatabase,
) -> Result<Vec<LinkedRespawnPersistenceRowLikeCpp>> {
    query_rows_like_cpp(db, WorldStatements::SEL_LINKED_RESPAWNS, |result| {
        Ok(LinkedRespawnPersistenceRowLikeCpp {
            guid: read_u64_like_cpp(result, 0, "linked_respawn.guid")?,
            linked_guid: read_u64_like_cpp(result, 1, "linked_respawn.linkedGuid")?,
            link_type: read_u8_like_cpp(result, 2, "linked_respawn.linkType")?,
        })
    })
    .await
}

async fn load_pool_members_like_cpp(
    db: &WorldDatabase,
    kind: u8,
) -> Result<Vec<PoolMemberPersistenceRowLikeCpp>> {
    let mut statement = db.prepare(WorldStatements::SEL_POOL_MEMBERS_BY_TYPE);
    statement.set_u8(0, kind);
    let mut result = db.query(&statement).await?;
    let mut rows = Vec::with_capacity(result.count());
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(PoolMemberPersistenceRowLikeCpp {
            spawn_id: result.read(0),
            pool_spawn_id: result.read(1),
            chance: result.read(2),
        });
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

fn classify_like_cpp<T>(result: Result<T>) -> CanonicalSpawnCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbCanonicalSpawnCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbCanonicalSpawnCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl CanonicalSpawnCatalogPersistencePortLikeCpp
    for MariaDbCanonicalSpawnCatalogPersistenceAdapterLikeCpp
{
    fn load_creature_spawns_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<CreatureSpawnPersistenceRowLikeCpp>>,
    > {
        Box::pin(
            async move { classify_like_cpp(load_creature_spawns_like_cpp(&self.world_db).await) },
        )
    }
    fn load_waypoint_paths_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<WaypointPathCatalogLikeCpp>,
    > {
        Box::pin(async move { classify_like_cpp(load_waypoints_like_cpp(&self.world_db).await) })
    }
    fn load_creature_formations_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<CreatureFormationPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { classify_like_cpp(load_formations_like_cpp(&self.world_db).await) })
    }
    fn load_gameobject_spawns_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<GameObjectSpawnPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { classify_like_cpp(load_gameobjects_like_cpp(&self.world_db).await) })
    }
    fn load_area_trigger_spawns_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<AreaTriggerSpawnPersistenceRowLikeCpp>>,
    > {
        Box::pin(
            async move { classify_like_cpp(load_area_triggers_like_cpp(&self.world_db).await) },
        )
    }
    fn load_linked_respawns_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<LinkedRespawnPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { classify_like_cpp(load_linked_like_cpp(&self.world_db).await) })
    }
    fn load_pool_templates_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<PoolTemplatePersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_POOL_TEMPLATES,
                    |result| {
                        Ok(PoolTemplatePersistenceRowLikeCpp {
                            entry: result.read(0),
                            max_limit: result.read(1),
                        })
                    },
                )
                .await,
            )
        })
    }
    fn load_pool_members_like_cpp(
        &self,
        kind: PoolMemberKindPersistenceLikeCpp,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<PoolMemberPersistenceRowLikeCpp>>,
    > {
        let kind = pool_member_kind_raw_like_cpp(kind);
        Box::pin(async move {
            classify_like_cpp(load_pool_members_like_cpp(&self.world_db, kind).await)
        })
    }
    fn load_pool_autospawn_candidates_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<PoolAutospawnCandidatePersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_POOL_AUTOSPAWN_CANDIDATES,
                    |result| {
                        Ok(PoolAutospawnCandidatePersistenceRowLikeCpp {
                            pool_entry: result.read(0),
                            child_pool_id: result.try_read(1).unwrap_or(0),
                            mother_pool_id: result.try_read(2).unwrap_or(0),
                        })
                    },
                )
                .await,
            )
        })
    }
    fn load_spawn_group_members_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp<Vec<SpawnGroupMemberPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move {
            classify_like_cpp(
                query_rows_like_cpp(
                    &self.world_db,
                    WorldStatements::SEL_SPAWN_GROUP_MEMBERS,
                    |result| {
                        Ok(SpawnGroupMemberPersistenceRowLikeCpp {
                            group_id: result.read(0),
                            spawn_type: result.read(1),
                            spawn_id: result.read(2),
                        })
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
    use crate::StatementDef;

    #[test]
    fn represented_integer_domains_and_defaults_stay_exact() {
        assert_eq!(creature_spawntimesecs_to_i32_like_cpp(0).unwrap(), 0);
        assert_eq!(
            creature_spawntimesecs_to_i32_like_cpp(i32::MAX as u32).unwrap(),
            i32::MAX
        );
        assert!(creature_spawntimesecs_to_i32_like_cpp(i32::MAX as u32 + 1).is_err());
        assert_eq!(CREATURE_GROUND_MOVEMENT_RUN_LIKE_CPP, 1);
        assert_eq!(CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP, 180_000);
    }

    #[test]
    fn pool_member_kinds_keep_the_cpp_statement_discriminators() {
        assert_eq!(
            pool_member_kind_raw_like_cpp(PoolMemberKindPersistenceLikeCpp::Creature),
            0
        );
        assert_eq!(
            pool_member_kind_raw_like_cpp(PoolMemberKindPersistenceLikeCpp::GameObject),
            1
        );
        assert_eq!(
            pool_member_kind_raw_like_cpp(PoolMemberKindPersistenceLikeCpp::Pool),
            2
        );
    }

    #[test]
    fn canonical_spawn_statements_keep_the_staged_manifest() {
        let statements = [
            WorldStatements::SEL_CREATURE_SPAWNS,
            WorldStatements::SEL_WAYPOINT_PATHS,
            WorldStatements::SEL_WAYPOINT_PATH_NODES,
            WorldStatements::SEL_CREATURE_FORMATIONS,
            WorldStatements::SEL_GAMEOBJECT_SPAWNS,
            WorldStatements::SEL_AREATRIGGER_SPAWNS,
            WorldStatements::SEL_LINKED_RESPAWNS,
            WorldStatements::SEL_POOL_TEMPLATES,
            WorldStatements::SEL_POOL_MEMBERS_BY_TYPE,
            WorldStatements::SEL_POOL_MEMBERS_BY_TYPE,
            WorldStatements::SEL_POOL_MEMBERS_BY_TYPE,
            WorldStatements::SEL_POOL_AUTOSPAWN_CANDIDATES,
            WorldStatements::SEL_SPAWN_GROUP_MEMBERS,
        ];
        assert!(
            statements
                .into_iter()
                .all(|statement| !statement.sql().is_empty())
        );
    }
}
