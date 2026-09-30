// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot table generation and conditional/quest item selection.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use super::*;

impl WorldSession {
    pub(super) async fn generate_represented_disenchant_loot_template_entries_like_cpp(
        &mut self,
        disenchant_id: u32,
        winner_guid: ObjectGuid,
    ) -> Vec<LootEntry> {
        let rows = self
            .load_represented_disenchant_loot_template_rows_like_cpp(
                DisenchantLootTemplateTable::Disenchant,
                disenchant_id,
            )
            .await;
        let mut builder = wow_loot::DisenchantLootBuilder::new(rows);
        let mut rng = self.represented_runtime_subrng_like_cpp();
        while let Some(reference_id) = builder.next_reference(
            &mut rng,
            |item_id| {
                self.item_storage_template(item_id)
                    .map(|template| template.max_stack_size)
            },
            |item_id| self.item_drop_rate_like_cpp(item_id),
            || self.loot_drop_rates_like_cpp().item_referenced,
        ) {
            let reference_rows = self
                .load_represented_disenchant_loot_template_rows_like_cpp(
                    DisenchantLootTemplateTable::Reference,
                    reference_id,
                )
                .await;
            if !builder.resume_reference(
                reference_rows,
                self.loot_drop_rates_like_cpp().item_referenced_amount,
            ) {
                warn!(
                    disenchant_id,
                    reference = reference_id,
                    "stopped represented disenchant loot reference processing after safety cap"
                );
                break;
            }
        }
        builder.into_entries(winner_guid)
    }

    async fn load_represented_disenchant_loot_template_rows_like_cpp(
        &self,
        table: DisenchantLootTemplateTable,
        entry: u32,
    ) -> Vec<LootStoreItem> {
        let Some(port) = self.loot_template_catalog_persistence_port_like_cpp() else {
            return Vec::new();
        };

        let persistence_table = match table {
            DisenchantLootTemplateTable::Disenchant => {
                wow_persistence::LootTemplateTablePersistenceLikeCpp::Disenchant
            }
            DisenchantLootTemplateTable::Reference => {
                wow_persistence::LootTemplateTablePersistenceLikeCpp::Reference
            }
        };
        let rows = match port
            .load_loot_template_rows_like_cpp(persistence_table, entry)
            .await
        {
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    entry,
                    table = table.name(),
                    error = %reason,
                    "failed to load represented disenchant loot template rows"
                );
                return Vec::new();
            }
        };
        rows.into_iter()
            .map(|row| LootStoreItem {
                item_id: row.item_id,
                reference: row.reference,
                chance: row.chance,
                needs_quest: false,
                loot_mode: row.loot_mode,
                group_id: row.group_id,
                min_count: row.min_count,
                max_count: row.max_count,
            })
            .collect()
    }

    pub(super) fn has_incomplete_quest_item_drop_for_item_like_cpp(&self, item_id: u32) -> bool {
        let Some(quest_store) = &self.quests.store else {
            return false;
        };

        self.player_quest_gameplay_snapshot_like_cpp()
            .is_some_and(|state| {
                state.statuses_like_cpp().values().any(|status| {
                    if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                        return false;
                    }

                    let Some(quest) = quest_store.get(status.quest_id) else {
                        return false;
                    };

                    quest
                        .item_drop
                        .iter()
                        .enumerate()
                        .any(|(index, drop_item_id)| {
                            if *drop_item_id != item_id {
                                return false;
                            }

                            let Some(template) = self.item_storage_template(item_id) else {
                                return false;
                            };

                            let quantity = quest.item_drop_quantity[index];
                            let mut max_allowed_count = if quantity != 0 {
                                quantity
                            } else {
                                template.max_stack_size
                            };
                            if template.max_count > 0 {
                                max_allowed_count =
                                    max_allowed_count.min(template.max_count as u32);
                            }

                            self.direct_inventory_item_count_like_cpp(item_id)
                                .is_some_and(|count| count < max_allowed_count)
                        })
                })
            })
    }

    pub(super) fn remote_has_incomplete_quest_item_drop_for_item_like_cpp(
        &self,
        item_id: u32,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        let Some(quest_store) = &self.quests.store else {
            return false;
        };

        player_context
            .active_quest_statuses
            .iter()
            .any(|(quest_id, status)| {
                if *status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                    return false;
                }

                let Some(quest) = quest_store.get(*quest_id) else {
                    return false;
                };

                quest
                    .item_drop
                    .iter()
                    .enumerate()
                    .any(|(index, drop_item_id)| {
                        if *drop_item_id != item_id {
                            return false;
                        }

                        let Some(template) = self.item_storage_template(item_id) else {
                            return false;
                        };

                        let quantity = quest.item_drop_quantity[index];
                        let mut max_allowed_count = if quantity != 0 {
                            quantity
                        } else {
                            template.max_stack_size
                        };
                        if template.max_count > 0 {
                            max_allowed_count = max_allowed_count.min(template.max_count as u32);
                        }

                        player_context.inventory_item_count(item_id) < max_allowed_count
                    })
            })
    }
}
