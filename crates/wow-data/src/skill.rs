// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SkillLineAbility.db2 + SkillRaceClassInfo.db2 reader.
//!
//! Determines which spells each race/class/level should auto-learn,
//! replicating C#'s `LearnDefaultSkills()` → `LearnSkillRewardedSpells()`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

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

// ── Store ───────────────────────────────────────────────────────────

/// In-memory store for auto-learned spells from DBC data.
pub struct SkillStore {
    /// SkillLineAbility records indexed by skill_line (the parent skill).
    abilities_by_skill: HashMap<u16, Vec<SkillLineAbilityRecord>>,
    /// SkillRaceClassInfo records indexed by (race, class).
    starting_skills: HashMap<(u8, u8), Vec<SkillRaceClassInfoRecord>>,
    /// Total number of SkillLineAbility records loaded.
    total_abilities: usize,
    /// Total number of SkillRaceClassInfo records loaded.
    total_race_class: usize,
}

impl SkillStore {
    /// Load both DB2 files from `{data_dir}/dbc/{locale}/`.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let dbc_dir = Path::new(data_dir).join("dbc").join(locale);

        // ── SkillLineAbility.db2 ──
        let sla_path = dbc_dir.join("SkillLineAbility.db2");
        let sla_reader = Wdc4Reader::open(&sla_path)
            .with_context(|| format!("failed to open {}", sla_path.display()))?;

        let mut abilities_by_skill: HashMap<u16, Vec<SkillLineAbilityRecord>> = HashMap::new();
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
            abilities_by_skill.entry(skill_line).or_default().push(record);
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
            abilities_by_skill,
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
                if !is_this_class_skill && ability.acquire_method != 1 && ability.acquire_method != 2 {
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

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const DATA_DIR: &str = "/path/to/wow-data";
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
            assert_eq!(sla.get_field_i32(idx, 2), 56, "field[2] should be skill_line 56");
            assert_eq!(sla.get_field_i32(idx, 3), 585, "field[3] should be spell 585");
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
