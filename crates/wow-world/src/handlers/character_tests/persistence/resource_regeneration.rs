//! Resource-regeneration scenarios and their shared local fixtures.

use super::*;

fn mana_power_type_store_like_cpp(
    regen_peace: f32,
    regen_combat: f32,
) -> wow_data::character_progression::PowerTypeStore {
    wow_data::character_progression::PowerTypeStore::from_entries([
        wow_data::character_progression::PowerTypeEntry {
            id: 0,
            name_global_string_tag: String::new(),
            cost_global_string_tag: String::new(),
            power_type_enum: PowerType::Mana as i8,
            min_power: 0,
            max_base_power: 0,
            center_power: 0,
            default_power: 0,
            display_modifier: 1,
            regen_interrupt_time_ms: 0,
            regen_peace,
            regen_combat,
            flags: 0,
        },
    ])
}

#[path = "resource_regeneration/health_and_power_regeneration.rs"]
mod health_and_power_regeneration;
#[path = "resource_regeneration/mana_and_food_emotes.rs"]
mod mana_and_food_emotes;
