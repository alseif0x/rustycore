// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature-loot condition loading, evaluation and item admission.

use super::*;

impl WorldSession {
    pub(in crate::handlers::loot) async fn load_represented_creature_loot_condition_rows_like_cpp(
        &self,
        condition_ids: &[LootConditionId],
    ) -> HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>> {
        let mut rows_by_id = HashMap::new();
        for &condition_id in condition_ids {
            let rows = self
                .load_represented_creature_loot_condition_rows_for_id_like_cpp(condition_id)
                .await;
            if !rows.is_empty() {
                rows_by_id.insert(condition_id, rows);
            }
        }
        rows_by_id
    }

    pub(in crate::handlers::loot) async fn load_represented_creature_loot_condition_reference_rows_like_cpp(
        &self,
        condition_rows: &HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>>,
    ) -> HashMap<u32, Vec<LootConditionRowLikeCpp>> {
        let mut references = HashMap::new();
        let mut pending = Vec::new();
        for rows in condition_rows.values() {
            pending.extend(loot_condition_reference_ids_like_cpp(rows));
        }

        while let Some(reference_id) = pending.pop() {
            if references.contains_key(&reference_id) {
                continue;
            }

            let rows = self
                .load_represented_creature_loot_condition_reference_rows_for_id_like_cpp(
                    reference_id,
                )
                .await;
            for nested_reference_id in loot_condition_reference_ids_like_cpp(&rows) {
                if !references.contains_key(&nested_reference_id) {
                    pending.push(nested_reference_id);
                }
            }
            references.insert(reference_id, rows);
        }

        references
    }

    async fn load_represented_creature_loot_condition_reference_rows_for_id_like_cpp(
        &self,
        reference_id: u32,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Ok(reference_source_type) = i32::try_from(reference_id).map(|id| -id) else {
            return Vec::new();
        };

        self.load_represented_creature_loot_condition_rows_for_id_like_cpp(LootConditionId {
            source_type: reference_source_type,
            source_group: 0,
            source_entry: 0,
        })
        .await
    }

    async fn load_represented_creature_loot_condition_rows_for_id_like_cpp(
        &self,
        condition_id: LootConditionId,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Some(port) = self.loot_template_catalog_persistence_port_like_cpp() else {
            return Vec::new();
        };

        let rows = match port
            .load_loot_condition_rows_like_cpp(
                condition_id.source_type,
                condition_id.source_group,
                condition_id.source_entry,
            )
            .await
        {
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    source_type = condition_id.source_type,
                    source_group = condition_id.source_group,
                    source_entry = condition_id.source_entry,
                    error = %reason,
                    "failed to load represented creature loot conditions"
                );
                return Vec::new();
            }
        };

        let mut conditions = Vec::new();
        for row in rows {
            let condition = LootConditionRowLikeCpp {
                else_group: row.else_group,
                condition_type_or_reference: row.condition_type_or_reference,
                condition_target: row.condition_target,
                value1: row.value1,
                value2: row.value2,
                value3: row.value3,
                string_value1: row.string_value1,
                negative: row.negative,
                script_name: row.script_name,
            };
            if !loot_condition_reference_self_references_like_cpp(
                condition_id.source_type,
                condition.condition_type_or_reference,
            ) {
                if let Some(condition) =
                    loot_condition_row_normalize_without_external_stores_like_cpp(condition)
                {
                    conditions.push(condition);
                }
            }
        }

        conditions
    }

    pub(in crate::handlers::loot) fn represented_creature_loot_item_allowed_like_cpp(
        &self,
        context: LootStoreItemContext,
        condition_rows: &HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>>,
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
        addon_metadata: &HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp>,
    ) -> bool {
        self.represented_creature_loot_item_allowed_for_player_like_cpp(
            context,
            self.player_guid().unwrap_or(ObjectGuid::EMPTY),
            condition_rows,
            condition_references,
            addon_metadata,
        )
    }

    pub(in crate::handlers::loot) fn represented_creature_loot_item_allowed_for_player_like_cpp(
        &self,
        context: LootStoreItemContext,
        player_guid: ObjectGuid,
        condition_rows: &HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>>,
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
        addon_metadata: &HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp>,
    ) -> bool {
        let Some(template) = self.item_storage_template(context.item.item_id) else {
            return false;
        };
        let Some(player_context) = self.represented_loot_player_context_like_cpp(player_guid)
        else {
            return false;
        };

        let flags2 = self.item_template_flags2_like_cpp(context.item.item_id);
        if represented_item_faction_flags_block_player_like_cpp(flags2, player_context.race) {
            return false;
        }

        let condition_id = LootConditionId {
            source_type: wow_loot::condition_source_type_for_loot_store_kind_like_cpp(
                context.store_kind,
            ),
            source_group: context.entry,
            source_entry: context.item.item_id,
        };
        if !loot_conditions_allow_player_with_references_like_cpp_representable(
            condition_rows
                .get(&condition_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            condition_references,
            |condition| {
                self.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                    condition,
                    &player_context,
                )
            },
        ) {
            return false;
        }

        let addon = addon_metadata
            .get(&context.item.item_id)
            .copied()
            .unwrap_or_default();
        self.item_loot_quest_status_allows_for_player_like_cpp(
            context.item.item_id,
            context.item.needs_quest,
            addon,
            &player_context,
        ) && template.max_stack_size != 0
    }

    pub(in crate::handlers::loot) fn evaluate_creature_loot_condition_for_player_like_cpp_representable(
        &self,
        condition: &LootConditionRowLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> Option<bool> {
        match condition.condition_type_or_reference {
            0 => Some(true),
            2 => {
                if condition.value3 != 0 {
                    return None;
                }
                let item_count = if player_context.is_current {
                    self.direct_inventory_item_count_like_cpp(condition.value1)
                } else {
                    Some(player_context.inventory_item_count(condition.value1))
                }?;
                Some(item_count >= condition.value2)
            }
            6 => Some(
                player_team_for_race_cpp_representable(player_context.race) == condition.value1,
            ),
            8 => Some(player_context.rewarded_quests.contains(&condition.value1)),
            9 => Some(
                player_context.quest_status(condition.value1) == QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            ),
            14 => Some(player_context.quest_status(condition.value1) == QUEST_STATUS_NONE_LIKE_CPP),
            15 => Some(
                player_class_mask_like_cpp(player_context.class)
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            16 => Some(
                player_race_mask_like_cpp(player_context.race)
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            20 => Some(u32::from(player_context.gender) == condition.value1),
            25 => i32::try_from(condition.value1)
                .ok()
                .map(|spell_id| player_context.known_spells.contains(&spell_id)),
            27 => condition_compare_values_like_cpp(
                condition.value2,
                u32::from(player_context.level),
                condition.value1,
            ),
            28 => Some(
                player_context.quest_status(condition.value1) == QUEST_STATUS_COMPLETE_LIKE_CPP
                    && !player_context.rewarded_quests.contains(&condition.value1),
            ),
            47 => Some(
                player_quest_status_mask_like_cpp(
                    player_context
                        .active_quest_statuses
                        .get(&condition.value1)
                        .copied(),
                    player_context.rewarded_quests.contains(&condition.value1),
                ) & condition.value2
                    != 0,
            ),
            48 => {
                let progress = if player_context.is_current {
                    self.player_quest_objective_progress_like_cpp(condition.value1)
                } else {
                    self.remote_player_quest_objective_progress_like_cpp(
                        condition.value1,
                        player_context,
                    )
                };
                Some(progress == Some(condition.value3 as i32))
            }
            CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP => {
                Some(condition.value1 == TYPEID_PLAYER_LIKE_CPP)
            }
            CONDITION_TYPE_MASK_LIKE_CPP => Some(condition.value1 & PLAYER_TYPE_MASK_LIKE_CPP != 0),
            _ => None,
        }
    }

    pub(in crate::handlers::loot) async fn load_creature_item_template_addon_loot_metadata_like_cpp(
        &self,
        item_id: u32,
    ) -> ItemTemplateAddonLootMetadataLikeCpp {
        let Some(port) = self.item_template_addon_catalog_persistence_port_like_cpp() else {
            return ItemTemplateAddonLootMetadataLikeCpp::default();
        };

        match port
            .load_item_template_addon_loot_metadata_like_cpp(
                wow_persistence::ItemTemplateAddonCatalogRequestLikeCpp {
                    item_entry: item_id,
                },
            )
            .await
        {
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Found(row) => {
                ItemTemplateAddonLootMetadataLikeCpp {
                    flags_cu: row.flags_cu,
                    quest_log_item_id: row.quest_log_item_id,
                }
            }
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Missing => {
                ItemTemplateAddonLootMetadataLikeCpp::default()
            }
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_id,
                    error = %reason,
                    "failed to load item_template_addon loot metadata for creature loot"
                );
                ItemTemplateAddonLootMetadataLikeCpp::default()
            }
        }
    }
}
