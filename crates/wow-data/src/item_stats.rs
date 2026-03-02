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

use anyhow::{Context, Result, ensure};
use tracing::{debug, info};

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

impl ItemStatEntry {
    /// Sum base stat bonuses: [STR, AGI, STA, INT, SPI].
    pub fn base_stat_bonuses(&self) -> [i32; 5] {
        let mut result = [0i32; 5];
        for &(stat_type, amount) in &self.stats {
            let amount = amount as i32;
            match stat_type {
                4 => result[0] += amount,  // Strength
                3 => result[1] += amount,  // Agility
                7 => result[2] += amount,  // Stamina
                5 => result[3] += amount,  // Intellect
                6 => result[4] += amount,  // Spirit
                _ => {}
            }
        }
        result
    }

    /// Total attack power bonus from items.
    pub fn attack_power_bonus(&self) -> i32 {
        self.stats.iter()
            .filter(|&&(t, _)| t == 38)
            .map(|&(_, v)| v as i32)
            .sum()
    }

    /// Total ranged attack power bonus.
    pub fn ranged_attack_power_bonus(&self) -> i32 {
        self.stats.iter()
            .filter(|&&(t, _)| t == 39)
            .map(|&(_, v)| v as i32)
            .sum()
    }

    /// Total health bonus.
    pub fn health_bonus(&self) -> i32 {
        self.stats.iter()
            .filter(|&&(t, _)| t == 1)
            .map(|&(_, v)| v as i32)
            .sum()
    }

    /// Total mana bonus.
    pub fn mana_bonus(&self) -> i32 {
        self.stats.iter()
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
                31 => { cr[5] += amount; cr[6] += amount; cr[7] += amount; } // HitRating → all
                32 => { cr[8] += amount; cr[9] += amount; cr[10] += amount; } // CritRating → all
                36 => { cr[17] += amount; cr[18] += amount; cr[19] += amount; } // HasteRating → all
                37 => cr[23] += amount,  // ExpertiseRating → Expertise
                44 => cr[24] += amount,  // ArmorPenetrationRating → ArmorPenetration
                _ => {}
            }
        }
        cr
    }

    /// Total spell power bonus from item stats.
    pub fn spell_power_bonus(&self) -> i32 {
        self.stats.iter()
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
    layout[1] = (0, true);  // Description
    layout[2] = (0, true);  // Display3
    layout[3] = (0, true);  // Display2
    layout[4] = (0, true);  // Display1
    layout[5] = (0, true);  // Display

    // Fields 6-11: f32/i32 (4 bytes each)
    for i in 6..=11 { layout[i] = (4, false); }

    // Field 12: StatPercentageOfSocket[10] (10 × f32 = 40 bytes)
    layout[12] = (40, false);

    // Field 13: StatPercentEditor[10] (10 × i32 = 40 bytes)
    layout[13] = (40, false);

    // Fields 14-22: i32/u32/f32 (4 bytes each)
    for i in 14..=22 { layout[i] = (4, false); }

    // Field 23: Flags[4] (4 × i32 = 16 bytes)
    layout[23] = (16, false);

    // Fields 24-28: i32/u32 (4 bytes each)
    for i in 24..=28 { layout[i] = (4, false); }

    // Fields 29-36: u16 (2 bytes each)
    for i in 29..=36 { layout[i] = (2, false); }

    // Field 37: ZoneBound[2] (2 × u16 = 4 bytes)
    layout[37] = (4, false);

    // Fields 38-48: u16/i16 (2 bytes each)
    for i in 38..=48 { layout[i] = (2, false); }

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
    for i in 54..=57 { layout[i] = (1, false); }

    // Field 58: SocketType[3] (3 × u8 = 3 bytes)
    layout[58] = (3, false);

    // Fields 59-64: u8/i8 (1 byte each)
    for i in 59..=64 { layout[i] = (1, false); }

    // Field 65: _statModifierBonusStat[10] (10 × i8 = 10 bytes)
    layout[65] = (10, false);

    // Fields 66-72: u8/i8 (1 byte each)
    for i in 66..=72 { layout[i] = (1, false); }

    layout
}

impl ItemStatsStore {
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
            let mut resistances_offset: usize = 0;
            let mut stat_amount_offset: usize = 0;
            let mut stat_type_offset: usize = 0;

            for (fi, &(byte_size, is_string)) in layout.iter().enumerate() {
                if fi == 51 {
                    resistances_offset = pos;
                }
                if fi == 53 {
                    stat_amount_offset = pos;
                }
                if fi == 65 {
                    stat_type_offset = pos;
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

            // Extract physical armor from Resistances[0] (field 51, first i16)
            let item_armor = if resistances_offset + 2 <= record.len() {
                i16::from_le_bytes([record[resistances_offset], record[resistances_offset + 1]]) as i32
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
                let stat_amount = i16::from_le_bytes([
                    record[amount_off],
                    record[amount_off + 1],
                ]);
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

        info!("Loaded {} items with stat modifiers from {}", loaded, path.display());
        Ok(Self { stats })
    }

    /// Look up stat modifiers for an item.
    pub fn get(&self, item_id: u32) -> Option<&ItemStatEntry> {
        self.stats.get(&item_id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_stat_bonuses() {
        let entry = ItemStatEntry {
            armor: 0,
            stats: [
                (4, 100),   // STR=100
                (7, 80),    // STA=80
                (3, 50),    // AGI=50
                (32, 40),   // CritRating=40 (not a base stat)
                (-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0),
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
                (38, 120),  // AttackPower=120
                (4, 50),    // STR=50
                (-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0),
            ],
        };
        assert_eq!(entry.attack_power_bonus(), 120);
        assert_eq!(entry.health_bonus(), 0);
    }

    #[test]
    fn test_load_item_stats_store() {
        let data_dir = "/path/to/wow-data";
        let locale = "esES";
        let path = std::path::Path::new(data_dir)
            .join("dbc").join(locale).join("ItemSparse.db2");
        if !path.exists() {
            eprintln!("Skipping test: ItemSparse.db2 not found");
            return;
        }

        let store = ItemStatsStore::load(data_dir, locale)
            .expect("failed to load ItemStatsStore");

        eprintln!("ItemStatsStore: {} items with stats", store.len());
        assert!(store.len() > 1000, "expected >1000 items with stats, got {}", store.len());

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
        assert!(store.get(6948).is_none(), "Hearthstone should have no stats");
    }
}
