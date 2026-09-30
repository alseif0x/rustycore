//! Adventure Journal and map POI DB2 entries and stores.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct AdventureJournalEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub button_text: String,
    pub reward_description: String,
    pub continue_description: String,
    pub journal_type: u8,
    pub player_condition_id: u32,
    pub flags: i32,
    pub button_action_type: u8,
    pub texture_file_data_id: i32,
    pub lfg_dungeon_id: u16,
    pub quest_id: u32,
    pub battle_master_list_id: u16,
    pub priority_min: u8,
    pub priority_max: u8,
    pub item_id: i32,
    pub item_quantity: u32,
    pub currency_type: u16,
    pub currency_quantity: u8,
    pub ui_map_id: u16,
    pub bonus_player_condition_id: [u32; 2],
    pub bonus_value: [u8; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdventureMapPoiEntry {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub world_position: [f32; 2],
    pub poi_type: i8,
    pub player_condition_id: u32,
    pub quest_id: u32,
    pub lfg_dungeon_id: u32,
    pub reward_item_id: i32,
    pub ui_texture_atlas_member_id: u32,
    pub ui_texture_kit_id: u32,
    pub map_id: i32,
    pub area_table_id: u32,
}

db2_store!(AdventureJournalStore, AdventureJournalEntry);
db2_store!(AdventureMapPoiStore, AdventureMapPoiEntry);

impl AdventureJournalStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "AdventureJournal.db2", |id, idx, r| {
            AdventureJournalEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                button_text: r.get_field_string(idx, 2),
                reward_description: r.get_field_string(idx, 3),
                continue_description: r.get_field_string(idx, 4),
                journal_type: r.get_field_u8(idx, 5),
                player_condition_id: r.get_field_u32(idx, 6),
                flags: r.get_field_i32(idx, 7),
                button_action_type: r.get_field_u8(idx, 8),
                texture_file_data_id: r.get_field_i32(idx, 9),
                lfg_dungeon_id: r.get_field_u16(idx, 10),
                quest_id: r.get_field_u32(idx, 11),
                battle_master_list_id: r.get_field_u16(idx, 12),
                priority_min: r.get_field_u8(idx, 13),
                priority_max: r.get_field_u8(idx, 14),
                item_id: r.get_field_i32(idx, 15),
                item_quantity: r.get_field_u32(idx, 16),
                currency_type: r.get_field_u16(idx, 17),
                currency_quantity: r.get_field_u8(idx, 18),
                ui_map_id: r.get_field_u16(idx, 19),
                bonus_player_condition_id: std::array::from_fn(|i| {
                    r.get_array_element(idx, 20, i, 32)
                }),
                bonus_value: std::array::from_fn(|i| r.get_array_element(idx, 21, i, 8) as u8),
            }
        })
    }
}

impl AdventureMapPoiStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "AdventureMapPOI.db2", |id, idx, r| {
            AdventureMapPoiEntry {
                id,
                title: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                world_position: f32_array::<2>(r, idx, 2),
                poi_type: r.get_field_i8(idx, 3),
                player_condition_id: r.get_field_u32(idx, 4),
                quest_id: r.get_field_u32(idx, 5),
                lfg_dungeon_id: r.get_field_u32(idx, 6),
                reward_item_id: r.get_field_i32(idx, 7),
                ui_texture_atlas_member_id: r.get_field_u32(idx, 8),
                ui_texture_kit_id: r.get_field_u32(idx, 9),
                map_id: r.get_field_i32(idx, 10),
                area_table_id: r.get_field_u32(idx, 11),
            }
        })
    }

    /// C++ `sAdventureMapPOIStore` scan used by `HandleAdventureMapStartQuest`.
    pub fn find_start_quest_poi_like_cpp(
        &self,
        quest_id: u32,
        mut meets_player_condition: impl FnMut(u32) -> bool,
    ) -> Option<&AdventureMapPoiEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by_key(|entry| entry.id);
        entries.into_iter().find(|entry| {
            entry.quest_id == quest_id && meets_player_condition(entry.player_condition_id)
        })
    }
}

impl_from_entries!(AdventureJournalStore, AdventureJournalEntry);
impl_from_entries!(AdventureMapPoiStore, AdventureMapPoiEntry);
