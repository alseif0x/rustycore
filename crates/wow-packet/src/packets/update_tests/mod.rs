//! Object update-block regressions.
//!
//! Separated from the update_tests.rs root under #640.

//! Behaviour tests for [`super`].
//!
//! Extracted from `update.rs`, which was 13,439 lines of which
//! 3,704 — 28% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::*;

fn test_item_data(mask: u64) -> ItemDataValuesDeltaUpdate {
    ItemDataValuesDeltaUpdate {
        changed_object_type_mask: VALUES_TYPE_ITEM,
        object_data: None,
        item_data_mask: mask,
        artifact_powers: Vec::new(),
        artifact_powers_update_mask: None,
        gems: Vec::new(),
        gems_update_mask: None,
        owner: ObjectGuid::EMPTY,
        contained_in: ObjectGuid::EMPTY,
        creator: ObjectGuid::EMPTY,
        gift_creator: ObjectGuid::EMPTY,
        stack_count: 5,
        expiration: 0,
        dynamic_flags: 0,
        property_seed: 0,
        random_properties_id: 0,
        durability: 0,
        max_durability: 0,
        create_played_time: 0,
        context: 0,
        create_time: 0,
        artifact_xp: 0,
        item_appearance_mod_id: 0,
        modifiers: ItemModListValuesUpdate {
            item_mod_list_mask: 0,
            values: Vec::new(),
            values_update_mask: None,
        },
        dynamic_flags2: 0,
        item_bonus_key: ItemBonusKeyValuesUpdate::default(),
        debug_item_level: 0,
        spell_charges: [0; 5],
        enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
    }
}

/// Regression for the real 3.4.3.54261 client world-entry crash: the
/// ActivePlayerData stats VALUES update must NOT mask or write field bit 50
/// (ShieldBlockCritPercentage). That field is reserved in the 54261 client
/// grammar (oracle `hp_ObjectUpdateBuilder.cs:567,841` skip it); emitting it
/// shifted every following field by +4 bytes and desynced the client.

/// A fully-zeroed `PlayerStatChanges` for focused serialization tests.
fn zeroed_stat_changes() -> PlayerStatChanges {
    PlayerStatChanges {
        health: 0,
        max_health: 0,
        min_damage: 0.0,
        max_damage: 0.0,
        base_mana: 0,
        base_health: 0,
        attack_power: 0,
        attack_power_mod_pos: 0,
        attack_power_mod_neg: 0,
        attack_power_multiplier: 0.0,
        ranged_attack_power: 0,
        ranged_attack_power_mod_pos: 0,
        ranged_attack_power_mod_neg: 0,
        ranged_attack_power_multiplier: 0.0,
        min_ranged_damage: 0.0,
        max_ranged_damage: 0.0,
        power0: 0,
        max_power0: 0,
        stats: [0; 5],
        stat_pos_buff: [0; 5],
        stat_neg_buff: [0; 5],
        armor: 0,
        combat_ratings: [0; 32],
        spell_power: 0,
        block_pct: 0.0,
        dodge_pct: 0.0,
        parry_pct: 0.0,
        crit_pct: 0.0,
        ranged_crit_pct: 0.0,
        spell_crit_pct: [0.0; 7],
        mana_regen: 0.0,
        mana_regen_combat: 0.0,
        mana_regen_mp5: 0.0,
        mainhand_expertise: 0.0,
        offhand_expertise: 0.0,
        ranged_expertise: 0.0,
        combat_rating_expertise: 0.0,
        dodge_from_attr: 0.0,
        parry_from_attr: 0.0,
        offhand_crit_pct: 0.0,
        shield_block: 0,
        shield_block_crit_pct: 0.0,
        mod_healing_pct: 0.0,
        mod_healing_done_pct: 0.0,
        mod_periodic_healing_pct: 0.0,
        mod_spell_power_pct: 0.0,
    }
}

fn set_active_player_bit(data: &mut ActivePlayerDataValuesUpdate, bit: usize) {
    data.active_player_data_mask[bit / 32] |= 1 << (bit % 32);
}

fn test_player_create_data_with_farsight(farsight_object: ObjectGuid) -> PlayerCreateData {
    PlayerCreateData {
        guid: ObjectGuid::create_player(1, 42),
        wow_account: ObjectGuid::EMPTY,
        bnet_account: ObjectGuid::EMPTY,
        race: 1,
        class: 1,
        sex: 0,
        level: 1,
        display_id: 49,
        native_display_id: 49,
        health: 100,
        max_health: 100,
        faction_template: PlayerCreateData::faction_for_race(1),
        current_area_id: 12,
        player_flags: 0,
        player_flags_ex: 0,
        stats: [0; 5],
        stat_pos_buff: [0; 5],
        stat_neg_buff: [0; 5],
        base_armor: 0,
        base_mana: 0,
        max_mana: 0,
        current_power0: 1000,
        attack_power: 0,
        attack_power_mod_pos: 0,
        ranged_attack_power: 0,
        ranged_attack_power_mod_pos: 0,
        min_damage: 1.0,
        max_damage: 2.0,
        min_ranged_damage: 0.0,
        max_ranged_damage: 0.0,
        block_pct: 0.0,
        dodge_pct: 0.0,
        dodge_from_attr: 0.0,
        parry_pct: 0.0,
        parry_from_attr: 0.0,
        crit_pct: 5.0,
        ranged_crit_pct: 5.0,
        offhand_crit_pct: 5.0,
        spell_crit_pct: [5.0; 7],
        combat_ratings: [0; 32],
        spell_power: 0,
        visible_items: [(0, 0, 0); 19],
        customizations: Vec::new(),
        inv_slots: [ObjectGuid::EMPTY; 141],
        farsight_object,
        action_buttons: [0; MAX_ACTION_BUTTONS],
        skill_info: Vec::new(),
        quest_log: Vec::new(),
        party_type: [0; 2],
        coinage: 0,
        xp: 0,
        next_level_xp: 400,
        max_level: 80,
        scaling_player_level_delta: 0,
        rest_info: [
            RestInfoValuesUpdate {
                rest_info_mask: 0x07,
                threshold: 0,
                state_id: 2,
            },
            RestInfoValuesUpdate {
                rest_info_mask: 0x07,
                threshold: 0,
                state_id: 2,
            },
        ],
        watched_faction_index: -1,
        heirlooms: Vec::new(),
        heirloom_flags: Vec::new(),
        toys: Vec::new(),
        transmog: Vec::new(),
        trait_configs: Vec::new(),
    }
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
mod scenarios_4;
mod scenarios_5;
