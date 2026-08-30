//! Skill, talent, PvP, glyph and journal DB2 readers.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::Db2HotfixRemovalStoreLikeCpp;
use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphBindableSpellEntry {
    pub id: u32,
    pub spell_id: i32,
    pub glyph_properties_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphPropertiesEntry {
    pub id: u32,
    pub spell_id: u32,
    pub glyph_type: u8,
    pub glyph_exclusive_category_id: u8,
    pub spell_icon_file_data_id: i32,
    pub glyph_slot_flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphRequiredSpecEntry {
    pub id: u32,
    pub chr_specialization_id: u16,
    pub glyph_properties_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphSlotEntry {
    pub id: u32,
    pub tooltip: i32,
    pub slot_type: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalEncounterEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub map: [f32; 2],
    pub journal_instance_id: u16,
    pub order_index: u32,
    pub first_section_id: u16,
    pub ui_map_id: u16,
    pub map_display_condition_id: u32,
    pub flags: i32,
    pub difficulty_mask: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEncounterSectionEntry {
    pub id: u32,
    pub title: String,
    pub body_text: String,
    pub journal_encounter_id: u16,
    pub order_index: u8,
    pub parent_section_id: u16,
    pub first_child_section_id: u16,
    pub next_sibling_section_id: u16,
    pub section_type: u8,
    pub icon_creature_display_info_id: u32,
    pub ui_model_scene_id: i32,
    pub spell_id: i32,
    pub icon_file_data_id: i32,
    pub flags: i32,
    pub icon_flags: i32,
    pub difficulty_mask: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalInstanceEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub map_id: u16,
    pub background_file_data_id: i32,
    pub button_file_data_id: i32,
    pub button_small_file_data_id: i32,
    pub lore_file_data_id: i32,
    pub flags: i32,
    pub area_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalTierEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvpSeasonEntry {
    pub id: u32,
    pub milestone_season: i32,
    pub alliance_achievement_id: i32,
    pub horde_achievement_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvpTalentEntry {
    pub id: u32,
    pub description: String,
    pub spec_id: u32,
    pub spell_id: i32,
    pub overrides_spell_id: i32,
    pub flags: i32,
    pub action_bar_spell_id: i32,
    pub pvp_talent_category_id: i32,
    pub level_required: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvpTalentCategoryEntry {
    pub id: u32,
    pub talent_slot_mask: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvpTalentSlotUnlockEntry {
    pub id: u32,
    pub slot: i8,
    pub level_required: i32,
    pub death_knight_level_required: i32,
    pub demon_hunter_level_required: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvpTierEntry {
    pub id: u32,
    pub name: String,
    pub min_rating: i16,
    pub max_rating: i16,
    pub prev_tier: i32,
    pub next_tier: i32,
    pub bracket_id: u8,
    pub rank: u8,
    pub rank_icon_file_data_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillLineEntry {
    pub id: u32,
    pub display_name: String,
    pub alternate_verb: String,
    pub description: String,
    pub horde_display_name: String,
    pub override_source_info_display_name: String,
    pub category_id: i8,
    pub spell_icon_file_id: i32,
    pub can_link: i8,
    pub parent_skill_line_id: u32,
    pub parent_tier_index: i32,
    pub flags: u16,
    pub spell_book_spell_id: i32,
}

/// Acquisition-relevant C++ `SkillLineEntry` payload.
///
/// Keeping this projection separate from [`SkillLineEntry`] lets SQL-only
/// effective records participate in authorization without manufacturing
/// localized or otherwise unhydrated fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillLineAcquisitionFieldsLikeCpp {
    pub category_id: i8,
    pub parent_skill_line_id: u32,
    pub parent_tier_index: i32,
}

/// Raw Hotfix projection used to compose the effective SkillLine catalog.
/// Wide values let the data owner retain an invalid overlay identity while
/// classifying its acquisition payload as incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillLineHotfixOverlayLikeCpp {
    pub id: u32,
    pub category_id: i128,
    pub parent_skill_line_id: i128,
    pub parent_tier_index: i128,
}

/// Coverage of the acquisition payload for one `SkillLine` record ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillLineAcquisitionPayloadLikeCpp {
    /// The record is absent from the final effective C++ identity set.
    Absent,
    /// The identity is effective, but Rust has not hydrated every required field.
    Incomplete,
    /// Every field needed by profession classification and parent activation is available.
    Complete(SkillLineAcquisitionFieldsLikeCpp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillLineXTraitTreeEntry {
    pub id: u32,
    pub skill_line_id: u32,
    pub trait_tree_id: i32,
    pub order_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentEntry {
    pub id: u32,
    pub description: String,
    pub tier_id: u8,
    pub flags: u8,
    pub column_index: u8,
    pub tab_id: u16,
    pub class_id: u8,
    pub spec_id: u16,
    pub spell_id: i32,
    pub overrides_spell_id: i32,
    pub required_spell_id: i32,
    pub category_mask: [i32; 2],
    pub spell_rank: [i32; 9],
    pub prereq_talent: [i32; 3],
    pub prereq_rank: [i32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalentTabEntry {
    pub id: u32,
    pub name: String,
    pub background_file: String,
    pub order_index: i32,
    pub race_mask: i32,
    pub class_mask: i32,
    pub pet_talent_mask: i32,
    pub spell_icon_id: i32,
}

macro_rules! db2_store {
    ($store:ident, $entry:ty) => {
        pub struct $store {
            entries: HashMap<u32, $entry>,
        }

        impl $store {
            pub fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                }
            }

            pub fn get(&self, id: u32) -> Option<&$entry> {
                self.entries.get(&id)
            }

            pub fn len(&self) -> usize {
                self.entries.len()
            }

            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
        }
    };
}

db2_store!(GlyphBindableSpellStore, GlyphBindableSpellEntry);
db2_store!(GlyphPropertiesStore, GlyphPropertiesEntry);
db2_store!(GlyphRequiredSpecStore, GlyphRequiredSpecEntry);
db2_store!(GlyphSlotStore, GlyphSlotEntry);
db2_store!(JournalEncounterStore, JournalEncounterEntry);
db2_store!(JournalEncounterSectionStore, JournalEncounterSectionEntry);
db2_store!(JournalInstanceStore, JournalInstanceEntry);
db2_store!(JournalTierStore, JournalTierEntry);
db2_store!(PvpSeasonStore, PvpSeasonEntry);
db2_store!(PvpTalentStore, PvpTalentEntry);
db2_store!(PvpTalentCategoryStore, PvpTalentCategoryEntry);
db2_store!(PvpTalentSlotUnlockStore, PvpTalentSlotUnlockEntry);
db2_store!(PvpTierStore, PvpTierEntry);
db2_store!(SkillLineXTraitTreeStore, SkillLineXTraitTreeEntry);
db2_store!(TalentStore, TalentEntry);
db2_store!(TalentTabStore, TalentTabEntry);

/// Hydrated WDC4 SkillLine payload plus the exact identities and
/// authorization fields visible through C++ `sSkillLineStore.LookupEntry`.
///
/// SQL overlays hydrate the category/parent pair needed for exact
/// primary-profession classification, but do not manufacture the remaining
/// payload fields. Startup foreign-key validation can still use the same
/// effective record authority as C++.
pub struct SkillLineStore {
    entries: HashMap<u32, SkillLineEntry>,
    effective_record_ids_like_cpp: HashSet<u32>,
    acquisition_fields_by_effective_record_like_cpp:
        BTreeMap<u32, SkillLineAcquisitionFieldsLikeCpp>,
    table_hash_like_cpp: Option<u32>,
}

impl SkillLineStore {
    pub fn from_entries(entries: impl IntoIterator<Item = SkillLineEntry>) -> Self {
        let entries: HashMap<_, _> = entries.into_iter().map(|entry| (entry.id, entry)).collect();
        let effective_record_ids_like_cpp = entries.keys().copied().collect();
        let acquisition_fields_by_effective_record_like_cpp = entries
            .iter()
            .map(|(id, entry)| {
                (
                    *id,
                    SkillLineAcquisitionFieldsLikeCpp {
                        category_id: entry.category_id,
                        parent_skill_line_id: entry.parent_skill_line_id,
                        parent_tier_index: entry.parent_tier_index,
                    },
                )
            })
            .collect();
        Self {
            entries,
            effective_record_ids_like_cpp,
            acquisition_fields_by_effective_record_like_cpp,
            table_hash_like_cpp: None,
        }
    }

    /// Constructs the same split authority produced by
    /// `load_effective_like_cpp`: hydrated payload can be a strict subset or
    /// superset of the final effective IDs after SQL overlays and removals.
    pub fn from_hydrated_entries_and_effective_ids_like_cpp(
        entries: impl IntoIterator<Item = SkillLineEntry>,
        effective_record_ids_like_cpp: impl IntoIterator<Item = u32>,
    ) -> Self {
        let mut store = Self::from_entries(entries);
        store.effective_record_ids_like_cpp = effective_record_ids_like_cpp.into_iter().collect();
        let effective_record_ids_like_cpp = &store.effective_record_ids_like_cpp;
        store
            .acquisition_fields_by_effective_record_like_cpp
            .retain(|record_id, _| effective_record_ids_like_cpp.contains(record_id));
        store
    }

    pub fn get(&self, id: u32) -> Option<&SkillLineEntry> {
        self.entries.get(&id)
    }

    pub fn contains_effective_record_like_cpp(&self, id: u32) -> bool {
        self.effective_record_ids_like_cpp.contains(&id)
    }

    /// Effective acquisition payload used by `Player::SetSkill` and
    /// `IsPrimaryProfessionSkill`.
    pub fn acquisition_payload_like_cpp(&self, id: u32) -> SkillLineAcquisitionPayloadLikeCpp {
        if !self.contains_effective_record_like_cpp(id) {
            return SkillLineAcquisitionPayloadLikeCpp::Absent;
        }

        self.acquisition_fields_by_effective_record_like_cpp
            .get(&id)
            .copied()
            .map(SkillLineAcquisitionPayloadLikeCpp::Complete)
            .unwrap_or(SkillLineAcquisitionPayloadLikeCpp::Incomplete)
    }

    /// Whether every final effective `SkillLine` identity has the payload
    /// required to project acquisition without guessing.
    pub fn has_complete_acquisition_payload_like_cpp(&self) -> bool {
        self.acquisition_fields_by_effective_record_like_cpp.len()
            == self.effective_record_ids_like_cpp.len()
    }

    /// C++ `DB2Manager::GetSkillLinesForParentSkill`, projected to the
    /// acquisition payload and ordered by final `RecordID`.
    ///
    /// An effective identity whose final parent payload is not hydrated is
    /// returned as [`SkillLineAcquisitionPayloadLikeCpp::Incomplete`] for
    /// every parent query. Rust cannot prove that such a record is not one of
    /// C++'s children, so silently filtering it out would turn missing
    /// authority into a deterministic (and potentially incomplete)
    /// `Player::SetSkill` projection.
    pub fn acquisition_children_for_parent_like_cpp(
        &self,
        parent_skill_line_id: u32,
    ) -> impl Iterator<Item = (u32, SkillLineAcquisitionPayloadLikeCpp)> {
        let mut children_or_indeterminate = self
            .effective_record_ids_like_cpp
            .iter()
            .copied()
            .filter_map(
                |record_id| match self.acquisition_payload_like_cpp(record_id) {
                    SkillLineAcquisitionPayloadLikeCpp::Complete(fields)
                        if fields.parent_skill_line_id == parent_skill_line_id =>
                    {
                        Some((
                            record_id,
                            SkillLineAcquisitionPayloadLikeCpp::Complete(fields),
                        ))
                    }
                    SkillLineAcquisitionPayloadLikeCpp::Incomplete => {
                        Some((record_id, SkillLineAcquisitionPayloadLikeCpp::Incomplete))
                    }
                    SkillLineAcquisitionPayloadLikeCpp::Absent
                    | SkillLineAcquisitionPayloadLikeCpp::Complete(_) => None,
                },
            )
            .collect::<Vec<_>>();
        children_or_indeterminate.sort_unstable_by_key(|(record_id, _)| *record_id);
        children_or_indeterminate.into_iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn effective_record_count_like_cpp(&self) -> usize {
        self.effective_record_ids_like_cpp.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn table_hash_like_cpp(&self) -> Option<u32> {
        self.table_hash_like_cpp
    }
}

impl GlyphBindableSpellStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GlyphBindableSpell.db2", |id, idx, r| {
            GlyphBindableSpellEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                glyph_properties_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl GlyphPropertiesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GlyphProperties.db2", |id, idx, r| {
            GlyphPropertiesEntry {
                id,
                spell_id: r.get_field_u32(idx, 0),
                glyph_type: r.get_field_u8(idx, 1),
                glyph_exclusive_category_id: r.get_field_u8(idx, 2),
                spell_icon_file_data_id: r.get_field_i32(idx, 3),
                glyph_slot_flags: r.get_field_u32(idx, 4),
            }
        })
    }
}

impl GlyphRequiredSpecStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GlyphRequiredSpec.db2", |id, idx, r| {
            GlyphRequiredSpecEntry {
                id,
                chr_specialization_id: r.get_field_u16(idx, 0),
                glyph_properties_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl GlyphSlotStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GlyphSlot.db2", |id, idx, r| {
            GlyphSlotEntry {
                id,
                tooltip: r.get_field_i32(idx, 0),
                slot_type: r.get_field_u32(idx, 1),
            }
        })
    }
}

impl JournalEncounterStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "JournalEncounter.db2", |id, idx, r| {
            JournalEncounterEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                map: f32_array::<2>(r, idx, 2),
                journal_instance_id: r.get_field_u16(idx, 3),
                order_index: r.get_field_u32(idx, 4),
                first_section_id: r.get_field_u16(idx, 5),
                ui_map_id: r.get_field_u16(idx, 6),
                map_display_condition_id: r.get_field_u32(idx, 7),
                flags: r.get_field_i32(idx, 8),
                difficulty_mask: r.get_field_i8(idx, 9),
            }
        })
    }
}

impl JournalEncounterSectionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "JournalEncounterSection.db2",
            |id, idx, r| JournalEncounterSectionEntry {
                id,
                title: r.get_field_string(idx, 0),
                body_text: r.get_field_string(idx, 1),
                journal_encounter_id: r.get_field_u16(idx, 2),
                order_index: r.get_field_u8(idx, 3),
                parent_section_id: r.get_field_u16(idx, 4),
                first_child_section_id: r.get_field_u16(idx, 5),
                next_sibling_section_id: r.get_field_u16(idx, 6),
                section_type: r.get_field_u8(idx, 7),
                icon_creature_display_info_id: r.get_field_u32(idx, 8),
                ui_model_scene_id: r.get_field_i32(idx, 9),
                spell_id: r.get_field_i32(idx, 10),
                icon_file_data_id: r.get_field_i32(idx, 11),
                flags: r.get_field_i32(idx, 12),
                icon_flags: r.get_field_i32(idx, 13),
                difficulty_mask: r.get_field_i8(idx, 14),
            },
        )
    }
}

impl JournalInstanceStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "JournalInstance.db2", |id, idx, r| {
            JournalInstanceEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                map_id: r.get_field_u16(idx, 3),
                background_file_data_id: r.get_field_i32(idx, 4),
                button_file_data_id: r.get_field_i32(idx, 5),
                button_small_file_data_id: r.get_field_i32(idx, 6),
                lore_file_data_id: r.get_field_i32(idx, 7),
                flags: r.get_field_i32(idx, 8),
                area_id: r.get_field_u16(idx, 9),
            }
        })
    }
}

impl JournalTierStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "JournalTier.db2", |id, idx, r| {
            JournalTierEntry {
                id,
                name: r.get_field_string(idx, 0),
            }
        })
    }
}

impl PvpSeasonStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PvpSeason.db2", |id, idx, r| {
            PvpSeasonEntry {
                id,
                milestone_season: r.get_field_i32(idx, 0),
                alliance_achievement_id: r.get_field_i32(idx, 1),
                horde_achievement_id: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl PvpTalentStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PvpTalent.db2", |id, idx, r| {
            PvpTalentEntry {
                id,
                description: r.get_field_string(idx, 0),
                spec_id: r.get_relationship_id(idx).unwrap_or(0),
                spell_id: r.get_field_i32(idx, 3),
                overrides_spell_id: r.get_field_i32(idx, 4),
                flags: r.get_field_i32(idx, 5),
                action_bar_spell_id: r.get_field_i32(idx, 6),
                pvp_talent_category_id: r.get_field_i32(idx, 7),
                level_required: r.get_field_i32(idx, 8),
            }
        })
    }
}

impl PvpTalentCategoryStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PvpTalentCategory.db2", |id, idx, r| {
            PvpTalentCategoryEntry {
                id,
                talent_slot_mask: r.get_field_u8(idx, 0),
            }
        })
    }
}

impl PvpTalentSlotUnlockStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PvpTalentSlotUnlock.db2", |id, idx, r| {
            PvpTalentSlotUnlockEntry {
                id,
                slot: r.get_field_i8(idx, 0),
                level_required: r.get_field_i32(idx, 1),
                death_knight_level_required: r.get_field_i32(idx, 2),
                demon_hunter_level_required: r.get_field_i32(idx, 3),
            }
        })
    }
}

impl PvpTierStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PvpTier.db2", |id, idx, r| PvpTierEntry {
            id,
            name: r.get_field_string(idx, 0),
            min_rating: r.get_field_i16(idx, 1),
            max_rating: r.get_field_i16(idx, 2),
            prev_tier: r.get_field_i32(idx, 3),
            next_tier: r.get_field_i32(idx, 4),
            bracket_id: r.get_relationship_id(idx).unwrap_or(0) as u8,
            rank: r.get_field_u8(idx, 6),
            rank_icon_file_data_id: r.get_field_i32(idx, 7),
        })
    }
}

impl SkillLineStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("SkillLine.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let table_hash = reader.table_hash();
        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.push(skill_line_entry_from_wdc4_like_cpp(id, idx, &reader));
        }

        let mut store = Self::from_entries(entries);
        store.table_hash_like_cpp = Some(table_hash);
        info!("Loaded {} rows from {}", store.len(), path.display());
        Ok(store)
    }

    /// Loads the direct-record authority C++ exposes through
    /// `sSkillLineStore.LookupEntry`.
    ///
    /// Full payload hydration remains the WDC4 subset above. The category,
    /// parent, and parent-tier fields needed for acquisition authorization are
    /// overlaid exactly, including collisions with WDC4 records. Identity and
    /// acquisition-payload composition follow C++ `DB2StorageBase::LoadFromDB` and
    /// `DB2Manager::LoadHotfixData`: DB2, official SQL, custom SQL, then final
    /// record removals.
    pub fn apply_hotfix_overlays_like_cpp(
        mut self,
        official_overlays: impl IntoIterator<Item = SkillLineHotfixOverlayLikeCpp>,
        custom_overlays: impl IntoIterator<Item = SkillLineHotfixOverlayLikeCpp>,
        removed_records: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let table_hash = self
            .table_hash_like_cpp
            .context("SkillLine.db2 is missing its WDC4 table hash")?;
        let official_overlays = official_overlays
            .into_iter()
            .map(|overlay| classify_skill_line_hotfix_overlay_like_cpp(overlay, true))
            .collect::<Vec<_>>();
        let custom_overlays = custom_overlays
            .into_iter()
            .map(|overlay| classify_skill_line_hotfix_overlay_like_cpp(overlay, false))
            .collect::<Vec<_>>();
        self.effective_record_ids_like_cpp = compose_effective_skill_line_ids_like_cpp(
            self.entries.keys().copied(),
            official_overlays.iter().map(|(id, _)| *id),
            custom_overlays.iter().map(|(id, _)| *id),
            table_hash,
            removed_records,
        );
        self.acquisition_fields_by_effective_record_like_cpp =
            compose_effective_skill_line_acquisition_payloads_like_cpp(
                std::mem::take(&mut self.acquisition_fields_by_effective_record_like_cpp),
                official_overlays,
                custom_overlays,
                &self.effective_record_ids_like_cpp,
            );
        Ok(self)
    }

    /// C++ `Player::GetProfessionSkillForExp`.
    pub fn profession_skill_for_exp_like_cpp(&self, skill_id: u32, mut expansion: i32) -> u32 {
        const SKILL_CATEGORY_SECONDARY_LIKE_CPP: i8 = 9;
        const SKILL_CATEGORY_PROFESSION_LIKE_CPP: i8 = 11;
        const CURRENT_EXPANSION_LIKE_CPP: i32 = 2;
        const BASE_PARENT_TIER_INDEX_LIKE_CPP: i32 = 4;

        let SkillLineAcquisitionPayloadLikeCpp::Complete(skill) =
            self.acquisition_payload_like_cpp(skill_id)
        else {
            return 0;
        };
        if skill.parent_skill_line_id != 0
            || !matches!(
                skill.category_id,
                SKILL_CATEGORY_PROFESSION_LIKE_CPP | SKILL_CATEGORY_SECONDARY_LIKE_CPP
            )
        {
            return 0;
        }

        if expansion < 0 {
            expansion = CURRENT_EXPANSION_LIKE_CPP;
        }

        self.acquisition_fields_by_effective_record_like_cpp
            .iter()
            .find(|(_, child)| {
                child.parent_skill_line_id == skill_id
                    && child
                        .parent_tier_index
                        .checked_sub(BASE_PARENT_TIER_INDEX_LIKE_CPP)
                        == Some(expansion)
            })
            .map(|(child_id, _)| *child_id)
            .unwrap_or(0)
    }

    /// Classifies an effective `SkillLine` using C++
    /// `IsPrimaryProfessionSkill`.
    ///
    /// `None` deliberately identifies an effective SQL/DB2 identity whose
    /// category/parent payload Rust has not hydrated. Production SQL overlays
    /// load that exact pair in official-then-custom order, including WDC4
    /// collisions. A genuinely absent ID returns `Some(false)`, matching
    /// C++'s failed `LookupEntry`; callers making an authorization decision
    /// must fail closed only for the effective-but-unhydrated case.
    pub fn is_primary_profession_skill_like_cpp(&self, skill_id: u32) -> Option<bool> {
        const SKILL_CATEGORY_PROFESSION_LIKE_CPP: i8 = 11;

        match self.acquisition_payload_like_cpp(skill_id) {
            SkillLineAcquisitionPayloadLikeCpp::Absent => Some(false),
            SkillLineAcquisitionPayloadLikeCpp::Incomplete => None,
            SkillLineAcquisitionPayloadLikeCpp::Complete(fields) => Some(
                fields.category_id == SKILL_CATEGORY_PROFESSION_LIKE_CPP
                    && fields.parent_skill_line_id == 0,
            ),
        }
    }
}

impl SkillLineXTraitTreeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SkillLineXTraitTree.db2", |id, idx, r| {
            SkillLineXTraitTreeEntry {
                id,
                skill_line_id: r.get_relationship_id(idx).unwrap_or(0),
                trait_tree_id: r.get_field_i32(idx, 1),
                order_index: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl TalentStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Talent.db2", |id, idx, r| TalentEntry {
            id,
            description: r.get_field_string(idx, 0),
            tier_id: r.get_field_u8(idx, 1),
            flags: r.get_field_u8(idx, 2),
            column_index: r.get_field_u8(idx, 3),
            tab_id: r.get_field_u16(idx, 4),
            class_id: r.get_field_u8(idx, 5),
            spec_id: r.get_field_u16(idx, 6),
            spell_id: r.get_field_i32(idx, 7),
            overrides_spell_id: r.get_field_i32(idx, 8),
            required_spell_id: r.get_field_i32(idx, 9),
            category_mask: std::array::from_fn(|i| r.get_array_element(idx, 10, i, 32) as i32),
            spell_rank: std::array::from_fn(|i| r.get_array_element(idx, 11, i, 32) as i32),
            prereq_talent: std::array::from_fn(|i| r.get_array_element(idx, 12, i, 32) as i32),
            prereq_rank: std::array::from_fn(|i| r.get_array_element(idx, 13, i, 32) as i32),
        })
    }

    pub fn talent_spell_ids_like_cpp(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries.values().flat_map(|entry| {
            entry
                .spell_rank
                .iter()
                .filter_map(|spell_rank| u32::try_from(*spell_rank).ok())
                .filter(|spell_rank| *spell_rank != 0)
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &TalentEntry> {
        self.entries.values()
    }
}

impl TalentTabStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TalentTab.db2", |id, idx, r| {
            TalentTabEntry {
                id,
                name: r.get_field_string(idx, 0),
                background_file: r.get_field_string(idx, 1),
                order_index: r.get_field_i32(idx, 2),
                race_mask: r.get_field_i32(idx, 3),
                class_mask: r.get_field_i32(idx, 4),
                pet_talent_mask: r.get_field_i32(idx, 5),
                spell_icon_id: r.get_field_i32(idx, 6),
            }
        })
    }
}

fn skill_line_entry_from_wdc4_like_cpp(
    id: u32,
    record_idx: usize,
    reader: &Wdc4Reader,
) -> SkillLineEntry {
    SkillLineEntry {
        id,
        display_name: reader.get_field_string(record_idx, 0),
        alternate_verb: reader.get_field_string(record_idx, 1),
        description: reader.get_field_string(record_idx, 2),
        horde_display_name: reader.get_field_string(record_idx, 3),
        override_source_info_display_name: reader.get_field_string(record_idx, 4),
        category_id: reader.get_field_i8(record_idx, 6),
        spell_icon_file_id: reader.get_field_i32(record_idx, 7),
        can_link: reader.get_field_i8(record_idx, 8),
        parent_skill_line_id: reader.get_field_u32(record_idx, 9),
        parent_tier_index: reader.get_field_i32(record_idx, 10),
        flags: reader.get_field_u16(record_idx, 11),
        spell_book_spell_id: reader.get_field_i32(record_idx, 12),
    }
}

fn classify_skill_line_hotfix_overlay_like_cpp(
    overlay: SkillLineHotfixOverlayLikeCpp,
    official: bool,
) -> (u32, Option<SkillLineAcquisitionFieldsLikeCpp>) {
    let acquisition_fields = match (
        i8::try_from(overlay.category_id),
        u32::try_from(overlay.parent_skill_line_id),
        i32::try_from(overlay.parent_tier_index),
    ) {
        (Ok(category_id), Ok(parent_skill_line_id), Ok(parent_tier_index)) => {
            Some(SkillLineAcquisitionFieldsLikeCpp {
                category_id,
                parent_skill_line_id,
                parent_tier_index,
            })
        }
        _ => {
            warn!(
                record_id = overlay.id,
                category_id = overlay.category_id,
                parent_skill_line_id = overlay.parent_skill_line_id,
                parent_tier_index = overlay.parent_tier_index,
                official,
                "SkillLine SQL overlay has an out-of-domain acquisition payload; retaining its \
                 effective identity as incomplete"
            );
            None
        }
    };
    (overlay.id, acquisition_fields)
}

fn compose_effective_skill_line_ids_like_cpp(
    base_ids: impl IntoIterator<Item = u32>,
    official_overlay_ids: impl IntoIterator<Item = u32>,
    custom_overlay_ids: impl IntoIterator<Item = u32>,
    table_hash: u32,
    removed_records: &Db2HotfixRemovalStoreLikeCpp,
) -> HashSet<u32> {
    let mut effective_ids: HashSet<_> = base_ids.into_iter().collect();
    effective_ids.extend(official_overlay_ids);
    effective_ids.extend(custom_overlay_ids);
    effective_ids
        .retain(|record_id| !removed_records.contains_like_cpp(table_hash, *record_id as i32));
    effective_ids
}

#[cfg(test)]
fn compose_effective_skill_line_acquisition_fields_like_cpp(
    base_fields: impl IntoIterator<Item = (u32, SkillLineAcquisitionFieldsLikeCpp)>,
    official_overlays: impl IntoIterator<Item = (u32, SkillLineAcquisitionFieldsLikeCpp)>,
    custom_overlays: impl IntoIterator<Item = (u32, SkillLineAcquisitionFieldsLikeCpp)>,
    effective_ids: &HashSet<u32>,
) -> BTreeMap<u32, SkillLineAcquisitionFieldsLikeCpp> {
    compose_effective_skill_line_acquisition_payloads_like_cpp(
        base_fields,
        official_overlays
            .into_iter()
            .map(|(record_id, fields)| (record_id, Some(fields))),
        custom_overlays
            .into_iter()
            .map(|(record_id, fields)| (record_id, Some(fields))),
        effective_ids,
    )
}

fn compose_effective_skill_line_acquisition_payloads_like_cpp(
    base_fields: impl IntoIterator<Item = (u32, SkillLineAcquisitionFieldsLikeCpp)>,
    official_overlays: impl IntoIterator<Item = (u32, Option<SkillLineAcquisitionFieldsLikeCpp>)>,
    custom_overlays: impl IntoIterator<Item = (u32, Option<SkillLineAcquisitionFieldsLikeCpp>)>,
    effective_ids: &HashSet<u32>,
) -> BTreeMap<u32, SkillLineAcquisitionFieldsLikeCpp> {
    let mut payloads: BTreeMap<_, _> = base_fields
        .into_iter()
        .map(|(record_id, fields)| (record_id, Some(fields)))
        .collect();
    payloads.extend(official_overlays);
    payloads.extend(custom_overlays);
    payloads
        .into_iter()
        .filter_map(|(record_id, fields)| {
            effective_ids
                .contains(&record_id)
                .then_some(fields)
                .flatten()
                .map(|fields| (record_id, fields))
        })
        .collect()
}

fn load_store<T, S>(
    data_dir: &str,
    locale: &str,
    file_name: &str,
    mut read: impl FnMut(u32, usize, &Wdc4Reader) -> T,
) -> Result<S>
where
    S: FromEntries<T>,
{
    let path = Path::new(data_dir).join("dbc").join(locale).join(file_name);
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;

    let mut entries = Vec::with_capacity(reader.total_count());
    for (id, idx) in reader.iter_records() {
        entries.push(read(id, idx, &reader));
    }

    let store = S::from_entries(entries);
    info!("Loaded {} rows from {}", store.len(), path.display());
    Ok(store)
}

fn f32_array<const N: usize>(reader: &Wdc4Reader, record_idx: usize, field: usize) -> [f32; N] {
    std::array::from_fn(|i| f32::from_bits(reader.get_array_element(record_idx, field, i, 32)))
}

trait FromEntries<T> {
    fn from_entries(entries: impl IntoIterator<Item = T>) -> Self;
    fn len(&self) -> usize;
}

macro_rules! impl_from_entries {
    ($store:ident, $entry:ty) => {
        impl FromEntries<$entry> for $store {
            fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self::from_entries(entries)
            }

            fn len(&self) -> usize {
                self.len()
            }
        }
    };
}

impl_from_entries!(GlyphBindableSpellStore, GlyphBindableSpellEntry);
impl_from_entries!(GlyphPropertiesStore, GlyphPropertiesEntry);
impl_from_entries!(GlyphRequiredSpecStore, GlyphRequiredSpecEntry);
impl_from_entries!(GlyphSlotStore, GlyphSlotEntry);
impl_from_entries!(JournalEncounterStore, JournalEncounterEntry);
impl_from_entries!(JournalEncounterSectionStore, JournalEncounterSectionEntry);
impl_from_entries!(JournalInstanceStore, JournalInstanceEntry);
impl_from_entries!(JournalTierStore, JournalTierEntry);
impl_from_entries!(PvpSeasonStore, PvpSeasonEntry);
impl_from_entries!(PvpTalentStore, PvpTalentEntry);
impl_from_entries!(PvpTalentCategoryStore, PvpTalentCategoryEntry);
impl_from_entries!(PvpTalentSlotUnlockStore, PvpTalentSlotUnlockEntry);
impl_from_entries!(PvpTierStore, PvpTierEntry);
impl_from_entries!(SkillLineXTraitTreeStore, SkillLineXTraitTreeEntry);
impl_from_entries!(TalentStore, TalentEntry);
impl_from_entries!(TalentTabStore, TalentTabEntry);

#[cfg(test)]
mod tests {
    use super::*;

    fn skill_line(
        id: u32,
        category_id: i8,
        parent_skill_line_id: u32,
        parent_tier_index: i32,
    ) -> SkillLineEntry {
        SkillLineEntry {
            id,
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id,
            parent_tier_index,
            flags: 0,
            spell_book_spell_id: 0,
        }
    }

    #[test]
    fn glyph_required_spec_uses_cpp_parent_relationship() {
        let store = GlyphRequiredSpecStore::from_entries([GlyphRequiredSpecEntry {
            id: 1,
            chr_specialization_id: 2,
            glyph_properties_id: 3,
        }]);

        assert_eq!(store.get(1).unwrap().glyph_properties_id, 3);
    }

    #[test]
    fn profession_skill_for_exp_matches_cpp_parent_child_rules() {
        let store = SkillLineStore::from_entries([
            skill_line(356, 9, 0, 0),
            skill_line(900, 9, 356, i32::MIN),
            skill_line(1_000, 9, 356, 4),
            skill_line(1_001, 9, 356, 5),
            skill_line(777, 11, 0, 0),
            skill_line(2_000, 11, 777, 6),
            skill_line(3_000, 7, 0, 0),
        ]);

        assert_eq!(store.profession_skill_for_exp_like_cpp(356, 0), 1_000);
        assert_eq!(store.profession_skill_for_exp_like_cpp(356, 1), 1_001);
        assert_eq!(store.profession_skill_for_exp_like_cpp(777, 2), 2_000);
        assert_eq!(store.profession_skill_for_exp_like_cpp(1_000, 0), 0);
        assert_eq!(store.profession_skill_for_exp_like_cpp(3_000, 0), 0);
        assert_eq!(store.profession_skill_for_exp_like_cpp(999, 0), 0);
    }

    #[test]
    fn profession_skill_for_negative_expansion_uses_current_expansion_like_cpp() {
        let store =
            SkillLineStore::from_entries([skill_line(356, 9, 0, 0), skill_line(1_002, 9, 356, 6)]);

        assert_eq!(store.profession_skill_for_exp_like_cpp(356, -3), 1_002);
    }

    #[test]
    fn primary_profession_requires_root_category_eleven_and_distinguishes_payload_state() {
        let store = SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
            [
                skill_line(100, 11, 0, 0),
                skill_line(101, 11, 100, 4),
                skill_line(200, 9, 0, 0),
                skill_line(400, 11, 0, 0),
            ],
            [101, 200, 300, 400],
        );

        assert_eq!(
            store.is_primary_profession_skill_like_cpp(100),
            Some(false),
            "a hydrated row removed from effective authority matches failed C++ LookupEntry"
        );
        assert_eq!(store.is_primary_profession_skill_like_cpp(101), Some(false));
        assert_eq!(store.is_primary_profession_skill_like_cpp(400), Some(true));
        assert_eq!(store.is_primary_profession_skill_like_cpp(200), Some(false));
        assert_eq!(
            store.is_primary_profession_skill_like_cpp(300),
            None,
            "effective identity without hydrated category/parent must remain distinguishable"
        );
        assert_eq!(
            store.acquisition_payload_like_cpp(300),
            SkillLineAcquisitionPayloadLikeCpp::Incomplete
        );
        assert!(!store.has_complete_acquisition_payload_like_cpp());
        assert_eq!(
            store.is_primary_profession_skill_like_cpp(999),
            Some(false),
            "C++ LookupEntry failure classifies a genuinely absent ID as non-primary"
        );
        assert_eq!(
            store.acquisition_payload_like_cpp(999),
            SkillLineAcquisitionPayloadLikeCpp::Absent
        );
    }

    #[test]
    fn effective_skill_line_ids_include_overlay_only_records_without_payload() {
        let table_hash = 0xB53D_C9D6;
        let removals = Db2HotfixRemovalStoreLikeCpp::default();
        let effective_ids = compose_effective_skill_line_ids_like_cpp(
            [100, 101],
            [101, 200],
            [200, 300],
            table_hash,
            &removals,
        );
        let mut store =
            SkillLineStore::from_entries([skill_line(100, 9, 0, 0), skill_line(101, 9, 0, 0)]);
        store.effective_record_ids_like_cpp = effective_ids;

        assert_eq!(store.effective_record_count_like_cpp(), 4);
        assert!(store.contains_effective_record_like_cpp(100));
        assert!(store.contains_effective_record_like_cpp(200));
        assert!(store.contains_effective_record_like_cpp(300));
        assert!(
            store.get(200).is_none(),
            "SQL-only identity must not manufacture a hydrated payload"
        );
    }

    #[test]
    fn primary_profession_classification_applies_official_then_custom_sql_collisions() {
        let effective_ids = HashSet::from([100, 200, 300]);
        let fields = compose_effective_skill_line_acquisition_fields_like_cpp(
            [
                (
                    100,
                    SkillLineAcquisitionFieldsLikeCpp {
                        category_id: 9,
                        parent_skill_line_id: 0,
                        parent_tier_index: 0,
                    },
                ),
                (
                    200,
                    SkillLineAcquisitionFieldsLikeCpp {
                        category_id: 11,
                        parent_skill_line_id: 0,
                        parent_tier_index: 0,
                    },
                ),
            ],
            [
                (
                    100,
                    SkillLineAcquisitionFieldsLikeCpp {
                        category_id: 11,
                        parent_skill_line_id: 0,
                        parent_tier_index: 0,
                    },
                ),
                (
                    300,
                    SkillLineAcquisitionFieldsLikeCpp {
                        category_id: 11,
                        parent_skill_line_id: 0,
                        parent_tier_index: 0,
                    },
                ),
            ],
            [(
                200,
                SkillLineAcquisitionFieldsLikeCpp {
                    category_id: 11,
                    parent_skill_line_id: 100,
                    parent_tier_index: 4,
                },
            )],
            &effective_ids,
        );
        let mut store =
            SkillLineStore::from_entries([skill_line(100, 9, 0, 0), skill_line(200, 11, 0, 0)]);
        store.effective_record_ids_like_cpp = effective_ids;
        store.acquisition_fields_by_effective_record_like_cpp = fields;

        assert_eq!(
            store.is_primary_profession_skill_like_cpp(100),
            Some(true),
            "official SQL must replace stale WDC4 category/parent fields"
        );
        assert_eq!(
            store.is_primary_profession_skill_like_cpp(200),
            Some(false),
            "custom SQL must replace the official/WDC4 classification"
        );
        assert_eq!(
            store.is_primary_profession_skill_like_cpp(300),
            Some(true),
            "SQL-only rows are decidable when the exact required fields are hydrated"
        );
        assert_eq!(
            store.acquisition_payload_like_cpp(200),
            SkillLineAcquisitionPayloadLikeCpp::Complete(SkillLineAcquisitionFieldsLikeCpp {
                category_id: 11,
                parent_skill_line_id: 100,
                parent_tier_index: 4,
            })
        );
        assert_eq!(
            store
                .acquisition_children_for_parent_like_cpp(100)
                .collect::<Vec<_>>(),
            vec![(
                200,
                SkillLineAcquisitionPayloadLikeCpp::Complete(SkillLineAcquisitionFieldsLikeCpp {
                    category_id: 11,
                    parent_skill_line_id: 100,
                    parent_tier_index: 4,
                },)
            )],
            "SetSkill parent activation must see final SQL parent/tier payload"
        );
        assert_eq!(
            store.profession_skill_for_exp_like_cpp(100, 0),
            200,
            "profession expansion lookup must use the effective parent/tier payload"
        );
        assert!(store.has_complete_acquisition_payload_like_cpp());
        assert!(
            store.get(300).is_none(),
            "classification hydration must not fabricate the remaining SkillLine payload"
        );
    }

    #[test]
    fn parent_child_projection_retains_unhydrated_effective_identities() {
        let store = SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
            [
                skill_line(100, 11, 0, 0),
                skill_line(200, 11, 100, 4),
                skill_line(300, 11, 999, 4),
            ],
            [100, 150, 200, 300],
        );

        assert_eq!(
            store
                .acquisition_children_for_parent_like_cpp(100)
                .collect::<Vec<_>>(),
            vec![
                (150, SkillLineAcquisitionPayloadLikeCpp::Incomplete),
                (
                    200,
                    SkillLineAcquisitionPayloadLikeCpp::Complete(
                        SkillLineAcquisitionFieldsLikeCpp {
                            category_id: 11,
                            parent_skill_line_id: 100,
                            parent_tier_index: 4,
                        },
                    ),
                ),
            ],
            "an effective identity with unknown parentage must not disappear"
        );
        assert_eq!(
            store
                .acquisition_children_for_parent_like_cpp(999)
                .collect::<Vec<_>>(),
            vec![
                (150, SkillLineAcquisitionPayloadLikeCpp::Incomplete),
                (
                    300,
                    SkillLineAcquisitionPayloadLikeCpp::Complete(
                        SkillLineAcquisitionFieldsLikeCpp {
                            category_id: 11,
                            parent_skill_line_id: 999,
                            parent_tier_index: 4,
                        },
                    ),
                ),
            ],
        );
    }

    #[test]
    fn final_invalid_skill_line_overlay_replaces_stale_payload_and_can_be_repaired_or_removed() {
        let root = SkillLineAcquisitionFieldsLikeCpp {
            category_id: 11,
            parent_skill_line_id: 0,
            parent_tier_index: 0,
        };
        let repaired_child = SkillLineAcquisitionFieldsLikeCpp {
            category_id: 11,
            parent_skill_line_id: 200,
            parent_tier_index: 4,
        };
        let effective_ids = HashSet::from([100, 200, 300]);
        let fields = compose_effective_skill_line_acquisition_payloads_like_cpp(
            [(100, root), (200, root), (300, root), (400, root)],
            [(100, None), (200, None), (400, None)],
            [(200, Some(repaired_child)), (300, None)],
            &effective_ids,
        );
        let mut store = SkillLineStore::from_entries([
            skill_line(100, 11, 0, 0),
            skill_line(200, 11, 0, 0),
            skill_line(300, 11, 0, 0),
            skill_line(400, 11, 0, 0),
        ]);
        store.effective_record_ids_like_cpp = effective_ids;
        store.acquisition_fields_by_effective_record_like_cpp = fields;

        assert_eq!(
            store.acquisition_payload_like_cpp(100),
            SkillLineAcquisitionPayloadLikeCpp::Incomplete,
            "an invalid official overlay must replace the stale WDC4 payload"
        );
        assert_eq!(
            store.acquisition_payload_like_cpp(200),
            SkillLineAcquisitionPayloadLikeCpp::Complete(repaired_child),
            "a valid custom overlay must repair an invalid official payload"
        );
        assert_eq!(
            store.acquisition_payload_like_cpp(300),
            SkillLineAcquisitionPayloadLikeCpp::Incomplete,
            "an invalid final custom overlay must replace the stale WDC4 payload"
        );
        assert_eq!(
            store.acquisition_payload_like_cpp(400),
            SkillLineAcquisitionPayloadLikeCpp::Absent,
            "a final removal must erase even an invalid SQL overlay identity"
        );
        assert!(!store.has_complete_acquisition_payload_like_cpp());
    }

    #[test]
    fn typed_hotfix_application_retains_invalid_identity_and_custom_repairs_payload() {
        const TABLE_HASH: u32 = 0x51A1_0001;
        let mut base = SkillLineStore::from_entries([skill_line(100, 11, 0, 0)]);
        base.table_hash_like_cpp = Some(TABLE_HASH);
        let store = base
            .apply_hotfix_overlays_like_cpp(
                [SkillLineHotfixOverlayLikeCpp {
                    id: 200,
                    category_id: i128::from(i8::MAX) + 1,
                    parent_skill_line_id: 100,
                    parent_tier_index: 4,
                }],
                [SkillLineHotfixOverlayLikeCpp {
                    id: 200,
                    category_id: 11,
                    parent_skill_line_id: 100,
                    parent_tier_index: 4,
                }],
                &Db2HotfixRemovalStoreLikeCpp::default(),
            )
            .unwrap();

        assert!(store.contains_effective_record_like_cpp(200));
        assert_eq!(
            store.acquisition_payload_like_cpp(200),
            SkillLineAcquisitionPayloadLikeCpp::Complete(SkillLineAcquisitionFieldsLikeCpp {
                category_id: 11,
                parent_skill_line_id: 100,
                parent_tier_index: 4,
            })
        );
    }

    #[test]
    fn effective_skill_line_ids_apply_only_final_matching_table_removals() {
        let table_hash = 0xB53D_C9D6;
        let other_table_hash = 0xAABB_CCDD;
        let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
            (table_hash, 100, 2),
            (table_hash, 100, 1),
            (table_hash, 101, 1),
            (table_hash, 101, 2),
            (other_table_hash, 102, 2),
            (table_hash, 200, 2),
        ]);

        let effective_ids = compose_effective_skill_line_ids_like_cpp(
            [100, 101, 102],
            [200],
            [],
            table_hash,
            &removals,
        );

        assert!(
            effective_ids.contains(&100),
            "a later non-removal status cancels the earlier removal"
        );
        assert!(!effective_ids.contains(&101));
        assert!(
            effective_ids.contains(&102),
            "another DB2 table hash must not erase SkillLine"
        );
        assert!(
            !effective_ids.contains(&200),
            "final removals also erase SQL-only records"
        );
    }

    #[test]
    fn trainer_validation_uses_effective_skill_line_identity_instead_of_payload() {
        let table_hash = 0xB53D_C9D6;
        let removals =
            Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(table_hash, 100, 2)]);
        let effective_ids =
            compose_effective_skill_line_ids_like_cpp([100], [200], [], table_hash, &removals);
        let mut skill_lines = SkillLineStore::from_entries([skill_line(100, 9, 0, 0)]);
        skill_lines.effective_record_ids_like_cpp = effective_ids;

        let trainer = crate::trainer::TrainerStoreLikeCpp::from_rows_like_cpp(
            [crate::trainer::TrainerRowLikeCpp {
                id: 10,
                trainer_type: crate::trainer::TRAINER_TYPE_TRADESKILL_LIKE_CPP,
                greeting: String::new(),
            }],
            [
                crate::trainer::TrainerSpellRowLikeCpp {
                    trainer_id: 10,
                    spell: crate::trainer::TrainerSpellLikeCpp {
                        spell_id: 1_000,
                        money_cost: 0,
                        req_skill_line: 100,
                        req_skill_rank: 0,
                        req_ability: [0; 3],
                        req_level: 0,
                    },
                },
                crate::trainer::TrainerSpellRowLikeCpp {
                    trainer_id: 10,
                    spell: crate::trainer::TrainerSpellLikeCpp {
                        spell_id: 1_001,
                        money_cost: 0,
                        req_skill_line: 200,
                        req_skill_rank: 0,
                        req_ability: [0; 3],
                        req_level: 0,
                    },
                },
            ],
            [],
            [],
            |_| true,
            |skill_line_id| skill_lines.contains_effective_record_like_cpp(skill_line_id),
            |_| true,
            |_, _| true,
        );

        let loaded = trainer.store.get_trainer_like_cpp(10).unwrap();
        assert!(
            loaded.get_spell_like_cpp(1_000).is_none(),
            "a hydrated SkillLine removed by hotfix_data is not a valid trainer requirement"
        );
        assert!(
            loaded.get_spell_like_cpp(1_001).is_some(),
            "an SQL-only effective SkillLine identity is valid without fabricated payload"
        );
        assert_eq!(
            trainer.report.skipped_spells_missing_skill_line,
            vec![(10, 1_000, 100)]
        );
        assert!(skill_lines.get(200).is_none());
    }

    #[test]
    fn load_skill_talent_db2_subbatch_when_fixtures_exist() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
        if !dbc_dir.exists() {
            eprintln!(
                "Skipping test: DB2 fixture directory not found at {}",
                dbc_dir.display()
            );
            return;
        }

        macro_rules! load_if_exists {
            ($file:literal, $store:ty) => {
                if dbc_dir.join($file).exists() {
                    let _store = <$store>::load(data_dir, locale)
                        .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
                }
            };
        }

        load_if_exists!("GlyphBindableSpell.db2", GlyphBindableSpellStore);
        load_if_exists!("GlyphProperties.db2", GlyphPropertiesStore);
        load_if_exists!("GlyphRequiredSpec.db2", GlyphRequiredSpecStore);
        load_if_exists!("GlyphSlot.db2", GlyphSlotStore);
        load_if_exists!("JournalEncounter.db2", JournalEncounterStore);
        load_if_exists!("JournalEncounterSection.db2", JournalEncounterSectionStore);
        load_if_exists!("JournalInstance.db2", JournalInstanceStore);
        load_if_exists!("JournalTier.db2", JournalTierStore);
        load_if_exists!("PvpSeason.db2", PvpSeasonStore);
        load_if_exists!("PvpTalent.db2", PvpTalentStore);
        load_if_exists!("PvpTalentCategory.db2", PvpTalentCategoryStore);
        load_if_exists!("PvpTalentSlotUnlock.db2", PvpTalentSlotUnlockStore);
        load_if_exists!("PvpTier.db2", PvpTierStore);
        if dbc_dir.join("SkillLine.db2").exists() {
            let store = SkillLineStore::load(data_dir, locale)
                .unwrap_or_else(|error| panic!("failed to load SkillLine.db2: {error:#}"));
            assert_eq!(
                store.table_hash_like_cpp(),
                Some(0xB53D_C9D6),
                "the 3.4.3 fixture must expose the WDC4 table hash, not layout hash 0x5CB7F941"
            );
        }
        load_if_exists!("SkillLineXTraitTree.db2", SkillLineXTraitTreeStore);
        load_if_exists!("Talent.db2", TalentStore);
        load_if_exists!("TalentTab.db2", TalentTabStore);
    }
}
