//! Entries packets.
//!
//! Separated from mod.rs under #709.

use std::sync::Arc;
use wow_constants::{ConditionSourceType, ConditionType, shared::DifficultyFlags};
use wow_data::{
    Condition, ConditionEntriesByTypeStore, DifficultyEntry, GraveyardStore, MapEntry, SpellInfo,
};

pub(super) fn currency_entry(id: u32) -> wow_data::CurrencyTypesEntry {
    wow_data::CurrencyTypesEntry {
        id,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }
}

pub(super) fn difficulty_entry(
    id: u32,
    instance_type: u8,
    flags: DifficultyFlags,
) -> DifficultyEntry {
    difficulty_entry_with_toggle(id, instance_type, flags, 0)
}

pub(super) fn difficulty_entry_with_toggle(
    id: u32,
    instance_type: u8,
    flags: DifficultyFlags,
    toggle_difficulty_id: u8,
) -> DifficultyEntry {
    DifficultyEntry {
        id,
        instance_type,
        flags: flags.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id,
    }
}

pub(super) fn map_entry(id: u32, instance_type: i8) -> MapEntry {
    MapEntry {
        id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }
}

pub(super) fn area_entry(id: u32, parent_area_id: u16, flags: u32) -> wow_data::AreaTableEntry {
    wow_data::AreaTableEntry {
        id,
        continent_id: 571,
        parent_area_id,
        area_bit: -1,
        exploration_level: 0,
        mount_flags: 0,
        flags,
    }
}

pub(super) fn graveyard_store_with_links(
    zone_id: u32,
    safe_loc_ids: impl IntoIterator<Item = u32>,
    conditions: impl IntoIterator<Item = Condition>,
) -> (Arc<GraveyardStore>, Arc<ConditionEntriesByTypeStore>) {
    let mut graveyard_store = GraveyardStore::default();
    for safe_loc_id in safe_loc_ids {
        graveyard_store.add_graveyard_link_like_cpp(safe_loc_id, zone_id);
    }
    let condition_store = Arc::new(ConditionEntriesByTypeStore::from_conditions_like_cpp(
        conditions,
    ));
    graveyard_store.attach_graveyard_conditions_like_cpp(condition_store.as_ref());
    (Arc::new(graveyard_store), condition_store)
}

pub(super) fn graveyard_team_condition(zone_id: u32, safe_loc_id: u32, team: u32) -> Condition {
    Condition {
        source_type: ConditionSourceType::Graveyard,
        source_group: zone_id,
        source_entry: safe_loc_id as i32,
        condition_type: ConditionType::Team,
        condition_value1: team,
        ..Condition::default()
    }
}

pub(super) fn battleground_queue_id_like_cpp(
    battlemaster_list_id: u16,
    queue_type: u8,
    rated: bool,
    team_size: u8,
) -> u64 {
    u64::from(battlemaster_list_id)
        | (u64::from(queue_type & 0x0F) << 16)
        | (u64::from(u8::from(rated)) << 20)
        | (u64::from(team_size & 0x3F) << 24)
        | 0x1F10_0000_0000_0000
}

pub(super) fn battlemaster_entry_like_cpp(
    id: u32,
    instance_type: i8,
    flags: i8,
) -> wow_data::BattlemasterListEntry {
    wow_data::BattlemasterListEntry {
        id,
        instance_type,
        holiday_world_state: 0,
        flags,
    }
}

pub(super) fn trade_test_spell_info(spell_id: i32) -> SpellInfo {
    SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

pub(super) fn cuf_profile(name: &str, frame_height: u16) -> wow_packet::packets::misc::CufProfile {
    wow_packet::packets::misc::CufProfile {
        profile_name: name.to_string(),
        frame_height,
        frame_width: 128,
        sort_by: 2,
        health_text: 3,
        top_point: 4,
        bottom_point: 5,
        left_point: 6,
        top_offset: 7,
        bottom_offset: 8,
        left_offset: 9,
        bool_options: (1 << 0) | (1 << 26),
    }
}
