//! MariaDB adapter for the C++ ObjectMgr quest startup catalog.

use crate::{SqlResult, WorldDatabase, WorldStatements};
use anyhow::Result;
use std::sync::Arc;
use wow_persistence::*;

pub struct MariaDbQuestCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbQuestCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn template_rows(&self) -> Result<Vec<QuestTemplatePersistenceRowLikeCpp>> {
        let stmt = self.world_db.prepare(WorldStatements::SEL_QUEST_TEMPLATE);
        let mut result = self.world_db.query(&stmt).await?;
        let mut templates = Vec::new();
        if !result.is_empty() {
            loop {
                templates.push(QuestTemplatePersistenceRowLikeCpp {
                    id: result.read(0),
                    quest_type: result.try_read(1).unwrap_or(2),
                    quest_level: result.try_read(2).unwrap_or(0),
                    quest_max_scaling_level: result.try_read(3).unwrap_or(0),
                    quest_package_id: result.try_read(4).unwrap_or(0),
                    min_level: result.try_read(5).unwrap_or(0),
                    quest_sort_id: result.try_read(6).unwrap_or(0),
                    quest_info_id: result.try_read(7).unwrap_or(0),
                    suggested_group_num: result.try_read(8).unwrap_or(0),
                    reward_next_quest: result.try_read(9).unwrap_or(0),
                    reward_xp_difficulty: result.try_read(10).unwrap_or(0),
                    reward_xp_multiplier: result.try_read(11).unwrap_or(1.0),
                    reward_money_difficulty: result.try_read(12).unwrap_or(0),
                    reward_money_multiplier: result.try_read(13).unwrap_or(1.0),
                    reward_bonus_money: result.try_read(14).unwrap_or(0),
                    reward_display_spell: [
                        result.try_read(15).unwrap_or(0),
                        result.try_read(16).unwrap_or(0),
                        result.try_read(17).unwrap_or(0),
                    ],
                    reward_spell: result.try_read(18).unwrap_or(0),
                    reward_honor: result.try_read(19).unwrap_or(0),
                    reward_title_id: result.try_read(89).unwrap_or(0),
                    reward_skill_line_id: result.try_read(87).unwrap_or(0),
                    reward_skill_points: result.try_read(88).unwrap_or(0),
                    reward_mail_template_id: read_u32(&result, 90).unwrap_or(0),
                    reward_mail_delay_secs: read_u32(&result, 91).unwrap_or(0),
                    reward_mail_sender_entry: read_u32(&result, 92).unwrap_or(0),
                    reward_faction_ids: [0, 1, 2, 3, 4]
                        .map(|i| result.try_read(93 + i * 4).unwrap_or(0)),
                    reward_faction_values: [0, 1, 2, 3, 4]
                        .map(|i| result.try_read(94 + i * 4).unwrap_or(0)),
                    reward_faction_overrides: [0, 1, 2, 3, 4]
                        .map(|i| result.try_read(95 + i * 4).unwrap_or(0)),
                    reward_faction_cap_in: [0, 1, 2, 3, 4]
                        .map(|i| result.try_read(96 + i * 4).unwrap_or(0)),
                    reward_faction_flags: result.try_read(113).unwrap_or(0),
                    source_item_id: result.try_read(69).unwrap_or(0),
                    source_item_count: read_u32(&result, 71).unwrap_or(0),
                    source_spell_id: read_u32(&result, 70).unwrap_or(0),
                    limit_time_secs: result.try_read(72).unwrap_or(0),
                    expansion: result.try_read(68).unwrap_or(0),
                    flags: result.try_read(20).unwrap_or(0),
                    flags_ex: result.try_read(21).unwrap_or(0),
                    flags_ex2: result.try_read(22).unwrap_or(0),
                    special_flags: read_u32(&result, 67).unwrap_or(0),
                    reward_items: [23, 27, 31, 35].map(|i| result.try_read(i).unwrap_or(0)),
                    reward_amounts: [24, 28, 32, 36].map(|i| result.try_read(i).unwrap_or(0)),
                    reward_currencies: [79, 81, 83, 85].map(|i| result.try_read(i).unwrap_or(0)),
                    reward_currency_amounts: [80, 82, 84, 86]
                        .map(|i| result.try_read(i).unwrap_or(0)),
                    item_drop: [25, 29, 33, 37].map(|i| result.try_read(i).unwrap_or(0)),
                    item_drop_quantity: [26, 30, 34, 38].map(|i| result.try_read(i).unwrap_or(0)),
                    log_title: result.try_read(39).unwrap_or_default(),
                    log_description: result.try_read(40).unwrap_or_default(),
                    quest_description: result.try_read(41).unwrap_or_default(),
                    area_description: result.try_read(42).unwrap_or_default(),
                    quest_completion_log: result.try_read(43).unwrap_or_default(),
                    allowable_races: read_u64(&result, 44).unwrap_or(0),
                    allowable_classes: read_u32(&result, 45).unwrap_or(0),
                    max_level: read_u32(&result, 46)
                        .and_then(|v| u8::try_from(v).ok())
                        .unwrap_or(0),
                    prev_quest_id: result.try_read(47).unwrap_or(0),
                    next_quest_id: read_u32(&result, 64).unwrap_or(0),
                    exclusive_group: result.try_read(65).unwrap_or(0),
                    breadcrumb_for_quest_id: result.try_read(66).unwrap_or(0),
                    required_min_rep_faction: read_u32(&result, 48).unwrap_or(0),
                    required_min_rep_value: result.try_read(49).unwrap_or(0),
                    required_max_rep_faction: read_u32(&result, 50).unwrap_or(0),
                    required_max_rep_value: result.try_read(51).unwrap_or(0),
                    required_skill_id: read_u32(&result, 114).unwrap_or(0),
                    required_skill_points: read_u32(&result, 115).unwrap_or(0),
                    reward_choice_items: [
                        (52, 53),
                        (54, 55),
                        (56, 57),
                        (58, 59),
                        (60, 61),
                        (62, 63),
                    ]
                    .map(|(id, count)| {
                        (
                            result.try_read(id).unwrap_or(0),
                            result.try_read(count).unwrap_or(0),
                        )
                    }),
                    reward_choice_item_types: [73, 74, 75, 76, 77, 78]
                        .map(|i| result.try_read(i).unwrap_or(0)),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(templates)
    }

    async fn special_flag_rows(&self) -> Result<Vec<(u32, u32)>> {
        let mut result = self
            .world_db
            .direct_query("SELECT ID, SpecialFlags FROM quest_template_addon")
            .await?;
        let mut special_flags = Vec::new();
        if !result.is_empty() {
            loop {
                special_flags.push((
                    result.try_read(0).unwrap_or(0),
                    read_u32(&result, 1).unwrap_or(0),
                ));
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(special_flags)
    }

    async fn objective_rows(&self) -> Result<Vec<QuestObjectivePersistenceRowLikeCpp>> {
        let stmt = self.world_db.prepare(WorldStatements::SEL_QUEST_OBJECTIVES);
        let mut result = self.world_db.query(&stmt).await?;
        let mut objectives = Vec::new();
        if !result.is_empty() {
            loop {
                objectives.push(QuestObjectivePersistenceRowLikeCpp {
                    id: result.try_read(0).unwrap_or(0),
                    quest_id: result.try_read(1).unwrap_or(0),
                    obj_type: result.try_read(2).unwrap_or(0),
                    order: result.try_read(3).unwrap_or(0),
                    storage_index: result.try_read(4).unwrap_or(0),
                    object_id: result.try_read(5).unwrap_or(0),
                    amount: result.try_read(6).unwrap_or(0),
                    flags: result.try_read(7).unwrap_or(0),
                    flags2: result.try_read(8).unwrap_or(0),
                    progress_bar_weight: result.try_read(9).unwrap_or(0.0),
                    description: result.try_read(10).unwrap_or_default(),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(objectives)
    }

    async fn pairs(&self, statement: WorldStatements) -> Result<Vec<(u32, u32)>> {
        let stmt = self.world_db.prepare(statement);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push((
                    result.try_read(0).unwrap_or(0),
                    result.try_read(1).unwrap_or(0),
                ));
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

fn read_u32(result: &SqlResult, column: usize) -> Option<u32> {
    result
        .try_read(column)
        .or_else(|| {
            result
                .try_read::<u64>(column)
                .and_then(|v| u32::try_from(v).ok())
        })
        .or_else(|| result.try_read::<u16>(column).map(u32::from))
        .or_else(|| result.try_read::<u8>(column).map(u32::from))
        .or_else(|| {
            result
                .try_read::<i32>(column)
                .and_then(|v| u32::try_from(v).ok())
        })
        .or_else(|| {
            result
                .try_read::<i64>(column)
                .and_then(|v| u32::try_from(v).ok())
        })
        .or_else(|| {
            result
                .try_read::<i16>(column)
                .and_then(|v| u32::try_from(v).ok())
        })
        .or_else(|| {
            result
                .try_read::<i8>(column)
                .and_then(|v| u32::try_from(v).ok())
        })
}
fn read_u64(result: &SqlResult, column: usize) -> Option<u64> {
    result
        .try_read(column)
        .or_else(|| result.try_read::<u32>(column).map(u64::from))
        .or_else(|| result.try_read::<u16>(column).map(u64::from))
        .or_else(|| result.try_read::<u8>(column).map(u64::from))
        .or_else(|| result.try_read::<i64>(column).map(|v| v as u64))
        .or_else(|| result.try_read::<i32>(column).map(|v| v as u64))
        .or_else(|| result.try_read::<i16>(column).map(|v| v as u64))
        .or_else(|| result.try_read::<i8>(column).map(|v| v as u64))
}

impl QuestCatalogPersistencePortLikeCpp for MariaDbQuestCatalogPersistenceAdapterLikeCpp {
    fn load_quest_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        QuestCatalogLoadOutcomeLikeCpp<QuestTemplatePersistenceRowLikeCpp>,
    > {
        Box::pin(async move { outcome(self.template_rows().await) })
    }

    fn load_quest_special_flag_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
        Box::pin(async move { outcome(self.special_flag_rows().await) })
    }

    fn load_seasonal_quest_relation_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
        Box::pin(async move {
            outcome(
                self.pairs(WorldStatements::SEL_GAME_EVENT_SEASONAL_QUEST_RELATIONS)
                    .await,
            )
        })
    }

    fn load_quest_objective_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        QuestCatalogLoadOutcomeLikeCpp<QuestObjectivePersistenceRowLikeCpp>,
    > {
        Box::pin(async move { outcome(self.objective_rows().await) })
    }

    fn load_creature_quest_starter_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
        Box::pin(async move { outcome(self.pairs(WorldStatements::SEL_QUEST_STARTERS).await) })
    }

    fn load_creature_quest_ender_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
        Box::pin(async move { outcome(self.pairs(WorldStatements::SEL_QUEST_ENDERS).await) })
    }

    fn load_gameobject_quest_starter_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
        Box::pin(async move {
            outcome(
                self.pairs(WorldStatements::SEL_GAMEOBJECT_QUEST_STARTERS)
                    .await,
            )
        })
    }

    fn load_gameobject_quest_ender_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
        Box::pin(async move {
            outcome(
                self.pairs(WorldStatements::SEL_GAMEOBJECT_QUEST_ENDERS)
                    .await,
            )
        })
    }
}

fn outcome<T>(result: Result<Vec<T>>) -> QuestCatalogLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => QuestCatalogLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => QuestCatalogLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}
