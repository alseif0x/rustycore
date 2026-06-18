//! Skill, talent, PvP, glyph and journal DB2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

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
db2_store!(SkillLineStore, SkillLineEntry);
db2_store!(SkillLineXTraitTreeStore, SkillLineXTraitTreeEntry);
db2_store!(TalentStore, TalentEntry);
db2_store!(TalentTabStore, TalentTabEntry);

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
        load_store(data_dir, locale, "SkillLine.db2", |id, idx, r| {
            SkillLineEntry {
                id,
                display_name: r.get_field_string(idx, 0),
                alternate_verb: r.get_field_string(idx, 1),
                description: r.get_field_string(idx, 2),
                horde_display_name: r.get_field_string(idx, 3),
                override_source_info_display_name: r.get_field_string(idx, 4),
                category_id: r.get_field_i8(idx, 6),
                spell_icon_file_id: r.get_field_i32(idx, 7),
                can_link: r.get_field_i8(idx, 8),
                parent_skill_line_id: r.get_field_u32(idx, 9),
                parent_tier_index: r.get_field_i32(idx, 10),
                flags: r.get_field_u16(idx, 11),
                spell_book_spell_id: r.get_field_i32(idx, 12),
            }
        })
    }

    /// C++ `Player::GetProfessionSkillForExp`.
    pub fn profession_skill_for_exp_like_cpp(&self, skill_id: u32, mut expansion: i32) -> u32 {
        const SKILL_CATEGORY_SECONDARY_LIKE_CPP: i8 = 9;
        const SKILL_CATEGORY_PROFESSION_LIKE_CPP: i8 = 11;
        const CURRENT_EXPANSION_LIKE_CPP: i32 = 2;
        const BASE_PARENT_TIER_INDEX_LIKE_CPP: i32 = 4;

        let Some(skill) = self.get(skill_id) else {
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

        self.entries
            .values()
            .find(|child| {
                child.parent_skill_line_id == skill.id
                    && child.parent_tier_index - BASE_PARENT_TIER_INDEX_LIKE_CPP == expansion
            })
            .map(|child| child.id)
            .unwrap_or(0)
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
impl_from_entries!(SkillLineStore, SkillLineEntry);
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
        load_if_exists!("SkillLine.db2", SkillLineStore);
        load_if_exists!("SkillLineXTraitTree.db2", SkillLineXTraitTreeStore);
        load_if_exists!("Talent.db2", TalentStore);
        load_if_exists!("TalentTab.db2", TalentTabStore);
    }
}
