//! Player-choice store regressions.
//!
//! Separated from player_choice.rs under #683.

use super::super::*;

use super::*;

fn choice(choice_id: i32, question: &str) -> PlayerChoiceRowLikeCpp {
    PlayerChoiceRowLikeCpp {
        choice_id,
        ui_texture_kit_id: 11,
        sound_kit_id: 22,
        close_sound_kit_id: 33,
        duration: 44,
        question: question.to_string(),
        pending_choice_text: "pending".to_string(),
        hide_warboard_header: 1,
        keep_open_after_choice: 0,
    }
}

fn response(
    choice_id: i32,
    response_id: i32,
    response_identifier: u16,
) -> PlayerChoiceResponseRowLikeCpp {
    PlayerChoiceResponseRowLikeCpp {
        choice_id,
        response_id,
        response_identifier,
        choice_art_file_id: 1,
        flags: 2,
        widget_set_id: 3,
        ui_texture_atlas_element_id: 4,
        sound_kit_id: 5,
        group_id: 6,
        ui_texture_kit_id: 7,
        answer: format!("answer {response_id}"),
        header: "header".to_string(),
        sub_header: "sub".to_string(),
        button_tooltip: "tip".to_string(),
        description: "desc".to_string(),
        confirmation: "confirm".to_string(),
        reward_quest_id: Some(42),
    }
}

fn reward(choice_id: i32, response_id: i32) -> PlayerChoiceResponseRewardRowLikeCpp {
    PlayerChoiceResponseRewardRowLikeCpp {
        choice_id,
        response_id,
        title_id: 100,
        package_id: 200,
        skill_line_id: 300,
        skill_point_count: 4,
        arena_point_count: 5,
        honor_point_count: 6,
        money: 7,
        xp: 8,
    }
}

fn reward_item(
    choice_id: i32,
    response_id: i32,
    item_id: u32,
    bonus_list_ids_raw: &str,
) -> PlayerChoiceResponseRewardItemRowLikeCpp {
    PlayerChoiceResponseRewardItemRowLikeCpp {
        choice_id,
        response_id,
        item_id,
        bonus_list_ids_raw: bonus_list_ids_raw.to_string(),
        quantity: 3,
    }
}

fn reward_currency(
    choice_id: i32,
    response_id: i32,
    currency_id: u32,
) -> PlayerChoiceResponseRewardCurrencyRowLikeCpp {
    PlayerChoiceResponseRewardCurrencyRowLikeCpp {
        choice_id,
        response_id,
        currency_id,
        quantity: 5,
    }
}

fn reward_faction(
    choice_id: i32,
    response_id: i32,
    faction_id: u32,
) -> PlayerChoiceResponseRewardFactionRowLikeCpp {
    PlayerChoiceResponseRewardFactionRowLikeCpp {
        choice_id,
        response_id,
        faction_id,
        quantity: 6,
    }
}

fn maw_power(
    choice_id: i32,
    response_id: i32,
    rarity: Option<i32>,
    rarity_color: Option<u32>,
) -> PlayerChoiceResponseMawPowerRowLikeCpp {
    PlayerChoiceResponseMawPowerRowLikeCpp {
        choice_id,
        response_id,
        type_art_file_id: 11,
        rarity,
        rarity_color,
        spell_id: 22,
        max_stacks: 33,
    }
}

fn choice_locale(choice_id: i32, locale: &str, question: &str) -> PlayerChoiceLocaleRowLikeCpp {
    PlayerChoiceLocaleRowLikeCpp {
        choice_id,
        locale: locale.to_string(),
        question: question.to_string(),
    }
}

fn response_locale(
    choice_id: i32,
    response_id: i32,
    locale: &str,
) -> PlayerChoiceResponseLocaleRowLikeCpp {
    PlayerChoiceResponseLocaleRowLikeCpp {
        choice_id,
        response_id,
        locale: locale.to_string(),
        answer: format!("answer {locale}"),
        header: format!("header {locale}"),
        sub_header: format!("sub {locale}"),
        button_tooltip: format!("tip {locale}"),
        description: format!("desc {locale}"),
        confirmation: format!("confirm {locale}"),
    }
}

mod scenarios;
