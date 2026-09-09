//! Skill tier and pet spell stores state definitions, part 1 of 2.
//!
//! Separated from the skill.rs root under #638. Behaviour is preserved.

use super::*;

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

pub(super) const RACE_HUMAN_LIKE_CPP: u8 = 1;

pub(super) const MAX_RACES_LIKE_CPP: u8 = 78;

pub(super) const CLASS_WARRIOR_LIKE_CPP: u8 = 1;

pub(super) const MAX_CLASSES_LIKE_CPP: u8 = 15;

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
pub struct SkillLineAbilitySourceRecordLikeCpp {
    pub source: SkillStoreLoadSourceLikeCpp,
    pub id: u32,
    pub race_mask: i128,
    pub skill_line: i128,
    pub spell: i128,
    pub min_skill_line_rank: i128,
    pub class_mask: i128,
    pub supercedes_spell: i128,
    pub acquire_method: i128,
    pub trivial_rank_high: i128,
    pub trivial_rank_low: i128,
    pub flags: i128,
    pub num_skill_ups: i128,
    pub skillup_skill_line_id: i128,
}

#[derive(Debug, Clone)]
pub struct SkillRaceClassInfoSourceRecordLikeCpp {
    pub source: SkillStoreLoadSourceLikeCpp,
    pub id: u32,
    pub race_mask: i128,
    pub skill_id: i128,
    pub class_mask: i128,
    pub flags: i128,
    pub availability: i128,
    pub min_level: i128,
    pub skill_tier_id: i128,
}

/// Opaque WDC4 half of the effective skill catalog. Keeping this value opaque
/// lets the composition root preserve C++ file/SQL order without exposing the
/// store's intermediate maps or table hashes.
pub struct SkillStoreWdc4BaseLikeCpp {
    pub(super) abilities: Vec<SkillLineAbilitySourceRecordLikeCpp>,
    pub(super) ability_table_hash: u32,
    pub(super) race_class_infos: Vec<SkillRaceClassInfoSourceRecordLikeCpp>,
    pub(super) race_class_table_hash: u32,
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
    pub(super) tiers: HashMap<u32, SkillTiersEntryLikeCpp>,
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
    pub(super) spells_by_level: BTreeMap<u32, Vec<u32>>,
    pub(super) count: usize,
}

impl PetLevelupSpellSetLikeCpp {
    pub(super) fn insert_like_cpp(&mut self, spell_level: u32, spell_id: u32) {
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
    pub(super) spells_by_family: HashMap<u32, PetLevelupSpellSetLikeCpp>,
    pub(super) count: usize,
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
    pub(super) spells_by_family: BTreeMap<u32, BTreeMap<u32, ()>>,
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

pub(super) const SPELL_EFFECT_SUMMON_LIKE_CPP: u32 = 28;

pub(super) const SPELL_EFFECT_SUMMON_PET_LIKE_CPP: u32 = 56;

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
    pub(super) default_spells_by_entry: HashMap<i32, PetDefaultSpellsEntryLikeCpp>,
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

pub(super) fn load_pet_default_spells_helper_like_cpp(
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

/// In-memory store for auto-learned spells from DBC data.
pub struct SkillStore {
    /// C++ `sSkillLineAbilityStore` row iteration, kept in load order for represented loaders.
    pub(super) abilities_like_cpp: Vec<SkillLineAbilityRecord>,
    /// SkillLineAbility records indexed by skill_line (the parent skill).
    pub(super) abilities_by_skill: HashMap<u16, Vec<SkillLineAbilityRecord>>,
    /// C++ `SpellMgr::mSkillLineAbilityMap`, indexed by `SkillLineAbilityEntry::Spell`.
    pub(super) abilities_by_spell_like_cpp: HashMap<i32, Vec<SkillLineAbilityRecord>>,
    /// SkillRaceClassInfo records indexed by (race, class).
    pub(super) starting_skills: HashMap<(u8, u8), Vec<SkillRaceClassInfoRecord>>,
    /// C++ `_skillRaceClassInfoBySkill`, preserving DB2 iteration order.
    pub(super) race_class_by_skill: HashMap<u16, Vec<SkillRaceClassInfoRecord>>,
    pub(super) invalid_abilities_by_spell_like_cpp:
        HashMap<i32, Vec<SkillStoreLoadDiagnosticLikeCpp>>,
    pub(super) invalid_abilities_by_skill_like_cpp:
        HashMap<u16, Vec<SkillStoreLoadDiagnosticLikeCpp>>,
    pub(super) invalid_race_class_by_skill_like_cpp:
        HashMap<u16, Vec<SkillStoreLoadDiagnosticLikeCpp>>,
    pub(super) rank_rows_like_cpp: Vec<SkillLineAbilityRankRowLikeCpp>,
    /// Total number of SkillLineAbility records loaded.
    pub(super) total_abilities: usize,
    /// Total number of SkillRaceClassInfo records loaded.
    pub(super) total_race_class: usize,
}

pub(super) fn skill_line_ability_source_from_wdc4_like_cpp(
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

pub(super) fn skill_race_class_info_source_from_wdc4_like_cpp(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn compose_effective_skill_store_like_cpp(
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

pub(super) fn record_removed_like_cpp(
    removed_records: &Db2HotfixRemovalStoreLikeCpp,
    table_hash: u32,
    record_id: u32,
) -> bool {
    // C++ hotfix keys store the signed `RecordID` bit pattern even though DB2
    // storage exposes the ID as `uint32`.
    removed_records.contains_like_cpp(table_hash, record_id as i32)
}

pub(super) fn skill_line_ability_rank_row_from_hydrated_like_cpp(
    record: &SkillLineAbilityRecord,
) -> Option<SkillLineAbilityRankRowLikeCpp> {
    skill_line_ability_rank_row_from_raw_like_cpp(
        record.id,
        i128::from(record.spell),
        i128::from(record.supercedes_spell),
    )
}
