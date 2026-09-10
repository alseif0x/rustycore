//! Player-choice store regressions.
//!
//! Moved out of player_choice.rs under #683; every test is unchanged.

use super::*;

#[test]
fn player_choices_load_core_fields_and_response_order_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_like_cpp(
        [choice(10, "question")],
        [response(10, 200, 2), response(10, 100, 1)],
    );

    assert_eq!(outcome.report.choice_rows_seen, 1);
    assert_eq!(outcome.report.response_rows_seen, 2);
    assert_eq!(outcome.report.loaded_responses, 2);
    assert_eq!(outcome.report.loaded_rewards, 0);
    assert!(outcome.report.locales_pending);

    let loaded = outcome.store.get_player_choice_like_cpp(10).unwrap();
    assert_eq!(loaded.choice_id, 10);
    assert_eq!(loaded.ui_texture_kit_id, 11);
    assert_eq!(loaded.sound_kit_id, 22);
    assert_eq!(loaded.close_sound_kit_id, 33);
    assert_eq!(loaded.duration, 44);
    assert_eq!(loaded.question, "question");
    assert_eq!(loaded.pending_choice_text, "pending");
    assert!(loaded.hide_warboard_header);
    assert!(!loaded.keep_open_after_choice);
    assert_eq!(loaded.responses[0].response_id, 200);
    assert_eq!(loaded.responses[1].response_id, 100);
    assert_eq!(
        loaded
            .get_response_like_cpp(100)
            .unwrap()
            .response_identifier,
        1
    );
    assert_eq!(
        loaded
            .get_response_by_identifier_like_cpp(2)
            .unwrap()
            .answer,
        "answer 200"
    );
}

#[test]
fn player_choices_skip_responses_with_missing_choice_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_like_cpp(
        [choice(1, "kept")],
        [response(99, 7, 1), response(1, 8, 2)],
    );

    assert_eq!(outcome.report.response_rows_seen, 2);
    assert_eq!(outcome.report.loaded_responses, 1);
    assert_eq!(outcome.report.skipped_responses_missing_choice, [(99, 7)]);
    assert_eq!(
        outcome
            .store
            .get_player_choice_like_cpp(1)
            .unwrap()
            .responses
            .len(),
        1
    );
}

#[test]
fn player_choices_duplicate_choice_id_overwrites_base_row_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_like_cpp(
        [choice(1, "first"), choice(1, "second")],
        [response(1, 77, 9)],
    );

    let loaded = outcome.store.get_player_choice_like_cpp(1).unwrap();
    assert_eq!(loaded.question, "second");
    assert_eq!(loaded.responses.len(), 1);
}

#[test]
fn player_choices_attach_and_validate_base_rewards_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_and_rewards_like_cpp(
        [choice(1, "rewarded")],
        [response(1, 10, 1)],
        [reward(1, 10)],
        |id| id == 100,
        |id| id == 200,
        |id| id == 300,
    );

    assert_eq!(outcome.report.reward_rows_seen, 1);
    assert_eq!(outcome.report.loaded_rewards, 1);
    let reward = outcome
        .store
        .get_player_choice_like_cpp(1)
        .unwrap()
        .get_response_like_cpp(10)
        .unwrap()
        .reward
        .as_ref()
        .unwrap();
    assert_eq!(reward.title_id, 100);
    assert_eq!(reward.package_id, 200);
    assert_eq!(reward.skill_line_id, 300);
    assert_eq!(reward.skill_point_count, 4);
    assert_eq!(reward.money, 7);
    assert_eq!(reward.xp, 8);
    assert!(reward.items.is_empty());
    assert!(reward.currency.is_empty());
    assert!(reward.faction.is_empty());
    assert!(reward.item_choices.is_empty());
}

#[test]
fn player_choices_skip_rewards_with_missing_choice_or_response_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_and_rewards_like_cpp(
        [choice(1, "rewarded")],
        [response(1, 10, 1)],
        [reward(99, 10), reward(1, 77), reward(1, 10)],
        |_| true,
        |_| true,
        |_| true,
    );

    assert_eq!(outcome.report.reward_rows_seen, 3);
    assert_eq!(outcome.report.loaded_rewards, 1);
    assert_eq!(outcome.report.skipped_rewards_missing_choice, [(99, 10)]);
    assert_eq!(outcome.report.skipped_rewards_missing_response, [(1, 77)]);
}

#[test]
fn player_choices_zero_invalid_reward_references_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_and_rewards_like_cpp(
        [choice(1, "rewarded")],
        [response(1, 10, 1)],
        [reward(1, 10)],
        |_| false,
        |_| false,
        |_| false,
    );

    let reward = outcome
        .store
        .get_player_choice_like_cpp(1)
        .unwrap()
        .get_response_like_cpp(10)
        .unwrap()
        .reward
        .as_ref()
        .unwrap();
    assert_eq!(reward.title_id, 0);
    assert_eq!(reward.package_id, 0);
    assert_eq!(reward.skill_line_id, 0);
    assert_eq!(reward.skill_point_count, 0);
    assert_eq!(outcome.report.invalid_reward_titles, [(1, 10, 100)]);
    assert_eq!(outcome.report.invalid_reward_packages, [(1, 10, 200)]);
    assert_eq!(outcome.report.invalid_reward_skill_lines, [(1, 10, 300)]);
}

#[test]
fn player_choices_duplicate_reward_overwrites_like_cpp_emplace() {
    let mut second = reward(1, 10);
    second.title_id = 101;
    second.package_id = 201;

    let outcome = PlayerChoiceStoreLikeCpp::from_rows_and_rewards_like_cpp(
        [choice(1, "rewarded")],
        [response(1, 10, 1)],
        [reward(1, 10), second],
        |_| true,
        |_| true,
        |_| true,
    );

    assert_eq!(outcome.report.loaded_rewards, 2);
    let reward = outcome
        .store
        .get_player_choice_like_cpp(1)
        .unwrap()
        .get_response_like_cpp(10)
        .unwrap()
        .reward
        .as_ref()
        .unwrap();
    assert_eq!(reward.title_id, 101);
    assert_eq!(reward.package_id, 201);
}

#[test]
fn player_choices_attach_reward_items_and_parse_bonus_lists_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_rewards_and_items_like_cpp(
        [choice(1, "rewarded")],
        [response(1, 10, 1)],
        [reward(1, 10)],
        [
            reward_item(1, 10, 700, "7 bad -9 7 0x10 12"),
            reward_item(1, 10, 701, ""),
        ],
        |_| true,
        |_| true,
        |_| true,
        |item_id| item_id == 700 || item_id == 701,
    );

    assert_eq!(outcome.report.reward_item_rows_seen, 2);
    assert_eq!(outcome.report.loaded_reward_items, 2);
    let items = &outcome
        .store
        .get_player_choice_like_cpp(1)
        .unwrap()
        .get_response_like_cpp(10)
        .unwrap()
        .reward
        .as_ref()
        .unwrap()
        .items;
    assert_eq!(items[0].id, 700);
    assert_eq!(items[0].bonus_list_ids, [7, -9, 7, 12]);
    assert_eq!(items[0].quantity, 3);
    assert_eq!(items[1].id, 701);
    assert!(items[1].bonus_list_ids.is_empty());
}

#[test]
fn player_choices_skip_reward_items_with_missing_refs_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_rewards_and_items_like_cpp(
        [choice(1, "rewarded"), choice(2, "no reward")],
        [response(1, 10, 1), response(2, 20, 2)],
        [reward(1, 10)],
        [
            reward_item(99, 10, 700, ""),
            reward_item(1, 77, 700, ""),
            reward_item(2, 20, 700, ""),
            reward_item(1, 10, 999, ""),
            reward_item(1, 10, 700, ""),
        ],
        |_| true,
        |_| true,
        |_| true,
        |item_id| item_id == 700,
    );

    assert_eq!(outcome.report.reward_item_rows_seen, 5);
    assert_eq!(outcome.report.loaded_reward_items, 1);
    assert_eq!(
        outcome.report.skipped_reward_items_missing_choice,
        [(99, 10)]
    );
    assert_eq!(
        outcome.report.skipped_reward_items_missing_response,
        [(1, 77)]
    );
    assert_eq!(
        outcome.report.skipped_reward_items_missing_reward,
        [(2, 20)]
    );
    assert_eq!(
        outcome.report.skipped_reward_items_missing_item,
        [(1, 10, 999)]
    );
}

#[test]
fn player_choices_attach_reward_currencies_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_rewards_items_and_currencies_like_cpp(
        [choice(1, "rewarded")],
        [response(1, 10, 1)],
        [reward(1, 10)],
        [],
        [reward_currency(1, 10, 777), reward_currency(1, 10, 778)],
        |_| true,
        |_| true,
        |_| true,
        |_| true,
        |currency_id| currency_id == 777 || currency_id == 778,
    );

    assert_eq!(outcome.report.reward_currency_rows_seen, 2);
    assert_eq!(outcome.report.loaded_reward_currencies, 2);
    let currency = &outcome
        .store
        .get_player_choice_like_cpp(1)
        .unwrap()
        .get_response_like_cpp(10)
        .unwrap()
        .reward
        .as_ref()
        .unwrap()
        .currency;
    assert_eq!(currency[0].id, 777);
    assert_eq!(currency[0].quantity, 5);
    assert_eq!(currency[1].id, 778);
}

#[test]
fn player_choices_skip_reward_currencies_with_missing_refs_like_cpp() {
    let outcome = PlayerChoiceStoreLikeCpp::from_rows_rewards_items_and_currencies_like_cpp(
        [choice(1, "rewarded"), choice(2, "no reward")],
        [response(1, 10, 1), response(2, 20, 2)],
        [reward(1, 10)],
        [],
        [
            reward_currency(99, 10, 777),
            reward_currency(1, 77, 777),
            reward_currency(2, 20, 777),
            reward_currency(1, 10, 999),
            reward_currency(1, 10, 777),
        ],
        |_| true,
        |_| true,
        |_| true,
        |_| true,
        |currency_id| currency_id == 777,
    );

    assert_eq!(outcome.report.reward_currency_rows_seen, 5);
    assert_eq!(outcome.report.loaded_reward_currencies, 1);
    assert_eq!(
        outcome.report.skipped_reward_currencies_missing_choice,
        [(99, 10)]
    );
    assert_eq!(
        outcome.report.skipped_reward_currencies_missing_response,
        [(1, 77)]
    );
    assert_eq!(
        outcome.report.skipped_reward_currencies_missing_reward,
        [(2, 20)]
    );
    assert_eq!(
        outcome.report.skipped_reward_currencies_missing_currency,
        [(1, 10, 999)]
    );
}

#[test]
fn player_choices_attach_reward_factions_like_cpp() {
    let outcome =
        PlayerChoiceStoreLikeCpp::from_rows_rewards_items_currencies_and_factions_like_cpp(
            [choice(1, "rewarded")],
            [response(1, 10, 1)],
            [reward(1, 10)],
            [],
            [],
            [reward_faction(1, 10, 777), reward_faction(1, 10, 778)],
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |faction_id| faction_id == 777 || faction_id == 778,
        );

    assert_eq!(outcome.report.reward_faction_rows_seen, 2);
    assert_eq!(outcome.report.loaded_reward_factions, 2);
    let faction = &outcome
        .store
        .get_player_choice_like_cpp(1)
        .unwrap()
        .get_response_like_cpp(10)
        .unwrap()
        .reward
        .as_ref()
        .unwrap()
        .faction;
    assert_eq!(faction[0].id, 777);
    assert_eq!(faction[0].quantity, 6);
    assert_eq!(faction[1].id, 778);
}

#[test]
fn player_choices_skip_reward_factions_with_missing_refs_like_cpp() {
    let outcome =
        PlayerChoiceStoreLikeCpp::from_rows_rewards_items_currencies_and_factions_like_cpp(
            [choice(1, "rewarded"), choice(2, "no reward")],
            [response(1, 10, 1), response(2, 20, 2)],
            [reward(1, 10)],
            [],
            [],
            [
                reward_faction(99, 10, 777),
                reward_faction(1, 77, 777),
                reward_faction(2, 20, 777),
                reward_faction(1, 10, 999),
                reward_faction(1, 10, 777),
            ],
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |faction_id| faction_id == 777,
        );

    assert_eq!(outcome.report.reward_faction_rows_seen, 5);
    assert_eq!(outcome.report.loaded_reward_factions, 1);
    assert_eq!(
        outcome.report.skipped_reward_factions_missing_choice,
        [(99, 10)]
    );
    assert_eq!(
        outcome.report.skipped_reward_factions_missing_response,
        [(1, 77)]
    );
    assert_eq!(
        outcome.report.skipped_reward_factions_missing_reward,
        [(2, 20)]
    );
    assert_eq!(
        outcome.report.skipped_reward_factions_missing_faction,
        [(1, 10, 999)]
    );
}

#[test]
fn player_choices_attach_reward_item_choices_and_parse_bonus_lists_like_cpp() {
    let outcome =
        PlayerChoiceStoreLikeCpp::from_rows_rewards_items_currencies_factions_and_item_choices_like_cpp(
            [choice(1, "rewarded")],
            [response(1, 10, 1)],
            [reward(1, 10)],
            [],
            [],
            [],
            [
                reward_item(1, 10, 700, "7 bad -9 7 0x10 12"),
                reward_item(1, 10, 701, ""),
            ],
            [],
            |_| true,
            |_| true,
            |_| true,
            |item_id| item_id == 700 || item_id == 701,
            |_| true,
            |_| true,
        );

    assert_eq!(outcome.report.reward_item_choice_rows_seen, 2);
    assert_eq!(outcome.report.loaded_reward_item_choices, 2);
    let item_choices = &outcome
        .store
        .get_player_choice_like_cpp(1)
        .unwrap()
        .get_response_like_cpp(10)
        .unwrap()
        .reward
        .as_ref()
        .unwrap()
        .item_choices;
    assert_eq!(item_choices[0].id, 700);
    assert_eq!(item_choices[0].bonus_list_ids, [7, -9, 7, 12]);
    assert_eq!(item_choices[0].quantity, 3);
    assert_eq!(item_choices[1].id, 701);
    assert!(item_choices[1].bonus_list_ids.is_empty());
}

#[test]
fn player_choices_skip_reward_item_choices_with_missing_refs_like_cpp() {
    let outcome =
        PlayerChoiceStoreLikeCpp::from_rows_rewards_items_currencies_factions_and_item_choices_like_cpp(
            [choice(1, "rewarded"), choice(2, "no reward")],
            [response(1, 10, 1), response(2, 20, 2)],
            [reward(1, 10)],
            [],
            [],
            [],
            [
                reward_item(99, 10, 700, ""),
                reward_item(1, 77, 700, ""),
                reward_item(2, 20, 700, ""),
                reward_item(1, 10, 999, ""),
                reward_item(1, 10, 700, ""),
            ],
            [],
            |_| true,
            |_| true,
            |_| true,
            |item_id| item_id == 700,
            |_| true,
            |_| true,
        );

    assert_eq!(outcome.report.reward_item_choice_rows_seen, 5);
    assert_eq!(outcome.report.loaded_reward_item_choices, 1);
    assert_eq!(
        outcome.report.skipped_reward_item_choices_missing_choice,
        [(99, 10)]
    );
    assert_eq!(
        outcome.report.skipped_reward_item_choices_missing_response,
        [(1, 77)]
    );
    assert_eq!(
        outcome.report.skipped_reward_item_choices_missing_reward,
        [(2, 20)]
    );
    assert_eq!(
        outcome.report.skipped_reward_item_choices_missing_item,
        [(1, 10, 999)]
    );
}

#[test]
fn player_choices_attach_maw_power_like_cpp() {
    let outcome =
        PlayerChoiceStoreLikeCpp::from_rows_rewards_items_currencies_factions_and_item_choices_like_cpp(
            [choice(1, "maw")],
            [response(1, 10, 1)],
            [],
            [],
            [],
            [],
            [],
            [
                maw_power(1, 10, Some(3), Some(0x00ff00)),
                maw_power(1, 10, None, None),
            ],
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
        );

    assert_eq!(outcome.report.maw_power_rows_seen, 2);
    assert_eq!(outcome.report.loaded_maw_powers, 2);
    let maw_power = outcome
        .store
        .get_player_choice_like_cpp(1)
        .unwrap()
        .get_response_like_cpp(10)
        .unwrap()
        .maw_power
        .as_ref()
        .unwrap();
    assert_eq!(maw_power.type_art_file_id, 11);
    assert_eq!(maw_power.rarity, None);
    assert_eq!(maw_power.rarity_color, None);
    assert_eq!(maw_power.spell_id, 22);
    assert_eq!(maw_power.max_stacks, 33);
}

#[test]
fn player_choices_skip_maw_power_with_missing_refs_like_cpp() {
    let outcome =
        PlayerChoiceStoreLikeCpp::from_rows_rewards_items_currencies_factions_and_item_choices_like_cpp(
            [choice(1, "maw")],
            [response(1, 10, 1)],
            [],
            [],
            [],
            [],
            [],
            [
                maw_power(99, 10, Some(3), Some(0x00ff00)),
                maw_power(1, 77, Some(3), Some(0x00ff00)),
                maw_power(1, 10, Some(3), Some(0x00ff00)),
            ],
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
        );

    assert_eq!(outcome.report.maw_power_rows_seen, 3);
    assert_eq!(outcome.report.loaded_maw_powers, 1);
    assert_eq!(outcome.report.skipped_maw_powers_missing_choice, [(99, 10)]);
    assert_eq!(
        outcome.report.skipped_maw_powers_missing_response,
        [(1, 77)]
    );
}

#[test]
fn player_choices_load_locales_like_cpp() {
    let mut outcome = PlayerChoiceStoreLikeCpp::from_rows_like_cpp(
        [choice(1, "Question"), choice(2, "Question 2")],
        [response(1, 10, 1), response(2, 20, 2)],
    );

    let report = outcome.store.load_locale_rows_like_cpp(
        [
            choice_locale(1, "esES", "Pregunta"),
            choice_locale(1, "enUS", "Ignored"),
            choice_locale(2, "bad", "Ignored"),
        ],
        [
            response_locale(1, 10, "esES"),
            response_locale(1, 10, "enUS"),
        ],
    );

    assert_eq!(report.choice_locale_rows_seen, 3);
    assert_eq!(report.loaded_choice_locale_entries, 1);
    assert_eq!(report.response_locale_rows_seen, 2);
    assert_eq!(report.loaded_response_locale_rows, 1);

    let locale = outcome.store.get_player_choice_locale_like_cpp(1).unwrap();
    assert_eq!(locale.question.get(&Locale::EsES).unwrap(), "Pregunta");
    assert!(!locale.question.contains_key(&Locale::EnUS));
    let response_locale = locale.responses.get(&10).unwrap();
    assert_eq!(
        response_locale.answer.get(&Locale::EsES).unwrap(),
        "answer esES"
    );
    assert_eq!(
        response_locale.header.get(&Locale::EsES).unwrap(),
        "header esES"
    );
    assert_eq!(
        response_locale.confirmation.get(&Locale::EsES).unwrap(),
        "confirm esES"
    );
}

#[test]
fn player_choices_skip_locales_with_missing_refs_like_cpp() {
    let mut outcome = PlayerChoiceStoreLikeCpp::from_rows_like_cpp(
        [choice(1, "Question"), choice(2, "Question 2")],
        [response(1, 10, 1), response(2, 20, 2)],
    );

    let report = outcome.store.load_locale_rows_like_cpp(
        [
            choice_locale(99, "esES", "Missing"),
            choice_locale(1, "esES", "Pregunta"),
        ],
        [
            response_locale(2, 20, "esES"),
            response_locale(1, 77, "esES"),
            response_locale(1, 10, "esES"),
        ],
    );

    assert_eq!(
        report.skipped_choice_locales_missing_choice,
        [(99, "esES".to_string())]
    );
    assert_eq!(
        report.skipped_response_locales_missing_choice_locale,
        [(2, 20, "esES".to_string())]
    );
    assert_eq!(
        report.skipped_response_locales_missing_response,
        [(1, 77, "esES".to_string())]
    );
    assert_eq!(report.loaded_response_locale_rows, 1);
}
