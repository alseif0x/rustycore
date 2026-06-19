// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item stat modifiers loaded from ItemSparse.db2.
//!
//! ItemSparse.db2 uses the WDC4 offset-map format where string fields
//! (Description, Display3/2/1/Display) are stored inline. This means
//! the field_storage_info offsets can't be used directly — we must read
//! each record sequentially, scanning past inline null-terminated strings.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_constants::ItemFlags;

use crate::wdc4::Wdc4Reader;

/// Item stat modifier types (from C# ItemModType enum).
///
/// Only the stat types we actually process are listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum ItemModType {
    None = -1,
    Mana = 0,
    Health = 1,
    Agility = 3,
    Strength = 4,
    Intellect = 5,
    Spirit = 6,
    Stamina = 7,
    DefenseSkillRating = 12,
    DodgeRating = 13,
    ParryRating = 14,
    BlockRating = 15,
    HitMeleeRating = 16,
    CritMeleeRating = 19,
    CritRangedRating = 20,
    CritSpellRating = 21,
    HitRating = 31,
    CritRating = 32,
    HasteRating = 36,
    AttackPower = 38,
    RangedAttackPower = 39,
    SpellPower = 45,
    ArmorPenetrationRating = 44,
    ExpertiseRating = 37,
}

/// Stat modifiers for a single item.
#[derive(Debug, Clone)]
pub struct ItemStatEntry {
    /// Up to 10 stat modifier slots: (stat_type, bonus_amount).
    /// stat_type -1 = unused slot.
    pub stats: [(i8, i16); 10],
    /// Physical armor from ItemSparse Resistances[0].
    pub armor: i32,
}

/// C++ `ItemSparseEntry` fields needed to build entity storage templates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemSparseTemplateEntry {
    pub flags: [u32; 4],
    pub bag_family: u32,
    pub start_quest_id: i32,
    pub stackable: i32,
    pub max_count: i32,
    pub lock_id: u16,
    pub required_reputation_rank: i32,
    pub sell_price: u32,
    pub buy_price: u32,
    pub vendor_stack_count: u32,
    pub price_variance: f32,
    pub price_random_value: f32,
    pub max_durability: u32,
    pub other_faction_item_id: i32,
    pub content_tuning_id: i32,
    pub player_level_to_item_level_curve_id: i32,
    pub limit_category: u16,
    pub instance_bound: u16,
    pub zone_bound: [u16; 2],
    pub required_reputation_faction: u16,
    pub allowable_class: i16,
    pub required_expansion: u8,
    pub bonding: u8,
    pub container_slots: u8,
    pub inventory_type: i8,
}

/// C++ `ItemSparseEntry` fields used by `ItemEnchantmentMgr::GenerateRandomProperties`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRandomPropertyTemplateEntry {
    pub item_level: u16,
    pub quality: i8,
    pub inventory_type: i8,
}

impl ItemSparseTemplateEntry {
    /// C++ `ItemTemplate::GetMaxStackSize`.
    pub fn max_stack_size(&self) -> u32 {
        if self.stackable == i32::MAX || self.stackable <= 0 {
            0x7FFF_FFFE
        } else {
            self.stackable as u32
        }
    }

    pub fn item_flags(&self) -> ItemFlags {
        ItemFlags::from_bits_retain(u64::from(self.flags[0]))
    }

    /// C++ `ItemTemplate::GetOtherFactionItemId`.
    pub fn other_faction_item_id_like_cpp(&self) -> u32 {
        self.other_faction_item_id as u32
    }

    /// C++ `ItemTemplate::GetScalingStatContentTuning`.
    pub fn scaling_stat_content_tuning_like_cpp(&self) -> u32 {
        self.content_tuning_id as u32
    }

    /// C++ `ItemTemplate::GetPlayerLevelToItemLevelCurveId`.
    pub fn player_level_to_item_level_curve_id_like_cpp(&self) -> u32 {
        self.player_level_to_item_level_curve_id as u32
    }
}

impl ItemStatEntry {
    /// Sum base stat bonuses: [STR, AGI, STA, INT, SPI].
    pub fn base_stat_bonuses(&self) -> [i32; 5] {
        let mut result = [0i32; 5];
        for &(stat_type, amount) in &self.stats {
            let amount = amount as i32;
            match stat_type {
                4 => result[0] += amount, // Strength
                3 => result[1] += amount, // Agility
                7 => result[2] += amount, // Stamina
                5 => result[3] += amount, // Intellect
                6 => result[4] += amount, // Spirit
                _ => {}
            }
        }
        result
    }

    /// Total attack power bonus from items.
    pub fn attack_power_bonus(&self) -> i32 {
        self.stats
            .iter()
            .filter(|&&(t, _)| t == 38)
            .map(|&(_, v)| v as i32)
            .sum()
    }

    /// Total ranged attack power bonus.
    pub fn ranged_attack_power_bonus(&self) -> i32 {
        self.stats
            .iter()
            .filter(|&&(t, _)| t == 39)
            .map(|&(_, v)| v as i32)
            .sum()
    }

    /// Total health bonus.
    pub fn health_bonus(&self) -> i32 {
        self.stats
            .iter()
            .filter(|&&(t, _)| t == 1)
            .map(|&(_, v)| v as i32)
            .sum()
    }

    /// Total mana bonus.
    pub fn mana_bonus(&self) -> i32 {
        self.stats
            .iter()
            .filter(|&&(t, _)| t == 0)
            .map(|&(_, v)| v as i32)
            .sum()
    }

    /// Sum combat rating bonuses: [CombatRating; 25] (indices per CombatRating enum).
    ///
    /// Unified stats (HitRating, CritRating, HasteRating) apply to all 3 sub-types
    /// (melee, ranged, spell) per C# Player.ApplyItemMods.
    pub fn combat_rating_bonuses(&self) -> [i32; 25] {
        let mut cr = [0i32; 25];
        for &(stat_type, amount) in &self.stats {
            let amount = amount as i32;
            match stat_type {
                12 => cr[1] += amount,  // DefenseSkillRating → DefenseSkill
                13 => cr[2] += amount,  // DodgeRating → Dodge
                14 => cr[3] += amount,  // ParryRating → Parry
                15 => cr[4] += amount,  // BlockRating → Block
                16 => cr[5] += amount,  // HitMeleeRating → HitMelee
                19 => cr[8] += amount,  // CritMeleeRating → CritMelee
                20 => cr[9] += amount,  // CritRangedRating → CritRanged
                21 => cr[10] += amount, // CritSpellRating → CritSpell
                31 => {
                    cr[5] += amount;
                    cr[6] += amount;
                    cr[7] += amount;
                } // HitRating → all
                32 => {
                    cr[8] += amount;
                    cr[9] += amount;
                    cr[10] += amount;
                } // CritRating → all
                36 => {
                    cr[17] += amount;
                    cr[18] += amount;
                    cr[19] += amount;
                } // HasteRating → all
                37 => cr[23] += amount, // ExpertiseRating → Expertise
                44 => cr[24] += amount, // ArmorPenetrationRating → ArmorPenetration
                _ => {}
            }
        }
        cr
    }

    /// Total spell power bonus from item stats.
    pub fn spell_power_bonus(&self) -> i32 {
        self.stats
            .iter()
            .filter(|&&(t, _)| t == 45)
            .map(|&(_, v)| v as i32)
            .sum()
    }

    /// Has at least one non-empty stat slot.
    pub fn has_stats(&self) -> bool {
        self.stats.iter().any(|&(t, a)| t != -1 && a != 0)
    }
}

/// In-memory store of item stat modifiers from ItemSparse.db2.
pub struct ItemStatsStore {
    stats: HashMap<u32, ItemStatEntry>,
    flags: HashMap<u32, [u32; 4]>,
    sparse_templates: HashMap<u32, ItemSparseTemplateEntry>,
    random_property_templates: HashMap<u32, ItemRandomPropertyTemplateEntry>,
}

/// Known field byte sizes for ItemSparse.db2 (from field_meta).
///
/// ItemSparse has 73 fields (0-72). Fields 1-5 are inline strings.
/// The remaining fields have fixed byte sizes derived from the field_meta
/// "bits" value: byte_size = (32 - bits) / 8.
///
/// Returns (field_byte_size, is_string) for each field index.
fn field_layout() -> [(u8, bool); 73] {
    let mut layout = [(0u8, false); 73];

    // Field 0: _allowableRace (i64, 8 bytes)
    layout[0] = (8, false);

    // Fields 1-5: inline strings
    layout[1] = (0, true); // Description
    layout[2] = (0, true); // Display3
    layout[3] = (0, true); // Display2
    layout[4] = (0, true); // Display1
    layout[5] = (0, true); // Display

    // Fields 6-11: f32/i32 (4 bytes each)
    for i in 6..=11 {
        layout[i] = (4, false);
    }

    // Field 12: StatPercentageOfSocket[10] (10 × f32 = 40 bytes)
    layout[12] = (40, false);

    // Field 13: StatPercentEditor[10] (10 × i32 = 40 bytes)
    layout[13] = (40, false);

    // Fields 14-22: i32/u32/f32 (4 bytes each)
    for i in 14..=22 {
        layout[i] = (4, false);
    }

    // Field 23: Flags[4] (4 × i32 = 16 bytes)
    layout[23] = (16, false);

    // Fields 24-28: i32/u32 (4 bytes each)
    for i in 24..=28 {
        layout[i] = (4, false);
    }

    // Fields 29-36: u16 (2 bytes each)
    for i in 29..=36 {
        layout[i] = (2, false);
    }

    // Field 37: ZoneBound[2] (2 × u16 = 4 bytes)
    layout[37] = (4, false);

    // Fields 38-48: u16/i16 (2 bytes each)
    for i in 38..=48 {
        layout[i] = (2, false);
    }

    // Field 49: MinDamage[5] (5 × u16 = 10 bytes)
    layout[49] = (10, false);

    // Field 50: MaxDamage[5] (5 × u16 = 10 bytes)
    layout[50] = (10, false);

    // Field 51: Resistances[7] (7 × i16 = 14 bytes)
    layout[51] = (14, false);

    // Field 52: ScalingStatDistributionID (u16, 2 bytes)
    layout[52] = (2, false);

    // Field 53: StatModifierBonusAmount[10] (10 × i16 = 20 bytes)
    layout[53] = (20, false);

    // Fields 54-57: u8/i8 (1 byte each)
    for i in 54..=57 {
        layout[i] = (1, false);
    }

    // Field 58: SocketType[3] (3 × u8 = 3 bytes)
    layout[58] = (3, false);

    // Fields 59-64: u8/i8 (1 byte each)
    for i in 59..=64 {
        layout[i] = (1, false);
    }

    // Field 65: _statModifierBonusStat[10] (10 × i8 = 10 bytes)
    layout[65] = (10, false);

    // Fields 66-72: u8/i8 (1 byte each)
    for i in 66..=72 {
        layout[i] = (1, false);
    }

    layout
}

impl ItemStatsStore {
    pub fn from_parts(
        stats: impl IntoIterator<Item = (u32, ItemStatEntry)>,
        flags: impl IntoIterator<Item = (u32, [u32; 4])>,
    ) -> Self {
        Self {
            stats: stats.into_iter().collect(),
            flags: flags.into_iter().collect(),
            sparse_templates: HashMap::new(),
            random_property_templates: HashMap::new(),
        }
    }

    pub fn from_sparse_templates(
        sparse_templates: impl IntoIterator<Item = (u32, ItemSparseTemplateEntry)>,
    ) -> Self {
        let sparse_templates: HashMap<_, _> = sparse_templates.into_iter().collect();
        let flags = sparse_templates
            .iter()
            .map(|(&id, template)| (id, template.flags))
            .collect();
        Self {
            stats: HashMap::new(),
            flags,
            sparse_templates,
            random_property_templates: HashMap::new(),
        }
    }

    pub fn from_random_property_templates(
        random_property_templates: impl IntoIterator<Item = (u32, ItemRandomPropertyTemplateEntry)>,
    ) -> Self {
        Self {
            stats: HashMap::new(),
            flags: HashMap::new(),
            sparse_templates: HashMap::new(),
            random_property_templates: random_property_templates.into_iter().collect(),
        }
    }

    pub fn from_sparse_and_random_property_templates(
        sparse_templates: impl IntoIterator<Item = (u32, ItemSparseTemplateEntry)>,
        random_property_templates: impl IntoIterator<Item = (u32, ItemRandomPropertyTemplateEntry)>,
    ) -> Self {
        let sparse_templates: HashMap<_, _> = sparse_templates.into_iter().collect();
        let flags = sparse_templates
            .iter()
            .map(|(&id, template)| (id, template.flags))
            .collect();
        Self {
            stats: HashMap::new(),
            flags,
            sparse_templates,
            random_property_templates: random_property_templates.into_iter().collect(),
        }
    }

    pub fn from_stats_sparse_and_random_property_templates(
        stats: impl IntoIterator<Item = (u32, ItemStatEntry)>,
        sparse_templates: impl IntoIterator<Item = (u32, ItemSparseTemplateEntry)>,
        random_property_templates: impl IntoIterator<Item = (u32, ItemRandomPropertyTemplateEntry)>,
    ) -> Self {
        let sparse_templates: HashMap<_, _> = sparse_templates.into_iter().collect();
        let flags = sparse_templates
            .iter()
            .map(|(&id, template)| (id, template.flags))
            .collect();
        Self {
            stats: stats.into_iter().collect(),
            flags,
            sparse_templates,
            random_property_templates: random_property_templates.into_iter().collect(),
        }
    }

    /// Load item stat modifiers from ItemSparse.db2.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemSparse.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let layout = field_layout();
        let mut stats = HashMap::new();
        let mut flags = HashMap::with_capacity(reader.total_count());
        let mut sparse_templates = HashMap::with_capacity(reader.total_count());
        let mut random_property_templates = HashMap::with_capacity(reader.total_count());
        let mut loaded = 0u32;

        for (id, idx) in reader.iter_records() {
            // Get raw record bytes
            let record_bytes = reader.record_bytes(idx);
            if record_bytes.is_none() {
                continue;
            }
            let record = record_bytes.unwrap();

            // Read fields sequentially, tracking position
            let mut pos = 0usize;

            // Field offsets for the stat fields we care about
            let mut bag_family_offset: usize = 0;
            let mut start_quest_id_offset: usize = 0;
            let mut stackable_offset: usize = 0;
            let mut max_count_offset: usize = 0;
            let mut lock_id_offset: usize = 0;
            let mut required_reputation_rank_offset: usize = 0;
            let mut sell_price_offset: usize = 0;
            let mut buy_price_offset: usize = 0;
            let mut vendor_stack_count_offset: usize = 0;
            let mut price_variance_offset: usize = 0;
            let mut price_random_value_offset: usize = 0;
            let mut flags_offset: usize = 0;
            let mut other_faction_item_id_offset: usize = 0;
            let mut content_tuning_id_offset: usize = 0;
            let mut player_level_to_item_level_curve_id_offset: usize = 0;
            let mut max_durability_offset: usize = 0;
            let mut limit_category_offset: usize = 0;
            let mut instance_bound_offset: usize = 0;
            let mut zone_bound_offset: usize = 0;
            let mut required_reputation_faction_offset: usize = 0;
            let mut allowable_class_offset: usize = 0;
            let mut resistances_offset: usize = 0;
            let mut stat_amount_offset: usize = 0;
            let mut item_level_offset: usize = 0;
            let mut expansion_id_offset: usize = 0;
            let mut bonding_offset: usize = 0;
            let mut stat_type_offset: usize = 0;
            let mut container_slots_offset: usize = 0;
            let mut inventory_type_offset: usize = 0;
            let mut quality_offset: usize = 0;

            for (fi, &(byte_size, is_string)) in layout.iter().enumerate() {
                if fi == 9 {
                    bag_family_offset = pos;
                }
                if fi == 10 {
                    start_quest_id_offset = pos;
                }
                if fi == 14 {
                    stackable_offset = pos;
                }
                if fi == 15 {
                    max_count_offset = pos;
                }
                if fi == 39 {
                    lock_id_offset = pos;
                }
                if fi == 16 {
                    required_reputation_rank_offset = pos;
                }
                if fi == 18 {
                    sell_price_offset = pos;
                }
                if fi == 19 {
                    buy_price_offset = pos;
                }
                if fi == 20 {
                    vendor_stack_count_offset = pos;
                }
                if fi == 21 {
                    price_variance_offset = pos;
                }
                if fi == 22 {
                    price_random_value_offset = pos;
                }
                if fi == 23 {
                    flags_offset = pos;
                }
                if fi == 24 {
                    other_faction_item_id_offset = pos;
                }
                if fi == 26 {
                    content_tuning_id_offset = pos;
                }
                if fi == 27 {
                    player_level_to_item_level_curve_id_offset = pos;
                }
                if fi == 28 {
                    max_durability_offset = pos;
                }
                if fi == 32 {
                    limit_category_offset = pos;
                }
                if fi == 36 {
                    instance_bound_offset = pos;
                }
                if fi == 37 {
                    zone_bound_offset = pos;
                }
                if fi == 42 {
                    required_reputation_faction_offset = pos;
                }
                if fi == 48 {
                    allowable_class_offset = pos;
                }
                if fi == 43 {
                    item_level_offset = pos;
                }
                if fi == 51 {
                    resistances_offset = pos;
                }
                if fi == 53 {
                    stat_amount_offset = pos;
                }
                if fi == 54 {
                    expansion_id_offset = pos;
                }
                if fi == 63 {
                    bonding_offset = pos;
                }
                if fi == 65 {
                    stat_type_offset = pos;
                }
                if fi == 66 {
                    container_slots_offset = pos;
                }
                if fi == 69 {
                    inventory_type_offset = pos;
                }
                if fi == 70 {
                    quality_offset = pos;
                }

                if is_string {
                    // Scan past inline null-terminated string
                    while pos < record.len() && record[pos] != 0 {
                        pos += 1;
                    }
                    if pos < record.len() {
                        pos += 1; // skip null terminator
                    }
                } else {
                    pos += byte_size as usize;
                }
            }

            // Extract stat data from the computed offsets
            if stat_amount_offset + 20 > record.len() || stat_type_offset + 10 > record.len() {
                continue;
            }

            let mut raw_flags = [0u32; 4];
            if flags_offset + 16 <= record.len() {
                for (flag_index, flag) in raw_flags.iter_mut().enumerate() {
                    let offset = flags_offset + flag_index * 4;
                    *flag = u32::from_le_bytes([
                        record[offset],
                        record[offset + 1],
                        record[offset + 2],
                        record[offset + 3],
                    ]);
                }
                flags.insert(id, raw_flags);
            }

            if bag_family_offset + 4 <= record.len()
                && start_quest_id_offset + 4 <= record.len()
                && stackable_offset + 4 <= record.len()
                && max_count_offset + 4 <= record.len()
                && lock_id_offset + 2 <= record.len()
                && required_reputation_rank_offset + 4 <= record.len()
                && sell_price_offset + 4 <= record.len()
                && buy_price_offset + 4 <= record.len()
                && vendor_stack_count_offset + 4 <= record.len()
                && price_variance_offset + 4 <= record.len()
                && price_random_value_offset + 4 <= record.len()
                && flags_offset + 16 <= record.len()
                && other_faction_item_id_offset + 4 <= record.len()
                && content_tuning_id_offset + 4 <= record.len()
                && player_level_to_item_level_curve_id_offset + 4 <= record.len()
                && max_durability_offset + 4 <= record.len()
                && limit_category_offset + 2 <= record.len()
                && instance_bound_offset + 2 <= record.len()
                && zone_bound_offset + 4 <= record.len()
                && required_reputation_faction_offset + 2 <= record.len()
                && allowable_class_offset + 2 <= record.len()
                && expansion_id_offset < record.len()
                && bonding_offset < record.len()
                && container_slots_offset < record.len()
                && inventory_type_offset < record.len()
            {
                sparse_templates.insert(
                    id,
                    ItemSparseTemplateEntry {
                        flags: raw_flags,
                        bag_family: read_u32(record, bag_family_offset),
                        start_quest_id: read_i32(record, start_quest_id_offset),
                        stackable: read_i32(record, stackable_offset),
                        max_count: read_i32(record, max_count_offset),
                        lock_id: read_u16(record, lock_id_offset),
                        required_reputation_rank: read_i32(record, required_reputation_rank_offset),
                        sell_price: read_u32(record, sell_price_offset),
                        buy_price: read_u32(record, buy_price_offset),
                        vendor_stack_count: read_u32(record, vendor_stack_count_offset),
                        price_variance: read_f32(record, price_variance_offset),
                        price_random_value: read_f32(record, price_random_value_offset),
                        other_faction_item_id: read_i32(record, other_faction_item_id_offset),
                        content_tuning_id: read_i32(record, content_tuning_id_offset),
                        player_level_to_item_level_curve_id: read_i32(
                            record,
                            player_level_to_item_level_curve_id_offset,
                        ),
                        max_durability: read_u32(record, max_durability_offset),
                        limit_category: read_u16(record, limit_category_offset),
                        instance_bound: read_u16(record, instance_bound_offset),
                        zone_bound: [
                            read_u16(record, zone_bound_offset),
                            read_u16(record, zone_bound_offset + 2),
                        ],
                        required_reputation_faction: read_u16(
                            record,
                            required_reputation_faction_offset,
                        ),
                        allowable_class: read_i16(record, allowable_class_offset),
                        required_expansion: record[expansion_id_offset],
                        bonding: record[bonding_offset],
                        container_slots: record[container_slots_offset],
                        inventory_type: record[inventory_type_offset] as i8,
                    },
                );
            }

            if item_level_offset + 2 <= record.len()
                && inventory_type_offset < record.len()
                && quality_offset < record.len()
            {
                random_property_templates.insert(
                    id,
                    ItemRandomPropertyTemplateEntry {
                        item_level: read_u16(record, item_level_offset),
                        quality: record[quality_offset] as i8,
                        inventory_type: record[inventory_type_offset] as i8,
                    },
                );
            }

            // Extract physical armor from Resistances[0] (field 51, first i16)
            let item_armor = if resistances_offset + 2 <= record.len() {
                i16::from_le_bytes([record[resistances_offset], record[resistances_offset + 1]])
                    as i32
            } else {
                0
            };

            let mut entry = ItemStatEntry {
                stats: [(-1i8, 0i16); 10],
                armor: item_armor,
            };

            let mut has_any = item_armor > 0;
            for i in 0..10 {
                let stat_type = record[stat_type_offset + i] as i8;
                let amount_off = stat_amount_offset + i * 2;
                let stat_amount = i16::from_le_bytes([record[amount_off], record[amount_off + 1]]);
                entry.stats[i] = (stat_type, stat_amount);
                if stat_type != -1 && stat_type != 0 && stat_amount != 0 {
                    has_any = true;
                } else if stat_type == 0 && stat_amount != 0 {
                    // stat_type=0 (Mana) with non-zero amount is valid
                    has_any = true;
                }
            }

            if has_any {
                stats.insert(id, entry);
                loaded += 1;
            }
        }

        info!(
            "Loaded {} items with stat modifiers from {}",
            loaded,
            path.display()
        );
        Ok(Self {
            stats,
            flags,
            sparse_templates,
            random_property_templates,
        })
    }

    /// Look up stat modifiers for an item.
    pub fn get(&self, item_id: u32) -> Option<&ItemStatEntry> {
        self.stats.get(&item_id)
    }

    /// Return C++ `ItemSparseEntry::Flags[0..4]` for an item.
    pub fn raw_flags(&self, item_id: u32) -> Option<[u32; 4]> {
        self.flags.get(&item_id).copied()
    }

    /// Return C++ `ItemSparseEntry::Flags[0]` as `ItemFlags`.
    pub fn item_flags(&self, item_id: u32) -> Option<ItemFlags> {
        self.raw_flags(item_id)
            .map(|flags| ItemFlags::from_bits_retain(u64::from(flags[0])))
    }

    /// Return the C++ `ItemSparseEntry` subset needed by `ItemTemplate` helpers.
    pub fn sparse_template(&self, item_id: u32) -> Option<&ItemSparseTemplateEntry> {
        self.sparse_templates.get(&item_id)
    }

    /// Iterate over the represented C++ `_itemTemplateStore` extended-data subset.
    pub fn sparse_templates_like_cpp(
        &self,
    ) -> impl Iterator<Item = (u32, &ItemSparseTemplateEntry)> {
        self.sparse_templates
            .iter()
            .map(|(&item_id, template)| (item_id, template))
    }

    /// Return the C++ `ItemSparseEntry` subset used by random-property generation.
    pub fn random_property_template(
        &self,
        item_id: u32,
    ) -> Option<&ItemRandomPropertyTemplateEntry> {
        self.random_property_templates.get(&item_id)
    }

    /// Number of items with stats.
    pub fn len(&self) -> usize {
        self.stats.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.stats.is_empty()
    }
}

fn read_u32(record: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        record[offset],
        record[offset + 1],
        record[offset + 2],
        record[offset + 3],
    ])
}

fn read_i32(record: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        record[offset],
        record[offset + 1],
        record[offset + 2],
        record[offset + 3],
    ])
}

fn read_f32(record: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        record[offset],
        record[offset + 1],
        record[offset + 2],
        record[offset + 3],
    ])
}

fn read_u16(record: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([record[offset], record[offset + 1]])
}

fn read_i16(record: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([record[offset], record[offset + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_stat_bonuses() {
        let entry = ItemStatEntry {
            armor: 0,
            stats: [
                (4, 100), // STR=100
                (7, 80),  // STA=80
                (3, 50),  // AGI=50
                (32, 40), // CritRating=40 (not a base stat)
                (-1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 0),
            ],
        };
        let [str, agi, sta, int, spi] = entry.base_stat_bonuses();
        assert_eq!(str, 100);
        assert_eq!(agi, 50);
        assert_eq!(sta, 80);
        assert_eq!(int, 0);
        assert_eq!(spi, 0);
    }

    #[test]
    fn test_attack_power_bonus() {
        let entry = ItemStatEntry {
            armor: 0,
            stats: [
                (38, 120), // AttackPower=120
                (4, 50),   // STR=50
                (-1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 0),
                (-1, 0),
            ],
        };
        assert_eq!(entry.attack_power_bonus(), 120);
        assert_eq!(entry.health_bonus(), 0);
    }

    #[test]
    fn item_sparse_flags_are_exposed_like_cpp_extended_data() {
        let store = ItemStatsStore::from_parts(
            [],
            [(
                1,
                [
                    ItemFlags::IS_BOUND_TO_ACCOUNT.bits() as u32,
                    0x20000,
                    0x400,
                    0,
                ],
            )],
        );

        assert_eq!(store.raw_flags(1), Some([0x0800_0000, 0x20000, 0x400, 0]));
        assert!(
            store
                .item_flags(1)
                .is_some_and(|flags| flags.contains(ItemFlags::IS_BOUND_TO_ACCOUNT))
        );
        assert_eq!(store.item_flags(2), None);
    }

    #[test]
    fn item_sparse_template_entry_matches_cpp_template_helpers() {
        let template = ItemSparseTemplateEntry {
            flags: [ItemFlags::IS_BOUND_TO_ACCOUNT.bits() as u32, 0, 0, 0],
            bag_family: 0x20,
            start_quest_id: 0,
            stackable: 20,
            max_count: 5,
            lock_id: 456,
            required_reputation_rank: 0,
            sell_price: 123,
            buy_price: 456,
            vendor_stack_count: 2,
            price_variance: 1.25,
            price_random_value: 0.75,
            other_faction_item_id: 999,
            content_tuning_id: 321,
            player_level_to_item_level_curve_id: 654,
            max_durability: 77,
            limit_category: 9,
            instance_bound: 11,
            zone_bound: [22, 33],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 4,
            bonding: 2,
            container_slots: 16,
            inventory_type: 18,
        };
        let store = ItemStatsStore::from_sparse_templates([(1, template)]);

        let loaded = store.sparse_template(1).unwrap();
        assert_eq!(loaded.max_stack_size(), 20);
        assert_eq!(loaded.max_count, 5);
        assert_eq!(loaded.lock_id, 456);
        assert_eq!(loaded.required_reputation_rank, 0);
        assert_eq!(loaded.required_reputation_faction, 0);
        assert_eq!(loaded.sell_price, 123);
        assert_eq!(loaded.buy_price, 456);
        assert_eq!(loaded.vendor_stack_count, 2);
        assert_eq!(loaded.price_variance, 1.25);
        assert_eq!(loaded.price_random_value, 0.75);
        assert_eq!(loaded.other_faction_item_id_like_cpp(), 999);
        assert_eq!(loaded.scaling_stat_content_tuning_like_cpp(), 321);
        assert_eq!(loaded.player_level_to_item_level_curve_id_like_cpp(), 654);
        assert_eq!(loaded.instance_bound, 11);
        assert_eq!(loaded.zone_bound, [22, 33]);
        assert_eq!(loaded.required_expansion, 4);
        assert_eq!(loaded.container_slots, 16);
        assert_eq!(loaded.inventory_type, 18);
        assert_eq!(loaded.allowable_class, -1);
        assert!(loaded.item_flags().contains(ItemFlags::IS_BOUND_TO_ACCOUNT));
        assert_eq!(store.raw_flags(1), Some(template.flags));

        let unlimited = ItemSparseTemplateEntry {
            stackable: 0,
            ..template
        };
        assert_eq!(unlimited.max_stack_size(), 0x7FFF_FFFE);
    }

    #[test]
    fn test_load_item_stats_store() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = std::path::Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemSparse.db2");
        if !path.exists() {
            eprintln!("Skipping test: ItemSparse.db2 not found");
            return;
        }

        let store = ItemStatsStore::load(data_dir, locale).expect("failed to load ItemStatsStore");

        eprintln!("ItemStatsStore: {} items with stats", store.len());
        assert!(
            store.len() > 1000,
            "expected >1000 items with stats, got {}",
            store.len()
        );

        // Check Shadowmourne (49623): should have STR, STA, CritRating
        if let Some(entry) = store.get(49623) {
            let [str, _agi, sta, _int, _spi] = entry.base_stat_bonuses();
            eprintln!("Shadowmourne: STR={str}, STA={sta}");
            eprintln!("  full stats: {:?}", entry.stats);
            assert!(str > 100, "Shadowmourne STR should be >100, got {str}");
            assert!(sta > 100, "Shadowmourne STA should be >100, got {sta}");
        } else {
            eprintln!("Shadowmourne (49623) not found in stats store");
        }

        // Hearthstone (6948) should NOT be in the store (no combat stats)
        assert!(
            store.get(6948).is_none(),
            "Hearthstone should have no stats"
        );
    }
}
