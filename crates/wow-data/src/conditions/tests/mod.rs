//! Condition entry model and load reporting regression scenarios.
//!
//! Separated from the conditions.rs root under #638.

use super::*;

fn spell_info(spell_id: i32) -> crate::SpellInfo {
    crate::SpellInfo {
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

fn quest_template(
    id: u32,
    objectives: Vec<crate::quest::QuestObjective>,
) -> crate::quest::QuestTemplate {
    crate::quest::QuestTemplate {
        id,
        quest_type: 2,
        quest_level: 1,
        quest_max_scaling_level: 0,
        quest_package_id: 0,
        min_level: 1,
        quest_sort_id: 0,
        quest_info_id: 0,
        suggested_group_num: 0,
        reward_next_quest: 0,
        reward_xp_difficulty: 0,
        reward_xp_multiplier: 1.0,
        reward_money_difficulty: 0,
        reward_money_multiplier: 1.0,
        reward_bonus_money: 0,
        reward_display_spell: [0; crate::quest::QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; crate::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; crate::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; crate::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; crate::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_flags: 0,
        source_item_id: 0,
        source_item_count: 0,
        source_spell_id: 0,
        limit_time_secs: 0,
        expansion: 0,
        flags: 0,
        flags_ex: 0,
        flags_ex2: 0,
        special_flags: 0,
        event_id_for_quest: 0,
        reward_items: [0; crate::quest::QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; crate::quest::QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; crate::quest::QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; crate::quest::QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; crate::quest::QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; crate::quest::QUEST_ITEM_DROP_COUNT],
        log_title: String::new(),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives,
        allowable_races: 0,
        allowable_classes: 0,
        max_level: 0,
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        breadcrumb_for_quest_id: 0,
        dependent_previous_quests: Vec::new(),
        dependent_breadcrumb_quests: Vec::new(),
        required_min_rep_faction: 0,
        required_min_rep_value: 0,
        required_max_rep_faction: 0,
        required_max_rep_value: 0,
        required_skill_id: 0,
        required_skill_points: 0,
        reward_choice_items: [(0, 0); crate::quest::QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; crate::quest::QUEST_REWARD_CHOICES_COUNT],
    }
}

fn quest_objective(id: u32, obj_type: u8, amount: i32) -> crate::quest::QuestObjective {
    crate::quest::QuestObjective {
        id,
        quest_id: 42,
        obj_type,
        order: 0,
        storage_index: 0,
        object_id: 0,
        amount,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    }
}

fn condition_row(
    source_type: ConditionSourceType,
    condition_type: ConditionType,
) -> ConditionDbRowLikeCpp {
    ConditionDbRowLikeCpp {
        source_type_or_reference_id: source_type as i32,
        source_group: 0,
        source_entry: 0,
        source_id: 0,
        else_group: 0,
        condition_type_or_reference: condition_type as i32,
        condition_target: 0,
        condition_value1: 0,
        condition_value2: 0,
        condition_value3: 0,
        condition_string_value1: String::new(),
        negative_condition: false,
        error_type: 0,
        error_text_id: 0,
        script_name: String::new(),
    }
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
mod scenarios_4;
