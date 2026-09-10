//! Creature template model and loader state definitions, part 2 of 2.
//!
//! Separated from the creature_template.rs root under #664. Behaviour is preserved.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureTemplateMountModelLikeCpp {
    pub display_id: u32,
    pub display_scale: f32,
    pub probability: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureTemplateMountEntryLikeCpp {
    pub entry: u32,
    pub vehicle_id: u32,
    pub models: Vec<CreatureTemplateMountModelLikeCpp>,
}

#[derive(Debug, Clone, Default)]
pub struct CreatureTemplateMountStoreLikeCpp {
    pub(super) entries: HashMap<u32, CreatureTemplateMountEntryLikeCpp>,
}

impl CreatureTemplateMountStoreLikeCpp {
    pub fn from_entries(
        entries: impl IntoIterator<Item = CreatureTemplateMountEntryLikeCpp>,
    ) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|entry| (entry.entry, entry))
                .collect(),
        }
    }

    pub fn from_rows_like_cpp(rows: impl IntoIterator<Item = (u32, u32, u32, f32, f32)>) -> Self {
        let mut entries = HashMap::new();
        for (entry_id, vehicle_id, display_id, display_scale, probability) in rows {
            let entry =
                entries
                    .entry(entry_id)
                    .or_insert_with(|| CreatureTemplateMountEntryLikeCpp {
                        entry: entry_id,
                        vehicle_id,
                        models: Vec::new(),
                    });
            entry.vehicle_id = vehicle_id;
            if display_id != 0 {
                entry.models.push(CreatureTemplateMountModelLikeCpp {
                    display_id,
                    display_scale,
                    probability,
                });
            }
        }
        Self { entries }
    }

    pub fn get(&self, entry: u32) -> Option<&CreatureTemplateMountEntryLikeCpp> {
        self.entries.get(&entry)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl CreatureTemplateMountEntryLikeCpp {
    pub fn choose_display_id_like_cpp<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<u32> {
        match self.models.as_slice() {
            [] => None,
            [model] => Some(model.display_id),
            models => {
                let total: f32 = models.iter().map(|model| model.probability.max(0.0)).sum();
                if total <= f32::EPSILON {
                    return models.first().map(|model| model.display_id);
                }

                let mut roll = rng.gen_range(0.0..total);
                for model in models {
                    roll -= model.probability.max(0.0);
                    if roll <= 0.0 {
                        return Some(model.display_id);
                    }
                }

                models.last().map(|model| model.display_id)
            }
        }
    }
}

/// C++ `CURRENT_EXPANSION` for this 3.4.3/TDB442 port.
///
/// Anchor: `SharedDefines.h:87-105` defines Wrath of the Lich King as 2 and
/// `CURRENT_EXPANSION` as `EXPANSION_WRATH_OF_THE_LICH_KING`.
pub const CREATURE_CURRENT_EXPANSION_LIKE_CPP: usize = 2;

/// C++ sentinel `EXPANSION_LEVEL_CURRENT` used by `CreatureDifficulty`.
pub const CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP: i32 = -1;

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureDifficultyRecordLikeCpp {
    pub entry: u32,
    pub difficulty_id: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub health_scaling_expansion: i32,
    pub health_modifier: f32,
    pub mana_modifier: f32,
    pub armor_modifier: f32,
    pub damage_modifier: f32,
    pub creature_difficulty_id: i32,
    pub type_flags: u32,
    pub type_flags2: u32,
    pub loot_id: u32,
    pub pickpocket_loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub static_flags: [u32; 8],
}

impl CreatureDifficultyRecordLikeCpp {
    /// Applies the C++ `ObjectMgr::LoadCreatureTemplateDifficulty` row fixes.
    ///
    /// `classification_damage_modifier` represents
    /// `Creature::GetDamageMod(template.Classification)`. The full creature
    /// template classification lookup remains a future integration slice; this
    /// pure data normalizer only applies the caller-provided multiplier.
    pub fn normalize_like_cpp(mut self, classification_damage_modifier: f32) -> Self {
        self.damage_modifier *= classification_damage_modifier;

        if self.min_level == 0 {
            self.min_level = 1;
        }
        if self.max_level == 0 {
            self.max_level = 1;
        }
        if self.min_level > self.max_level {
            self.min_level = self.max_level;
        }
        if self.health_scaling_expansion < CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP
            || self.health_scaling_expansion > CREATURE_CURRENT_EXPANSION_LIKE_CPP as i32
        {
            self.health_scaling_expansion = 0;
        }
        if self.gold_min > self.gold_max {
            self.gold_max = self.gold_min;
        }

        self
    }

    /// Matches `CreatureDifficulty::GetHealthScalingExpansion`: `-1` maps to
    /// C++ `CURRENT_EXPANSION`, otherwise the normalized DB value is used.
    pub fn health_scaling_expansion_index_like_cpp(&self) -> usize {
        if self.health_scaling_expansion == CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP {
            CREATURE_CURRENT_EXPANSION_LIKE_CPP
        } else {
            self.health_scaling_expansion as usize
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureBaseStatsRecordLikeCpp {
    pub base_health: [u32; CREATURE_CURRENT_EXPANSION_LIKE_CPP + 1],
    pub base_mana: u32,
    pub base_armor: u32,
    pub attack_power: u32,
    pub ranged_attack_power: u32,
    pub base_damage: [f32; CREATURE_CURRENT_EXPANSION_LIKE_CPP + 1],
}

impl Default for CreatureBaseStatsRecordLikeCpp {
    fn default() -> Self {
        Self {
            base_health: [0; CREATURE_CURRENT_EXPANSION_LIKE_CPP + 1],
            base_mana: 0,
            base_armor: 0,
            attack_power: 0,
            ranged_attack_power: 0,
            base_damage: [0.0; CREATURE_CURRENT_EXPANSION_LIKE_CPP + 1],
        }
    }
}

impl CreatureBaseStatsRecordLikeCpp {
    /// Applies C++ `LoadCreatureClassLevelStats` fixes: loaded row HP zero -> 1,
    /// negative base damage -> 0. Missing rows are handled by the store default
    /// and intentionally retain all-zero health/damage arrays like C++ static
    /// zero-initialized fallback stats.
    pub fn normalize_loaded_row_like_cpp(mut self) -> Self {
        for hp in &mut self.base_health {
            if *hp == 0 {
                *hp = 1;
            }
        }
        for damage in &mut self.base_damage {
            if *damage < 0.0 {
                *damage = 0.0;
            }
        }
        self
    }

    pub fn generate_health_like_cpp(&self, difficulty: &CreatureDifficultyRecordLikeCpp) -> u32 {
        (self.base_health[difficulty.health_scaling_expansion_index_like_cpp()] as f32
            * difficulty.health_modifier)
            .ceil() as u32
    }

    pub fn generate_mana_like_cpp(&self, difficulty: &CreatureDifficultyRecordLikeCpp) -> u32 {
        if self.base_mana == 0 {
            return 0;
        }

        (self.base_mana as f32 * difficulty.mana_modifier).ceil() as u32
    }

    pub fn generate_armor_like_cpp(&self, difficulty: &CreatureDifficultyRecordLikeCpp) -> u32 {
        (self.base_armor as f32 * difficulty.armor_modifier).ceil() as u32
    }

    pub fn generate_base_damage_like_cpp(
        &self,
        difficulty: &CreatureDifficultyRecordLikeCpp,
    ) -> f32 {
        self.base_damage[difficulty.health_scaling_expansion_index_like_cpp()]
    }
}

#[derive(Debug, Clone, Default)]
pub struct CreatureBaseStatsStoreLikeCpp {
    pub(super) records: HashMap<(u8, u8), CreatureBaseStatsRecordLikeCpp>,
    pub(super) default_record: CreatureBaseStatsRecordLikeCpp,
}

impl CreatureBaseStatsStoreLikeCpp {
    pub fn from_records(
        records: impl IntoIterator<Item = (u8, u8, CreatureBaseStatsRecordLikeCpp)>,
    ) -> Self {
        Self {
            records: records
                .into_iter()
                .map(|(level, unit_class, record)| {
                    ((level, unit_class), record.normalize_loaded_row_like_cpp())
                })
                .collect(),
            default_record: CreatureBaseStatsRecordLikeCpp::default(),
        }
    }

    pub fn get_like_cpp(&self, level: u8, unit_class: u8) -> &CreatureBaseStatsRecordLikeCpp {
        self.records
            .get(&(level, unit_class))
            .unwrap_or(&self.default_record)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct CreatureDifficultyStoreLikeCpp {
    pub(super) records: HashMap<(u32, u8), CreatureDifficultyRecordLikeCpp>,
    pub(super) difficulty_fallbacks: HashMap<u8, u8>,
    pub(super) default_record: CreatureDifficultyRecordLikeCpp,
}

impl Default for CreatureDifficultyStoreLikeCpp {
    fn default() -> Self {
        Self {
            records: HashMap::new(),
            difficulty_fallbacks: HashMap::new(),
            default_record: CreatureDifficultyRecordLikeCpp::default_fallback_like_cpp(),
        }
    }
}

impl CreatureDifficultyStoreLikeCpp {
    pub fn from_records(
        records: impl IntoIterator<Item = CreatureDifficultyRecordLikeCpp>,
        classification_damage_modifier_for_entry: impl Fn(u32) -> f32,
    ) -> Self {
        Self::from_records_with_difficulty_fallbacks(
            records,
            classification_damage_modifier_for_entry,
            std::iter::empty(),
        )
    }

    pub fn from_records_with_difficulty_fallbacks(
        records: impl IntoIterator<Item = CreatureDifficultyRecordLikeCpp>,
        classification_damage_modifier_for_entry: impl Fn(u32) -> f32,
        difficulty_fallbacks: impl IntoIterator<Item = (u8, u8)>,
    ) -> Self {
        Self {
            records: records
                .into_iter()
                .map(|record| {
                    let key = (record.entry, record.difficulty_id);
                    let classification_damage_modifier =
                        classification_damage_modifier_for_entry(record.entry);
                    let normalized = record.normalize_like_cpp(classification_damage_modifier);
                    (key, normalized)
                })
                .collect(),
            difficulty_fallbacks: difficulty_fallbacks.into_iter().collect(),
            default_record: CreatureDifficultyRecordLikeCpp::default_fallback_like_cpp(),
        }
    }

    pub fn from_records_and_difficulty_store_like_cpp(
        records: impl IntoIterator<Item = CreatureDifficultyRecordLikeCpp>,
        difficulty_store: &crate::DifficultyStore,
        classification_damage_modifier_for_entry: impl Fn(u32) -> f32,
    ) -> Self {
        Self::from_records_with_difficulty_fallbacks(
            records,
            classification_damage_modifier_for_entry,
            difficulty_fallback_pairs_like_cpp(difficulty_store),
        )
    }

    pub fn get_like_cpp(&self, entry: u32, difficulty_id: u8) -> &CreatureDifficultyRecordLikeCpp {
        let mut current = difficulty_id;
        let mut seen = [false; 256];
        loop {
            if let Some(record) = self.records.get(&(entry, current)) {
                return record;
            }

            if seen[current as usize] {
                return &self.default_record;
            }
            seen[current as usize] = true;

            let Some(fallback) = self.difficulty_fallbacks.get(&current).copied() else {
                return &self.default_record;
            };
            current = fallback;
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl CreatureDifficultyRecordLikeCpp {
    pub fn default_fallback_like_cpp() -> Self {
        Self {
            entry: 0,
            difficulty_id: 0,
            min_level: 1,
            max_level: 1,
            health_scaling_expansion: 0,
            health_modifier: 1.0,
            mana_modifier: 1.0,
            armor_modifier: 1.0,
            damage_modifier: 1.0,
            creature_difficulty_id: 0,
            type_flags: 0,
            type_flags2: 0,
            loot_id: 0,
            pickpocket_loot_id: 0,
            skin_loot_id: 0,
            gold_min: 0,
            gold_max: 0,
            static_flags: [0; 8],
        }
    }
}

pub(super) fn difficulty_fallback_pairs_like_cpp(
    difficulty_store: &crate::DifficultyStore,
) -> impl Iterator<Item = (u8, u8)> + '_ {
    (0u8..=u8::MAX).filter_map(|difficulty_id| {
        difficulty_store
            .fallback_difficulty_id_like_cpp(difficulty_id)
            .map(|fallback| (difficulty_id, fallback))
    })
}
