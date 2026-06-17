// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SkillLineAbility.db2 + SkillRaceClassInfo.db2 reader.
//!
//! Determines which spells each race/class/level should auto-learn,
//! replicating C#'s `LearnDefaultSkills()` → `LearnSkillRewardedSpells()`.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_database::{WorldDatabase, WorldStatements};

use crate::entities_movement::CreatureFamilyEntry;
use crate::wdc4::Wdc4Reader;

// ── Records ─────────────────────────────────────────────────────────

/// A single record from SkillLineAbility.db2.
#[derive(Debug, Clone)]
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
}

/// A single record from SkillRaceClassInfo.db2.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Copy)]
pub struct SkillInfoEntry {
    pub skill_id: u16,
    pub step: u16,
    pub rank: u16,
    pub starting_rank: u16,
    pub max_rank: u16,
    pub temp_bonus: i16,
    pub perm_bonus: u16,
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
        let mut total_abilities = 0usize;

        for ability in abilities {
            abilities_like_cpp.push(ability.clone());
            abilities_by_skill
                .entry(ability.skill_line)
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
            total_abilities,
            total_race_class: 0,
        }
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
            // WDC4 field layout (empirically verified):
            // The WDC4 file has 16 fields; C# struct has 14. Field[1] is an
            // extra inline field (possibly id_parent duplicate), shifting all
            // subsequent fields by +1 compared to C# indices.
            //  0: RaceMask (i64, 64 bits)
            //  1: [extra field — skip]
            //  2: id_parent / SkillLine (11 bits signed) ← C# field 1
            //  3: Spell (20 bits signed)                 ← C# field 2
            //  4: MinSkillLineRank (10 bits signed)      ← C# field 3
            //  5: ClassMask (Common)                     ← C# field 4
            //  6: SupercedesSpell (Common)               ← C# field 5
            //  7: AcquireMethod (3 bits signed)          ← C# field 6
            //  8: TrivialSkillLineRankHigh (Common)      ← C# field 7
            //  9: TrivialSkillLineRankLow (Common)       ← C# field 8
            // 10: Flags (2 bits signed)                  ← C# field 9
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
            };
            abilities_like_cpp.push(record.clone());
            abilities_by_skill
                .entry(skill_line)
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
            // Field order from C# SkillRaceClassInfoRecord:
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

        // Index by (race, class) — expand masks into individual (race, class) pairs
        // for all 10 races × 11 classes
        let mut starting_skills: HashMap<(u8, u8), Vec<SkillRaceClassInfoRecord>> = HashMap::new();
        for record in &all_records {
            for race in 1u8..=11 {
                if !matches_race(record.race_mask, race) {
                    continue;
                }
                for class in 1u8..=11 {
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

        info!(
            "Loaded {} skill line abilities across {} skills, {} starting skill entries",
            total_abilities, skill_count, total_race_class
        );

        Ok(Self {
            abilities_like_cpp,
            abilities_by_skill,
            abilities_by_spell_like_cpp,
            starting_skills,
            total_abilities,
            total_race_class,
        })
    }

    /// Get the SkillInfo entries for a character's starting skills.
    ///
    /// Returns up to 256 entries matching C#'s `LearnDefaultSkills()` → `SetSkill()`.
    /// Each entry contains the skill ID, current rank, and max rank.
    pub fn starting_skill_info(&self, race: u8, class: u8, level: u8) -> Vec<SkillInfoEntry> {
        let max_rank = (level as u16) * 5;

        let skills = match self.starting_skills.get(&(race, class)) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut entries: Vec<SkillInfoEntry> = Vec::new();
        let mut seen_skills: std::collections::HashSet<u16> = std::collections::HashSet::new();

        for skill_info in skills {
            let skill_id = skill_info.skill_id;
            if skill_id == 0 || !seen_skills.insert(skill_id) {
                continue;
            }
            // Only include skills that have at least one ability for this race/class
            let has_abilities = self.abilities_by_skill.get(&skill_id).is_some();
            if !has_abilities {
                continue;
            }

            entries.push(SkillInfoEntry {
                skill_id,
                step: 0,
                rank: max_rank,
                starting_rank: 1,
                max_rank,
                temp_bonus: 0,
                perm_bonus: 0,
            });

            if entries.len() >= 256 {
                break;
            }
        }

        entries
    }

    /// Get all spells that a character of the given race/class/level should
    /// automatically know from DBC data (LearnDefaultSkills + LearnSkillRewardedSpells).
    ///
    /// `known_skill_ids` is the set of skill IDs from the character's `character_skills`
    /// table. When provided, only skills that are either class-specific (exactly one class
    /// bit in the SkillRaceClassInfo class_mask matching this class) or present in the
    /// character's known skills will be processed. This matches C# behavior where
    /// `LearnSkillRewardedSpells()` is only called for skills the character actually has.
    ///
    /// Pass `None` to disable filtering (useful for tests / backward compat).
    ///
    /// Returns a deduplicated Vec of spell IDs.
    pub fn starting_spells(
        &self,
        race: u8,
        class: u8,
        level: u8,
        known_skill_ids: Option<&std::collections::HashSet<u16>>,
    ) -> Vec<i32> {
        let mut spells: Vec<i32> = Vec::new();
        let mut seen: std::collections::HashSet<i32> = std::collections::HashSet::new();

        // C# Player.GetMaxSkillValueForLevel() = level * 5
        let max_skill_rank = (level as i16) * 5;

        // Get starting skills for this race/class combination
        let skills = match self.starting_skills.get(&(race, class)) {
            Some(s) => s,
            None => return spells,
        };

        for skill_info in skills {
            let skill_id = skill_info.skill_id;

            // Skip purely racial skills — handled separately by racial_spells()
            let is_purely_racial = skill_info.race_mask != 0 && skill_info.class_mask == 0;
            if is_purely_racial {
                continue;
            }

            // Only process skills that either:
            // 1. Are class-specific to this player's class (class_mask has exactly 1 bit set
            //    matching this class). This covers class skills like Priest, Holy, Shadow, etc.
            // 2. Are in the character's actual known skills (from character_skills table).
            //    This covers weapons, languages, racials, worn armor type, etc.
            //
            // This matches C# behavior: LearnSkillRewardedSpells() only runs for skills the
            // character HAS. Professions (class_mask=0, available to all) are excluded unless
            // the character has actually learned them.
            let is_this_class_skill = skill_info.class_mask != 0
                && (skill_info.class_mask as u32).count_ones() == 1
                && (skill_info.class_mask & (1i32 << (class as i32 - 1))) != 0;

            let character_has_skill = known_skill_ids
                .map(|ids| ids.contains(&skill_id))
                .unwrap_or(true); // If no filter provided, allow all (backward compat)

            if !is_this_class_skill && !character_has_skill {
                continue; // Skip: profession/other-class skill the character doesn't have
            }

            // Get all abilities for this skill
            let abilities = match self.abilities_by_skill.get(&skill_id) {
                Some(a) => a,
                None => continue,
            };

            for ability in abilities {
                // For class-exclusive skills (Priest, Holy, Shadow, etc.): grant all ranks
                // including trainer-learned (acquire_method=0). A level 80 character should
                // have all ranks of their class spells.
                // For non-class skills (racials, languages, etc.): only auto-learned (1 or 2).
                if !is_this_class_skill
                    && ability.acquire_method != 1
                    && ability.acquire_method != 2
                {
                    continue;
                }

                // Check race/class masks on the ability itself
                if !matches_race(ability.race_mask, race) {
                    continue;
                }
                if !matches_class(ability.class_mask, class) {
                    continue;
                }

                // Check skill rank requirement against the character's
                // effective skill value (level * 5 for class skills).
                if ability.min_skill_line_rank > max_skill_rank {
                    continue;
                }

                let spell_id = ability.spell;
                if spell_id <= 0 {
                    continue;
                }

                // Handle supercedes_spell: if this spell replaces another,
                // remove the old one (only include highest rank)
                if ability.supercedes_spell > 0 {
                    seen.remove(&ability.supercedes_spell);
                    spells.retain(|&s| s != ability.supercedes_spell);
                }

                if seen.insert(spell_id) {
                    spells.push(spell_id);
                }
            }
        }

        spells
    }

    /// Returns racial spells for this race — spells tied to skills that are
    /// race-specific AND NOT class-specific (purely racial skills like Blood Elf 756).
    /// These are always granted based on race, not via the skill-learning system.
    pub fn racial_spells(&self, race: u8) -> Vec<i32> {
        let mut spells: Vec<i32> = Vec::new();
        let mut seen: std::collections::HashSet<i32> = std::collections::HashSet::new();

        for ((_r, _c), skills) in &self.starting_skills {
            for skill_info in skills {
                // Purely racial skill: race_mask set, class_mask == 0
                let is_racial = skill_info.race_mask != 0 && skill_info.class_mask == 0;
                if !is_racial {
                    continue;
                }
                // Must match this race
                if !matches_race(skill_info.race_mask, race) {
                    continue;
                }

                let abilities = match self.abilities_by_skill.get(&skill_info.skill_id) {
                    Some(a) => a,
                    None => continue,
                };

                for ability in abilities {
                    // Only auto-granted (OnSkillValue=1, OnSkillLearn=2)
                    if ability.acquire_method != 1 && ability.acquire_method != 2 {
                        continue;
                    }
                    // Race filter on the ability itself
                    if !matches_race(ability.race_mask, race) {
                        continue;
                    }
                    // No class filter here (these are racial, class_mask should be 0)
                    let spell_id = ability.spell;
                    if spell_id <= 0 {
                        continue;
                    }
                    // Handle supercedes_spell
                    if ability.supercedes_spell > 0 {
                        seen.remove(&ability.supercedes_spell);
                        spells.retain(|&s| s != ability.supercedes_spell);
                    }
                    if seen.insert(spell_id) {
                        spells.push(spell_id);
                    }
                }
            }
        }
        spells
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

    /// C++ `sSkillLineStore.LookupEntry(skillId)` existence check for loaded skill lines.
    pub fn contains_skill_line_like_cpp(&self, skill_id: u32) -> bool {
        u16::try_from(skill_id)
            .ok()
            .is_some_and(|skill_id| self.abilities_by_skill.contains_key(&skill_id))
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

    /// C++ `DB2Manager::GetSkillLineAbilitiesBySkill(skillId)`.
    pub fn skill_line_abilities_by_skill_like_cpp(
        &self,
        skill_id: u16,
    ) -> Option<&[SkillLineAbilityRecord]> {
        self.abilities_by_skill.get(&skill_id).map(Vec::as_slice)
    }

    /// C++ `sSkillLineAbilityStore` full row iteration.
    pub fn skill_line_abilities_like_cpp(&self) -> &[SkillLineAbilityRecord] {
        &self.abilities_like_cpp
    }

    /// Number of SkillRaceClassInfo records loaded.
    pub fn race_class_count(&self) -> usize {
        self.total_race_class
    }
}

/// Check if a race matches a race mask. Mask of 0 means "all races".
fn matches_race(mask: i64, race: u8) -> bool {
    mask == 0 || (mask & (1i64 << (race as i64 - 1))) != 0
}

/// Check if a class matches a class mask. Mask of 0 means "all classes".
fn matches_class(mask: i32, class: u8) -> bool {
    mask == 0 || (mask & (1i32 << (class as i32 - 1))) != 0
}

const SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP: i8 = 2;

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

        // Record 320: Smite (spell=585) for Priest (skill_line=56)
        if let Some(idx) = sla.get_record_index(320) {
            assert_eq!(
                sla.get_field_i32(idx, 2),
                56,
                "field[2] should be skill_line 56"
            );
            assert_eq!(
                sla.get_field_i32(idx, 3),
                585,
                "field[3] should be spell 585"
            );
        }
    }

    #[test]
    fn test_priest_starting_spells() {
        let store = match load_store() {
            Some(s) => s,
            None => return,
        };

        // Race 1 = Human, Class 5 = Priest, Level 80
        let spells = store.starting_spells(1, 5, 80, None);
        assert!(
            spells.len() > 10,
            "expected >10 starting spells for Human Priest L80, got {}",
            spells.len()
        );
        // Reasonable range: class skills + a few shared auto-learns
        assert!(
            spells.len() < 1000,
            "too many spells ({}), likely including profession recipes",
            spells.len()
        );

        // Priest should know Smite (585)
        assert!(
            spells.contains(&585),
            "Human Priest should know Smite (585), got: {:?}",
            &spells[..spells.len().min(20)]
        );
    }

    #[test]
    fn test_warrior_starting_spells() {
        let store = match load_store() {
            Some(s) => s,
            None => return,
        };

        // Race 2 = Orc, Class 1 = Warrior, Level 80
        let spells = store.starting_spells(2, 1, 80, None);
        assert!(
            spells.len() > 5,
            "expected >5 starting spells for Orc Warrior L80, got {}",
            spells.len()
        );

        // Warrior should know Battle Stance (2457)
        assert!(
            spells.contains(&2457),
            "Orc Warrior should know Battle Stance (2457), got: {:?}",
            &spells[..spells.len().min(20)]
        );
    }

    #[test]
    fn test_no_duplicate_spells() {
        let store = match load_store() {
            Some(s) => s,
            None => return,
        };

        let spells = store.starting_spells(1, 5, 80, None);
        let mut unique = spells.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            spells.len(),
            unique.len(),
            "starting_spells should not contain duplicates"
        );
    }

    #[test]
    fn test_different_classes_different_spells() {
        let store = match load_store() {
            Some(s) => s,
            None => return,
        };

        let priest_spells = store.starting_spells(1, 5, 80, None);
        let warrior_spells = store.starting_spells(1, 1, 80, None);

        // They should not be identical
        assert_ne!(
            priest_spells, warrior_spells,
            "Priest and Warrior should have different spell lists"
        );
    }
}
