// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SkillLineAbility.db2 + SkillRaceClassInfo.db2 reader.
//!
//! Determines which spells each race/class/level should auto-learn,
//! replicating TrinityCore C++ `LearnDefaultSkills()` → `SetSkill()` →
//! `LearnSkillRewardedSpells()`.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_database::{HotfixDatabase, HotfixStatements, SqlResult, WorldDatabase, WorldStatements};

use crate::Db2HotfixRemovalStoreLikeCpp;
use crate::entities_movement::CreatureFamilyEntry;
use crate::skill_talent::{SkillLineAcquisitionPayloadLikeCpp, SkillLineStore};
use crate::wdc4::Wdc4Reader;

// ── Records ─────────────────────────────────────────────────────────

/// A single record from SkillLineAbility.db2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillLineAbilityRecord {
    pub id: u32,
    pub race_mask: i64,
    pub skill_line: u16,
    pub spell: i32,
    pub min_skill_line_rank: i16,
    pub class_mask: i32,
    pub supercedes_spell: i32,
    /// 0=None, 1=OnSkillValue, 2=OnSkillLearn
    pub acquire_method: i8,
    pub trivial_rank_high: i16,
    pub trivial_rank_low: i16,
    pub flags: i8,
    pub num_skill_ups: i8,
    pub skillup_skill_line_id: i16,
}

/// C++ `SKILL_LINE_ABILITY_REWARDED_FROM_QUEST`.
pub const SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP: i8 = 4;
pub const SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP: i8 = 1;
pub const SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP: i8 = 2;
/// C++ `SkillLineAbilityFlags::CanFallbackToLearnedOnSkillLearn`.
pub const SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP: i8 = 0x80u8 as i8;
pub const SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP: u16 = 0x10;
pub const SKILL_RUNEFORGING_LIKE_CPP: u16 = 960;
pub const SKILL_RIDING_LIKE_CPP: u16 = 762;
pub const SKILL_CATEGORY_ARMOR_LIKE_CPP: i8 = 8;
pub const SKILL_CATEGORY_LANGUAGES_LIKE_CPP: i8 = 10;
pub const SKILL_CATEGORY_SECONDARY_LIKE_CPP: i8 = 9;
pub const SKILL_CATEGORY_PROFESSION_LIKE_CPP: i8 = 11;
pub const CLASS_DEATH_KNIGHT_LIKE_CPP: u8 = 6;
const RACE_HUMAN_LIKE_CPP: u8 = 1;
const MAX_RACES_LIKE_CPP: u8 = 78;
const CLASS_WARRIOR_LIKE_CPP: u8 = 1;
const MAX_CLASSES_LIKE_CPP: u8 = 15;

/// A single record from SkillRaceClassInfo.db2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRaceClassInfoRecord {
    pub id: u32,
    pub race_mask: i64,
    pub skill_id: u16,
    pub class_mask: i32,
    pub flags: u16,
    /// 1 = available at creation
    pub availability: i8,
    pub min_level: i8,
    pub skill_tier_id: i16,
}

/// Input layer that supplied an effective DB2 record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillStoreLoadSourceLikeCpp {
    Wdc4,
    OfficialSql,
    CustomSql,
}

/// Source table associated with an effective-skill diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillStoreTableLikeCpp {
    SkillLineAbility,
    SkillRaceClassInfo,
}

/// Fail-closed diagnostics retained while composing effective skill metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillStoreLoadDiagnosticLikeCpp {
    InvalidSkillLineAbilityIdentifier {
        source: SkillStoreLoadSourceLikeCpp,
        record_id: u32,
        spell: i128,
        skill_line: i128,
        skillup_skill_line_id: i128,
    },
    InvalidSkillRaceClassInfoIdentifier {
        source: SkillStoreLoadSourceLikeCpp,
        record_id: u32,
        race_mask: i128,
        skill_id: i128,
        class_mask: i128,
    },
    InvalidSourceField {
        table: SkillStoreTableLikeCpp,
        source: SkillStoreLoadSourceLikeCpp,
        record_id: u32,
        field: &'static str,
        value: i128,
    },
    MissingEffectiveSkillLine {
        record_id: u32,
        skill_id: u16,
    },
    ConflictingRaceClassInfo {
        skill_id: u16,
        first_record_id: u32,
        second_record_id: u32,
    },
}

/// Production evidence for WDC4/SQL/removal composition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillStoreEffectiveLoadReportLikeCpp {
    pub skill_line_ability_wdc4_rows: usize,
    pub skill_line_ability_official_sql_rows: usize,
    pub skill_line_ability_custom_sql_rows: usize,
    pub skill_line_ability_removed_rows: usize,
    /// Final identities after overlays and removals, including invalid payload.
    pub skill_line_ability_effective_rows: usize,
    pub skill_line_ability_indexed_rows: usize,
    pub skill_line_ability_invalid_rows: usize,
    pub skill_race_class_info_wdc4_rows: usize,
    pub skill_race_class_info_official_sql_rows: usize,
    pub skill_race_class_info_custom_sql_rows: usize,
    pub skill_race_class_info_removed_rows: usize,
    /// Final identities after overlays and removals, including invalid payload.
    pub skill_race_class_info_effective_rows: usize,
    pub skill_race_class_info_indexed_rows: usize,
    pub skill_race_class_info_invalid_rows: usize,
    pub skill_race_class_info_missing_skill_line_rows: usize,
    pub diagnostics_in_record_order_like_cpp: Vec<SkillStoreLoadDiagnosticLikeCpp>,
}

pub struct SkillStoreEffectiveLoadOutcomeLikeCpp {
    pub store: SkillStore,
    pub report: SkillStoreEffectiveLoadReportLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillLineAbilityCoverageLikeCpp<'a> {
    CoveredZero,
    Rows(&'a [SkillLineAbilityRecord]),
    Indeterminate(&'a [SkillStoreLoadDiagnosticLikeCpp]),
}

/// Rank-specific projection of every final effective
/// `SkillLineAbility` identity.
///
/// C++ `SpellMgr::LoadSpellRanks` reads only `Spell` and
/// `SupercedesSpell`. Keeping these endpoints independently from the richer
/// hydrated row preserves a valid rank edge when an unrelated field is
/// malformed, while retaining unrepresentable endpoints for fail-closed
/// chain coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillLineAbilityRankRowLikeCpp {
    Edge {
        record_id: u32,
        spell_id: u32,
        supercedes_spell_id: u32,
    },
    Indeterminate {
        record_id: u32,
        spell_raw: i128,
        supercedes_spell_raw: i128,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRaceClassInfoCoverageLikeCpp<'a> {
    CoveredZero,
    Rows(&'a [SkillRaceClassInfoRecord]),
    Indeterminate(&'a [SkillStoreLoadDiagnosticLikeCpp]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRaceClassInfoMatchCoverageLikeCpp<'a> {
    CoveredZero,
    Row(&'a SkillRaceClassInfoRecord),
    Indeterminate(&'a [SkillStoreLoadDiagnosticLikeCpp]),
}

#[derive(Debug, Clone)]
struct SkillLineAbilitySourceRecordLikeCpp {
    source: SkillStoreLoadSourceLikeCpp,
    id: u32,
    race_mask: i128,
    skill_line: i128,
    spell: i128,
    min_skill_line_rank: i128,
    class_mask: i128,
    supercedes_spell: i128,
    acquire_method: i128,
    trivial_rank_high: i128,
    trivial_rank_low: i128,
    flags: i128,
    num_skill_ups: i128,
    skillup_skill_line_id: i128,
}

#[derive(Debug, Clone)]
struct SkillRaceClassInfoSourceRecordLikeCpp {
    source: SkillStoreLoadSourceLikeCpp,
    id: u32,
    race_mask: i128,
    skill_id: i128,
    class_mask: i128,
    flags: i128,
    availability: i128,
    min_level: i128,
    skill_tier_id: i128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRangeTypeLikeCpp {
    Language,
    Level,
    Mono,
    Rank,
    None,
}

pub const MAX_SKILL_STEP_LIKE_CPP: usize = 16;

/// C++ `SkillTiersEntry`, loaded by `ObjectMgr::LoadSkillTiers` from `world.skill_tiers`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTiersEntryLikeCpp {
    pub id: u32,
    pub value: [u32; MAX_SKILL_STEP_LIKE_CPP],
}

impl SkillTiersEntryLikeCpp {
    /// C++ `SkillTiersEntry::GetValueForTierIndex`.
    pub fn get_value_for_tier_index_like_cpp(&self, mut tier_index: u32) -> u32 {
        if tier_index as usize >= MAX_SKILL_STEP_LIKE_CPP {
            tier_index = (MAX_SKILL_STEP_LIKE_CPP - 1) as u32;
        }

        while self.value[tier_index as usize] == 0 && tier_index > 0 {
            tier_index -= 1;
        }

        self.value[tier_index as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillTiersRowLikeCpp {
    pub id: u32,
    pub value: [u32; MAX_SKILL_STEP_LIKE_CPP],
}

/// Represented C++ `ObjectMgr::_skillTiers`.
#[derive(Debug, Clone, Default)]
pub struct SkillTiersStoreLikeCpp {
    tiers: HashMap<u32, SkillTiersEntryLikeCpp>,
}

impl SkillTiersStoreLikeCpp {
    pub fn from_rows_like_cpp(rows: impl IntoIterator<Item = SkillTiersRowLikeCpp>) -> Self {
        let mut tiers = HashMap::new();
        for row in rows {
            tiers.insert(
                row.id,
                SkillTiersEntryLikeCpp {
                    id: row.id,
                    value: row.value,
                },
            );
        }

        Self { tiers }
    }

    /// C++ `ObjectMgr::LoadSkillTiers`.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let stmt = db.prepare(WorldStatements::SEL_SKILL_TIERS);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                let mut value = [0u32; MAX_SKILL_STEP_LIKE_CPP];
                for (field_index, tier_value) in value.iter_mut().enumerate() {
                    *tier_value = result.read(1 + field_index);
                }

                rows.push(SkillTiersRowLikeCpp {
                    id: result.read(0),
                    value,
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        let store = Self::from_rows_like_cpp(rows);
        info!("Loaded {} skill max values", store.len());
        Ok(store)
    }

    /// C++ `ObjectMgr::GetSkillTier`.
    pub fn get_skill_tier_like_cpp(&self, skill_tier_id: u32) -> Option<&SkillTiersEntryLikeCpp> {
        self.tiers.get(&skill_tier_id)
    }

    pub fn len(&self) -> usize {
        self.tiers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiers.is_empty()
    }
}

/// A single skill slot entry for the player's SkillInfo update fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillInfoEntry {
    pub skill_id: u16,
    pub step: u16,
    pub rank: u16,
    pub starting_rank: u16,
    pub max_rank: u16,
    pub temp_bonus: i16,
    pub perm_bonus: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillRewardedSpellChangesLikeCpp {
    pub learn: Vec<i32>,
    pub remove: Vec<i32>,
}

/// Minimal C++ `SpellInfo` view used by `LoadPetLevelupSpellMap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetLevelupSpellInfoLikeCpp {
    pub id: u32,
    pub spell_level: u32,
}

/// Represented C++ `PetLevelupSpellSet` (`std::multimap<SpellLevel, SpellId>`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PetLevelupSpellSetLikeCpp {
    spells_by_level: BTreeMap<u32, Vec<u32>>,
    count: usize,
}

impl PetLevelupSpellSetLikeCpp {
    fn insert_like_cpp(&mut self, spell_level: u32, spell_id: u32) {
        self.spells_by_level
            .entry(spell_level)
            .or_default()
            .push(spell_id);
        self.count += 1;
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn len(&self) -> usize {
        self.count
    }

    /// Iterate like C++ `std::multimap`: ordered by level, preserving duplicates.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.spells_by_level
            .iter()
            .flat_map(|(level, spells)| spells.iter().map(move |spell| (*level, *spell)))
    }
}

/// Represented C++ `SpellMgr::mPetLevelupSpellMap`.
#[derive(Debug, Clone, Default)]
pub struct PetLevelupSpellStoreLikeCpp {
    spells_by_family: HashMap<u32, PetLevelupSpellSetLikeCpp>,
    count: usize,
}

impl PetLevelupSpellStoreLikeCpp {
    /// C++ `SpellMgr::LoadPetLevelupSpellMap`, represented without live `SpellMgr`.
    ///
    /// The callback is the future `GetSpellInfo(spell, DIFFICULTY_NONE)` seam.
    pub fn load_like_cpp(
        creature_families: impl IntoIterator<Item = CreatureFamilyEntry>,
        skill_store: &SkillStore,
        mut spell_info: impl FnMut(i32) -> Option<PetLevelupSpellInfoLikeCpp>,
    ) -> Self {
        let mut spells_by_family: HashMap<u32, PetLevelupSpellSetLikeCpp> = HashMap::new();
        let mut count = 0usize;

        for creature_family in creature_families {
            for skill_line in creature_family.skill_line {
                if skill_line <= 0 {
                    continue;
                }

                let Ok(skill_line) = u16::try_from(skill_line) else {
                    continue;
                };

                let Some(skill_line_abilities) =
                    skill_store.skill_line_abilities_by_skill_like_cpp(skill_line)
                else {
                    continue;
                };

                for skill_line_ability in skill_line_abilities {
                    if skill_line_ability.acquire_method
                        != SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP
                    {
                        continue;
                    }

                    let Some(spell) = spell_info(skill_line_ability.spell) else {
                        continue;
                    };

                    if spell.spell_level == 0 {
                        continue;
                    }

                    spells_by_family
                        .entry(creature_family.id)
                        .or_default()
                        .insert_like_cpp(spell.spell_level, spell.id);
                    count += 1;
                }
            }
        }

        Self {
            spells_by_family,
            count,
        }
    }

    /// C++ `SpellMgr::GetPetLevelupSpellList(petFamily)`.
    pub fn get_pet_levelup_spell_list_like_cpp(
        &self,
        pet_family: u32,
    ) -> Option<&PetLevelupSpellSetLikeCpp> {
        self.spells_by_family.get(&pet_family)
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn family_count(&self) -> usize {
        self.spells_by_family.len()
    }
}

/// Minimal C++ `SpellInfo` view used by `LoadPetFamilySpellsStore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetFamilySpellInfoLikeCpp {
    pub id: u32,
    pub is_passive: bool,
}

/// Minimal C++ `SpellLevelsEntry` view used by `LoadPetFamilySpellsStore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetFamilySpellLevelLikeCpp {
    pub spell_id: i32,
    pub difficulty_id: u32,
    pub spell_level: i16,
}

/// Represented C++ `PetFamilySpellsStore` (`std::map<uint32, std::set<uint32>>`).
#[derive(Debug, Clone, Default)]
pub struct PetFamilySpellStoreLikeCpp {
    spells_by_family: BTreeMap<u32, BTreeMap<u32, ()>>,
}

impl PetFamilySpellStoreLikeCpp {
    /// C++ `SpellMgr::LoadPetFamilySpellsStore`, represented without live `SpellMgr`.
    pub fn load_like_cpp(
        skill_store: &SkillStore,
        creature_families: impl IntoIterator<Item = CreatureFamilyEntry>,
        spell_levels: impl IntoIterator<Item = PetFamilySpellLevelLikeCpp>,
        mut spell_info: impl FnMut(i32) -> Option<PetFamilySpellInfoLikeCpp>,
    ) -> Self {
        let mut levels_by_spell = HashMap::new();
        for levels in spell_levels {
            if levels.difficulty_id == 0 {
                levels_by_spell.insert(levels.spell_id, levels);
            }
        }

        let creature_families: Vec<_> = creature_families.into_iter().collect();
        let mut spells_by_family: BTreeMap<u32, BTreeMap<u32, ()>> = BTreeMap::new();

        for skill_line in skill_store.skill_line_abilities_like_cpp() {
            let Some(spell_info) = spell_info(skill_line.spell) else {
                continue;
            };

            if levels_by_spell
                .get(&skill_line.spell)
                .is_some_and(|levels| levels.spell_level != 0)
            {
                continue;
            }

            if !spell_info.is_passive {
                continue;
            }

            for creature_family in &creature_families {
                if u16::try_from(creature_family.skill_line[0]).ok() != Some(skill_line.skill_line)
                    && u16::try_from(creature_family.skill_line[1]).ok()
                        != Some(skill_line.skill_line)
                {
                    continue;
                }

                if skill_line.acquire_method != SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP {
                    continue;
                }

                spells_by_family
                    .entry(creature_family.id)
                    .or_default()
                    .insert(spell_info.id, ());
            }
        }

        Self { spells_by_family }
    }

    pub fn get_pet_family_spells_like_cpp(&self, pet_family: u32) -> Option<Vec<u32>> {
        self.spells_by_family
            .get(&pet_family)
            .map(|spells| spells.keys().copied().collect())
    }

    pub fn family_count(&self) -> usize {
        self.spells_by_family.len()
    }

    pub fn spell_count(&self) -> usize {
        self.spells_by_family.values().map(BTreeMap::len).sum()
    }
}

pub const MAX_CREATURE_SPELL_DATA_SLOT_LIKE_CPP: usize = 4;

const SPELL_EFFECT_SUMMON_LIKE_CPP: u32 = 28;
const SPELL_EFFECT_SUMMON_PET_LIKE_CPP: u32 = 56;

/// Minimal C++ `CreatureTemplate` view used by `LoadPetDefaultSpells`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetDefaultSpellCreatureTemplateLikeCpp {
    pub entry: u32,
    pub family: u32,
    pub spells: [u32; MAX_CREATURE_SPELL_DATA_SLOT_LIKE_CPP],
}

/// Minimal C++ `SpellEffectInfo` view used by `LoadPetDefaultSpells`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetDefaultSpellEffectLikeCpp {
    pub effect: u32,
    pub misc_value: i32,
}

/// Minimal C++ `SpellInfo` view used by `LoadPetDefaultSpells`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetDefaultSpellInfoLikeCpp {
    pub difficulty_none: bool,
    pub effects: Vec<PetDefaultSpellEffectLikeCpp>,
}

/// C++ `PetDefaultSpellsEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetDefaultSpellsEntryLikeCpp {
    pub spellid: [u32; MAX_CREATURE_SPELL_DATA_SLOT_LIKE_CPP],
}

/// Represented C++ `SpellMgr::mPetDefaultSpellsMap`.
#[derive(Debug, Clone, Default)]
pub struct PetDefaultSpellStoreLikeCpp {
    default_spells_by_entry: HashMap<i32, PetDefaultSpellsEntryLikeCpp>,
}

impl PetDefaultSpellStoreLikeCpp {
    /// C++ `SpellMgr::LoadPetDefaultSpells`, represented without live `SpellMgr`.
    pub fn load_like_cpp(
        spell_infos: impl IntoIterator<Item = PetDefaultSpellInfoLikeCpp>,
        creature_templates: impl IntoIterator<Item = PetDefaultSpellCreatureTemplateLikeCpp>,
        pet_levelup_spells: &PetLevelupSpellStoreLikeCpp,
    ) -> Self {
        let creature_templates: HashMap<u32, PetDefaultSpellCreatureTemplateLikeCpp> =
            creature_templates
                .into_iter()
                .map(|template| (template.entry, template))
                .collect();
        let mut default_spells_by_entry = HashMap::new();

        for spell_info in spell_infos {
            if !spell_info.difficulty_none {
                continue;
            }

            for spell_effect in spell_info.effects {
                if spell_effect.effect != SPELL_EFFECT_SUMMON_LIKE_CPP
                    && spell_effect.effect != SPELL_EFFECT_SUMMON_PET_LIKE_CPP
                {
                    continue;
                }

                let creature_id = spell_effect.misc_value as u32;
                let Some(creature_template) = creature_templates.get(&creature_id) else {
                    continue;
                };

                let pet_spells_id = creature_template.entry as i32;
                if default_spells_by_entry.contains_key(&pet_spells_id) {
                    continue;
                }

                let mut pet_default_spells = PetDefaultSpellsEntryLikeCpp {
                    spellid: creature_template.spells,
                };

                if load_pet_default_spells_helper_like_cpp(
                    creature_template,
                    &mut pet_default_spells,
                    pet_levelup_spells,
                ) {
                    default_spells_by_entry.insert(pet_spells_id, pet_default_spells);
                }
            }
        }

        Self {
            default_spells_by_entry,
        }
    }

    /// C++ `SpellMgr::GetPetDefaultSpellsEntry(id)`.
    pub fn get_pet_default_spells_entry_like_cpp(
        &self,
        id: i32,
    ) -> Option<&PetDefaultSpellsEntryLikeCpp> {
        self.default_spells_by_entry.get(&id)
    }

    pub fn count(&self) -> usize {
        self.default_spells_by_entry.len()
    }
}

fn load_pet_default_spells_helper_like_cpp(
    creature_template: &PetDefaultSpellCreatureTemplateLikeCpp,
    pet_default_spells: &mut PetDefaultSpellsEntryLikeCpp,
    pet_levelup_spells: &PetLevelupSpellStoreLikeCpp,
) -> bool {
    if !pet_default_spells.spellid.iter().any(|spell| *spell != 0) {
        return false;
    }

    if creature_template.family != 0 {
        if let Some(levelup_spells) =
            pet_levelup_spells.get_pet_levelup_spell_list_like_cpp(creature_template.family)
        {
            for spell in &mut pet_default_spells.spellid {
                if *spell == 0 {
                    continue;
                }

                if levelup_spells
                    .iter()
                    .any(|(_, levelup_spell)| levelup_spell == *spell)
                {
                    *spell = 0;
                }
            }
        }
    }

    pet_default_spells.spellid.iter().any(|spell| *spell != 0)
}

// ── Store ───────────────────────────────────────────────────────────

/// In-memory store for auto-learned spells from DBC data.
pub struct SkillStore {
    /// C++ `sSkillLineAbilityStore` row iteration, kept in load order for represented loaders.
    abilities_like_cpp: Vec<SkillLineAbilityRecord>,
    /// SkillLineAbility records indexed by skill_line (the parent skill).
    abilities_by_skill: HashMap<u16, Vec<SkillLineAbilityRecord>>,
    /// C++ `SpellMgr::mSkillLineAbilityMap`, indexed by `SkillLineAbilityEntry::Spell`.
    abilities_by_spell_like_cpp: HashMap<i32, Vec<SkillLineAbilityRecord>>,
    /// SkillRaceClassInfo records indexed by (race, class).
    starting_skills: HashMap<(u8, u8), Vec<SkillRaceClassInfoRecord>>,
    /// C++ `_skillRaceClassInfoBySkill`, preserving DB2 iteration order.
    race_class_by_skill: HashMap<u16, Vec<SkillRaceClassInfoRecord>>,
    invalid_abilities_by_spell_like_cpp: HashMap<i32, Vec<SkillStoreLoadDiagnosticLikeCpp>>,
    invalid_abilities_by_skill_like_cpp: HashMap<u16, Vec<SkillStoreLoadDiagnosticLikeCpp>>,
    invalid_race_class_by_skill_like_cpp: HashMap<u16, Vec<SkillStoreLoadDiagnosticLikeCpp>>,
    rank_rows_like_cpp: Vec<SkillLineAbilityRankRowLikeCpp>,
    /// Total number of SkillLineAbility records loaded.
    total_abilities: usize,
    /// Total number of SkillRaceClassInfo records loaded.
    total_race_class: usize,
}

impl SkillStore {
    /// Build a minimal skill-line store for validation/tests.
    pub fn from_skill_lines_like_cpp(skill_ids: impl IntoIterator<Item = u16>) -> Self {
        Self {
            abilities_like_cpp: Vec::new(),
            abilities_by_skill: skill_ids
                .into_iter()
                .map(|skill_id| (skill_id, Vec::new()))
                .collect(),
            abilities_by_spell_like_cpp: HashMap::new(),
            starting_skills: HashMap::new(),
            race_class_by_skill: HashMap::new(),
            invalid_abilities_by_spell_like_cpp: HashMap::new(),
            invalid_abilities_by_skill_like_cpp: HashMap::new(),
            invalid_race_class_by_skill_like_cpp: HashMap::new(),
            rank_rows_like_cpp: Vec::new(),
            total_abilities: 0,
            total_race_class: 0,
        }
    }

    /// Build a represented C++ `sSkillLineAbilityStore` fixture.
    pub fn from_skill_line_abilities_like_cpp(
        abilities: impl IntoIterator<Item = SkillLineAbilityRecord>,
    ) -> Self {
        let mut abilities_by_skill: HashMap<u16, Vec<SkillLineAbilityRecord>> = HashMap::new();
        let mut abilities_by_spell_like_cpp: HashMap<i32, Vec<SkillLineAbilityRecord>> =
            HashMap::new();
        let mut abilities_like_cpp = Vec::new();
        let mut rank_rows_like_cpp = Vec::new();
        let mut total_abilities = 0usize;

        for ability in abilities {
            if let Some(rank_row) = skill_line_ability_rank_row_from_hydrated_like_cpp(&ability) {
                rank_rows_like_cpp.push(rank_row);
            }
            abilities_like_cpp.push(ability.clone());
            let skillup_skill_line = u16::try_from(ability.skillup_skill_line_id)
                .ok()
                .filter(|skill| *skill != 0)
                .unwrap_or(ability.skill_line);
            abilities_by_skill
                .entry(skillup_skill_line)
                .or_default()
                .push(ability.clone());
            abilities_by_spell_like_cpp
                .entry(ability.spell)
                .or_default()
                .push(ability);
            total_abilities += 1;
        }

        Self {
            abilities_like_cpp,
            abilities_by_skill,
            abilities_by_spell_like_cpp,
            starting_skills: HashMap::new(),
            race_class_by_skill: HashMap::new(),
            invalid_abilities_by_spell_like_cpp: HashMap::new(),
            invalid_abilities_by_skill_like_cpp: HashMap::new(),
            invalid_race_class_by_skill_like_cpp: HashMap::new(),
            rank_rows_like_cpp,
            total_abilities,
            total_race_class: 0,
        }
    }

    /// Build represented `SkillLineAbility` + `SkillRaceClassInfo` fixtures.
    pub fn from_skill_line_abilities_and_race_class_like_cpp(
        abilities: impl IntoIterator<Item = SkillLineAbilityRecord>,
        race_class_infos: impl IntoIterator<Item = SkillRaceClassInfoRecord>,
    ) -> Self {
        let mut store = Self::from_skill_line_abilities_like_cpp(abilities);
        let mut total_race_class = 0usize;
        for record in race_class_infos {
            store
                .race_class_by_skill
                .entry(record.skill_id)
                .or_default()
                .push(record.clone());

            if record.availability == 1 {
                for race in RACE_HUMAN_LIKE_CPP..MAX_RACES_LIKE_CPP {
                    if race_mask_for_race_like_cpp(race) == 0 {
                        continue;
                    }
                    if !matches_race(record.race_mask, race) {
                        continue;
                    }
                    for class in CLASS_WARRIOR_LIKE_CPP..MAX_CLASSES_LIKE_CPP {
                        if !matches_class(record.class_mask, class) {
                            continue;
                        }
                        store
                            .starting_skills
                            .entry((race, class))
                            .or_default()
                            .push(record.clone());
                    }
                }
            }

            total_race_class += 1;
        }
        store.total_race_class = total_race_class;
        store
    }

    /// Load both DB2 files from `{data_dir}/dbc/{locale}/`.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let dbc_dir = Path::new(data_dir).join("dbc").join(locale);

        // ── SkillLineAbility.db2 ──
        let sla_path = dbc_dir.join("SkillLineAbility.db2");
        let sla_reader = Wdc4Reader::open(&sla_path)
            .with_context(|| format!("failed to open {}", sla_path.display()))?;

        let mut abilities_by_skill: HashMap<u16, Vec<SkillLineAbilityRecord>> = HashMap::new();
        let mut abilities_by_spell_like_cpp: HashMap<i32, Vec<SkillLineAbilityRecord>> =
            HashMap::new();
        let mut abilities_like_cpp = Vec::new();
        let mut total_abilities = 0usize;

        for (id, idx) in sla_reader.iter_records() {
            // C++ `SkillLineAbilityEntry` / `SkillLineAbilityLoadInfo`
            // logical field order. `iter_records()` already returns the ID,
            // while physical field[1] also stores that C++ ID column.
            //  0: RaceMask (i64, 64 bits)
            //  1: ID (already returned by iter_records)
            //  2: SkillLine
            //  3: Spell
            //  4: MinSkillLineRank
            //  5: ClassMask
            //  6: SupercedesSpell
            //  7: AcquireMethod
            //  8: TrivialSkillLineRankHigh
            //  9: TrivialSkillLineRankLow
            // 10: Flags
            // 11+: remaining fields
            let skill_line = sla_reader.get_field_u16(idx, 2);
            let record = SkillLineAbilityRecord {
                id,
                race_mask: sla_reader.get_field_i64(idx, 0),
                skill_line,
                spell: sla_reader.get_field_i32(idx, 3),
                min_skill_line_rank: sla_reader.get_field_i16(idx, 4),
                class_mask: sla_reader.get_field_i32(idx, 5),
                supercedes_spell: sla_reader.get_field_i32(idx, 6),
                acquire_method: sla_reader.get_field_i8(idx, 7),
                trivial_rank_high: sla_reader.get_field_i16(idx, 8),
                trivial_rank_low: sla_reader.get_field_i16(idx, 9),
                flags: sla_reader.get_field_i8(idx, 10),
                num_skill_ups: sla_reader.get_field_i8(idx, 11),
                skillup_skill_line_id: sla_reader.get_field_i16(idx, 14),
            };
            abilities_like_cpp.push(record.clone());
            let skillup_skill_line = u16::try_from(record.skillup_skill_line_id)
                .ok()
                .filter(|skill| *skill != 0)
                .unwrap_or(skill_line);
            abilities_by_skill
                .entry(skillup_skill_line)
                .or_default()
                .push(record.clone());
            abilities_by_spell_like_cpp
                .entry(record.spell)
                .or_default()
                .push(record);
            total_abilities += 1;
        }

        let skill_count = abilities_by_skill.len();

        // ── SkillRaceClassInfo.db2 ──
        let srci_path = dbc_dir.join("SkillRaceClassInfo.db2");
        let srci_reader = Wdc4Reader::open(&srci_path)
            .with_context(|| format!("failed to open {}", srci_path.display()))?;

        // First pass: collect all records
        let mut all_records: Vec<SkillRaceClassInfoRecord> = Vec::new();
        for (id, idx) in srci_reader.iter_records() {
            // C++ `SkillRaceClassInfoEntry` logical field order
            // (`DB2Structure.h` / `SkillRaceClassInfoLoadInfo`):
            //  0: RaceMask (i64)
            //  1: SkillID (u16)
            //  2: ClassMask (i32)
            //  3: Flags (u16)
            //  4: Availability (i8)
            //  5: MinLevel (i8)
            //  6: SkillTierID (i16)
            let record = SkillRaceClassInfoRecord {
                id,
                race_mask: srci_reader.get_field_i64(idx, 0),
                skill_id: srci_reader.get_field_u16(idx, 1),
                class_mask: srci_reader.get_field_i32(idx, 2),
                flags: srci_reader.get_field_u16(idx, 3),
                availability: srci_reader.get_field_i8(idx, 4),
                min_level: srci_reader.get_field_i8(idx, 5),
                skill_tier_id: srci_reader.get_field_i16(idx, 6),
            };
            all_records.push(record);
        }

        let total_race_class = all_records.len();

        let mut race_class_by_skill: HashMap<u16, Vec<SkillRaceClassInfoRecord>> = HashMap::new();
        for record in &all_records {
            race_class_by_skill
                .entry(record.skill_id)
                .or_default()
                .push(record.clone());
        }

        // Index by (race, class) using the full C++ race/class enum ranges and
        // the non-contiguous RaceMask bit mapping.
        let mut starting_skills: HashMap<(u8, u8), Vec<SkillRaceClassInfoRecord>> = HashMap::new();
        for record in &all_records {
            // C++ `ObjectMgr::LoadPlayerInfo` only adds Availability == 1
            // records to `PlayerInfo::skills`.
            if record.availability == 1 {
                for race in RACE_HUMAN_LIKE_CPP..MAX_RACES_LIKE_CPP {
                    if race_mask_for_race_like_cpp(race) == 0 {
                        continue;
                    }
                    if !matches_race(record.race_mask, race) {
                        continue;
                    }
                    for class in CLASS_WARRIOR_LIKE_CPP..MAX_CLASSES_LIKE_CPP {
                        if !matches_class(record.class_mask, class) {
                            continue;
                        }
                        starting_skills
                            .entry((race, class))
                            .or_default()
                            .push(record.clone());
                    }
                }
            }
        }

        info!(
            "Loaded {} skill line abilities across {} skills, {} starting skill entries",
            total_abilities, skill_count, total_race_class
        );

        let rank_rows_like_cpp = abilities_like_cpp
            .iter()
            .filter_map(skill_line_ability_rank_row_from_hydrated_like_cpp)
            .collect();
        Ok(Self {
            abilities_like_cpp,
            abilities_by_skill,
            abilities_by_spell_like_cpp,
            starting_skills,
            race_class_by_skill,
            invalid_abilities_by_spell_like_cpp: HashMap::new(),
            invalid_abilities_by_skill_like_cpp: HashMap::new(),
            invalid_race_class_by_skill_like_cpp: HashMap::new(),
            rank_rows_like_cpp,
            total_abilities,
            total_race_class,
        })
    }

    /// Load the final effective C++ skill authority.
    ///
    /// Composition follows `DB2StorageBase::LoadFromDB` and
    /// `DB2Manager::LoadHotfixData`: WDC4, official SQL, custom SQL, then the
    /// final removal status. Unlike C++'s historical initialization order,
    /// every derived index is rebuilt only after removal.
    pub async fn load_effective_like_cpp(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
        removed_records: &Db2HotfixRemovalStoreLikeCpp,
        skill_line_store: &SkillLineStore,
    ) -> Result<SkillStoreEffectiveLoadOutcomeLikeCpp> {
        const SKILL_LINE_ABILITY_OVERLAY_SQL: &str = concat!(
            "SELECT RaceMask, ID, SkillLine, Spell, MinSkillLineRank, ClassMask, ",
            "SupercedesSpell, AcquireMethod, TrivialSkillLineRankHigh, ",
            "TrivialSkillLineRankLow, Flags, NumSkillUps, UniqueBit, ",
            "TradeSkillCategoryID, SkillupSkillLineID, CharacterPoints1, ",
            "CharacterPoints2 FROM skill_line_ability ",
            "WHERE (`VerifiedBuild` > 0) = ?"
        );
        const SKILL_RACE_CLASS_INFO_OVERLAY_SQL: &str = concat!(
            "SELECT ID, RaceMask, SkillID, ClassMask, Flags, Availability, ",
            "MinLevel, SkillTierID FROM skill_race_class_info ",
            "WHERE (`VerifiedBuild` > 0) = ?"
        );

        let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
        let sla_path = dbc_dir.join("SkillLineAbility.db2");
        let sla_reader = Wdc4Reader::open(&sla_path)
            .with_context(|| format!("failed to open {}", sla_path.display()))?;
        let sla_table_hash = sla_reader.table_hash();
        let base_abilities = sla_reader
            .iter_records()
            .map(|(id, idx)| skill_line_ability_source_from_wdc4_like_cpp(id, idx, &sla_reader))
            .collect::<Vec<_>>();

        let srci_path = dbc_dir.join("SkillRaceClassInfo.db2");
        let srci_reader = Wdc4Reader::open(&srci_path)
            .with_context(|| format!("failed to open {}", srci_path.display()))?;
        let srci_table_hash = srci_reader.table_hash();
        let base_race_class_infos = srci_reader
            .iter_records()
            .map(|(id, idx)| skill_race_class_info_source_from_wdc4_like_cpp(id, idx, &srci_reader))
            .collect::<Vec<_>>();

        let mut ability_overlay_batches = [Vec::new(), Vec::new()];
        let mut race_class_overlay_batches = [Vec::new(), Vec::new()];
        for (batch_index, (source, official)) in [
            (SkillStoreLoadSourceLikeCpp::OfficialSql, true),
            (SkillStoreLoadSourceLikeCpp::CustomSql, false),
        ]
        .into_iter()
        .enumerate()
        {
            let mut statement =
                hotfix_db.prepare(HotfixStatements::base(SKILL_LINE_ABILITY_OVERLAY_SQL));
            statement.set_bool(0, official);
            let mut result = hotfix_db
                .query(&statement)
                .await
                .context("failed to load SkillLineAbility.db2 SQL overlay")?;
            if !result.is_empty() {
                loop {
                    ability_overlay_batches[batch_index].push(
                        skill_line_ability_source_from_sql_like_cpp(&result, source)?,
                    );
                    if !result.next_row() {
                        break;
                    }
                }
            }

            let mut statement =
                hotfix_db.prepare(HotfixStatements::base(SKILL_RACE_CLASS_INFO_OVERLAY_SQL));
            statement.set_bool(0, official);
            let mut result = hotfix_db
                .query(&statement)
                .await
                .context("failed to load SkillRaceClassInfo.db2 SQL overlay")?;
            if !result.is_empty() {
                loop {
                    race_class_overlay_batches[batch_index].push(
                        skill_race_class_info_source_from_sql_like_cpp(&result, source)?,
                    );
                    if !result.next_row() {
                        break;
                    }
                }
            }
        }

        let [official_abilities, custom_abilities] = ability_overlay_batches;
        let [official_race_class_infos, custom_race_class_infos] = race_class_overlay_batches;
        let outcome = compose_effective_skill_store_like_cpp(
            base_abilities,
            official_abilities,
            custom_abilities,
            sla_table_hash,
            base_race_class_infos,
            official_race_class_infos,
            custom_race_class_infos,
            srci_table_hash,
            removed_records,
            skill_line_store,
        );

        info!(
            "Loaded {}/{} effective/indexed skill line abilities and {}/{} effective/indexed \
             race/class skill rows ({} diagnostics)",
            outcome.report.skill_line_ability_effective_rows,
            outcome.report.skill_line_ability_indexed_rows,
            outcome.report.skill_race_class_info_effective_rows,
            outcome.report.skill_race_class_info_indexed_rows,
            outcome.report.diagnostics_in_record_order_like_cpp.len()
        );
        Ok(outcome)
    }

    /// C++ `DB2Manager::GetSkillRaceClassInfo(skill, race, class)`, with a
    /// bounded fail-closed repair for overlapping rows whose acquisition
    /// payloads disagree. C++ returns whichever `unordered_multimap` entry is
    /// visited first in that corrupt/ambiguous case.
    pub fn skill_race_class_info_like_cpp(
        &self,
        skill_id: u16,
        race: u8,
        class: u8,
    ) -> Option<&SkillRaceClassInfoRecord> {
        match self.skill_race_class_info_coverage_for_player_like_cpp(skill_id, race, class) {
            SkillRaceClassInfoMatchCoverageLikeCpp::Row(record) => Some(record),
            SkillRaceClassInfoMatchCoverageLikeCpp::CoveredZero
            | SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(_) => None,
        }
    }

    /// Exact coverage for the C++ first-match race/class lookup.
    pub fn skill_race_class_info_coverage_for_player_like_cpp(
        &self,
        skill_id: u16,
        race: u8,
        class: u8,
    ) -> SkillRaceClassInfoMatchCoverageLikeCpp<'_> {
        if let Some(diagnostics) = self.invalid_race_class_by_skill_like_cpp.get(&skill_id) {
            return SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(diagnostics);
        }

        let Some(records) = self.race_class_by_skill.get(&skill_id) else {
            return SkillRaceClassInfoMatchCoverageLikeCpp::CoveredZero;
        };
        let mut candidates = records.iter().filter(|record| {
            (record.race_mask == 0 || matches_race(record.race_mask, race))
                && (record.class_mask == 0 || matches_class(record.class_mask, class))
        });
        let Some(first) = candidates.next() else {
            return SkillRaceClassInfoMatchCoverageLikeCpp::CoveredZero;
        };
        if candidates.any(|candidate| !same_race_class_payload_like_cpp(first, candidate)) {
            let diagnostics = self
                .invalid_race_class_by_skill_like_cpp
                .get(&skill_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            return SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(diagnostics);
        }
        SkillRaceClassInfoMatchCoverageLikeCpp::Row(first)
    }

    /// C++ free function `GetSkillRangeType(SkillRaceClassInfoEntry const*)`.
    pub fn skill_range_type_like_cpp(
        &self,
        rc_info: &SkillRaceClassInfoRecord,
        skill_line_store: &SkillLineStore,
        skill_tiers_store: &SkillTiersStoreLikeCpp,
    ) -> SkillRangeTypeLikeCpp {
        let SkillLineAcquisitionPayloadLikeCpp::Complete(skill) =
            skill_line_store.acquisition_payload_like_cpp(u32::from(rc_info.skill_id))
        else {
            return SkillRangeTypeLikeCpp::None;
        };

        if u32::try_from(rc_info.skill_tier_id)
            .ok()
            .and_then(|skill_tier_id| skill_tiers_store.get_skill_tier_like_cpp(skill_tier_id))
            .is_some()
        {
            return SkillRangeTypeLikeCpp::Rank;
        }

        if rc_info.skill_id == SKILL_RUNEFORGING_LIKE_CPP {
            return SkillRangeTypeLikeCpp::Mono;
        }

        match skill.category_id {
            SKILL_CATEGORY_ARMOR_LIKE_CPP => SkillRangeTypeLikeCpp::Mono,
            SKILL_CATEGORY_LANGUAGES_LIKE_CPP => SkillRangeTypeLikeCpp::Language,
            _ => SkillRangeTypeLikeCpp::Level,
        }
    }

    /// C++ `Player::LearnDefaultSkills` -> `Player::LearnDefaultSkill`.
    pub fn default_starting_skill_info_like_cpp(
        &self,
        race: u8,
        class: u8,
        level: u8,
        skill_line_store: &SkillLineStore,
        skill_tiers_store: &SkillTiersStoreLikeCpp,
    ) -> Vec<SkillInfoEntry> {
        let skills = match self.starting_skills.get(&(race, class)) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut entries: Vec<SkillInfoEntry> = Vec::new();
        let mut seen_skills: std::collections::HashSet<u16> = std::collections::HashSet::new();

        for skill_info in skills {
            let skill_id = skill_info.skill_id;
            if skill_id == 0
                || i16::from(skill_info.min_level) > i16::from(level)
                || seen_skills.contains(&skill_id)
            {
                continue;
            }

            let Some(resolved_skill_info) =
                self.skill_race_class_info_like_cpp(skill_id, race, class)
            else {
                seen_skills.insert(skill_id);
                continue;
            };
            if resolved_skill_info.id != skill_info.id {
                continue;
            }

            let Some(entry) = self.default_skill_info_like_cpp(
                skill_info,
                class,
                level,
                skill_line_store,
                skill_tiers_store,
            ) else {
                continue;
            };
            seen_skills.insert(skill_id);
            entries.push(entry);

            if entries.len() >= 256 {
                break;
            }
        }

        entries
    }

    fn default_skill_info_like_cpp(
        &self,
        rc_info: &SkillRaceClassInfoRecord,
        class: u8,
        level: u8,
        skill_line_store: &SkillLineStore,
        skill_tiers_store: &SkillTiersStoreLikeCpp,
    ) -> Option<SkillInfoEntry> {
        let max_for_level = u16::from(level).saturating_mul(5);
        let (step, rank, max_rank) =
            match self.skill_range_type_like_cpp(rc_info, skill_line_store, skill_tiers_store) {
                SkillRangeTypeLikeCpp::Language => (0, 300, 300),
                SkillRangeTypeLikeCpp::Level => {
                    let rank = if rc_info.flags & SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                        max_for_level
                    } else if class == CLASS_DEATH_KNIGHT_LIKE_CPP {
                        u16::from(level.saturating_sub(1))
                            .saturating_mul(5)
                            .max(1)
                            .min(max_for_level)
                    } else {
                        1
                    };
                    (0, rank, max_for_level)
                }
                SkillRangeTypeLikeCpp::Mono => (0, 1, 1),
                SkillRangeTypeLikeCpp::Rank => {
                    let tier = u32::try_from(rc_info.skill_tier_id)
                        .ok()
                        .and_then(|id| skill_tiers_store.get_skill_tier_like_cpp(id))?;
                    let max_rank = u16::try_from(tier.get_value_for_tier_index_like_cpp(0))
                        .unwrap_or(u16::MAX);
                    let rank = if rc_info.flags & SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                        max_rank
                    } else if class == CLASS_DEATH_KNIGHT_LIKE_CPP {
                        u16::from(level.saturating_sub(1))
                            .saturating_mul(5)
                            .max(1)
                            .min(max_rank)
                    } else {
                        1
                    };
                    (1, rank, max_rank)
                }
                SkillRangeTypeLikeCpp::None => return None,
            };

        Some(SkillInfoEntry {
            skill_id: rc_info.skill_id,
            step,
            rank,
            starting_rank: 1,
            max_rank,
            temp_bonus: 0,
            perm_bonus: 0,
        })
    }

    /// C++ `Player::_LoadSkills` followed by `Player::UpdateSkillsForLevel`.
    pub fn loaded_skill_info_like_cpp(
        &self,
        skill_id: u16,
        race: u8,
        class: u8,
        level: u8,
        mut rank: u16,
        mut max_rank: u16,
        skill_line_store: &SkillLineStore,
        skill_tiers_store: &SkillTiersStoreLikeCpp,
    ) -> Option<SkillInfoEntry> {
        let rc_info = self.skill_race_class_info_like_cpp(skill_id, race, class)?;
        match self.skill_range_type_like_cpp(rc_info, skill_line_store, skill_tiers_store) {
            SkillRangeTypeLikeCpp::Language => {
                rank = 300;
                max_rank = 300;
            }
            SkillRangeTypeLikeCpp::Level => {
                max_rank = u16::from(level).saturating_mul(5);
                if rc_info.flags & SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                    rank = max_rank;
                }
            }
            SkillRangeTypeLikeCpp::Mono => {
                rank = 1;
                max_rank = 1;
            }
            SkillRangeTypeLikeCpp::Rank | SkillRangeTypeLikeCpp::None => {}
        }

        let step = match skill_line_store.acquisition_payload_like_cpp(u32::from(skill_id)) {
            SkillLineAcquisitionPayloadLikeCpp::Complete(skill)
                if matches!(
                    skill.category_id,
                    SKILL_CATEGORY_SECONDARY_LIKE_CPP | SKILL_CATEGORY_PROFESSION_LIKE_CPP
                ) =>
            // Pinned 3.4.3 C++ `Player::_LoadSkills` computes both secondary
            // and profession steps as `max / 75`. It does not reverse-map
            // custom `SkillTiersEntry::Value` rows on this load path.
            {
                max_rank / 75
            }
            _ => 0,
        };

        Some(SkillInfoEntry {
            skill_id,
            step,
            rank,
            starting_rank: 1,
            max_rank,
            temp_bonus: 0,
            perm_bonus: 0,
        })
    }

    /// Return the subset of `known_spells` that are abilities for `skill_id`.
    ///
    /// Used by the `ShowTradeSkill` handler to build the response recipe list.
    pub fn trade_skill_spells(&self, skill_id: u16, known_spells: &[i32]) -> Vec<i32> {
        let abilities = match self.abilities_by_skill.get(&skill_id) {
            Some(a) => a,
            None => return Vec::new(),
        };
        let ability_spell_set: std::collections::HashSet<i32> =
            abilities.iter().map(|a| a.spell).collect();
        known_spells
            .iter()
            .filter(|&&s| ability_spell_set.contains(&s))
            .copied()
            .collect()
    }

    /// Number of SkillLineAbility records loaded.
    pub fn ability_count(&self) -> usize {
        self.total_abilities
    }

    /// Number of distinct skills (unique skill_line IDs).
    pub fn skill_count(&self) -> usize {
        self.abilities_by_skill.len()
    }

    /// C++ `SpellMgr::GetSkillLineAbilityMapBounds(spell_id)`.
    pub fn get_skill_line_ability_map_bounds_like_cpp(
        &self,
        spell_id: i32,
    ) -> &[SkillLineAbilityRecord] {
        self.abilities_by_spell_like_cpp
            .get(&spell_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Acquisition-authority coverage for one spell's final
    /// `SkillLineAbility` rows.
    pub fn skill_line_ability_coverage_by_spell_like_cpp(
        &self,
        spell_id: i32,
    ) -> SkillLineAbilityCoverageLikeCpp<'_> {
        if let Some(diagnostics) = self.invalid_abilities_by_spell_like_cpp.get(&spell_id) {
            return SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics);
        }

        match self.abilities_by_spell_like_cpp.get(&spell_id) {
            Some(rows) => SkillLineAbilityCoverageLikeCpp::Rows(rows),
            None => SkillLineAbilityCoverageLikeCpp::CoveredZero,
        }
    }

    /// C++ `DB2Manager::GetSkillLineAbilitiesBySkill(skillId)`.
    pub fn skill_line_abilities_by_skill_like_cpp(
        &self,
        skill_id: u16,
    ) -> Option<&[SkillLineAbilityRecord]> {
        self.abilities_by_skill.get(&skill_id).map(Vec::as_slice)
    }

    /// Acquisition-authority coverage for one skill's final
    /// `SkillLineAbility` rows.
    pub fn skill_line_ability_coverage_by_skill_like_cpp(
        &self,
        skill_id: u16,
    ) -> SkillLineAbilityCoverageLikeCpp<'_> {
        if let Some(diagnostics) = self.invalid_abilities_by_skill_like_cpp.get(&skill_id) {
            return SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics);
        }

        match self.abilities_by_skill.get(&skill_id) {
            Some(rows) => SkillLineAbilityCoverageLikeCpp::Rows(rows),
            None => SkillLineAbilityCoverageLikeCpp::CoveredZero,
        }
    }

    /// Acquisition-authority coverage for one skill's final
    /// `SkillRaceClassInfo` rows.
    pub fn skill_race_class_info_coverage_by_skill_like_cpp(
        &self,
        skill_id: u16,
    ) -> SkillRaceClassInfoCoverageLikeCpp<'_> {
        if let Some(diagnostics) = self.invalid_race_class_by_skill_like_cpp.get(&skill_id) {
            return SkillRaceClassInfoCoverageLikeCpp::Indeterminate(diagnostics);
        }

        match self.race_class_by_skill.get(&skill_id) {
            Some(rows) => SkillRaceClassInfoCoverageLikeCpp::Rows(rows),
            None => SkillRaceClassInfoCoverageLikeCpp::CoveredZero,
        }
    }

    /// Represented C++ `Player::LearnSkillRewardedSpells`.
    pub fn skill_rewarded_spells_like_cpp<SpellLevels, QuestFallback>(
        &self,
        skill_id: u16,
        skill_value: u16,
        race: u8,
        class: u8,
        level: u8,
        spell_levels: SpellLevels,
        quest_fallback_allowed: QuestFallback,
    ) -> Vec<i32>
    where
        SpellLevels: FnMut(i32) -> Option<(u32, u32)>,
        QuestFallback: FnMut(i32) -> bool,
    {
        self.skill_rewarded_spell_changes_like_cpp(
            skill_id,
            skill_value,
            race,
            class,
            level,
            spell_levels,
            quest_fallback_allowed,
        )
        .learn
    }

    /// C++ `Player::LearnSkillRewardedSpells`, including the below-rank
    /// `RemoveSpell` branch for `LEARNED_ON_SKILL_VALUE` rows.
    pub fn skill_rewarded_spell_changes_like_cpp<SpellLevels, QuestFallback>(
        &self,
        skill_id: u16,
        skill_value: u16,
        race: u8,
        class: u8,
        level: u8,
        mut spell_levels: SpellLevels,
        mut quest_fallback_allowed: QuestFallback,
    ) -> SkillRewardedSpellChangesLikeCpp
    where
        SpellLevels: FnMut(i32) -> Option<(u32, u32)>,
        QuestFallback: FnMut(i32) -> bool,
    {
        let Some(abilities) = self.skill_line_abilities_by_skill_like_cpp(skill_id) else {
            return SkillRewardedSpellChangesLikeCpp::default();
        };

        let class_mask = 1i32 << (class as i32 - 1);
        let mut changes = SkillRewardedSpellChangesLikeCpp::default();
        for ability in abilities {
            let Some((base_level, spell_level)) = spell_levels(ability.spell) else {
                continue;
            };

            match ability.acquire_method {
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP
                | SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP => {}
                SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP => {
                    if (ability.flags
                        & SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP)
                        == 0
                        || !quest_fallback_allowed(ability.spell)
                    {
                        continue;
                    }
                }
                _ => continue,
            }

            if skill_id == SKILL_RIDING_LIKE_CPP
                && (ability.acquire_method != SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP
                    || ability.num_skill_ups != 1)
            {
                continue;
            }

            if !matches_race(ability.race_mask, race) {
                continue;
            }
            if ability.class_mask != 0 && (ability.class_mask & class_mask) == 0 {
                continue;
            }

            let required_level = base_level.max(spell_level);
            if required_level > u32::from(level) {
                continue;
            }

            if i32::from(skill_value) < i32::from(ability.min_skill_line_rank)
                && ability.acquire_method == SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP
            {
                if ability.spell > 0 {
                    changes.remove.push(ability.spell);
                }
            } else if ability.spell > 0 {
                changes.learn.push(ability.spell);
            }
        }

        changes
    }

    /// C++ `sSkillLineAbilityStore` full row iteration.
    pub fn skill_line_abilities_like_cpp(&self) -> &[SkillLineAbilityRecord] {
        &self.abilities_like_cpp
    }

    /// Final RecordID-ordered rank endpoints, including rows whose unrelated
    /// acquisition fields could not be hydrated.
    pub fn skill_line_ability_rank_rows_like_cpp(&self) -> &[SkillLineAbilityRankRowLikeCpp] {
        &self.rank_rows_like_cpp
    }

    /// Number of SkillRaceClassInfo records loaded.
    pub fn race_class_count(&self) -> usize {
        self.total_race_class
    }

    /// All effective candidates C++ could select for this skill/race/class.
    ///
    /// C++ returns the first entry from an `unordered_multimap`. Callers that
    /// authorize acquisition can use this complete, RecordID-ordered set to
    /// fail closed when overlapping rows disagree.
    pub fn skill_race_class_info_candidates_like_cpp(
        &self,
        skill_id: u16,
        race: u8,
        class: u8,
    ) -> Vec<&SkillRaceClassInfoRecord> {
        self.race_class_by_skill
            .get(&skill_id)
            .into_iter()
            .flatten()
            .filter(|record| {
                (record.race_mask == 0 || matches_race(record.race_mask, race))
                    && (record.class_mask == 0 || matches_class(record.class_mask, class))
            })
            .collect()
    }
}

fn skill_line_ability_source_from_wdc4_like_cpp(
    id: u32,
    record_idx: usize,
    reader: &Wdc4Reader,
) -> SkillLineAbilitySourceRecordLikeCpp {
    // Pinned 3.4.3 C++ declares `SkillLineAbilityEntry::SkillLine` and
    // `SkillupSkillLineID` as `int16` (`DB2Structure.h`) and marks both
    // `FT_SHORT` fields signed (`DB2LoadInfo.h`). The hotfix columns are
    // signed `smallint` too. Preserve that source domain here: raw `0x8000`
    // is `-32768`, not skill 32768.
    SkillLineAbilitySourceRecordLikeCpp {
        source: SkillStoreLoadSourceLikeCpp::Wdc4,
        id,
        race_mask: i128::from(reader.get_field_i64(record_idx, 0)),
        skill_line: i128::from(reader.get_field_i16(record_idx, 2)),
        spell: i128::from(reader.get_field_i32(record_idx, 3)),
        min_skill_line_rank: i128::from(reader.get_field_i16(record_idx, 4)),
        class_mask: i128::from(reader.get_field_i32(record_idx, 5)),
        supercedes_spell: i128::from(reader.get_field_i32(record_idx, 6)),
        acquire_method: i128::from(reader.get_field_i8(record_idx, 7)),
        trivial_rank_high: i128::from(reader.get_field_i16(record_idx, 8)),
        trivial_rank_low: i128::from(reader.get_field_i16(record_idx, 9)),
        flags: i128::from(reader.get_field_i8(record_idx, 10)),
        num_skill_ups: i128::from(reader.get_field_i8(record_idx, 11)),
        skillup_skill_line_id: i128::from(reader.get_field_i16(record_idx, 14)),
    }
}

fn skill_race_class_info_source_from_wdc4_like_cpp(
    id: u32,
    record_idx: usize,
    reader: &Wdc4Reader,
) -> SkillRaceClassInfoSourceRecordLikeCpp {
    // Pinned 3.4.3 C++ declares `SkillRaceClassInfoEntry::SkillID` as
    // `int16`; its DB2 load metadata and hotfix SQL `smallint` column are
    // signed as well. Do not reinterpret a negative source bit pattern as
    // an unsigned skill ID.
    SkillRaceClassInfoSourceRecordLikeCpp {
        source: SkillStoreLoadSourceLikeCpp::Wdc4,
        id,
        race_mask: i128::from(reader.get_field_i64(record_idx, 0)),
        skill_id: i128::from(reader.get_field_i16(record_idx, 1)),
        class_mask: i128::from(reader.get_field_i32(record_idx, 2)),
        flags: i128::from(reader.get_field_u16(record_idx, 3)),
        availability: i128::from(reader.get_field_i8(record_idx, 4)),
        min_level: i128::from(reader.get_field_i8(record_idx, 5)),
        skill_tier_id: i128::from(reader.get_field_i16(record_idx, 6)),
    }
}

fn skill_line_ability_source_from_sql_like_cpp(
    result: &SqlResult,
    source: SkillStoreLoadSourceLikeCpp,
) -> Result<SkillLineAbilitySourceRecordLikeCpp> {
    let id =
        read_sql_source_field_like_cpp(result, 1, "SkillLineAbility.ID").and_then(|value| {
            u32::try_from(value)
                .with_context(|| format!("SkillLineAbility SQL ID {value} is not u32"))
        })?;
    Ok(SkillLineAbilitySourceRecordLikeCpp {
        source,
        id,
        race_mask: read_sql_source_field_like_cpp(result, 0, "SkillLineAbility.RaceMask")?,
        skill_line: read_sql_source_field_like_cpp(result, 2, "SkillLineAbility.SkillLine")?,
        spell: read_sql_source_field_like_cpp(result, 3, "SkillLineAbility.Spell")?,
        min_skill_line_rank: read_sql_source_field_like_cpp(
            result,
            4,
            "SkillLineAbility.MinSkillLineRank",
        )?,
        class_mask: read_sql_source_field_like_cpp(result, 5, "SkillLineAbility.ClassMask")?,
        supercedes_spell: read_sql_source_field_like_cpp(
            result,
            6,
            "SkillLineAbility.SupercedesSpell",
        )?,
        acquire_method: read_sql_source_field_like_cpp(
            result,
            7,
            "SkillLineAbility.AcquireMethod",
        )?,
        trivial_rank_high: read_sql_source_field_like_cpp(
            result,
            8,
            "SkillLineAbility.TrivialSkillLineRankHigh",
        )?,
        trivial_rank_low: read_sql_source_field_like_cpp(
            result,
            9,
            "SkillLineAbility.TrivialSkillLineRankLow",
        )?,
        flags: read_sql_source_field_like_cpp(result, 10, "SkillLineAbility.Flags")?,
        num_skill_ups: read_sql_source_field_like_cpp(result, 11, "SkillLineAbility.NumSkillUps")?,
        skillup_skill_line_id: read_sql_source_field_like_cpp(
            result,
            14,
            "SkillLineAbility.SkillupSkillLineID",
        )?,
    })
}

fn skill_race_class_info_source_from_sql_like_cpp(
    result: &SqlResult,
    source: SkillStoreLoadSourceLikeCpp,
) -> Result<SkillRaceClassInfoSourceRecordLikeCpp> {
    let id =
        read_sql_source_field_like_cpp(result, 0, "SkillRaceClassInfo.ID").and_then(|value| {
            u32::try_from(value)
                .with_context(|| format!("SkillRaceClassInfo SQL ID {value} is not u32"))
        })?;
    Ok(SkillRaceClassInfoSourceRecordLikeCpp {
        source,
        id,
        race_mask: read_sql_source_field_like_cpp(result, 1, "SkillRaceClassInfo.RaceMask")?,
        skill_id: read_sql_source_field_like_cpp(result, 2, "SkillRaceClassInfo.SkillID")?,
        class_mask: read_sql_source_field_like_cpp(result, 3, "SkillRaceClassInfo.ClassMask")?,
        flags: read_sql_source_field_like_cpp(result, 4, "SkillRaceClassInfo.Flags")?,
        availability: read_sql_source_field_like_cpp(result, 5, "SkillRaceClassInfo.Availability")?,
        min_level: read_sql_source_field_like_cpp(result, 6, "SkillRaceClassInfo.MinLevel")?,
        skill_tier_id: read_sql_source_field_like_cpp(result, 7, "SkillRaceClassInfo.SkillTierID")?,
    })
}

fn read_sql_source_field_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<i128> {
    result
        .try_read::<i64>(column)
        .map(i128::from)
        .or_else(|| result.try_read::<u64>(column).map(i128::from))
        .or_else(|| result.try_read::<i32>(column).map(i128::from))
        .or_else(|| result.try_read::<u32>(column).map(i128::from))
        .or_else(|| result.try_read::<i16>(column).map(i128::from))
        .or_else(|| result.try_read::<u16>(column).map(i128::from))
        .or_else(|| result.try_read::<i8>(column).map(i128::from))
        .or_else(|| result.try_read::<u8>(column).map(i128::from))
        .with_context(|| format!("missing or non-integer {field} SQL column {column}"))
}

#[allow(clippy::too_many_arguments)]
fn compose_effective_skill_store_like_cpp(
    base_abilities: impl IntoIterator<Item = SkillLineAbilitySourceRecordLikeCpp>,
    official_abilities: impl IntoIterator<Item = SkillLineAbilitySourceRecordLikeCpp>,
    custom_abilities: impl IntoIterator<Item = SkillLineAbilitySourceRecordLikeCpp>,
    ability_table_hash: u32,
    base_race_class_infos: impl IntoIterator<Item = SkillRaceClassInfoSourceRecordLikeCpp>,
    official_race_class_infos: impl IntoIterator<Item = SkillRaceClassInfoSourceRecordLikeCpp>,
    custom_race_class_infos: impl IntoIterator<Item = SkillRaceClassInfoSourceRecordLikeCpp>,
    race_class_table_hash: u32,
    removed_records: &Db2HotfixRemovalStoreLikeCpp,
    skill_line_store: &SkillLineStore,
) -> SkillStoreEffectiveLoadOutcomeLikeCpp {
    let base_abilities = base_abilities.into_iter().collect::<Vec<_>>();
    let official_abilities = official_abilities.into_iter().collect::<Vec<_>>();
    let custom_abilities = custom_abilities.into_iter().collect::<Vec<_>>();
    let base_race_class_infos = base_race_class_infos.into_iter().collect::<Vec<_>>();
    let official_race_class_infos = official_race_class_infos.into_iter().collect::<Vec<_>>();
    let custom_race_class_infos = custom_race_class_infos.into_iter().collect::<Vec<_>>();

    let mut report = SkillStoreEffectiveLoadReportLikeCpp {
        skill_line_ability_wdc4_rows: base_abilities.len(),
        skill_line_ability_official_sql_rows: official_abilities.len(),
        skill_line_ability_custom_sql_rows: custom_abilities.len(),
        skill_race_class_info_wdc4_rows: base_race_class_infos.len(),
        skill_race_class_info_official_sql_rows: official_race_class_infos.len(),
        skill_race_class_info_custom_sql_rows: custom_race_class_infos.len(),
        ..SkillStoreEffectiveLoadReportLikeCpp::default()
    };

    let mut abilities_by_record_id = BTreeMap::new();
    for record in base_abilities
        .into_iter()
        .chain(official_abilities)
        .chain(custom_abilities)
    {
        abilities_by_record_id.insert(record.id, record);
    }
    let abilities_before_removal = abilities_by_record_id.len();
    abilities_by_record_id.retain(|record_id, _| {
        !record_removed_like_cpp(removed_records, ability_table_hash, *record_id)
    });
    report.skill_line_ability_removed_rows =
        abilities_before_removal - abilities_by_record_id.len();
    report.skill_line_ability_effective_rows = abilities_by_record_id.len();
    let rank_rows_like_cpp = abilities_by_record_id
        .values()
        .filter_map(skill_line_ability_rank_row_from_source_like_cpp)
        .collect::<Vec<_>>();

    let mut abilities = Vec::new();
    let mut invalid_abilities_by_spell_like_cpp =
        HashMap::<i32, Vec<SkillStoreLoadDiagnosticLikeCpp>>::new();
    let mut invalid_abilities_by_skill_like_cpp =
        HashMap::<u16, Vec<SkillStoreLoadDiagnosticLikeCpp>>::new();
    for record in abilities_by_record_id.into_values() {
        let spell_key = i32::try_from(record.spell).ok();
        let skill_key = skill_line_ability_skill_key_from_source_like_cpp(&record);
        let diagnostics_start = report.diagnostics_in_record_order_like_cpp.len();
        match skill_line_ability_from_source_like_cpp(
            record,
            &mut report.diagnostics_in_record_order_like_cpp,
        ) {
            Some(record) => abilities.push(record),
            None => {
                report.skill_line_ability_invalid_rows += 1;
                if let Some(spell_key) = spell_key {
                    invalid_abilities_by_spell_like_cpp
                        .entry(spell_key)
                        .or_default()
                        .extend_from_slice(
                            &report.diagnostics_in_record_order_like_cpp[diagnostics_start..],
                        );
                }
                if let Some(skill_key) = skill_key {
                    invalid_abilities_by_skill_like_cpp
                        .entry(skill_key)
                        .or_default()
                        .extend_from_slice(
                            &report.diagnostics_in_record_order_like_cpp[diagnostics_start..],
                        );
                }
            }
        }
    }
    report.skill_line_ability_indexed_rows = abilities.len();

    let mut race_class_by_record_id = BTreeMap::new();
    for record in base_race_class_infos
        .into_iter()
        .chain(official_race_class_infos)
        .chain(custom_race_class_infos)
    {
        race_class_by_record_id.insert(record.id, record);
    }
    let race_class_before_removal = race_class_by_record_id.len();
    race_class_by_record_id.retain(|record_id, _| {
        !record_removed_like_cpp(removed_records, race_class_table_hash, *record_id)
    });
    report.skill_race_class_info_removed_rows =
        race_class_before_removal - race_class_by_record_id.len();
    report.skill_race_class_info_effective_rows = race_class_by_record_id.len();

    let mut converted_race_class_infos = Vec::new();
    let mut invalid_race_class_by_skill_like_cpp =
        HashMap::<u16, Vec<SkillStoreLoadDiagnosticLikeCpp>>::new();
    for record in race_class_by_record_id.into_values() {
        let skill_key = i16::try_from(record.skill_id)
            .ok()
            .and_then(|value| u16::try_from(value).ok());
        let diagnostics_start = report.diagnostics_in_record_order_like_cpp.len();
        match skill_race_class_info_from_source_like_cpp(
            record,
            &mut report.diagnostics_in_record_order_like_cpp,
        ) {
            Some(record) => converted_race_class_infos.push(record),
            None => {
                report.skill_race_class_info_invalid_rows += 1;
                if let Some(skill_key) = skill_key {
                    invalid_race_class_by_skill_like_cpp
                        .entry(skill_key)
                        .or_default()
                        .extend_from_slice(
                            &report.diagnostics_in_record_order_like_cpp[diagnostics_start..],
                        );
                }
            }
        }
    }

    let mut race_class_infos = Vec::new();
    for record in converted_race_class_infos {
        if skill_line_store.contains_effective_record_like_cpp(u32::from(record.skill_id)) {
            race_class_infos.push(record);
            continue;
        }

        let diagnostic = SkillStoreLoadDiagnosticLikeCpp::MissingEffectiveSkillLine {
            record_id: record.id,
            skill_id: record.skill_id,
        };
        report.skill_race_class_info_missing_skill_line_rows += 1;
        report
            .diagnostics_in_record_order_like_cpp
            .push(diagnostic.clone());
        invalid_race_class_by_skill_like_cpp
            .entry(record.skill_id)
            .or_default()
            .push(diagnostic);
    }
    report.skill_race_class_info_indexed_rows = race_class_infos.len();

    let conflict_diagnostics_start = report.diagnostics_in_record_order_like_cpp.len();
    append_conflicting_race_class_diagnostics_like_cpp(
        &race_class_infos,
        &mut report.diagnostics_in_record_order_like_cpp,
    );
    for diagnostic in &report.diagnostics_in_record_order_like_cpp[conflict_diagnostics_start..] {
        let SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo { skill_id, .. } = diagnostic
        else {
            continue;
        };
        invalid_race_class_by_skill_like_cpp
            .entry(*skill_id)
            .or_default()
            .push(diagnostic.clone());
    }

    let mut store =
        SkillStore::from_skill_line_abilities_and_race_class_like_cpp(abilities, race_class_infos);
    store.invalid_abilities_by_spell_like_cpp = invalid_abilities_by_spell_like_cpp;
    store.invalid_abilities_by_skill_like_cpp = invalid_abilities_by_skill_like_cpp;
    store.invalid_race_class_by_skill_like_cpp = invalid_race_class_by_skill_like_cpp;
    store.rank_rows_like_cpp = rank_rows_like_cpp;
    SkillStoreEffectiveLoadOutcomeLikeCpp { store, report }
}

fn record_removed_like_cpp(
    removed_records: &Db2HotfixRemovalStoreLikeCpp,
    table_hash: u32,
    record_id: u32,
) -> bool {
    // C++ hotfix keys store the signed `RecordID` bit pattern even though DB2
    // storage exposes the ID as `uint32`.
    removed_records.contains_like_cpp(table_hash, record_id as i32)
}

fn skill_line_ability_rank_row_from_hydrated_like_cpp(
    record: &SkillLineAbilityRecord,
) -> Option<SkillLineAbilityRankRowLikeCpp> {
    skill_line_ability_rank_row_from_raw_like_cpp(
        record.id,
        i128::from(record.spell),
        i128::from(record.supercedes_spell),
    )
}

fn skill_line_ability_rank_row_from_source_like_cpp(
    record: &SkillLineAbilitySourceRecordLikeCpp,
) -> Option<SkillLineAbilityRankRowLikeCpp> {
    skill_line_ability_rank_row_from_raw_like_cpp(record.id, record.spell, record.supercedes_spell)
}

fn skill_line_ability_rank_row_from_raw_like_cpp(
    record_id: u32,
    spell_raw: i128,
    supercedes_spell_raw: i128,
) -> Option<SkillLineAbilityRankRowLikeCpp> {
    if i32::try_from(supercedes_spell_raw).ok() == Some(0) {
        return None;
    }

    // Both DB2 members are signed `int32`, but `LoadSpellRanks` passes them
    // to `GetSpellInfo(uint32)` and stores them in `std::map<uint32, uint32>`.
    // Preserve that defined modulo-2^32 conversion for every representable
    // source value; only a value outside C++'s `int32` domain is ambiguous.
    let spell_id = i32::try_from(spell_raw).ok().map(|value| value as u32);
    let supercedes_spell_id = i32::try_from(supercedes_spell_raw)
        .ok()
        .map(|value| value as u32);
    match (spell_id, supercedes_spell_id) {
        (Some(spell_id), Some(supercedes_spell_id)) => Some(SkillLineAbilityRankRowLikeCpp::Edge {
            record_id,
            spell_id,
            supercedes_spell_id,
        }),
        _ => Some(SkillLineAbilityRankRowLikeCpp::Indeterminate {
            record_id,
            spell_raw,
            supercedes_spell_raw,
        }),
    }
}

fn skill_line_ability_from_source_like_cpp(
    record: SkillLineAbilitySourceRecordLikeCpp,
    diagnostics: &mut Vec<SkillStoreLoadDiagnosticLikeCpp>,
) -> Option<SkillLineAbilityRecord> {
    // Source records retain SQL values in i128 so out-of-schema overlays can
    // be diagnosed. Enforce C++'s signed-i16 source domain first, then expose
    // only a nonnegative, validated identifier through the public u16 field.
    let skill_line = i16::try_from(record.skill_line)
        .ok()
        .and_then(|value| u16::try_from(value).ok());
    let skillup_skill_line_id = i16::try_from(record.skillup_skill_line_id)
        .ok()
        .filter(|value| *value >= 0);
    let (Some(skill_line), Some(skillup_skill_line_id)) = (skill_line, skillup_skill_line_id)
    else {
        diagnostics.push(
            SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                source: record.source,
                record_id: record.id,
                spell: record.spell,
                skill_line: record.skill_line,
                skillup_skill_line_id: record.skillup_skill_line_id,
            },
        );
        return None;
    };

    Some(SkillLineAbilityRecord {
        id: record.id,
        race_mask: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "RaceMask",
            record.race_mask,
            diagnostics,
        )?,
        skill_line,
        spell: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "Spell",
            record.spell,
            diagnostics,
        )?,
        min_skill_line_rank: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "MinSkillLineRank",
            record.min_skill_line_rank,
            diagnostics,
        )?,
        class_mask: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "ClassMask",
            record.class_mask,
            diagnostics,
        )?,
        supercedes_spell: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "SupercedesSpell",
            record.supercedes_spell,
            diagnostics,
        )?,
        acquire_method: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "AcquireMethod",
            record.acquire_method,
            diagnostics,
        )?,
        trivial_rank_high: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "TrivialSkillLineRankHigh",
            record.trivial_rank_high,
            diagnostics,
        )?,
        trivial_rank_low: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "TrivialSkillLineRankLow",
            record.trivial_rank_low,
            diagnostics,
        )?,
        flags: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "Flags",
            record.flags,
            diagnostics,
        )?,
        num_skill_ups: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillLineAbility,
            record.source,
            record.id,
            "NumSkillUps",
            record.num_skill_ups,
            diagnostics,
        )?,
        skillup_skill_line_id,
    })
}

fn skill_line_ability_skill_key_from_source_like_cpp(
    record: &SkillLineAbilitySourceRecordLikeCpp,
) -> Option<u16> {
    let skillup_skill_line_id = i16::try_from(record.skillup_skill_line_id).ok()?;
    if skillup_skill_line_id != 0 {
        return u16::try_from(skillup_skill_line_id).ok();
    }

    i16::try_from(record.skill_line)
        .ok()
        .and_then(|skill_line| u16::try_from(skill_line).ok())
}

fn skill_race_class_info_from_source_like_cpp(
    record: SkillRaceClassInfoSourceRecordLikeCpp,
    diagnostics: &mut Vec<SkillStoreLoadDiagnosticLikeCpp>,
) -> Option<SkillRaceClassInfoRecord> {
    // As above, the public u16 is a post-validation representation; it does
    // not widen C++ `SkillRaceClassInfoEntry::SkillID` beyond signed i16.
    let Some(skill_id) = i16::try_from(record.skill_id)
        .ok()
        .and_then(|value| u16::try_from(value).ok())
    else {
        diagnostics.push(
            SkillStoreLoadDiagnosticLikeCpp::InvalidSkillRaceClassInfoIdentifier {
                source: record.source,
                record_id: record.id,
                race_mask: record.race_mask,
                skill_id: record.skill_id,
                class_mask: record.class_mask,
            },
        );
        return None;
    };

    Some(SkillRaceClassInfoRecord {
        id: record.id,
        race_mask: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "RaceMask",
            record.race_mask,
            diagnostics,
        )?,
        skill_id,
        class_mask: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "ClassMask",
            record.class_mask,
            diagnostics,
        )?,
        flags: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "Flags",
            record.flags,
            diagnostics,
        )?,
        availability: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "Availability",
            record.availability,
            diagnostics,
        )?,
        min_level: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "MinLevel",
            record.min_level,
            diagnostics,
        )?,
        skill_tier_id: checked_source_field_like_cpp(
            SkillStoreTableLikeCpp::SkillRaceClassInfo,
            record.source,
            record.id,
            "SkillTierID",
            record.skill_tier_id,
            diagnostics,
        )?,
    })
}

fn checked_source_field_like_cpp<T>(
    table: SkillStoreTableLikeCpp,
    source: SkillStoreLoadSourceLikeCpp,
    record_id: u32,
    field: &'static str,
    value: i128,
    diagnostics: &mut Vec<SkillStoreLoadDiagnosticLikeCpp>,
) -> Option<T>
where
    T: TryFrom<i128>,
{
    match T::try_from(value) {
        Ok(value) => Some(value),
        Err(_) => {
            diagnostics.push(SkillStoreLoadDiagnosticLikeCpp::InvalidSourceField {
                table,
                source,
                record_id,
                field,
                value,
            });
            None
        }
    }
}

fn append_conflicting_race_class_diagnostics_like_cpp(
    records: &[SkillRaceClassInfoRecord],
    diagnostics: &mut Vec<SkillStoreLoadDiagnosticLikeCpp>,
) {
    for (index, first) in records.iter().enumerate() {
        for second in &records[index + 1..] {
            if first.skill_id != second.skill_id
                || !race_masks_overlap_like_cpp(first.race_mask, second.race_mask)
                || !class_masks_overlap_like_cpp(first.class_mask, second.class_mask)
            {
                continue;
            }

            if same_race_class_payload_like_cpp(first, second) {
                continue;
            }

            diagnostics.push(SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo {
                skill_id: first.skill_id,
                first_record_id: first.id,
                second_record_id: second.id,
            });
        }
    }
}

fn same_race_class_payload_like_cpp(
    first: &SkillRaceClassInfoRecord,
    second: &SkillRaceClassInfoRecord,
) -> bool {
    first.flags == second.flags
        && first.availability == second.availability
        && first.min_level == second.min_level
        && first.skill_tier_id == second.skill_tier_id
}

fn race_masks_overlap_like_cpp(first: i64, second: i64) -> bool {
    first == 0 || second == 0 || first & second != 0
}

fn class_masks_overlap_like_cpp(first: i32, second: i32) -> bool {
    matches!(first, -1 | 0) || matches!(second, -1 | 0) || first & second != 0
}

/// Check if a race matches a race mask. Mask of 0 means "all races".
fn matches_race(mask: i64, race: u8) -> bool {
    mask == 0 || (mask & race_mask_for_race_like_cpp(race)) != 0
}

/// C++ `Trinity::RaceMask::GetMaskForRace`.
///
/// Returns zero for IDs that are not player races in C++ `Races`.
pub fn race_mask_for_race_like_cpp(race: u8) -> i64 {
    let bit = match race {
        1..=11 | 22 | 24..=32 => Some(race - 1),
        34 => Some(11),
        35 => Some(12),
        36 => Some(13),
        37 => Some(14),
        52 => Some(16),
        70 => Some(15),
        _ => None,
    };
    bit.map(|bit| 1_i64 << bit).unwrap_or(0)
}

/// Check if a class matches a class mask. Mask of 0 means "all classes".
fn matches_class(mask: i32, class: u8) -> bool {
    if matches!(mask, -1 | 0) {
        return true;
    }
    if !(CLASS_WARRIOR_LIKE_CPP..MAX_CLASSES_LIKE_CPP).contains(&class) {
        return false;
    }
    mask & (1_i32 << (class - 1)) != 0
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const DATA_DIR: &str = "/home/server/woltk-server-core/Data";
    const LOCALE: &str = "esES";

    fn load_store() -> Option<SkillStore> {
        let path = Path::new(DATA_DIR)
            .join("dbc")
            .join(LOCALE)
            .join("SkillLineAbility.db2");
        if !path.exists() {
            eprintln!("Skipping test: SkillLineAbility.db2 not found");
            return None;
        }
        Some(SkillStore::load(DATA_DIR, LOCALE).expect("failed to load SkillStore"))
    }

    fn ability(id: u32, skill_line: u16, spell: i32) -> SkillLineAbilityRecord {
        SkillLineAbilityRecord {
            id,
            race_mask: 0,
            skill_line,
            spell,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 0,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        }
    }

    fn race_class_info(
        id: u32,
        skill_id: u16,
        flags: u16,
        availability: i8,
        min_level: i8,
        skill_tier_id: i16,
    ) -> SkillRaceClassInfoRecord {
        SkillRaceClassInfoRecord {
            id,
            race_mask: 1,
            skill_id,
            class_mask: 1,
            flags,
            availability,
            min_level,
            skill_tier_id,
        }
    }

    fn skill_line(id: u32, category_id: i8) -> crate::SkillLineEntry {
        crate::SkillLineEntry {
            id,
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id: 0,
            parent_tier_index: 0,
            flags: 0,
            spell_book_spell_id: 0,
        }
    }

    fn ability_source(
        record: SkillLineAbilityRecord,
        source: SkillStoreLoadSourceLikeCpp,
    ) -> SkillLineAbilitySourceRecordLikeCpp {
        SkillLineAbilitySourceRecordLikeCpp {
            source,
            id: record.id,
            race_mask: i128::from(record.race_mask),
            skill_line: i128::from(record.skill_line),
            spell: i128::from(record.spell),
            min_skill_line_rank: i128::from(record.min_skill_line_rank),
            class_mask: i128::from(record.class_mask),
            supercedes_spell: i128::from(record.supercedes_spell),
            acquire_method: i128::from(record.acquire_method),
            trivial_rank_high: i128::from(record.trivial_rank_high),
            trivial_rank_low: i128::from(record.trivial_rank_low),
            flags: i128::from(record.flags),
            num_skill_ups: i128::from(record.num_skill_ups),
            skillup_skill_line_id: i128::from(record.skillup_skill_line_id),
        }
    }

    fn race_class_source(
        record: SkillRaceClassInfoRecord,
        source: SkillStoreLoadSourceLikeCpp,
    ) -> SkillRaceClassInfoSourceRecordLikeCpp {
        SkillRaceClassInfoSourceRecordLikeCpp {
            source,
            id: record.id,
            race_mask: i128::from(record.race_mask),
            skill_id: i128::from(record.skill_id),
            class_mask: i128::from(record.class_mask),
            flags: i128::from(record.flags),
            availability: i128::from(record.availability),
            min_level: i128::from(record.min_level),
            skill_tier_id: i128::from(record.skill_tier_id),
        }
    }

    fn pet_ability(
        id: u32,
        skill_line: u16,
        spell: i32,
        acquire_method: i8,
    ) -> SkillLineAbilityRecord {
        SkillLineAbilityRecord {
            acquire_method,
            ..ability(id, skill_line, spell)
        }
    }

    fn creature_family(id: u32, skill_line: [i16; 2]) -> CreatureFamilyEntry {
        CreatureFamilyEntry {
            id,
            name: String::new(),
            min_scale: 0.0,
            min_scale_level: 0,
            max_scale: 0.0,
            max_scale_level: 0,
            pet_food_mask: 0,
            pet_talent_type: 0,
            category_enum_id: 0,
            icon_file_id: 0,
            skill_line,
        }
    }

    fn skill_tier_row(id: u32, value: [u32; MAX_SKILL_STEP_LIKE_CPP]) -> SkillTiersRowLikeCpp {
        SkillTiersRowLikeCpp { id, value }
    }

    #[test]
    fn signed_skill_identifier_0x8000_is_rejected_without_unsigned_reinterpretation() {
        let signed_raw_0x8000 = i128::from(i16::MIN);
        assert_eq!(u16::from_ne_bytes(i16::MIN.to_ne_bytes()), 0x8000);

        for source in [
            SkillStoreLoadSourceLikeCpp::Wdc4,
            SkillStoreLoadSourceLikeCpp::OfficialSql,
            SkillStoreLoadSourceLikeCpp::CustomSql,
        ] {
            let mut ability = ability_source(ability(1, 100, 1_000), source);
            ability.skill_line = signed_raw_0x8000;
            assert_eq!(
                skill_line_ability_skill_key_from_source_like_cpp(&ability),
                None,
                "the signed DB2/SQL bit pattern must not become skill 32768"
            );

            let mut diagnostics = Vec::new();
            assert!(skill_line_ability_from_source_like_cpp(ability, &mut diagnostics).is_none());
            assert_eq!(
                diagnostics,
                [
                    SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                        source,
                        record_id: 1,
                        spell: 1_000,
                        skill_line: signed_raw_0x8000,
                        skillup_skill_line_id: 0,
                    }
                ]
            );

            let mut race_class = race_class_source(race_class_info(2, 100, 0, 1, 0, 0), source);
            race_class.skill_id = signed_raw_0x8000;

            let mut diagnostics = Vec::new();
            assert!(
                skill_race_class_info_from_source_like_cpp(race_class, &mut diagnostics).is_none()
            );
            assert_eq!(
                diagnostics,
                [
                    SkillStoreLoadDiagnosticLikeCpp::InvalidSkillRaceClassInfoIdentifier {
                        source,
                        record_id: 2,
                        race_mask: 1,
                        skill_id: signed_raw_0x8000,
                        class_mask: 1,
                    }
                ]
            );
        }
    }

    #[test]
    fn effective_rank_projection_preserves_only_rank_fields_and_final_removals() {
        const ABILITY_TABLE_HASH: u32 = 0xA100_0001;
        const RACE_CLASS_TABLE_HASH: u32 = 0xB200_0002;
        let skill_lines =
            SkillLineStore::from_entries([skill_line(100, SKILL_CATEGORY_PROFESSION_LIKE_CPP)]);

        let mut unrelated_invalid =
            ability_source(ability(1, 100, 3), SkillStoreLoadSourceLikeCpp::CustomSql);
        unrelated_invalid.supercedes_spell = 2;
        unrelated_invalid.race_mask = i128::from(i64::MAX) + 1;

        let mut invalid_supercedes =
            ability_source(ability(2, 100, 4), SkillStoreLoadSourceLikeCpp::CustomSql);
        invalid_supercedes.supercedes_spell = i128::from(i32::MAX) + 1;

        let mut invalid_spell =
            ability_source(ability(3, 100, 5), SkillStoreLoadSourceLikeCpp::CustomSql);
        invalid_spell.spell = i128::from(i32::MAX) + 1;
        invalid_spell.supercedes_spell = 2;

        let mut one_signed_endpoint =
            ability_source(ability(4, 100, 6), SkillStoreLoadSourceLikeCpp::CustomSql);
        one_signed_endpoint.spell = i128::from(i32::MAX) + 1;
        one_signed_endpoint.supercedes_spell = -1;

        let mut no_rank =
            ability_source(ability(5, 100, 7), SkillStoreLoadSourceLikeCpp::CustomSql);
        no_rank.race_mask = i128::from(i64::MAX) + 1;

        let mut removed_rank =
            ability_source(ability(6, 100, 8), SkillStoreLoadSourceLikeCpp::CustomSql);
        removed_rank.supercedes_spell = 7;
        let removals =
            Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(ABILITY_TABLE_HASH, 6, 2)]);

        let mut signed_endpoints =
            ability_source(ability(7, 100, -2), SkillStoreLoadSourceLikeCpp::CustomSql);
        signed_endpoints.supercedes_spell = -1;

        let mut base_replaced_rank =
            ability_source(ability(8, 100, 20), SkillStoreLoadSourceLikeCpp::Wdc4);
        base_replaced_rank.supercedes_spell = 19;
        let mut official_replaced_rank = ability_source(
            ability(8, 100, 30),
            SkillStoreLoadSourceLikeCpp::OfficialSql,
        );
        official_replaced_rank.supercedes_spell = 29;
        let mut final_overlay_rank =
            ability_source(ability(8, 100, 40), SkillStoreLoadSourceLikeCpp::CustomSql);
        final_overlay_rank.supercedes_spell = 39;
        final_overlay_rank.race_mask = i128::from(i64::MAX) + 1;

        let mut no_representable_endpoint =
            ability_source(ability(9, 100, 50), SkillStoreLoadSourceLikeCpp::CustomSql);
        no_representable_endpoint.spell = i128::from(i32::MAX) + 1;
        no_representable_endpoint.supercedes_spell = i128::from(i32::MAX) + 2;

        let outcome = compose_effective_skill_store_like_cpp(
            [base_replaced_rank],
            [official_replaced_rank],
            [
                unrelated_invalid,
                invalid_supercedes,
                invalid_spell,
                one_signed_endpoint,
                no_rank,
                removed_rank,
                signed_endpoints,
                final_overlay_rank,
                no_representable_endpoint,
            ],
            ABILITY_TABLE_HASH,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            RACE_CLASS_TABLE_HASH,
            &removals,
            &skill_lines,
        );

        assert_eq!(
            outcome.store.skill_line_ability_rank_rows_like_cpp(),
            [
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 1,
                    spell_id: 3,
                    supercedes_spell_id: 2,
                },
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 2,
                    spell_raw: 4,
                    supercedes_spell_raw: i128::from(i32::MAX) + 1,
                },
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 3,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: 2,
                },
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 4,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: -1,
                },
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 7,
                    spell_id: u32::MAX - 1,
                    supercedes_spell_id: u32::MAX,
                },
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 8,
                    spell_id: 40,
                    supercedes_spell_id: 39,
                },
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 9,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: i128::from(i32::MAX) + 2,
                },
            ]
        );
        assert!(
            matches!(
                outcome
                    .store
                    .skill_line_ability_coverage_by_spell_like_cpp(3),
                SkillLineAbilityCoverageLikeCpp::Indeterminate(_)
            ),
            "the richer acquisition row remains invalid even though its rank edge is valid"
        );
    }

    #[test]
    fn effective_skill_store_composes_collisions_sql_only_rows_and_removals_by_record_id() {
        const ABILITY_TABLE_HASH: u32 = 0xA100_0001;
        const RACE_CLASS_TABLE_HASH: u32 = 0xB200_0002;
        let skill_lines = SkillLineStore::from_entries([
            skill_line(100, SKILL_CATEGORY_PROFESSION_LIKE_CPP),
            skill_line(200, SKILL_CATEGORY_SECONDARY_LIKE_CPP),
        ]);
        let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
            (ABILITY_TABLE_HASH, 30, 2),
            (ABILITY_TABLE_HASH, 40, 2),
            (ABILITY_TABLE_HASH, 40, 1),
            (RACE_CLASS_TABLE_HASH, 60, 2),
            (RACE_CLASS_TABLE_HASH, 55, 2),
            (RACE_CLASS_TABLE_HASH, 55, 1),
        ]);

        let outcome = compose_effective_skill_store_like_cpp(
            [
                ability_source(ability(20, 100, 1000), SkillStoreLoadSourceLikeCpp::Wdc4),
                ability_source(ability(10, 100, 900), SkillStoreLoadSourceLikeCpp::Wdc4),
            ],
            [
                ability_source(
                    ability(20, 100, 2000),
                    SkillStoreLoadSourceLikeCpp::OfficialSql,
                ),
                ability_source(
                    ability(30, 100, 3000),
                    SkillStoreLoadSourceLikeCpp::OfficialSql,
                ),
            ],
            [
                ability_source(
                    ability(20, 100, 4000),
                    SkillStoreLoadSourceLikeCpp::CustomSql,
                ),
                ability_source(
                    ability(40, 200, 5000),
                    SkillStoreLoadSourceLikeCpp::CustomSql,
                ),
            ],
            ABILITY_TABLE_HASH,
            [race_class_source(
                race_class_info(50, 100, 1, 1, 0, 0),
                SkillStoreLoadSourceLikeCpp::Wdc4,
            )],
            [
                race_class_source(
                    race_class_info(50, 100, 2, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::OfficialSql,
                ),
                race_class_source(
                    race_class_info(60, 100, 4, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::OfficialSql,
                ),
            ],
            [
                race_class_source(
                    race_class_info(50, 100, 8, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::CustomSql,
                ),
                race_class_source(
                    race_class_info(55, 200, 16, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::CustomSql,
                ),
            ],
            RACE_CLASS_TABLE_HASH,
            &removals,
            &skill_lines,
        );

        assert_eq!(
            outcome
                .store
                .skill_line_abilities_like_cpp()
                .iter()
                .map(|record| (record.id, record.spell))
                .collect::<Vec<_>>(),
            vec![(10, 900), (20, 4000), (40, 5000)],
            "custom replaces official/base, SQL-only survives, removed rows vanish, and final IDs sort"
        );
        assert_eq!(
            outcome
                .store
                .skill_race_class_info_candidates_like_cpp(100, 1, 1)
                .iter()
                .map(|record| (record.id, record.flags))
                .collect::<Vec<_>>(),
            vec![(50, 8)]
        );
        assert_eq!(
            outcome
                .store
                .skill_race_class_info_candidates_like_cpp(200, 1, 1)
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>(),
            vec![55]
        );
        assert_eq!(
            outcome.report,
            SkillStoreEffectiveLoadReportLikeCpp {
                skill_line_ability_wdc4_rows: 2,
                skill_line_ability_official_sql_rows: 2,
                skill_line_ability_custom_sql_rows: 2,
                skill_line_ability_removed_rows: 1,
                skill_line_ability_effective_rows: 3,
                skill_line_ability_indexed_rows: 3,
                skill_line_ability_invalid_rows: 0,
                skill_race_class_info_wdc4_rows: 1,
                skill_race_class_info_official_sql_rows: 2,
                skill_race_class_info_custom_sql_rows: 2,
                skill_race_class_info_removed_rows: 1,
                skill_race_class_info_effective_rows: 2,
                skill_race_class_info_indexed_rows: 2,
                skill_race_class_info_invalid_rows: 0,
                skill_race_class_info_missing_skill_line_rows: 0,
                diagnostics_in_record_order_like_cpp: Vec::new(),
            }
        );
    }

    #[test]
    fn final_invalid_overlay_replaces_stale_payload_and_valid_custom_can_repair_it() {
        const ABILITY_TABLE_HASH: u32 = 0xA100_0001;
        const RACE_CLASS_TABLE_HASH: u32 = 0xB200_0002;
        let skill_lines = SkillLineStore::from_entries([
            skill_line(100, SKILL_CATEGORY_PROFESSION_LIKE_CPP),
            skill_line(200, SKILL_CATEGORY_SECONDARY_LIKE_CPP),
        ]);

        let mut invalid_final_ability = ability_source(
            ability(1, 100, 1000),
            SkillStoreLoadSourceLikeCpp::CustomSql,
        );
        invalid_final_ability.skill_line = -1;
        let mut repaired_official_ability = ability_source(
            ability(2, 100, 2000),
            SkillStoreLoadSourceLikeCpp::OfficialSql,
        );
        repaired_official_ability.skill_line = -2;
        let mut invalid_skillup_ability = ability_source(
            ability(3, 100, 3000),
            SkillStoreLoadSourceLikeCpp::CustomSql,
        );
        invalid_skillup_ability.skillup_skill_line_id = -1;
        let mut invalid_payload_ability = ability_source(
            ability(4, 200, 2222),
            SkillStoreLoadSourceLikeCpp::CustomSql,
        );
        invalid_payload_ability.race_mask = i128::MAX;

        let mut invalid_final_race_class = race_class_source(
            race_class_info(10, 100, 0, 1, 0, 0),
            SkillStoreLoadSourceLikeCpp::CustomSql,
        );
        invalid_final_race_class.skill_id = -3;
        let mut repaired_official_race_class = race_class_source(
            race_class_info(11, 100, 0, 1, 0, 0),
            SkillStoreLoadSourceLikeCpp::OfficialSql,
        );
        repaired_official_race_class.skill_id = -4;

        let outcome = compose_effective_skill_store_like_cpp(
            [
                ability_source(ability(1, 100, 1000), SkillStoreLoadSourceLikeCpp::Wdc4),
                ability_source(ability(2, 100, 2000), SkillStoreLoadSourceLikeCpp::Wdc4),
            ],
            [repaired_official_ability],
            [
                invalid_final_ability,
                ability_source(
                    ability(2, 200, 2222),
                    SkillStoreLoadSourceLikeCpp::CustomSql,
                ),
                invalid_skillup_ability,
                invalid_payload_ability,
            ],
            ABILITY_TABLE_HASH,
            [
                race_class_source(
                    race_class_info(10, 100, 0, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::Wdc4,
                ),
                race_class_source(
                    race_class_info(11, 100, 0, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::Wdc4,
                ),
            ],
            [repaired_official_race_class],
            [
                invalid_final_race_class,
                race_class_source(
                    race_class_info(11, 200, 0, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::CustomSql,
                ),
            ],
            RACE_CLASS_TABLE_HASH,
            &Db2HotfixRemovalStoreLikeCpp::default(),
            &skill_lines,
        );

        assert_eq!(
            outcome
                .store
                .skill_line_abilities_like_cpp()
                .iter()
                .map(|record| (record.id, record.skill_line, record.spell))
                .collect::<Vec<_>>(),
            vec![(2, 200, 2222)],
            "an invalid final custom row must not uncover the stale WDC4 record"
        );
        assert!(matches!(
            outcome
                .store
                .skill_line_ability_coverage_by_spell_like_cpp(1000),
            SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics)
                if diagnostics.iter().any(|diagnostic| matches!(
                    diagnostic,
                    SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                        record_id: 1,
                        ..
                    }
                ))
        ));
        assert!(matches!(
            outcome
                .store
                .skill_line_ability_coverage_by_spell_like_cpp(2222),
            SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics)
                if diagnostics.iter().any(|diagnostic| matches!(
                    diagnostic,
                    SkillStoreLoadDiagnosticLikeCpp::InvalidSourceField {
                        record_id: 4,
                        field: "RaceMask",
                        ..
                    }
                ))
        ));
        assert!(matches!(
            outcome
                .store
                .skill_line_ability_coverage_by_skill_like_cpp(200),
            SkillLineAbilityCoverageLikeCpp::Indeterminate(diagnostics)
                if diagnostics.iter().any(|diagnostic| matches!(
                    diagnostic,
                    SkillStoreLoadDiagnosticLikeCpp::InvalidSourceField {
                        record_id: 4,
                        field: "RaceMask",
                        ..
                    }
                ))
        ));
        assert_eq!(
            outcome
                .store
                .skill_line_ability_coverage_by_spell_like_cpp(9999),
            SkillLineAbilityCoverageLikeCpp::CoveredZero
        );
        assert_eq!(
            outcome
                .store
                .skill_line_ability_coverage_by_skill_like_cpp(999),
            SkillLineAbilityCoverageLikeCpp::CoveredZero
        );
        assert_eq!(outcome.store.race_class_count(), 1);
        assert_eq!(
            outcome
                .store
                .skill_race_class_info_candidates_like_cpp(200, 1, 1)[0]
                .id,
            11
        );
        assert!(
            outcome
                .report
                .diagnostics_in_record_order_like_cpp
                .contains(
                    &SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                        source: SkillStoreLoadSourceLikeCpp::CustomSql,
                        record_id: 1,
                        spell: 1000,
                        skill_line: -1,
                        skillup_skill_line_id: 0,
                    }
                )
        );
        assert!(
            outcome
                .report
                .diagnostics_in_record_order_like_cpp
                .contains(
                    &SkillStoreLoadDiagnosticLikeCpp::InvalidSkillLineAbilityIdentifier {
                        source: SkillStoreLoadSourceLikeCpp::CustomSql,
                        record_id: 3,
                        spell: 3000,
                        skill_line: 100,
                        skillup_skill_line_id: -1,
                    }
                )
        );
        assert!(
            outcome
                .report
                .diagnostics_in_record_order_like_cpp
                .contains(
                    &SkillStoreLoadDiagnosticLikeCpp::InvalidSkillRaceClassInfoIdentifier {
                        source: SkillStoreLoadSourceLikeCpp::CustomSql,
                        record_id: 10,
                        race_mask: 1,
                        skill_id: -3,
                        class_mask: 1,
                    }
                )
        );
    }

    #[test]
    fn missing_skill_lines_and_conflicting_race_class_payloads_fail_closed_with_diagnostics() {
        let skill_lines =
            SkillLineStore::from_entries([skill_line(100, SKILL_CATEGORY_PROFESSION_LIKE_CPP)]);
        let outcome = compose_effective_skill_store_like_cpp(
            [],
            [],
            [],
            1,
            [
                race_class_source(
                    race_class_info(1, 999, 0, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::Wdc4,
                ),
                race_class_source(
                    race_class_info(2, 100, 0, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::Wdc4,
                ),
                race_class_source(
                    race_class_info(3, 100, 1, 1, 0, 0),
                    SkillStoreLoadSourceLikeCpp::Wdc4,
                ),
            ],
            [],
            [],
            2,
            &Db2HotfixRemovalStoreLikeCpp::default(),
            &skill_lines,
        );

        assert_eq!(outcome.store.race_class_count(), 2);
        assert!(matches!(
            outcome
                .store
                .skill_race_class_info_coverage_by_skill_like_cpp(999),
            SkillRaceClassInfoCoverageLikeCpp::Indeterminate(diagnostics)
                if diagnostics == [SkillStoreLoadDiagnosticLikeCpp::MissingEffectiveSkillLine {
                    record_id: 1,
                    skill_id: 999,
                }]
        ));
        assert!(matches!(
            outcome
                .store
                .skill_race_class_info_coverage_by_skill_like_cpp(100),
            SkillRaceClassInfoCoverageLikeCpp::Indeterminate(diagnostics)
                if diagnostics == [SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo {
                    skill_id: 100,
                    first_record_id: 2,
                    second_record_id: 3,
                }]
        ));
        assert!(matches!(
            outcome
                .store
                .skill_race_class_info_coverage_for_player_like_cpp(100, 1, 1),
            SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(diagnostics)
                if diagnostics == [SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo {
                    skill_id: 100,
                    first_record_id: 2,
                    second_record_id: 3,
                }]
        ));
        assert!(
            outcome
                .store
                .skill_race_class_info_like_cpp(100, 1, 1)
                .is_none(),
            "an unordered C++ first-match conflict must fail closed"
        );
        assert!(
            outcome
                .store
                .default_starting_skill_info_like_cpp(
                    1,
                    1,
                    80,
                    &skill_lines,
                    &SkillTiersStoreLikeCpp::default(),
                )
                .is_empty(),
            "the starting-skill consumer must not bypass the ambiguity guard"
        );
        assert!(
            !outcome.store.starting_skills.contains_key(&(1, 1))
                || outcome.store.starting_skills[&(1, 1)]
                    .iter()
                    .all(|record| record.skill_id != 999),
            "the #163 fail-closed hardening also excludes missing SkillLine rows from starting skills"
        );
        assert!(
            outcome
                .report
                .diagnostics_in_record_order_like_cpp
                .contains(
                    &SkillStoreLoadDiagnosticLikeCpp::MissingEffectiveSkillLine {
                        record_id: 1,
                        skill_id: 999,
                    }
                )
        );
        assert!(
            outcome
                .report
                .diagnostics_in_record_order_like_cpp
                .contains(&SkillStoreLoadDiagnosticLikeCpp::ConflictingRaceClassInfo {
                    skill_id: 100,
                    first_record_id: 2,
                    second_record_id: 3,
                })
        );
    }

    #[test]
    fn effective_indices_use_cpp_race_bits_and_full_race_class_ranges() {
        let mut race_52 = race_class_info(1, 100, 0, 1, 0, 0);
        race_52.race_mask = 1_i64 << 16;
        race_52.class_mask = 1_i32 << (13 - 1);
        let mut race_70 = race_class_info(2, 100, 0, 1, 0, 0);
        race_70.race_mask = 1_i64 << 15;
        race_70.class_mask = -1;
        let store =
            SkillStore::from_skill_line_abilities_and_race_class_like_cpp([], [race_52, race_70]);

        assert_eq!(
            store
                .skill_race_class_info_candidates_like_cpp(100, 52, 13)
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            store
                .skill_race_class_info_candidates_like_cpp(100, 70, 14)
                .iter()
                .map(|record| record.id)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert!(store.starting_skills.contains_key(&(52, 13)));
        assert!(store.starting_skills.contains_key(&(70, 14)));
        assert!(!store.starting_skills.contains_key(&(70, 15)));
    }

    #[test]
    fn removal_lookup_preserves_unsigned_record_id_bit_pattern_like_cpp() {
        let table_hash = 0xA100_0001;
        let removals =
            Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(table_hash, -1, 2)]);
        assert!(record_removed_like_cpp(&removals, table_hash, u32::MAX));
    }

    #[test]
    fn skill_rewarded_spells_match_cpp_filters() {
        let store = SkillStore::from_skill_line_abilities_like_cpp([
            SkillLineAbilityRecord {
                acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                ..ability(1, 756, 822)
            },
            SkillLineAbilityRecord {
                acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                ..ability(2, 756, 28877)
            },
            SkillLineAbilityRecord {
                acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
                min_skill_line_rank: 450,
                ..ability(3, 756, 999)
            },
            SkillLineAbilityRecord {
                acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                race_mask: 1,
                ..ability(4, 756, 1000)
            },
            SkillLineAbilityRecord {
                acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                class_mask: 1,
                ..ability(5, 756, 1001)
            },
            SkillLineAbilityRecord {
                acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                ..ability(6, 756, 1002)
            },
        ]);

        let spells = store.skill_rewarded_spells_like_cpp(
            756,
            400,
            10,
            5,
            80,
            |spell_id| match spell_id {
                1002 => Some((81, 81)),
                _ => Some((0, 0)),
            },
            |_| false,
        );

        assert_eq!(spells, vec![822, 28877]);

        let changes = store.skill_rewarded_spell_changes_like_cpp(
            756,
            400,
            10,
            5,
            80,
            |spell_id| match spell_id {
                1002 => Some((81, 81)),
                _ => Some((0, 0)),
            },
            |_| false,
        );
        assert_eq!(changes.learn, vec![822, 28877]);
        assert_eq!(
            changes.remove,
            vec![999],
            "C++ removes LEARNED_ON_SKILL_VALUE spells below MinSkillLineRank"
        );
    }

    #[test]
    fn skill_rewarded_spells_applies_cpp_riding_exception() {
        let store = SkillStore::from_skill_line_abilities_like_cpp([
            SkillLineAbilityRecord {
                acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
                num_skill_ups: 1,
                ..ability(1, SKILL_RIDING_LIKE_CPP, 333)
            },
            SkillLineAbilityRecord {
                acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                num_skill_ups: 0,
                ..ability(2, SKILL_RIDING_LIKE_CPP, 444)
            },
            SkillLineAbilityRecord {
                acquire_method: SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                num_skill_ups: 1,
                ..ability(3, SKILL_RIDING_LIKE_CPP, 555)
            },
        ]);

        let spells = store.skill_rewarded_spells_like_cpp(
            SKILL_RIDING_LIKE_CPP,
            1,
            1,
            1,
            80,
            |_| Some((0, 0)),
            |_| false,
        );

        assert_eq!(spells, vec![555]);
    }

    fn pet_default_template(
        entry: u32,
        family: u32,
        spells: [u32; MAX_CREATURE_SPELL_DATA_SLOT_LIKE_CPP],
    ) -> PetDefaultSpellCreatureTemplateLikeCpp {
        PetDefaultSpellCreatureTemplateLikeCpp {
            entry,
            family,
            spells,
        }
    }

    #[test]
    fn skill_tiers_store_replaces_duplicate_ids_like_cpp() {
        let store = SkillTiersStoreLikeCpp::from_rows_like_cpp([
            skill_tier_row(12, [75, 150, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            skill_tier_row(12, [1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        ]);

        assert_eq!(store.len(), 1);
        assert_eq!(
            store
                .get_skill_tier_like_cpp(12)
                .expect("duplicate ID should leave one C++ map entry")
                .value[5],
            6,
            "C++ _skillTiers[id] overwrites the existing entry for duplicate IDs"
        );
    }

    #[test]
    fn skill_tier_value_falls_back_to_previous_nonzero_like_cpp() {
        let tier = SkillTiersEntryLikeCpp {
            id: 1,
            value: [75, 150, 225, 0, 0, 0, 450, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };

        assert_eq!(tier.get_value_for_tier_index_like_cpp(0), 75);
        assert_eq!(tier.get_value_for_tier_index_like_cpp(3), 225);
        assert_eq!(tier.get_value_for_tier_index_like_cpp(6), 450);
    }

    #[test]
    fn default_skills_filter_availability_and_min_level_and_use_cpp_ranges() {
        let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            std::iter::empty(),
            [
                race_class_info(1, 100, 0, 1, 1, 0),
                race_class_info(2, 101, 0, 1, 1, 0),
                race_class_info(3, 102, SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP, 1, 1, 0),
                race_class_info(4, 103, 0, 1, 1, 0),
                race_class_info(5, 104, 0, 1, 1, 12),
                race_class_info(6, 105, 0, 0, 1, 0),
                race_class_info(7, 106, 0, 1, 11, 0),
            ],
        );
        let skill_lines = SkillLineStore::from_entries([
            skill_line(100, SKILL_CATEGORY_LANGUAGES_LIKE_CPP),
            skill_line(101, 0),
            skill_line(102, 0),
            skill_line(103, SKILL_CATEGORY_ARMOR_LIKE_CPP),
            skill_line(104, SKILL_CATEGORY_PROFESSION_LIKE_CPP),
            skill_line(105, 0),
            skill_line(106, 0),
        ]);
        let tiers = SkillTiersStoreLikeCpp::from_rows_like_cpp([skill_tier_row(
            12,
            [75, 150, 225, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )]);

        assert_eq!(
            store.default_starting_skill_info_like_cpp(1, 1, 10, &skill_lines, &tiers),
            vec![
                SkillInfoEntry {
                    skill_id: 100,
                    step: 0,
                    rank: 300,
                    starting_rank: 1,
                    max_rank: 300,
                    temp_bonus: 0,
                    perm_bonus: 0,
                },
                SkillInfoEntry {
                    skill_id: 101,
                    step: 0,
                    rank: 1,
                    starting_rank: 1,
                    max_rank: 50,
                    temp_bonus: 0,
                    perm_bonus: 0,
                },
                SkillInfoEntry {
                    skill_id: 102,
                    step: 0,
                    rank: 50,
                    starting_rank: 1,
                    max_rank: 50,
                    temp_bonus: 0,
                    perm_bonus: 0,
                },
                SkillInfoEntry {
                    skill_id: 103,
                    step: 0,
                    rank: 1,
                    starting_rank: 1,
                    max_rank: 1,
                    temp_bonus: 0,
                    perm_bonus: 0,
                },
                SkillInfoEntry {
                    skill_id: 104,
                    step: 1,
                    rank: 1,
                    starting_rank: 1,
                    max_rank: 75,
                    temp_bonus: 0,
                    perm_bonus: 0,
                },
            ],
            "C++ includes default skills even without abilities, but excludes Availability != 1 and future MinLevel rows"
        );
    }

    #[test]
    fn default_death_knight_skill_uses_level_minus_one_value_like_cpp() {
        let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            std::iter::empty(),
            [SkillRaceClassInfoRecord {
                class_mask: 1 << (CLASS_DEATH_KNIGHT_LIKE_CPP - 1),
                ..race_class_info(1, 200, 0, 1, 1, 0)
            }],
        );
        let skill_lines = SkillLineStore::from_entries([skill_line(200, 0)]);

        assert_eq!(
            store.default_starting_skill_info_like_cpp(
                1,
                CLASS_DEATH_KNIGHT_LIKE_CPP,
                58,
                &skill_lines,
                &SkillTiersStoreLikeCpp::default(),
            )[0]
            .rank,
            285
        );
    }

    #[test]
    fn loaded_skill_info_applies_cpp_fixed_ranges_and_steps() {
        let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            std::iter::empty(),
            [
                race_class_info(1, 300, 0, 0, 0, 0),
                race_class_info(2, 301, SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP, 0, 0, 0),
            ],
        );
        let skill_lines = SkillLineStore::from_entries([
            skill_line(300, SKILL_CATEGORY_LANGUAGES_LIKE_CPP),
            skill_line(301, SKILL_CATEGORY_SECONDARY_LIKE_CPP),
        ]);
        let tiers = SkillTiersStoreLikeCpp::default();

        assert_eq!(
            store.loaded_skill_info_like_cpp(300, 1, 1, 10, 12, 25, &skill_lines, &tiers),
            Some(SkillInfoEntry {
                skill_id: 300,
                step: 0,
                rank: 300,
                starting_rank: 1,
                max_rank: 300,
                temp_bonus: 0,
                perm_bonus: 0,
            })
        );
        assert_eq!(
            store.loaded_skill_info_like_cpp(301, 1, 1, 80, 12, 25, &skill_lines, &tiers),
            Some(SkillInfoEntry {
                skill_id: 301,
                step: 5,
                rank: 400,
                starting_rank: 1,
                max_rank: 400,
                temp_bonus: 0,
                perm_bonus: 0,
            })
        );
    }

    #[test]
    fn loaded_profession_step_uses_cpp_max_div_75_even_for_nonstandard_tier() {
        let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            std::iter::empty(),
            [race_class_info(1, 301, 0, 0, 0, 12)],
        );
        let skill_lines =
            SkillLineStore::from_entries([skill_line(301, SKILL_CATEGORY_PROFESSION_LIKE_CPP)]);
        let tiers = SkillTiersStoreLikeCpp::from_rows_like_cpp([skill_tier_row(
            12,
            [73, 181, 400, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )]);

        let loaded = store
            .loaded_skill_info_like_cpp(301, 1, 1, 80, 12, 400, &skill_lines, &tiers)
            .expect("the persisted profession is valid for this race/class");

        assert_eq!(
            loaded.step, 5,
            "pinned C++ _LoadSkills uses max / 75, not the matching custom tier index"
        );
    }

    #[test]
    fn loaded_zero_value_skill_is_retained_for_cpp_default_reactivation() {
        let store = SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            std::iter::empty(),
            [race_class_info(1, 301, 0, 0, 0, 0)],
        );
        let skill_lines =
            SkillLineStore::from_entries([skill_line(301, SKILL_CATEGORY_PROFESSION_LIKE_CPP)]);

        let loaded = store
            .loaded_skill_info_like_cpp(
                301,
                1,
                1,
                80,
                0,
                400,
                &skill_lines,
                &SkillTiersStoreLikeCpp::default(),
            )
            .expect("C++ _LoadSkills keeps the zero-valued status/update-field entry");

        assert_eq!(loaded.rank, 0);
        assert_eq!(loaded.max_rank, 400);
        assert_eq!(loaded.step, 5);
    }

    #[test]
    fn skill_tier_value_clamps_large_index_like_cpp() {
        let tier = SkillTiersEntryLikeCpp {
            id: 1,
            value: [75, 150, 225, 300, 375, 450, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };

        assert_eq!(tier.get_value_for_tier_index_like_cpp(99), 450);
    }

    fn summon_spell(
        difficulty_none: bool,
        effect: u32,
        misc_value: i32,
    ) -> PetDefaultSpellInfoLikeCpp {
        PetDefaultSpellInfoLikeCpp {
            difficulty_none,
            effects: vec![PetDefaultSpellEffectLikeCpp { effect, misc_value }],
        }
    }

    #[test]
    fn skill_line_ability_map_bounds_group_by_spell_like_cpp() {
        let store = SkillStore::from_skill_line_abilities_like_cpp([
            ability(1, 56, 585),
            ability(2, 56, 2050),
            ability(3, 78, 585),
        ]);

        let smite_bounds = store.get_skill_line_ability_map_bounds_like_cpp(585);
        assert_eq!(smite_bounds.len(), 2);
        assert_eq!(smite_bounds[0].id, 1);
        assert_eq!(smite_bounds[1].id, 3);
        assert_eq!(
            store
                .get_skill_line_ability_map_bounds_like_cpp(2050)
                .iter()
                .map(|ability| ability.id)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert!(
            store
                .get_skill_line_ability_map_bounds_like_cpp(999)
                .is_empty()
        );
    }

    #[test]
    fn skill_line_ability_map_bounds_preserve_cpp_multimap_duplicates() {
        let store = SkillStore::from_skill_line_abilities_like_cpp([
            ability(10, 100, 777),
            ability(11, 100, 777),
        ]);

        assert_eq!(
            store
                .get_skill_line_ability_map_bounds_like_cpp(777)
                .iter()
                .map(|ability| ability.id)
                .collect::<Vec<_>>(),
            vec![10, 11],
            "C++ mSkillLineAbilityMap is a multimap and preserves every inserted row"
        );
    }

    #[test]
    fn pet_levelup_spell_map_filters_like_cpp() {
        let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
            pet_ability(
                1,
                10,
                1000,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(2, 10, 1001, 1),
            pet_ability(
                3,
                10,
                1002,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                4,
                10,
                1003,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                5,
                20,
                2000,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
        ]);

        let store = PetLevelupSpellStoreLikeCpp::load_like_cpp(
            [creature_family(42, [10, 20]), creature_family(77, [0, 99])],
            &skill_store,
            |spell_id| match spell_id {
                1000 => Some(PetLevelupSpellInfoLikeCpp {
                    id: 1000,
                    spell_level: 4,
                }),
                1002 => None,
                1003 => Some(PetLevelupSpellInfoLikeCpp {
                    id: 1003,
                    spell_level: 0,
                }),
                2000 => Some(PetLevelupSpellInfoLikeCpp {
                    id: 2000,
                    spell_level: 7,
                }),
                _ => panic!("unexpected spell lookup {spell_id}"),
            },
        );

        let pet_family_42 = store
            .get_pet_levelup_spell_list_like_cpp(42)
            .expect("family should have levelup spells");
        assert_eq!(
            pet_family_42.iter().collect::<Vec<_>>(),
            vec![(4, 1000), (7, 2000)]
        );
        assert_eq!(store.count(), 2);
        assert_eq!(store.family_count(), 1);
        assert!(store.get_pet_levelup_spell_list_like_cpp(77).is_none());
    }

    #[test]
    fn pet_levelup_spell_map_orders_like_cpp_multimap_by_spell_level() {
        let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
            pet_ability(
                1,
                10,
                3000,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                2,
                10,
                3001,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                3,
                10,
                3002,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
        ]);

        let store = PetLevelupSpellStoreLikeCpp::load_like_cpp(
            [creature_family(42, [10, 0])],
            &skill_store,
            |spell_id| {
                let spell_level = match spell_id {
                    3000 => 20,
                    3001 => 10,
                    3002 => 20,
                    _ => unreachable!(),
                };
                Some(PetLevelupSpellInfoLikeCpp {
                    id: spell_id as u32,
                    spell_level,
                })
            },
        );

        assert_eq!(
            store
                .get_pet_levelup_spell_list_like_cpp(42)
                .expect("family should have levelup spells")
                .iter()
                .collect::<Vec<_>>(),
            vec![(10, 3001), (20, 3000), (20, 3002)],
            "C++ PetLevelupSpellSet is a multimap keyed by SpellLevel"
        );
    }

    #[test]
    fn pet_default_spells_loads_summon_templates_like_cpp() {
        let levelup_spells = PetLevelupSpellStoreLikeCpp::default();
        let store = PetDefaultSpellStoreLikeCpp::load_like_cpp(
            [
                summon_spell(true, SPELL_EFFECT_SUMMON_LIKE_CPP, 500),
                summon_spell(true, SPELL_EFFECT_SUMMON_PET_LIKE_CPP, 501),
                summon_spell(false, SPELL_EFFECT_SUMMON_LIKE_CPP, 502),
                summon_spell(true, 2, 503),
                summon_spell(true, SPELL_EFFECT_SUMMON_LIKE_CPP, 999),
            ],
            [
                pet_default_template(500, 0, [10, 0, 11, 0]),
                pet_default_template(501, 0, [20, 21, 0, 0]),
                pet_default_template(502, 0, [30, 0, 0, 0]),
                pet_default_template(503, 0, [40, 0, 0, 0]),
            ],
            &levelup_spells,
        );

        assert_eq!(store.count(), 2);
        assert_eq!(
            store
                .get_pet_default_spells_entry_like_cpp(500)
                .expect("summon creature template should be loaded")
                .spellid,
            [10, 0, 11, 0]
        );
        assert_eq!(
            store
                .get_pet_default_spells_entry_like_cpp(501)
                .expect("summon pet creature template should be loaded")
                .spellid,
            [20, 21, 0, 0]
        );
        assert!(store.get_pet_default_spells_entry_like_cpp(502).is_none());
        assert!(store.get_pet_default_spells_entry_like_cpp(503).is_none());
        assert!(store.get_pet_default_spells_entry_like_cpp(999).is_none());
    }

    #[test]
    fn pet_default_spells_removes_levelup_duplicates_like_cpp() {
        let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
            pet_ability(
                1,
                10,
                100,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                2,
                10,
                101,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
        ]);
        let levelup_spells = PetLevelupSpellStoreLikeCpp::load_like_cpp(
            [creature_family(7, [10, 0])],
            &skill_store,
            |spell_id| {
                Some(PetLevelupSpellInfoLikeCpp {
                    id: spell_id as u32,
                    spell_level: 1,
                })
            },
        );

        let store = PetDefaultSpellStoreLikeCpp::load_like_cpp(
            [summon_spell(true, SPELL_EFFECT_SUMMON_PET_LIKE_CPP, 500)],
            [pet_default_template(500, 7, [100, 999, 101, 0])],
            &levelup_spells,
        );

        assert_eq!(
            store
                .get_pet_default_spells_entry_like_cpp(500)
                .expect("non-levelup default spell keeps entry alive")
                .spellid,
            [0, 999, 0, 0]
        );
    }

    #[test]
    fn pet_default_spells_skips_empty_after_levelup_duplicate_removal_like_cpp() {
        let skill_store = SkillStore::from_skill_line_abilities_like_cpp([pet_ability(
            1,
            10,
            100,
            SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
        )]);
        let levelup_spells = PetLevelupSpellStoreLikeCpp::load_like_cpp(
            [creature_family(7, [10, 0])],
            &skill_store,
            |_| {
                Some(PetLevelupSpellInfoLikeCpp {
                    id: 100,
                    spell_level: 1,
                })
            },
        );

        let store = PetDefaultSpellStoreLikeCpp::load_like_cpp(
            [summon_spell(true, SPELL_EFFECT_SUMMON_PET_LIKE_CPP, 500)],
            [pet_default_template(500, 7, [100, 0, 0, 0])],
            &levelup_spells,
        );

        assert_eq!(store.count(), 0);
        assert!(store.get_pet_default_spells_entry_like_cpp(500).is_none());
    }

    #[test]
    fn pet_family_spells_store_filters_like_cpp() {
        let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
            pet_ability(
                1,
                10,
                100,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                2,
                20,
                101,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(3, 10, 102, 1),
            pet_ability(
                4,
                10,
                103,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                5,
                10,
                104,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                6,
                99,
                105,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
        ]);

        let store = PetFamilySpellStoreLikeCpp::load_like_cpp(
            &skill_store,
            [creature_family(7, [10, 20]), creature_family(8, [30, 0])],
            [
                PetFamilySpellLevelLikeCpp {
                    spell_id: 100,
                    difficulty_id: 0,
                    spell_level: 0,
                },
                PetFamilySpellLevelLikeCpp {
                    spell_id: 101,
                    difficulty_id: 1,
                    spell_level: 80,
                },
                PetFamilySpellLevelLikeCpp {
                    spell_id: 103,
                    difficulty_id: 0,
                    spell_level: 5,
                },
            ],
            |spell_id| match spell_id {
                100 | 101 | 103 | 105 => Some(PetFamilySpellInfoLikeCpp {
                    id: spell_id as u32,
                    is_passive: true,
                }),
                102 => Some(PetFamilySpellInfoLikeCpp {
                    id: 102,
                    is_passive: true,
                }),
                104 => Some(PetFamilySpellInfoLikeCpp {
                    id: 104,
                    is_passive: false,
                }),
                _ => None,
            },
        );

        assert_eq!(
            store.get_pet_family_spells_like_cpp(7),
            Some(vec![100, 101]),
            "difficulty-specific SpellLevels rows do not exclude the DIFFICULTY_NONE lookup"
        );
        assert_eq!(store.family_count(), 1);
        assert_eq!(store.spell_count(), 2);
        assert!(store.get_pet_family_spells_like_cpp(8).is_none());
    }

    #[test]
    fn pet_family_spells_store_deduplicates_and_orders_like_cpp_set() {
        let skill_store = SkillStore::from_skill_line_abilities_like_cpp([
            pet_ability(
                1,
                10,
                300,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                2,
                10,
                200,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            pet_ability(
                3,
                10,
                300,
                SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
        ]);

        let store = PetFamilySpellStoreLikeCpp::load_like_cpp(
            &skill_store,
            [creature_family(7, [10, 0])],
            [],
            |spell_id| {
                Some(PetFamilySpellInfoLikeCpp {
                    id: spell_id as u32,
                    is_passive: true,
                })
            },
        );

        assert_eq!(
            store.get_pet_family_spells_like_cpp(7),
            Some(vec![200, 300]),
            "C++ PetFamilySpellsSet is std::set<uint32>"
        );
    }

    #[test]
    fn test_load_skill_store() {
        let store = match load_store() {
            Some(s) => s,
            None => return,
        };
        assert!(
            store.ability_count() > 1000,
            "expected >1000 abilities, got {}",
            store.ability_count()
        );
        assert!(
            store.skill_count() > 100,
            "expected >100 skills, got {}",
            store.skill_count()
        );
        assert!(
            store.race_class_count() > 100,
            "expected >100 race/class entries, got {}",
            store.race_class_count()
        );
    }

    #[test]
    fn live_hunter_starting_skill_rewards_match_cpp_capture() {
        let Some(store) = load_store() else {
            return;
        };
        let spell_names =
            crate::SpellNameStore::load(DATA_DIR, LOCALE).expect("failed to load SpellNameStore");
        let spell_levels = crate::SpellLevelsStore::load(DATA_DIR, LOCALE)
            .expect("failed to load SpellLevelsStore");
        let mut learned = Vec::new();

        for (skill_id, skill_value) in [
            (45, 1),
            (51, 15),
            (95, 300),
            (109, 300),
            (137, 300),
            (162, 1),
            (163, 15),
            (172, 1),
            (173, 1),
            (183, 15),
            (414, 1),
            (415, 1),
            (756, 15),
            (777, 1),
        ] {
            learned.extend(
                store
                    .skill_rewarded_spell_changes_like_cpp(
                        skill_id,
                        skill_value,
                        10,
                        3,
                        3,
                        |spell_id| {
                            let spell_id = u32::try_from(spell_id).ok()?;
                            spell_names.get(spell_id)?;
                            spell_levels
                                .entry_for_spell_difficulty_like_cpp(spell_id, 0)
                                .map(|entry| {
                                    (
                                        u32::try_from(entry.base_level).unwrap_or(0),
                                        u32::try_from(entry.spell_level).unwrap_or(0),
                                    )
                                })
                                .or(Some((0, 0)))
                        },
                        |_| false,
                    )
                    .learn,
            );
        }

        learned.sort_unstable();
        learned.dedup();
        assert_eq!(
            learned,
            vec![
                75, 81, 197, 203, 204, 264, 522, 669, 813, 822, 1180, 2382, 2973, 3050, 3365, 6233,
                6246, 6247, 6477, 6478, 6603, 7266, 7267, 7355, 8386, 9077, 9078, 9125, 13358,
                21651, 21652, 22027, 22810, 24949, 28730, 28877, 34082, 45927, 61437, 63644, 63645,
                68398, 349794,
            ],
            "C++ Player::LearnSkillRewardedSpells trace for TESTBOT1 guid 14"
        );
    }

    #[test]
    fn test_matches_race() {
        assert!(matches_race(0, 1)); // mask=0 matches all
        assert!(matches_race(0, 5));
        assert!(matches_race(1, 1)); // bit 0 = race 1 (Human)
        assert!(!matches_race(1, 2)); // bit 0 only matches race 1
        assert!(matches_race(0b11, 2)); // bit 1 = race 2 (Orc)
    }

    #[test]
    fn test_matches_class() {
        assert!(matches_class(0, 1)); // mask=0 matches all
        assert!(matches_class(0, 9));
        assert!(matches_class(1, 1)); // bit 0 = class 1 (Warrior)
        assert!(!matches_class(1, 2)); // bit 0 only matches class 1
    }

    #[test]
    fn test_field_mapping_verified() {
        let dbc_dir = Path::new(DATA_DIR).join("dbc").join(LOCALE);
        let sla_path = dbc_dir.join("SkillLineAbility.db2");
        if !sla_path.exists() {
            eprintln!("Skipping: SkillLineAbility.db2 not found");
            return;
        }
        let sla = Wdc4Reader::open(&sla_path).unwrap();

        // Record 246: one-handed axes (spell=264) for skill line 45.
        // This DB2 has no external ID list: C++ reads its inline field[1].
        let idx = sla
            .get_record_index(246)
            .expect("inline SkillLineAbility ID 246 must be indexed");
        assert_eq!(
            sla.get_field_u32(idx, 1),
            246,
            "field[1] should be the inline record ID"
        );
        assert_eq!(
            sla.get_field_i32(idx, 2),
            45,
            "field[2] should be skill_line 45"
        );
        assert_eq!(
            sla.get_field_i32(idx, 3),
            264,
            "field[3] should be spell 264"
        );
    }
}
