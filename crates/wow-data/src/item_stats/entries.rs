//! Entries packets.
//!
//! Separated from item_stats.rs under #691.

use super::*;

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
    /// C++ `ItemSparseEntry::Resistances` indexed by `SpellSchools`.
    pub resistances: [i16; 7],
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

/// C++ `ItemSparseEntry` weapon fields used by `Player::_ApplyWeaponDamage`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemWeaponTemplateEntry {
    pub dmg_variance: f32,
    pub item_delay: u16,
    pub min_damage: [u16; 5],
    pub max_damage: [u16; 5],
    pub damage_damage_type: u8,
}

/// C++ `ItemSparseEntry` fields used by socket-enchantment requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSocketTemplateEntry {
    pub socket_types: [u8; 3],
    pub required_skill_id: u16,
    pub required_skill_rank: u16,
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
