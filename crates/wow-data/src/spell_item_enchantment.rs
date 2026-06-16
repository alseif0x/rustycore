// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SpellItemEnchantment.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_constants::SpellItemEnchantmentFlags;

use crate::wdc4::Wdc4Reader;

pub const SPELL_ITEM_ENCHANTMENT_EFFECTS: usize = 3;

/// C++ `SpellItemEnchantmentEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellItemEnchantmentEntry {
    pub id: u32,
    pub effect_arg: [u32; SPELL_ITEM_ENCHANTMENT_EFFECTS],
    pub effect_points_min: [i16; SPELL_ITEM_ENCHANTMENT_EFFECTS],
    pub item_visual: u16,
    pub flags: SpellItemEnchantmentFlags,
    pub required_skill_id: u16,
    pub required_skill_rank: u16,
    pub item_level: u16,
    pub charges: u8,
    pub effect: [u8; SPELL_ITEM_ENCHANTMENT_EFFECTS],
    pub condition_id: u8,
    pub min_level: u8,
    pub max_level: u8,
}

/// In-memory store for `SpellItemEnchantment.db2`.
pub struct SpellItemEnchantmentStore {
    entries: HashMap<u32, SpellItemEnchantmentEntry>,
}

impl SpellItemEnchantmentStore {
    pub fn from_entries(entries: impl IntoIterator<Item = SpellItemEnchantmentEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load SpellItemEnchantment.db2 from `{data_dir}/dbc/{locale}/SpellItemEnchantment.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::SpellItemEnchantmentEntry`
    /// - `DB2LoadInfo.h::SpellItemEnchantmentLoadInfo`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("SpellItemEnchantment.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let record = SpellItemEnchantmentEntry {
                id,
                effect_arg: [
                    reader.get_array_element(idx, 2, 0, 32),
                    reader.get_array_element(idx, 2, 1, 32),
                    reader.get_array_element(idx, 2, 2, 32),
                ],
                effect_points_min: [
                    reader.get_array_i16(idx, 8, 0),
                    reader.get_array_i16(idx, 8, 1),
                    reader.get_array_i16(idx, 8, 2),
                ],
                item_visual: reader.get_field_u16(idx, 9),
                flags: SpellItemEnchantmentFlags::from_bits_truncate(reader.get_field_u16(idx, 10)),
                required_skill_id: reader.get_field_u16(idx, 11),
                required_skill_rank: reader.get_field_u16(idx, 12),
                item_level: reader.get_field_u16(idx, 13),
                charges: reader.get_field_u8(idx, 14),
                effect: [
                    reader.get_array_element(idx, 15, 0, 8) as u8,
                    reader.get_array_element(idx, 15, 1, 8) as u8,
                    reader.get_array_element(idx, 15, 2, 8) as u8,
                ],
                condition_id: reader.get_field_u8(idx, 18),
                min_level: reader.get_field_u8(idx, 19),
                max_level: reader.get_field_u8(idx, 20),
            };
            entries.insert(id, record);
        }

        info!(
            "Loaded {} spell item enchantments from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&SpellItemEnchantmentEntry> {
        self.entries.get(&id)
    }

    /// C++ `SpellMgr::IsArenaAllowedEnchancment`.
    pub fn is_arena_allowed_enchantment(&self, id: u32) -> bool {
        self.get(id).is_some_and(|entry| {
            entry
                .flags
                .contains(SpellItemEnchantmentFlags::ALLOW_ENTERING_ARENA)
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellEnchantProcRowLikeCpp {
    pub enchant_id: u32,
    pub chance: f32,
    pub procs_per_minute: f32,
    pub hit_mask: u32,
    pub attributes_mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellEnchantProcEntryLikeCpp {
    pub chance: f32,
    pub procs_per_minute: f32,
    pub hit_mask: u32,
    pub attributes_mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellEnchantProcLoadErrorLikeCpp {
    pub row: SpellEnchantProcRowLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellEnchantProcStoreLikeCpp {
    pub entries_by_enchant_id: HashMap<u32, SpellEnchantProcEntryLikeCpp>,
}

impl SpellEnchantProcStoreLikeCpp {
    pub fn from_rows_like_cpp<I>(
        rows: I,
        enchantment_store: &SpellItemEnchantmentStore,
    ) -> SpellEnchantProcLoadOutcomeLikeCpp
    where
        I: IntoIterator<Item = SpellEnchantProcRowLikeCpp>,
    {
        let mut store = Self::default();
        let mut loaded_row_count = 0;
        let mut errors = Vec::new();

        for row in rows {
            if enchantment_store.get(row.enchant_id).is_none() {
                errors.push(SpellEnchantProcLoadErrorLikeCpp { row });
                continue;
            }

            store.entries_by_enchant_id.insert(
                row.enchant_id,
                SpellEnchantProcEntryLikeCpp {
                    chance: row.chance,
                    procs_per_minute: row.procs_per_minute,
                    hit_mask: row.hit_mask,
                    attributes_mask: row.attributes_mask,
                },
            );
            loaded_row_count += 1;
        }

        SpellEnchantProcLoadOutcomeLikeCpp {
            store,
            loaded_row_count,
            errors,
        }
    }

    pub fn get_spell_enchant_proc_event_like_cpp(
        &self,
        enchant_id: u32,
    ) -> Option<&SpellEnchantProcEntryLikeCpp> {
        self.entries_by_enchant_id.get(&enchant_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellEnchantProcLoadOutcomeLikeCpp {
    pub store: SpellEnchantProcStoreLikeCpp,
    pub loaded_row_count: usize,
    pub errors: Vec<SpellEnchantProcLoadErrorLikeCpp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_spell_item_enchantment_store() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("SpellItemEnchantment.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: SpellItemEnchantment.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store = SpellItemEnchantmentStore::load(data_dir, locale)
            .expect("failed to load SpellItemEnchantmentStore");
        assert!(!store.is_empty());
        assert!(
            store
                .entries
                .values()
                .any(|entry| entry.effect.iter().any(|value| *value != 0))
        );
    }

    #[test]
    fn arena_allowed_matches_cpp_flag_check() {
        let store = SpellItemEnchantmentStore::from_entries([
            SpellItemEnchantmentEntry {
                id: 1,
                effect_arg: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
                effect_points_min: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
                item_visual: 0,
                flags: SpellItemEnchantmentFlags::ALLOW_ENTERING_ARENA,
                required_skill_id: 0,
                required_skill_rank: 0,
                item_level: 0,
                charges: 0,
                effect: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
                condition_id: 0,
                min_level: 0,
                max_level: 0,
            },
            SpellItemEnchantmentEntry {
                id: 2,
                effect_arg: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
                effect_points_min: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
                item_visual: 0,
                flags: SpellItemEnchantmentFlags::SOULBOUND,
                required_skill_id: 0,
                required_skill_rank: 0,
                item_level: 0,
                charges: 0,
                effect: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
                condition_id: 0,
                min_level: 0,
                max_level: 0,
            },
        ]);

        assert!(store.is_arena_allowed_enchantment(1));
        assert!(!store.is_arena_allowed_enchantment(2));
        assert!(!store.is_arena_allowed_enchantment(3));
    }

    #[test]
    fn spell_enchant_proc_store_skips_missing_enchantments_like_cpp() {
        let enchantments = SpellItemEnchantmentStore::from_entries([SpellItemEnchantmentEntry {
            id: 10,
            effect_arg: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
            effect_points_min: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 0,
            charges: 0,
            effect: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
            condition_id: 0,
            min_level: 0,
            max_level: 0,
        }]);

        let outcome = SpellEnchantProcStoreLikeCpp::from_rows_like_cpp(
            [
                SpellEnchantProcRowLikeCpp {
                    enchant_id: 10,
                    chance: 4.5,
                    procs_per_minute: 1.0,
                    hit_mask: 3,
                    attributes_mask: 1,
                },
                SpellEnchantProcRowLikeCpp {
                    enchant_id: 20,
                    chance: 9.0,
                    procs_per_minute: 2.0,
                    hit_mask: 7,
                    attributes_mask: 0,
                },
            ],
            &enchantments,
        );

        assert_eq!(outcome.loaded_row_count, 1);
        assert_eq!(outcome.errors.len(), 1);
        assert_eq!(outcome.errors[0].row.enchant_id, 20);
        assert_eq!(
            outcome.store.get_spell_enchant_proc_event_like_cpp(10),
            Some(&SpellEnchantProcEntryLikeCpp {
                chance: 4.5,
                procs_per_minute: 1.0,
                hit_mask: 3,
                attributes_mask: 1,
            })
        );
        assert_eq!(
            outcome.store.get_spell_enchant_proc_event_like_cpp(20),
            None
        );
    }

    #[test]
    fn spell_enchant_proc_store_duplicate_rows_last_wins_like_cpp() {
        let enchantments = SpellItemEnchantmentStore::from_entries([SpellItemEnchantmentEntry {
            id: 30,
            effect_arg: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
            effect_points_min: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 0,
            charges: 0,
            effect: [0; SPELL_ITEM_ENCHANTMENT_EFFECTS],
            condition_id: 0,
            min_level: 0,
            max_level: 0,
        }]);

        let outcome = SpellEnchantProcStoreLikeCpp::from_rows_like_cpp(
            [
                SpellEnchantProcRowLikeCpp {
                    enchant_id: 30,
                    chance: 1.0,
                    procs_per_minute: 0.0,
                    hit_mask: 1,
                    attributes_mask: 0,
                },
                SpellEnchantProcRowLikeCpp {
                    enchant_id: 30,
                    chance: 0.0,
                    procs_per_minute: 3.5,
                    hit_mask: 5,
                    attributes_mask: 2,
                },
            ],
            &enchantments,
        );

        assert_eq!(
            outcome.loaded_row_count, 2,
            "C++ increments count for every valid row before unordered_map overwrite visibility"
        );
        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.store.entries_by_enchant_id.len(), 1);
        assert_eq!(
            outcome.store.get_spell_enchant_proc_event_like_cpp(30),
            Some(&SpellEnchantProcEntryLikeCpp {
                chance: 0.0,
                procs_per_minute: 3.5,
                hit_mask: 5,
                attributes_mask: 2,
            })
        );
    }
}
