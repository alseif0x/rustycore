//! Scaling packets.
//!
//! Separated from progression_rewards.rs under #691.

use super::*;

/// C++ `MAX_LEVEL` from `DataStores/DBCEnums.h` for the 3.4.3 client data set.
pub const MAX_LEVEL_LIKE_CPP: u8 = 123;

pub const CONTENT_TUNING_FLAG_DISABLED_FOR_ITEM_LIKE_CPP: i32 = 0x04;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentTuningEntry {
    pub id: u32,
    pub min_level: i32,
    pub max_level: i32,
    pub flags: i32,
    pub expected_stat_mod_id: i32,
    pub difficulty_esm_id: i32,
}

impl ContentTuningEntry {
    pub const fn disabled_for_item_like_cpp(&self) -> bool {
        (self.flags & CONTENT_TUNING_FLAG_DISABLED_FOR_ITEM_LIKE_CPP) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentTuningLevelsLikeCpp {
    pub min_level: i32,
    pub max_level: i32,
    pub min_level_with_delta: i32,
    pub max_level_with_delta: i32,
    pub target_level_min: i32,
    pub target_level_max: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumTalentsAtLevelEntry {
    pub id: u32,
    pub num_talents: i32,
    pub num_talents_death_knight: i32,
    pub num_talents_demon_hunter: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalingStatDistributionEntry {
    pub id: u32,
    pub player_level_to_item_level_curve_id: u16,
    pub min_level: i32,
    pub max_level: i32,
    pub bonus: [i32; 10],
    pub stat_id: [i32; 10],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalingStatValuesEntry {
    pub id: u32,
    pub char_level: i32,
    pub weapon_dps_1h: i32,
    pub weapon_dps_2h: i32,
    pub spellcaster_dps_1h: i32,
    pub spellcaster_dps_2h: i32,
    pub ranged_dps: i32,
    pub wand_dps: i32,
    pub spell_power: i32,
    pub shoulder_budget: i32,
    pub trinket_budget: i32,
    pub weapon_budget_1h: i32,
    pub primary_budget: i32,
    pub ranged_budget: i32,
    pub tertiary_budget: i32,
    pub cloth_shoulder_armor: i32,
    pub leather_shoulder_armor: i32,
    pub mail_shoulder_armor: i32,
    pub plate_shoulder_armor: i32,
    pub cloth_cloak_armor: i32,
    pub cloth_chest_armor: i32,
    pub leather_chest_armor: i32,
    pub mail_chest_armor: i32,
    pub plate_chest_armor: i32,
}

impl ScalingStatValuesEntry {
    /// C++ `ScalingStatValuesEntry::getssdMultiplier`.
    pub fn ssd_multiplier_like_cpp(&self, mask: u32) -> i32 {
        if mask & 0x4001F == 0 {
            return 0;
        }
        if mask & 0x00000001 != 0 {
            return self.shoulder_budget;
        }
        if mask & 0x00000002 != 0 {
            return self.trinket_budget;
        }
        if mask & 0x00000004 != 0 {
            return self.weapon_budget_1h;
        }
        if mask & 0x00000008 != 0 {
            return self.primary_budget;
        }
        if mask & 0x00000010 != 0 {
            return self.ranged_budget;
        }
        if mask & 0x00040000 != 0 {
            return self.tertiary_budget;
        }
        0
    }

    /// C++ `ScalingStatValuesEntry::getArmorMod`.
    pub fn armor_mod_like_cpp(&self, mask: u32) -> i32 {
        if mask & 0x00F001E0 == 0 {
            return 0;
        }
        if mask & 0x00000020 != 0 {
            return self.cloth_shoulder_armor;
        }
        if mask & 0x00000040 != 0 {
            return self.leather_shoulder_armor;
        }
        if mask & 0x00000080 != 0 {
            return self.mail_shoulder_armor;
        }
        if mask & 0x00000100 != 0 {
            return self.plate_shoulder_armor;
        }
        if mask & 0x00080000 != 0 {
            return self.cloth_cloak_armor;
        }
        if mask & 0x00100000 != 0 {
            return self.cloth_chest_armor;
        }
        if mask & 0x00200000 != 0 {
            return self.leather_chest_armor;
        }
        if mask & 0x00400000 != 0 {
            return self.mail_chest_armor;
        }
        if mask & 0x00800000 != 0 {
            return self.plate_chest_armor;
        }
        0
    }

    /// C++ `ScalingStatValuesEntry::getDPSMod`.
    pub fn dps_mod_like_cpp(&self, mask: u32) -> i32 {
        if mask & 0x7E00 == 0 {
            return 0;
        }
        if mask & 0x00000200 != 0 {
            return self.weapon_dps_1h;
        }
        if mask & 0x00000400 != 0 {
            return self.weapon_dps_2h;
        }
        if mask & 0x00000800 != 0 {
            return self.spellcaster_dps_1h;
        }
        if mask & 0x00001000 != 0 {
            return self.spellcaster_dps_2h;
        }
        if mask & 0x00002000 != 0 {
            return self.ranged_dps;
        }
        if mask & 0x00004000 != 0 {
            return self.wand_dps;
        }
        0
    }

    /// C++ `ScalingStatValuesEntry::isTwoHand`.
    pub fn is_two_hand_like_cpp(&self, mask: u32) -> bool {
        mask & 0x7E00 != 0 && (mask & 0x00000400 != 0 || mask & 0x00001000 != 0)
    }

    /// C++ `ScalingStatValuesEntry::getSpellBonus`.
    pub fn spell_bonus_like_cpp(&self, mask: u32) -> i32 {
        if mask & 0x00008000 != 0 {
            self.spell_power
        } else {
            0
        }
    }
}

impl ContentTuningStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ContentTuning.db2", |id, idx, r| {
            ContentTuningEntry {
                id,
                min_level: r.get_field_i32(idx, 1),
                max_level: r.get_field_i32(idx, 2),
                flags: r.get_field_i32(idx, 3),
                expected_stat_mod_id: r.get_field_i32(idx, 4),
                difficulty_esm_id: r.get_field_i32(idx, 5),
            }
        })
    }

    pub fn content_tuning_data_like_cpp(
        &self,
        content_tuning_id: u32,
        for_item: bool,
    ) -> Option<ContentTuningLevelsLikeCpp> {
        let content_tuning = self.get(content_tuning_id)?;
        if for_item && content_tuning.disabled_for_item_like_cpp() {
            return None;
        }

        let max_level = i32::from(MAX_LEVEL_LIKE_CPP);
        let min_level_with_delta = content_tuning.min_level.clamp(1, max_level);
        let max_level_with_delta = content_tuning.max_level.clamp(1, max_level);
        let min_level = content_tuning.min_level.clamp(1, max_level);
        let max_level = content_tuning.max_level.clamp(1, max_level);

        Some(ContentTuningLevelsLikeCpp {
            min_level,
            max_level,
            min_level_with_delta,
            max_level_with_delta,
            target_level_min: min_level_with_delta,
            target_level_max: max_level_with_delta,
        })
    }
}

impl NumTalentsAtLevelStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "NumTalentsAtLevel.db2", |id, idx, r| {
            NumTalentsAtLevelEntry {
                id,
                num_talents: r.get_field_i32(idx, 1),
                num_talents_death_knight: r.get_field_i32(idx, 2),
                num_talents_demon_hunter: r.get_field_i32(idx, 3),
            }
        })
    }

    pub fn num_talents_at_level_like_cpp(&self, level: u32, class_id: u8) -> u32 {
        let entry = self.get(level).or_else(|| {
            self.entries
                .keys()
                .max()
                .and_then(|highest_level| self.get(*highest_level))
        });

        let Some(entry) = entry else {
            return 0;
        };

        let points = match class_id {
            6 => entry.num_talents_death_knight,
            12 => entry.num_talents_demon_hunter,
            _ => entry.num_talents,
        };
        points.max(0) as u32
    }
}

impl ScalingStatDistributionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ScalingStatDistribution.db2",
            |id, idx, r| ScalingStatDistributionEntry {
                id,
                player_level_to_item_level_curve_id: r.get_field_u16(idx, 0),
                min_level: r.get_field_i32(idx, 1),
                max_level: r.get_field_i32(idx, 2),
                bonus: std::array::from_fn(|i| r.get_array_element(idx, 3, i, 32) as i32),
                stat_id: std::array::from_fn(|i| r.get_array_element(idx, 4, i, 32) as i32),
            },
        )
    }
}

impl ScalingStatValuesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ScalingStatValues.db2", |id, idx, r| {
            ScalingStatValuesEntry {
                id,
                char_level: r.get_field_i32(idx, 0),
                weapon_dps_1h: r.get_field_i32(idx, 1),
                weapon_dps_2h: r.get_field_i32(idx, 2),
                spellcaster_dps_1h: r.get_field_i32(idx, 3),
                spellcaster_dps_2h: r.get_field_i32(idx, 4),
                ranged_dps: r.get_field_i32(idx, 5),
                wand_dps: r.get_field_i32(idx, 6),
                spell_power: r.get_field_i32(idx, 7),
                shoulder_budget: r.get_field_i32(idx, 8),
                trinket_budget: r.get_field_i32(idx, 9),
                weapon_budget_1h: r.get_field_i32(idx, 10),
                primary_budget: r.get_field_i32(idx, 11),
                ranged_budget: r.get_field_i32(idx, 12),
                tertiary_budget: r.get_field_i32(idx, 13),
                cloth_shoulder_armor: r.get_field_i32(idx, 14),
                leather_shoulder_armor: r.get_field_i32(idx, 15),
                mail_shoulder_armor: r.get_field_i32(idx, 16),
                plate_shoulder_armor: r.get_field_i32(idx, 17),
                cloth_cloak_armor: r.get_field_i32(idx, 18),
                cloth_chest_armor: r.get_field_i32(idx, 19),
                leather_chest_armor: r.get_field_i32(idx, 20),
                mail_chest_armor: r.get_field_i32(idx, 21),
                plate_chest_armor: r.get_field_i32(idx, 22),
            }
        })
    }

    /// C++ `DB2Manager::GetScalingStatValuesForLevel`.
    pub fn get_for_character_level_like_cpp(
        &self,
        character_level: u32,
    ) -> Option<&ScalingStatValuesEntry> {
        self.entries
            .values()
            .find(|entry| entry.char_level == character_level as i32)
    }
}
