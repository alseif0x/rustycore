//! Disenchant template expansion with application-owned reference loading.
//!
//! C++ a5f8da2e: LootMgr.cpp::LootStoreItem::Roll (277), LootGroup::Roll (371),
//! LootGroup::Process (446), LootTemplate::Process (556), and Loot.cpp::AddItem (826).
//! The LIFO continuation, frame cap and async loading boundary preserve the existing
//! Rust bridge. Its chance draw is intentionally not the general store roll helper.

use crate::{LootEntry, LootEntryFlags, LootStoreItem, LootTemplate, MAX_NR_LOOT_ITEMS_LIKE_CPP};
use rand::Rng;
use wow_core::ObjectGuid;

#[cfg(test)]
mod tests;

const DEFAULT_LOOT_MODE: u16 = 0x01;
const MAX_REFERENCE_FRAMES: u32 = 64;

pub struct DisenchantLootBuilder {
    loot_items: Vec<LootEntry>,
    frames: Vec<DisenchantLootFrame>,
    pending_row: Option<LootStoreItem>,
    processed_frames: u32,
}

impl DisenchantLootBuilder {
    pub fn new(rows: Vec<LootStoreItem>) -> Self {
        Self {
            loot_items: Vec::new(),
            frames: vec![disenchant_frame(rows, 0)],
            pending_row: None,
            processed_frames: 0,
        }
    }

    /// Advances until reference rows are needed or the frame stack is exhausted.
    /// A pending request remains unchanged until resume_reference supplies its rows.
    pub fn next_reference<R: Rng + ?Sized>(
        &mut self,
        rng: &mut R,
        mut item_max_stack: impl FnMut(u32) -> Option<u32>,
        mut item_rate: impl FnMut(u32) -> f32,
        mut reference_rate: impl FnMut() -> f32,
    ) -> Option<u32> {
        if let Some(row) = self.pending_row {
            return Some(row.reference);
        }

        while let Some(mut frame) = self.frames.pop() {
            if frame.requested_group_id > 0 {
                let group_index = usize::from(frame.requested_group_id - 1);
                if let Some(group) = frame.template.groups().get(group_index) {
                    if let Some(row) = group.roll_like_cpp(DEFAULT_LOOT_MODE, rng, |item| {
                        item_max_stack(item.item_id).is_some()
                    }) {
                        let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
                        add_item_stacks(
                            &mut self.loot_items,
                            row.item_id,
                            count,
                            item_max_stack(row.item_id).unwrap_or(1).max(1),
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
                self.frames.push(frame.clone());

                if let Some(row) = frame.template.groups()[group_index].roll_like_cpp(
                    DEFAULT_LOOT_MODE,
                    rng,
                    |item| item_max_stack(item.item_id).is_some(),
                ) {
                    let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
                    add_item_stacks(
                        &mut self.loot_items,
                        row.item_id,
                        count,
                        item_max_stack(row.item_id).unwrap_or(1).max(1),
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
            self.frames.push(frame);

            if row.reference > 0 {
                if !reference_row_can_roll(&row) {
                    continue;
                }
                if row.chance < 100.0
                    && !roll_chance_with_rate(row.chance, reference_rate(), rng)
                {
                    continue;
                }

                self.pending_row = Some(row);
                return Some(row.reference);
            }

            if !plain_row_can_roll(&row, item_max_stack(row.item_id).is_some()) {
                continue;
            }
            if row.chance < 100.0
                && !roll_chance_with_rate(row.chance, item_rate(row.item_id), rng)
            {
                continue;
            }

            let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
            add_item_stacks(
                &mut self.loot_items,
                row.item_id,
                count,
                item_max_stack(row.item_id).unwrap_or(1).max(1),
                LootEntryFlags {
                    follow_loot_rules: true,
                    ..Default::default()
                },
            );
        }

        None
    }

    /// Called after the application loads the requested rows and samples amount_rate.
    /// False stops expansion at the existing cap; into_entries retains partial loot.
    pub fn resume_reference(&mut self, rows: Vec<LootStoreItem>, amount_rate: f32) -> bool {
        let row = self
            .pending_row
            .take()
            .expect("reference rows require a pending request");
        let max_count = reference_max_count(row.max_count, amount_rate);
        for _ in 0..max_count {
            self.frames.push(disenchant_frame(rows.clone(), row.group_id));
        }
        self.processed_frames = self.processed_frames.saturating_add(1);
        self.processed_frames <= MAX_REFERENCE_FRAMES
    }

    pub fn into_entries(self, winner: ObjectGuid) -> Vec<LootEntry> {
        let mut loot_items = self.loot_items;
        for (index, loot_entry) in loot_items.iter_mut().enumerate() {
            loot_entry.loot_list_id = index as u8;
            loot_entry.allowed_looters = vec![winner];
            loot_entry.roll_winner = winner;
        }
        loot_items
    }
}

fn plain_row_can_roll(row: &LootStoreItem, item_exists: bool) -> bool {
    row.can_roll_as_plain_entry_like_cpp(item_exists, DEFAULT_LOOT_MODE)
}

fn reference_row_can_roll(row: &LootStoreItem) -> bool {
    row.can_roll_as_reference_entry_like_cpp(DEFAULT_LOOT_MODE)
}

#[derive(Debug, Clone)]
struct DisenchantLootFrame {
    template: LootTemplate,
    entry_index: usize,
    group_index: usize,
    requested_group_id: u8,
}

fn disenchant_frame(rows: Vec<LootStoreItem>, requested_group_id: u8) -> DisenchantLootFrame {
    let mut template = LootTemplate::default();
    for row in rows {
        template.add_entry_like_cpp(row);
    }
    DisenchantLootFrame {
        template,
        entry_index: 0,
        group_index: 0,
        requested_group_id,
    }
}

fn roll_chance_with_rate<R: Rng + ?Sized>(chance: f32, rate: f32, rng: &mut R) -> bool {
    if chance >= 100.0 {
        return true;
    }
    rng.gen_range(0.0f32..100.0f32) < chance * rate
}

fn reference_max_count(max_count: u8, rate: f32) -> u32 {
    ((max_count as f32) * rate) as u32
}

fn add_item_stacks(
    loot_items: &mut Vec<LootEntry>,
    item_id: u32,
    mut count: u32,
    max_stack_size: u32,
    flags: LootEntryFlags,
) {
    while count > 0 && loot_items.len() < MAX_NR_LOOT_ITEMS_LIKE_CPP {
        let quantity = count.min(max_stack_size);
        loot_items.push(LootEntry {
            loot_list_id: loot_items.len() as u8,
            item_id,
            quantity,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags,
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        });
        count = count.saturating_sub(max_stack_size);
    }
}
