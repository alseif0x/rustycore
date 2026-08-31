//! MariaDB adapter for the staged GameEvent startup World catalogs.

use std::sync::Arc;

use anyhow::{Result, bail};
use wow_persistence::{
    CreatureEquipmentIdPersistenceRowLikeCpp, GameEventConditionPersistenceRowLikeCpp,
    GameEventDataPersistenceRowLikeCpp, GameEventModelEquipPersistenceRowLikeCpp,
    GameEventNpcFlagPersistenceRowLikeCpp, GameEventNpcVendorPersistenceRowLikeCpp,
    GameEventObjectGuidPersistenceRowLikeCpp, GameEventPoolPersistenceRowLikeCpp,
    GameEventPrerequisitePersistenceRowLikeCpp, GameEventQuestConditionPersistenceRowLikeCpp,
    GameEventQuestRelationPersistenceRowLikeCpp, GameEventWorldCatalogLoadOutcomeLikeCpp,
    GameEventWorldCatalogPersistencePortLikeCpp, GameEventWorldCatalogPrefixLikeCpp,
    GameEventWorldCatalogSuffixLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

const GAME_EVENT_WORLD_PREFIX_STATEMENTS_LIKE_CPP: [WorldStatements; 4] = [
    WorldStatements::SEL_MAX_GAME_EVENT_ENTRY,
    WorldStatements::SEL_GAME_EVENTS,
    WorldStatements::SEL_GAME_EVENT_PREREQUISITES,
    WorldStatements::SEL_GAME_EVENT_CONDITIONS,
];

const GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP: [WorldStatements; 10] = [
    WorldStatements::SEL_GAME_EVENT_QUEST_CONDITIONS,
    WorldStatements::SEL_GAME_EVENT_POOLS,
    WorldStatements::SEL_GAME_EVENT_CREATURES,
    WorldStatements::SEL_GAME_EVENT_GAMEOBJECTS,
    WorldStatements::SEL_CREATURE_EQUIP_TEMPLATE_IDS,
    WorldStatements::SEL_GAME_EVENT_MODEL_EQUIP,
    WorldStatements::SEL_GAME_EVENT_CREATURE_QUESTS,
    WorldStatements::SEL_GAME_EVENT_GAMEOBJECT_QUESTS,
    WorldStatements::SEL_GAME_EVENT_NPC_FLAGS,
    WorldStatements::SEL_GAME_EVENT_NPC_VENDOR,
];

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

fn read_i8_like_cpp(result: &SqlResult, column: usize, field: &str) -> Result<i8> {
    if result.is_null(column) {
        return Ok(0);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return Ok(value);
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return Ok(value as i8);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return i8_value_like_cpp(i64::from(value), field);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return i8_value_like_cpp(i64::from(value), field);
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return i8_value_like_cpp(value, field);
    }
    bail!("could not decode {field} at column {column} as a C++ signed 8-bit DB field")
}

fn i8_value_like_cpp(value: i64, field: &str) -> Result<i8> {
    i8::try_from(value)
        .map_err(|_| anyhow::anyhow!("{field} value {value} exceeds the represented i8 domain"))
}

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

async fn load_prefix_rows_like_cpp(
    db: &WorldDatabase,
) -> Result<GameEventWorldCatalogPrefixLikeCpp> {
    let max_result = db
        .query(&db.prepare(GAME_EVENT_WORLD_PREFIX_STATEMENTS_LIKE_CPP[0]))
        .await?;
    let max_event_entry = if max_result.is_empty() {
        None
    } else {
        max_result.try_read(0)
    };

    let events = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_PREFIX_STATEMENTS_LIKE_CPP[1],
        |result| {
            Ok(GameEventDataPersistenceRowLikeCpp {
                event_id: u16::from(read_u8_like_cpp(result, 0, "game_event.eventEntry")?),
                start: read_u64_like_cpp(result, 1, "game_event.start_time")?,
                end: read_u64_like_cpp(result, 2, "game_event.end_time")?,
                occurence: read_u32_like_cpp(result, 3, "game_event.occurence")?,
                length: read_u32_like_cpp(result, 4, "game_event.length")?,
                holiday_id: read_u32_like_cpp(result, 5, "game_event.holiday")?,
                holiday_stage: read_u8_like_cpp(result, 6, "game_event.holidayStage")?,
                description: result.read(7),
                state_raw: read_u8_like_cpp(result, 8, "game_event.world_event")?,
                announce: read_u8_like_cpp(result, 9, "game_event.announce")?,
            })
        },
    )
    .await?;
    let prerequisites = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_PREFIX_STATEMENTS_LIKE_CPP[2],
        |result| {
            Ok(GameEventPrerequisitePersistenceRowLikeCpp {
                event_id: u16::from(read_u8_like_cpp(
                    result,
                    0,
                    "game_event_prerequisite.eventEntry",
                )?),
                prerequisite_event: read_u32_like_cpp(
                    result,
                    1,
                    "game_event_prerequisite.prerequisite_event",
                )?,
            })
        },
    )
    .await?;
    let conditions = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_PREFIX_STATEMENTS_LIKE_CPP[3],
        |result| {
            Ok(GameEventConditionPersistenceRowLikeCpp {
                event_id: u16::from(read_u8_like_cpp(
                    result,
                    0,
                    "game_event_condition.eventEntry",
                )?),
                condition_id: read_u32_like_cpp(result, 1, "game_event_condition.condition_id")?,
                req_num: result.read(2),
                max_world_state: read_u16_like_cpp(
                    result,
                    3,
                    "game_event_condition.max_world_state_field",
                )?,
                done_world_state: read_u16_like_cpp(
                    result,
                    4,
                    "game_event_condition.done_world_state_field",
                )?,
            })
        },
    )
    .await?;

    Ok(GameEventWorldCatalogPrefixLikeCpp {
        max_event_entry,
        events,
        prerequisites,
        conditions,
    })
}

async fn load_suffix_rows_like_cpp(
    db: &WorldDatabase,
) -> Result<GameEventWorldCatalogSuffixLikeCpp> {
    let quest_conditions = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[0],
        |result| {
            Ok(GameEventQuestConditionPersistenceRowLikeCpp {
                quest_id: read_u32_like_cpp(result, 0, "game_event_quest_condition.quest")?,
                event_id: u16::from(read_u8_like_cpp(
                    result,
                    1,
                    "game_event_quest_condition.eventEntry",
                )?),
                condition_id: read_u32_like_cpp(
                    result,
                    2,
                    "game_event_quest_condition.condition_id",
                )?,
                num: result.read(3),
            })
        },
    )
    .await?;
    let pools = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[1],
        |result| {
            Ok(GameEventPoolPersistenceRowLikeCpp {
                pool_entry: read_u32_like_cpp(result, 0, "game_event_pool.pool_entry")?,
                event_id: i16::from(read_i8_like_cpp(result, 1, "game_event_pool.eventEntry")?),
            })
        },
    )
    .await?;
    let creature_guids = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[2],
        |result| object_guid_row_like_cpp(result, "game_event_creature"),
    )
    .await?;
    let gameobject_guids = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[3],
        |result| object_guid_row_like_cpp(result, "game_event_gameobject"),
    )
    .await?;
    let equipment_ids = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[4],
        |result| {
            Ok(CreatureEquipmentIdPersistenceRowLikeCpp {
                // Preserve the pre-extraction Rust loader's required typed reads
                // for these two already-normalized statement columns.
                creature_id: result.read(0),
                equipment_id: result.read(1),
            })
        },
    )
    .await?;
    let model_equips = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[5],
        |result| {
            Ok(GameEventModelEquipPersistenceRowLikeCpp {
                spawn_id: read_u64_like_cpp(result, 0, "game_event_model_equip.guid")?,
                entry: read_u32_like_cpp(result, 1, "game_event_model_equip.creature.id")?,
                event_id: u16::from(read_u8_like_cpp(
                    result,
                    2,
                    "game_event_model_equip.eventEntry",
                )?),
                model_id: read_u32_like_cpp(result, 3, "game_event_model_equip.modelid")?,
                equipment_id: read_u8_like_cpp(result, 4, "game_event_model_equip.equipment_id")?,
            })
        },
    )
    .await?;
    let creature_quest_relations = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[6],
        |result| quest_relation_row_like_cpp(result, "game_event_creature_quest"),
    )
    .await?;
    let gameobject_quest_relations = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[7],
        |result| quest_relation_row_like_cpp(result, "game_event_gameobject_quest"),
    )
    .await?;
    let npc_flags = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[8],
        |result| {
            Ok(GameEventNpcFlagPersistenceRowLikeCpp {
                spawn_id: read_u64_like_cpp(result, 0, "game_event_npcflag.guid")?,
                event_id: u16::from(read_u8_like_cpp(
                    result,
                    1,
                    "game_event_npcflag.eventEntry",
                )?),
                npcflag: read_u64_like_cpp(result, 2, "game_event_npcflag.npcflag")?,
            })
        },
    )
    .await?;
    let npc_vendors = query_rows_like_cpp(
        db,
        GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP[9],
        |result| {
            Ok(GameEventNpcVendorPersistenceRowLikeCpp {
                event_id: read_u8_like_cpp(result, 0, "game_event_npc_vendor.eventEntry")?,
                spawn_id: read_u64_like_cpp(result, 1, "game_event_npc_vendor.guid")?,
                item: read_u32_like_cpp(result, 2, "game_event_npc_vendor.item")?,
                maxcount: read_u32_like_cpp(result, 3, "game_event_npc_vendor.maxcount")?,
                incrtime: read_u32_like_cpp(result, 4, "game_event_npc_vendor.incrtime")?,
                extended_cost: read_u32_like_cpp(result, 5, "game_event_npc_vendor.ExtendedCost")?,
                vendor_type: read_u8_like_cpp(result, 6, "game_event_npc_vendor.type")?,
                bonus_list_ids: result.read_string(7),
                player_condition_id: read_u32_like_cpp(
                    result,
                    8,
                    "game_event_npc_vendor.PlayerConditionId",
                )?,
                ignore_filtering: read_u8_like_cpp(
                    result,
                    9,
                    "game_event_npc_vendor.IgnoreFiltering",
                )? != 0,
            })
        },
    )
    .await?;

    Ok(GameEventWorldCatalogSuffixLikeCpp {
        quest_conditions,
        pools,
        creature_guids,
        gameobject_guids,
        equipment_ids,
        model_equips,
        creature_quest_relations,
        gameobject_quest_relations,
        npc_flags,
        npc_vendors,
    })
}

fn object_guid_row_like_cpp(
    result: &SqlResult,
    table: &str,
) -> Result<GameEventObjectGuidPersistenceRowLikeCpp> {
    Ok(GameEventObjectGuidPersistenceRowLikeCpp {
        guid: read_u64_like_cpp(result, 0, &format!("{table}.guid"))?,
        event_id: i16::from(read_i8_like_cpp(result, 1, &format!("{table}.eventEntry"))?),
    })
}

fn quest_relation_row_like_cpp(
    result: &SqlResult,
    table: &str,
) -> Result<GameEventQuestRelationPersistenceRowLikeCpp> {
    Ok(GameEventQuestRelationPersistenceRowLikeCpp {
        giver_id: read_u32_like_cpp(result, 0, &format!("{table}.id"))?,
        quest_id: read_u32_like_cpp(result, 1, &format!("{table}.quest"))?,
        event_id: read_u8_like_cpp(result, 2, &format!("{table}.eventEntry"))?,
    })
}

fn classify_like_cpp<T>(result: Result<T>) -> GameEventWorldCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => GameEventWorldCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => GameEventWorldCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

pub struct MariaDbGameEventWorldCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbGameEventWorldCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl GameEventWorldCatalogPersistencePortLikeCpp
    for MariaDbGameEventWorldCatalogPersistenceAdapterLikeCpp
{
    fn load_prefix_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameEventWorldCatalogLoadOutcomeLikeCpp<GameEventWorldCatalogPrefixLikeCpp>,
    > {
        Box::pin(async move { classify_like_cpp(load_prefix_rows_like_cpp(&self.world_db).await) })
    }

    fn load_suffix_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameEventWorldCatalogLoadOutcomeLikeCpp<GameEventWorldCatalogSuffixLikeCpp>,
    > {
        Box::pin(async move { classify_like_cpp(load_suffix_rows_like_cpp(&self.world_db).await) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn signed_event_ids_keep_cpp_int8_reinterpretation() {
        assert_eq!(i16::from(255_u8 as i8), -1);
        assert_eq!(i16::from(128_u8 as i8), -128);
        assert_eq!(i8_value_like_cpp(-1, "eventEntry").unwrap(), -1);
        assert_eq!(i8_value_like_cpp(127, "eventEntry").unwrap(), 127);
        assert!(i8_value_like_cpp(-129, "eventEntry").is_err());
        assert!(i8_value_like_cpp(128, "eventEntry").is_err());
    }

    #[test]
    fn statements_keep_the_represented_prefix_and_suffix_order() {
        assert_eq!(
            GAME_EVENT_WORLD_PREFIX_STATEMENTS_LIKE_CPP.map(WorldStatements::sql),
            [
                "SELECT MAX(eventEntry) FROM game_event",
                WorldStatements::SEL_GAME_EVENTS.sql(),
                WorldStatements::SEL_GAME_EVENT_PREREQUISITES.sql(),
                WorldStatements::SEL_GAME_EVENT_CONDITIONS.sql(),
            ]
        );
        assert_eq!(
            GAME_EVENT_WORLD_SUFFIX_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_GAME_EVENT_QUEST_CONDITIONS,
                WorldStatements::SEL_GAME_EVENT_POOLS,
                WorldStatements::SEL_GAME_EVENT_CREATURES,
                WorldStatements::SEL_GAME_EVENT_GAMEOBJECTS,
                WorldStatements::SEL_CREATURE_EQUIP_TEMPLATE_IDS,
                WorldStatements::SEL_GAME_EVENT_MODEL_EQUIP,
                WorldStatements::SEL_GAME_EVENT_CREATURE_QUESTS,
                WorldStatements::SEL_GAME_EVENT_GAMEOBJECT_QUESTS,
                WorldStatements::SEL_GAME_EVENT_NPC_FLAGS,
                WorldStatements::SEL_GAME_EVENT_NPC_VENDOR,
            ]
        );
    }
}
