use super::*;

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

db2_store!(JournalEncounterStore, JournalEncounterEntry);
db2_store!(JournalEncounterSectionStore, JournalEncounterSectionEntry);
db2_store!(JournalInstanceStore, JournalInstanceEntry);
db2_store!(JournalTierStore, JournalTierEntry);

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

impl_from_entries!(JournalEncounterStore, JournalEncounterEntry);
impl_from_entries!(JournalEncounterSectionStore, JournalEncounterSectionEntry);
impl_from_entries!(JournalInstanceStore, JournalInstanceEntry);
impl_from_entries!(JournalTierStore, JournalTierEntry);
