// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot table generation and conditional/quest item selection.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::WorldStatements;

use super::*;

impl WorldSession {
    pub(super) async fn generate_represented_disenchant_loot_template_entries_like_cpp(
        &mut self,
        disenchant_id: u32,
        winner_guid: ObjectGuid,
    ) -> Vec<LootEntry> {
        let mut loot_items = Vec::new();
        let mut frames = vec![disenchant_loot_template_frame_like_cpp(
            self.load_represented_disenchant_loot_template_rows_like_cpp(
                DisenchantLootTemplateTable::Disenchant,
                disenchant_id,
            )
            .await,
            0,
        )];

        let mut rng = self.represented_runtime_subrng_like_cpp();
        let mut processed_frames = 0u32;
        while let Some(mut frame) = frames.pop() {
            if frame.requested_group_id > 0 {
                let group_index = usize::from(frame.requested_group_id - 1);
                if let Some(group) = frame.template.groups().get(group_index) {
                    if let Some(row) =
                        group.roll_like_cpp(LOOT_MODE_DEFAULT_LIKE_CPP, &mut rng, |item| {
                            self.item_storage_template(item.item_id).is_some()
                        })
                    {
                        let count =
                            rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
                        add_loot_item_stacks_like_cpp(
                            &mut loot_items,
                            row.item_id,
                            count,
                            self.item_storage_template(row.item_id)
                                .map(|template| template.max_stack_size)
                                .unwrap_or(1)
                                .max(1),
                            LootEntryFlags {
                                follow_loot_rules: true,
                                ..Default::default()
                            },
                        );
                    }
                }
                continue;
            }

            if frame.entry_index >= frame.template.entries().len() {
                if frame.group_index >= frame.template.groups().len() {
                    continue;
                }

                let group_index = frame.group_index;
                frame.group_index += 1;
                frames.push(frame.clone());

                if let Some(row) = frame.template.groups()[group_index].roll_like_cpp(
                    LOOT_MODE_DEFAULT_LIKE_CPP,
                    &mut rng,
                    |item| self.item_storage_template(item.item_id).is_some(),
                ) {
                    let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
                    add_loot_item_stacks_like_cpp(
                        &mut loot_items,
                        row.item_id,
                        count,
                        self.item_storage_template(row.item_id)
                            .map(|template| template.max_stack_size)
                            .unwrap_or(1)
                            .max(1),
                        LootEntryFlags {
                            follow_loot_rules: true,
                            ..Default::default()
                        },
                    );
                }
                continue;
            }

            let row = frame.template.entries()[frame.entry_index];
            frame.entry_index += 1;
            frames.push(frame);

            if row.reference > 0 {
                if !represented_disenchant_loot_reference_row_can_roll_like_cpp(&row) {
                    continue;
                }
                if row.chance < 100.0
                    && !roll_chance_with_rate_like_cpp(
                        row.chance,
                        self.loot_drop_rates_like_cpp().item_referenced,
                        &mut rng,
                    )
                {
                    continue;
                }

                let reference_rows = self
                    .load_represented_disenchant_loot_template_rows_like_cpp(
                        DisenchantLootTemplateTable::Reference,
                        row.reference,
                    )
                    .await;
                let max_count = referenced_loot_max_count_like_cpp(
                    row.max_count,
                    self.loot_drop_rates_like_cpp().item_referenced_amount,
                );
                for _ in 0..max_count {
                    frames.push(disenchant_loot_template_frame_like_cpp(
                        reference_rows.clone(),
                        row.group_id,
                    ));
                }
                processed_frames = processed_frames.saturating_add(1);
                if processed_frames > MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP {
                    warn!(
                        disenchant_id,
                        reference = row.reference,
                        "stopped represented disenchant loot reference processing after safety cap"
                    );
                    break;
                }
                continue;
            }

            if !represented_disenchant_loot_plain_row_can_roll_like_cpp(
                &row,
                self.item_storage_template(row.item_id).is_some(),
            ) {
                continue;
            }
            if row.chance < 100.0
                && !roll_chance_with_rate_like_cpp(
                    row.chance,
                    self.item_drop_rate_like_cpp(row.item_id),
                    &mut rng,
                )
            {
                continue;
            }

            let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
            add_loot_item_stacks_like_cpp(
                &mut loot_items,
                row.item_id,
                count,
                self.item_storage_template(row.item_id)
                    .map(|template| template.max_stack_size)
                    .unwrap_or(1)
                    .max(1),
                LootEntryFlags {
                    follow_loot_rules: true,
                    ..Default::default()
                },
            );
        }

        for (index, loot_entry) in loot_items.iter_mut().enumerate() {
            loot_entry.loot_list_id = index as u8;
            loot_entry.allowed_looters = vec![winner_guid];
            loot_entry.roll_winner = winner_guid;
        }

        loot_items
    }

    async fn load_represented_disenchant_loot_template_rows_like_cpp(
        &self,
        table: DisenchantLootTemplateTable,
        entry: u32,
    ) -> Vec<LootStoreItem> {
        let Some(world_db) = self.world_db() else {
            return Vec::new();
        };

        let statement = match table {
            DisenchantLootTemplateTable::Disenchant => {
                WorldStatements::SEL_DISENCHANT_LOOT_TEMPLATE_ROWS
            }
            DisenchantLootTemplateTable::Reference => {
                WorldStatements::SEL_REFERENCE_LOOT_TEMPLATE_ROWS
            }
        };
        let mut stmt = world_db.prepare(statement);
        stmt.set_u32(0, entry);

        let mut result = match world_db.query(&stmt).await {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    entry,
                    table = table.name(),
                    error = %err,
                    "failed to load represented disenchant loot template rows"
                );
                return Vec::new();
            }
        };

        let mut rows = Vec::new();
        if result.is_empty() {
            return rows;
        }

        loop {
            rows.push(LootStoreItem {
                item_id: result.try_read::<u32>(0).unwrap_or(0),
                reference: result.try_read::<u32>(1).unwrap_or(0),
                chance: result.try_read::<f32>(2).unwrap_or(0.0),
                needs_quest: false,
                loot_mode: result.try_read::<u16>(4).unwrap_or(0),
                group_id: result.try_read::<u8>(5).unwrap_or(0),
                min_count: result.try_read::<u8>(6).unwrap_or(0),
                max_count: result.try_read::<u8>(7).unwrap_or(0),
            });

            if !result.next_row() {
                break;
            }
        }

        rows
    }

    pub(super) fn has_incomplete_quest_item_drop_for_item_like_cpp(&self, item_id: u32) -> bool {
        let Some(quest_store) = &self.quest_store else {
            return false;
        };

        self.player_quests.values().any(|status| {
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
                        max_allowed_count = max_allowed_count.min(template.max_count as u32);
                    }

                    self.direct_inventory_item_count_like_cpp(item_id) < max_allowed_count
                })
        })
    }

    pub(super) fn remote_has_incomplete_quest_item_drop_for_item_like_cpp(
        &self,
        item_id: u32,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        let Some(quest_store) = &self.quest_store else {
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
