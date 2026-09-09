//! Miscellaneous DB2 readers regression scenarios.
//!
//! Separated from the misc_generated.rs root under #664.

use super::*;

fn adventure_map_poi(id: u32, quest_id: u32, player_condition_id: u32) -> AdventureMapPoiEntry {
    AdventureMapPoiEntry {
        id,
        title: String::new(),
        description: String::new(),
        world_position: [0.0, 0.0],
        poi_type: 0,
        player_condition_id,
        quest_id,
        lfg_dungeon_id: 0,
        reward_item_id: 0,
        ui_texture_atlas_member_id: 0,
        ui_texture_kit_id: 0,
        map_id: 0,
        area_table_id: 0,
    }
}

mod scenarios_1;
