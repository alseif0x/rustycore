//! Skill tier and pet spell stores operations, part 1 of 1.
//!
//! The inherent `SkillStore` impl is divided by responsibility under
//! #638; every method keeps its original body.

use super::*;

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
    /// Load the WDC4 half of the effective C++ skill authority before any SQL
    /// overlay is requested, matching `DB2Manager::LoadStores`.
    pub fn load_wdc4_base_like_cpp(
        data_dir: &str,
        locale: &str,
    ) -> Result<SkillStoreWdc4BaseLikeCpp> {
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

        Ok(SkillStoreWdc4BaseLikeCpp {
            abilities: base_abilities,
            ability_table_hash: sla_table_hash,
            race_class_infos: base_race_class_infos,
            race_class_table_hash: srci_table_hash,
        })
    }
    /// Compose already decoded Hotfix overlays over an opaque WDC4 base.
    /// Derived indexes are rebuilt only after final tombstones.
    pub fn compose_effective_from_hotfix_overlays_like_cpp(
        base: SkillStoreWdc4BaseLikeCpp,
        official_abilities: impl IntoIterator<Item = SkillLineAbilitySourceRecordLikeCpp>,
        custom_abilities: impl IntoIterator<Item = SkillLineAbilitySourceRecordLikeCpp>,
        official_race_class_infos: impl IntoIterator<Item = SkillRaceClassInfoSourceRecordLikeCpp>,
        custom_race_class_infos: impl IntoIterator<Item = SkillRaceClassInfoSourceRecordLikeCpp>,
        removed_records: &Db2HotfixRemovalStoreLikeCpp,
        skill_line_store: &SkillLineStore,
    ) -> SkillStoreEffectiveLoadOutcomeLikeCpp {
        let outcome = compose_effective_skill_store_like_cpp(
            base.abilities,
            official_abilities,
            custom_abilities,
            base.ability_table_hash,
            base.race_class_infos,
            official_race_class_infos,
            custom_race_class_infos,
            base.race_class_table_hash,
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
        outcome
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
    pub(super) fn default_skill_info_like_cpp(
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
