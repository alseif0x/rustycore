use super::*;
use wow_constants::quest::*;
use wow_constants::quest::{
    QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY as QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
    QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM as QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
};

#[test]
fn quest_giver_choose_reward_choice_validation_matches_loaded_cpp_type() {
    let mut quest = quest_template(7002);
    quest.reward_choice_items[0] = (19019, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    quest.reward_choice_items[1] = (392, 5);
    quest.reward_choice_item_types[1] = QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP;

    assert!(represented_reward_choice_matches_loaded_type_for_test(
        &quest,
        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
        19019,
        1,
    ));
    assert!(represented_reward_choice_matches_loaded_type_for_test(
        &quest,
        QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
        392,
        5,
    ));
    assert!(!represented_reward_choice_matches_loaded_type_for_test(
        &quest,
        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
        392,
        5,
    ));
}

fn quest_template(id: u32) -> QuestTemplate {
    QuestTemplate {
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
        reward_display_spell: [0; QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; QUEST_REWARD_REPUTATIONS_COUNT],
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
        reward_items: [0; QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; QUEST_ITEM_DROP_COUNT],
        log_title: format!("Quest {id}"),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
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
        reward_choice_items: [(0, 0); QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; QUEST_REWARD_CHOICES_COUNT],
    }
}

fn represented_reward_choice_matches_loaded_type_for_test(
    quest: &QuestTemplate,
    loot_item_type: u8,
    item_id: u32,
    _quantity: i32,
) -> bool {
    quest.reward_choice_matches_loaded_type(loot_item_type, item_id)
}

#[test]
fn loaded_reward_choice_ignores_quantity_and_rejects_empty_slots() {
    let mut quest = quest_template(7002);
    quest.reward_choice_items[0] = (19019, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    assert!(represented_reward_choice_matches_loaded_type_for_test(
        &quest,
        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
        19019,
        -1,
    ));
    assert!(represented_reward_choice_matches_loaded_type_for_test(
        &quest,
        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
        19019,
        i32::MAX,
    ));
    assert!(!quest.reward_choice_matches_loaded_type(QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP, 0));
    assert!(
        !quest.reward_choice_matches_loaded_type(
            QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
            19019
        )
    );
}

#[path = "tests/level_requirements.rs"]
mod level_requirements;
