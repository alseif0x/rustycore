// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadPlayerChoices` represented core store.

use std::collections::HashMap;

use wow_constants::shared::Locale;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseLikeCpp {
    pub response_id: i32,
    pub response_identifier: u16,
    pub choice_art_file_id: i32,
    pub flags: i32,
    pub widget_set_id: u32,
    pub ui_texture_atlas_element_id: u32,
    pub sound_kit_id: u32,
    pub group_id: u8,
    pub ui_texture_kit_id: i32,
    pub answer: String,
    pub header: String,
    pub sub_header: String,
    pub button_tooltip: String,
    pub description: String,
    pub confirmation: String,
    pub reward_quest_id: Option<u32>,
    pub reward: Option<PlayerChoiceResponseRewardLikeCpp>,
    pub maw_power: Option<PlayerChoiceResponseMawPowerLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseRewardLikeCpp {
    pub title_id: i32,
    pub package_id: i32,
    pub skill_line_id: i32,
    pub skill_point_count: u32,
    pub arena_point_count: u32,
    pub honor_point_count: u32,
    pub money: u64,
    pub xp: u32,
    pub items: Vec<PlayerChoiceResponseRewardItemLikeCpp>,
    pub currency: Vec<PlayerChoiceResponseRewardEntryLikeCpp>,
    pub faction: Vec<PlayerChoiceResponseRewardEntryLikeCpp>,
    pub item_choices: Vec<PlayerChoiceResponseRewardItemLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseRewardItemLikeCpp {
    pub id: u32,
    pub bonus_list_ids: Vec<i32>,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseRewardEntryLikeCpp {
    pub id: u32,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseMawPowerLikeCpp {
    pub type_art_file_id: i32,
    pub rarity: Option<i32>,
    pub rarity_color: Option<u32>,
    pub spell_id: i32,
    pub max_stacks: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceLikeCpp {
    pub choice_id: i32,
    pub ui_texture_kit_id: i32,
    pub sound_kit_id: u32,
    pub close_sound_kit_id: u32,
    pub duration: i64,
    pub question: String,
    pub pending_choice_text: String,
    pub responses: Vec<PlayerChoiceResponseLikeCpp>,
    pub hide_warboard_header: bool,
    pub keep_open_after_choice: bool,
}

impl PlayerChoiceLikeCpp {
    /// C++ `PlayerChoice::GetResponse`.
    pub fn get_response_like_cpp(&self, response_id: i32) -> Option<&PlayerChoiceResponseLikeCpp> {
        self.responses
            .iter()
            .find(|response| response.response_id == response_id)
    }

    /// C++ `PlayerChoice::GetResponseByIdentifier`.
    pub fn get_response_by_identifier_like_cpp(
        &self,
        response_identifier: u16,
    ) -> Option<&PlayerChoiceResponseLikeCpp> {
        self.responses
            .iter()
            .find(|response| response.response_identifier == response_identifier)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceRowLikeCpp {
    pub choice_id: i32,
    pub ui_texture_kit_id: i32,
    pub sound_kit_id: u32,
    pub close_sound_kit_id: u32,
    pub duration: i64,
    pub question: String,
    pub pending_choice_text: String,
    pub hide_warboard_header: u8,
    pub keep_open_after_choice: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseRowLikeCpp {
    pub choice_id: i32,
    pub response_id: i32,
    pub response_identifier: u16,
    pub choice_art_file_id: i32,
    pub flags: i32,
    pub widget_set_id: u32,
    pub ui_texture_atlas_element_id: u32,
    pub sound_kit_id: u32,
    pub group_id: u8,
    pub ui_texture_kit_id: i32,
    pub answer: String,
    pub header: String,
    pub sub_header: String,
    pub button_tooltip: String,
    pub description: String,
    pub confirmation: String,
    pub reward_quest_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseRewardRowLikeCpp {
    pub choice_id: i32,
    pub response_id: i32,
    pub title_id: i32,
    pub package_id: i32,
    pub skill_line_id: i32,
    pub skill_point_count: u32,
    pub arena_point_count: u32,
    pub honor_point_count: u32,
    pub money: u64,
    pub xp: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseRewardItemRowLikeCpp {
    pub choice_id: i32,
    pub response_id: i32,
    pub item_id: u32,
    pub bonus_list_ids_raw: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseRewardCurrencyRowLikeCpp {
    pub choice_id: i32,
    pub response_id: i32,
    pub currency_id: u32,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseRewardFactionRowLikeCpp {
    pub choice_id: i32,
    pub response_id: i32,
    pub faction_id: u32,
    pub quantity: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseMawPowerRowLikeCpp {
    pub choice_id: i32,
    pub response_id: i32,
    pub type_art_file_id: i32,
    pub rarity: Option<i32>,
    pub rarity_color: Option<u32>,
    pub spell_id: i32,
    pub max_stacks: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceLocaleRowLikeCpp {
    pub choice_id: i32,
    pub locale: String,
    pub question: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseLocaleRowLikeCpp {
    pub choice_id: i32,
    pub response_id: i32,
    pub locale: String,
    pub answer: String,
    pub header: String,
    pub sub_header: String,
    pub button_tooltip: String,
    pub description: String,
    pub confirmation: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerChoiceResponseLocaleLikeCpp {
    pub answer: HashMap<Locale, String>,
    pub header: HashMap<Locale, String>,
    pub sub_header: HashMap<Locale, String>,
    pub button_tooltip: HashMap<Locale, String>,
    pub description: HashMap<Locale, String>,
    pub confirmation: HashMap<Locale, String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerChoiceLocaleLikeCpp {
    pub question: HashMap<Locale, String>,
    pub responses: HashMap<i32, PlayerChoiceResponseLocaleLikeCpp>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerChoiceLoadReportLikeCpp {
    pub choice_rows_seen: usize,
    pub response_rows_seen: usize,
    pub reward_rows_seen: usize,
    pub reward_item_rows_seen: usize,
    pub reward_currency_rows_seen: usize,
    pub reward_faction_rows_seen: usize,
    pub reward_item_choice_rows_seen: usize,
    pub maw_power_rows_seen: usize,
    /// C++ `responseCount`; increments only for responses attached to an existing choice.
    pub loaded_responses: usize,
    /// C++ `rewardCount`; increments only for rewards attached to an existing response.
    pub loaded_rewards: usize,
    /// C++ `itemRewardCount`.
    pub loaded_reward_items: usize,
    /// C++ `currencyRewardCount`.
    pub loaded_reward_currencies: usize,
    /// C++ `factionRewardCount`.
    pub loaded_reward_factions: usize,
    /// C++ `itemChoiceRewardCount`.
    pub loaded_reward_item_choices: usize,
    /// C++ `mawPowersCount`.
    pub loaded_maw_powers: usize,
    pub skipped_responses_missing_choice: Vec<(i32, i32)>,
    pub skipped_rewards_missing_choice: Vec<(i32, i32)>,
    pub skipped_rewards_missing_response: Vec<(i32, i32)>,
    pub skipped_reward_items_missing_choice: Vec<(i32, i32)>,
    pub skipped_reward_items_missing_response: Vec<(i32, i32)>,
    pub skipped_reward_items_missing_reward: Vec<(i32, i32)>,
    pub skipped_reward_items_missing_item: Vec<(i32, i32, u32)>,
    pub skipped_reward_currencies_missing_choice: Vec<(i32, i32)>,
    pub skipped_reward_currencies_missing_response: Vec<(i32, i32)>,
    pub skipped_reward_currencies_missing_reward: Vec<(i32, i32)>,
    pub skipped_reward_currencies_missing_currency: Vec<(i32, i32, u32)>,
    pub skipped_reward_factions_missing_choice: Vec<(i32, i32)>,
    pub skipped_reward_factions_missing_response: Vec<(i32, i32)>,
    pub skipped_reward_factions_missing_reward: Vec<(i32, i32)>,
    pub skipped_reward_factions_missing_faction: Vec<(i32, i32, u32)>,
    pub skipped_reward_item_choices_missing_choice: Vec<(i32, i32)>,
    pub skipped_reward_item_choices_missing_response: Vec<(i32, i32)>,
    pub skipped_reward_item_choices_missing_reward: Vec<(i32, i32)>,
    pub skipped_reward_item_choices_missing_item: Vec<(i32, i32, u32)>,
    pub skipped_maw_powers_missing_choice: Vec<(i32, i32)>,
    pub skipped_maw_powers_missing_response: Vec<(i32, i32)>,
    pub invalid_reward_titles: Vec<(i32, i32, i32)>,
    pub invalid_reward_packages: Vec<(i32, i32, i32)>,
    pub invalid_reward_skill_lines: Vec<(i32, i32, i32)>,
    pub rewards_pending: bool,
    pub locales_pending: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerChoiceLocaleLoadReportLikeCpp {
    pub choice_locale_rows_seen: usize,
    /// C++ logs `_playerChoiceLocales.size()` after `playerchoice_locale`.
    pub loaded_choice_locale_entries: usize,
    pub response_locale_rows_seen: usize,
    /// C++ `count`; increments per accepted `playerchoice_response_locale` row.
    pub loaded_response_locale_rows: usize,
    pub skipped_choice_locales_missing_choice: Vec<(i32, String)>,
    /// C++ checks `_playerChoiceLocales`, not only `_playerChoices`, before response locales.
    pub skipped_response_locales_missing_choice_locale: Vec<(i32, i32, String)>,
    pub skipped_response_locales_missing_response: Vec<(i32, i32, String)>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerChoiceStoreLikeCpp {
    choices: HashMap<i32, PlayerChoiceLikeCpp>,
    locales: HashMap<i32, PlayerChoiceLocaleLikeCpp>,
}

pub struct PlayerChoiceLoadOutcomeLikeCpp {
    pub store: PlayerChoiceStoreLikeCpp,
    pub report: PlayerChoiceLoadReportLikeCpp,
}

impl PlayerChoiceStoreLikeCpp {
    pub fn from_rows_like_cpp(
        choice_rows: impl IntoIterator<Item = PlayerChoiceRowLikeCpp>,
        response_rows: impl IntoIterator<Item = PlayerChoiceResponseRowLikeCpp>,
    ) -> PlayerChoiceLoadOutcomeLikeCpp {
        Self::from_rows_rewards_items_currencies_and_factions_like_cpp(
            choice_rows,
            response_rows,
            [],
            [],
            [],
            [],
            |_| false,
            |_| false,
            |_| false,
            |_| false,
            |_| false,
            |_| false,
        )
    }

    pub fn from_rows_and_rewards_like_cpp(
        choice_rows: impl IntoIterator<Item = PlayerChoiceRowLikeCpp>,
        response_rows: impl IntoIterator<Item = PlayerChoiceResponseRowLikeCpp>,
        reward_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardRowLikeCpp>,
        title_exists: impl Fn(u32) -> bool,
        quest_package_exists: impl Fn(u32) -> bool,
        skill_line_exists: impl Fn(u32) -> bool,
    ) -> PlayerChoiceLoadOutcomeLikeCpp {
        Self::from_rows_rewards_items_currencies_and_factions_like_cpp(
            choice_rows,
            response_rows,
            reward_rows,
            [],
            [],
            [],
            title_exists,
            quest_package_exists,
            skill_line_exists,
            |_| false,
            |_| false,
            |_| false,
        )
    }

    pub fn from_rows_rewards_and_items_like_cpp(
        choice_rows: impl IntoIterator<Item = PlayerChoiceRowLikeCpp>,
        response_rows: impl IntoIterator<Item = PlayerChoiceResponseRowLikeCpp>,
        reward_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardRowLikeCpp>,
        reward_item_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardItemRowLikeCpp>,
        title_exists: impl Fn(u32) -> bool,
        quest_package_exists: impl Fn(u32) -> bool,
        skill_line_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
    ) -> PlayerChoiceLoadOutcomeLikeCpp {
        Self::from_rows_rewards_items_currencies_and_factions_like_cpp(
            choice_rows,
            response_rows,
            reward_rows,
            reward_item_rows,
            [],
            [],
            title_exists,
            quest_package_exists,
            skill_line_exists,
            item_exists,
            |_| false,
            |_| false,
        )
    }

    pub fn from_rows_rewards_items_and_currencies_like_cpp(
        choice_rows: impl IntoIterator<Item = PlayerChoiceRowLikeCpp>,
        response_rows: impl IntoIterator<Item = PlayerChoiceResponseRowLikeCpp>,
        reward_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardRowLikeCpp>,
        reward_item_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardItemRowLikeCpp>,
        reward_currency_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardCurrencyRowLikeCpp>,
        title_exists: impl Fn(u32) -> bool,
        quest_package_exists: impl Fn(u32) -> bool,
        skill_line_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
        currency_exists: impl Fn(u32) -> bool,
    ) -> PlayerChoiceLoadOutcomeLikeCpp {
        Self::from_rows_rewards_items_currencies_and_factions_like_cpp(
            choice_rows,
            response_rows,
            reward_rows,
            reward_item_rows,
            reward_currency_rows,
            [],
            title_exists,
            quest_package_exists,
            skill_line_exists,
            item_exists,
            currency_exists,
            |_| false,
        )
    }

    pub fn from_rows_rewards_items_currencies_factions_and_item_choices_like_cpp(
        choice_rows: impl IntoIterator<Item = PlayerChoiceRowLikeCpp>,
        response_rows: impl IntoIterator<Item = PlayerChoiceResponseRowLikeCpp>,
        reward_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardRowLikeCpp>,
        reward_item_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardItemRowLikeCpp>,
        reward_currency_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardCurrencyRowLikeCpp>,
        reward_faction_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardFactionRowLikeCpp>,
        reward_item_choice_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardItemRowLikeCpp>,
        maw_power_rows: impl IntoIterator<Item = PlayerChoiceResponseMawPowerRowLikeCpp>,
        title_exists: impl Fn(u32) -> bool,
        quest_package_exists: impl Fn(u32) -> bool,
        skill_line_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
        currency_exists: impl Fn(u32) -> bool,
        faction_exists: impl Fn(u32) -> bool,
    ) -> PlayerChoiceLoadOutcomeLikeCpp {
        Self::build_rows_like_cpp(
            choice_rows,
            response_rows,
            reward_rows,
            reward_item_rows,
            reward_currency_rows,
            reward_faction_rows,
            reward_item_choice_rows,
            maw_power_rows,
            title_exists,
            quest_package_exists,
            skill_line_exists,
            item_exists,
            currency_exists,
            faction_exists,
        )
    }

    pub fn from_rows_rewards_items_currencies_and_factions_like_cpp(
        choice_rows: impl IntoIterator<Item = PlayerChoiceRowLikeCpp>,
        response_rows: impl IntoIterator<Item = PlayerChoiceResponseRowLikeCpp>,
        reward_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardRowLikeCpp>,
        reward_item_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardItemRowLikeCpp>,
        reward_currency_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardCurrencyRowLikeCpp>,
        reward_faction_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardFactionRowLikeCpp>,
        title_exists: impl Fn(u32) -> bool,
        quest_package_exists: impl Fn(u32) -> bool,
        skill_line_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
        currency_exists: impl Fn(u32) -> bool,
        faction_exists: impl Fn(u32) -> bool,
    ) -> PlayerChoiceLoadOutcomeLikeCpp {
        Self::build_rows_like_cpp(
            choice_rows,
            response_rows,
            reward_rows,
            reward_item_rows,
            reward_currency_rows,
            reward_faction_rows,
            [],
            [],
            title_exists,
            quest_package_exists,
            skill_line_exists,
            item_exists,
            currency_exists,
            faction_exists,
        )
    }

    fn build_rows_like_cpp(
        choice_rows: impl IntoIterator<Item = PlayerChoiceRowLikeCpp>,
        response_rows: impl IntoIterator<Item = PlayerChoiceResponseRowLikeCpp>,
        reward_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardRowLikeCpp>,
        reward_item_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardItemRowLikeCpp>,
        reward_currency_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardCurrencyRowLikeCpp>,
        reward_faction_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardFactionRowLikeCpp>,
        reward_item_choice_rows: impl IntoIterator<Item = PlayerChoiceResponseRewardItemRowLikeCpp>,
        maw_power_rows: impl IntoIterator<Item = PlayerChoiceResponseMawPowerRowLikeCpp>,
        title_exists: impl Fn(u32) -> bool,
        quest_package_exists: impl Fn(u32) -> bool,
        skill_line_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
        currency_exists: impl Fn(u32) -> bool,
        faction_exists: impl Fn(u32) -> bool,
    ) -> PlayerChoiceLoadOutcomeLikeCpp {
        let mut choices = HashMap::new();
        let mut report = PlayerChoiceLoadReportLikeCpp {
            locales_pending: true,
            ..PlayerChoiceLoadReportLikeCpp::default()
        };

        for row in choice_rows {
            report.choice_rows_seen += 1;
            choices.insert(
                row.choice_id,
                PlayerChoiceLikeCpp {
                    choice_id: row.choice_id,
                    ui_texture_kit_id: row.ui_texture_kit_id,
                    sound_kit_id: row.sound_kit_id,
                    close_sound_kit_id: row.close_sound_kit_id,
                    duration: row.duration,
                    question: row.question,
                    pending_choice_text: row.pending_choice_text,
                    responses: Vec::new(),
                    hide_warboard_header: row.hide_warboard_header != 0,
                    keep_open_after_choice: row.keep_open_after_choice != 0,
                },
            );
        }

        for row in response_rows {
            report.response_rows_seen += 1;
            let Some(choice) = choices.get_mut(&row.choice_id) else {
                report
                    .skipped_responses_missing_choice
                    .push((row.choice_id, row.response_id));
                continue;
            };

            choice.responses.push(PlayerChoiceResponseLikeCpp {
                response_id: row.response_id,
                response_identifier: row.response_identifier,
                choice_art_file_id: row.choice_art_file_id,
                flags: row.flags,
                widget_set_id: row.widget_set_id,
                ui_texture_atlas_element_id: row.ui_texture_atlas_element_id,
                sound_kit_id: row.sound_kit_id,
                group_id: row.group_id,
                ui_texture_kit_id: row.ui_texture_kit_id,
                answer: row.answer,
                header: row.header,
                sub_header: row.sub_header,
                button_tooltip: row.button_tooltip,
                description: row.description,
                confirmation: row.confirmation,
                reward_quest_id: row.reward_quest_id,
                reward: None,
                maw_power: None,
            });
            report.loaded_responses += 1;
        }

        for row in reward_rows {
            report.reward_rows_seen += 1;
            let Some(choice) = choices.get_mut(&row.choice_id) else {
                report
                    .skipped_rewards_missing_choice
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(response) = choice
                .responses
                .iter_mut()
                .find(|response| response.response_id == row.response_id)
            else {
                report
                    .skipped_rewards_missing_response
                    .push((row.choice_id, row.response_id));
                continue;
            };

            let mut reward = PlayerChoiceResponseRewardLikeCpp {
                title_id: row.title_id,
                package_id: row.package_id,
                skill_line_id: row.skill_line_id,
                skill_point_count: row.skill_point_count,
                arena_point_count: row.arena_point_count,
                honor_point_count: row.honor_point_count,
                money: row.money,
                xp: row.xp,
                items: Vec::new(),
                currency: Vec::new(),
                faction: Vec::new(),
                item_choices: Vec::new(),
            };
            report.loaded_rewards += 1;

            if reward.title_id != 0
                && !u32::try_from(reward.title_id)
                    .ok()
                    .is_some_and(&title_exists)
            {
                report.invalid_reward_titles.push((
                    row.choice_id,
                    row.response_id,
                    reward.title_id,
                ));
                reward.title_id = 0;
            }
            if reward.package_id != 0
                && !u32::try_from(reward.package_id)
                    .ok()
                    .is_some_and(&quest_package_exists)
            {
                report.invalid_reward_packages.push((
                    row.choice_id,
                    row.response_id,
                    reward.package_id,
                ));
                reward.package_id = 0;
            }
            if reward.skill_line_id != 0
                && !u32::try_from(reward.skill_line_id)
                    .ok()
                    .is_some_and(&skill_line_exists)
            {
                report.invalid_reward_skill_lines.push((
                    row.choice_id,
                    row.response_id,
                    reward.skill_line_id,
                ));
                reward.skill_line_id = 0;
                reward.skill_point_count = 0;
            }

            response.reward = Some(reward);
        }

        for row in reward_item_rows {
            report.reward_item_rows_seen += 1;
            let Some(choice) = choices.get_mut(&row.choice_id) else {
                report
                    .skipped_reward_items_missing_choice
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(response) = choice
                .responses
                .iter_mut()
                .find(|response| response.response_id == row.response_id)
            else {
                report
                    .skipped_reward_items_missing_response
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(reward) = response.reward.as_mut() else {
                report
                    .skipped_reward_items_missing_reward
                    .push((row.choice_id, row.response_id));
                continue;
            };
            if !item_exists(row.item_id) {
                report.skipped_reward_items_missing_item.push((
                    row.choice_id,
                    row.response_id,
                    row.item_id,
                ));
                continue;
            }

            reward.items.push(PlayerChoiceResponseRewardItemLikeCpp {
                id: row.item_id,
                bonus_list_ids: parse_bonus_list_ids_like_cpp(&row.bonus_list_ids_raw),
                quantity: row.quantity,
            });
            report.loaded_reward_items += 1;
        }

        for row in reward_currency_rows {
            report.reward_currency_rows_seen += 1;
            let Some(choice) = choices.get_mut(&row.choice_id) else {
                report
                    .skipped_reward_currencies_missing_choice
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(response) = choice
                .responses
                .iter_mut()
                .find(|response| response.response_id == row.response_id)
            else {
                report
                    .skipped_reward_currencies_missing_response
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(reward) = response.reward.as_mut() else {
                report
                    .skipped_reward_currencies_missing_reward
                    .push((row.choice_id, row.response_id));
                continue;
            };
            if !currency_exists(row.currency_id) {
                report.skipped_reward_currencies_missing_currency.push((
                    row.choice_id,
                    row.response_id,
                    row.currency_id,
                ));
                continue;
            }

            reward
                .currency
                .push(PlayerChoiceResponseRewardEntryLikeCpp {
                    id: row.currency_id,
                    quantity: row.quantity,
                });
            report.loaded_reward_currencies += 1;
        }

        for row in reward_faction_rows {
            report.reward_faction_rows_seen += 1;
            let Some(choice) = choices.get_mut(&row.choice_id) else {
                report
                    .skipped_reward_factions_missing_choice
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(response) = choice
                .responses
                .iter_mut()
                .find(|response| response.response_id == row.response_id)
            else {
                report
                    .skipped_reward_factions_missing_response
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(reward) = response.reward.as_mut() else {
                report
                    .skipped_reward_factions_missing_reward
                    .push((row.choice_id, row.response_id));
                continue;
            };
            if !faction_exists(row.faction_id) {
                report.skipped_reward_factions_missing_faction.push((
                    row.choice_id,
                    row.response_id,
                    row.faction_id,
                ));
                continue;
            }

            reward.faction.push(PlayerChoiceResponseRewardEntryLikeCpp {
                id: row.faction_id,
                quantity: row.quantity,
            });
            report.loaded_reward_factions += 1;
        }

        for row in reward_item_choice_rows {
            report.reward_item_choice_rows_seen += 1;
            let Some(choice) = choices.get_mut(&row.choice_id) else {
                report
                    .skipped_reward_item_choices_missing_choice
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(response) = choice
                .responses
                .iter_mut()
                .find(|response| response.response_id == row.response_id)
            else {
                report
                    .skipped_reward_item_choices_missing_response
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(reward) = response.reward.as_mut() else {
                report
                    .skipped_reward_item_choices_missing_reward
                    .push((row.choice_id, row.response_id));
                continue;
            };
            if !item_exists(row.item_id) {
                report.skipped_reward_item_choices_missing_item.push((
                    row.choice_id,
                    row.response_id,
                    row.item_id,
                ));
                continue;
            }

            reward
                .item_choices
                .push(PlayerChoiceResponseRewardItemLikeCpp {
                    id: row.item_id,
                    bonus_list_ids: parse_bonus_list_ids_like_cpp(&row.bonus_list_ids_raw),
                    quantity: row.quantity,
                });
            report.loaded_reward_item_choices += 1;
        }

        for row in maw_power_rows {
            report.maw_power_rows_seen += 1;
            let Some(choice) = choices.get_mut(&row.choice_id) else {
                report
                    .skipped_maw_powers_missing_choice
                    .push((row.choice_id, row.response_id));
                continue;
            };
            let Some(response) = choice
                .responses
                .iter_mut()
                .find(|response| response.response_id == row.response_id)
            else {
                report
                    .skipped_maw_powers_missing_response
                    .push((row.choice_id, row.response_id));
                continue;
            };

            response.maw_power = Some(PlayerChoiceResponseMawPowerLikeCpp {
                type_art_file_id: row.type_art_file_id,
                rarity: row.rarity,
                rarity_color: row.rarity_color,
                spell_id: row.spell_id,
                max_stacks: row.max_stacks,
            });
            report.loaded_maw_powers += 1;
        }

        PlayerChoiceLoadOutcomeLikeCpp {
            store: Self {
                choices,
                locales: HashMap::new(),
            },
            report,
        }
    }

    pub fn load_locale_rows_like_cpp(
        &mut self,
        choice_locale_rows: impl IntoIterator<Item = PlayerChoiceLocaleRowLikeCpp>,
        response_locale_rows: impl IntoIterator<Item = PlayerChoiceResponseLocaleRowLikeCpp>,
    ) -> PlayerChoiceLocaleLoadReportLikeCpp {
        self.locales.clear();
        let mut report = PlayerChoiceLocaleLoadReportLikeCpp::default();

        for row in choice_locale_rows {
            report.choice_locale_rows_seen += 1;
            if !self.choices.contains_key(&row.choice_id) {
                report
                    .skipped_choice_locales_missing_choice
                    .push((row.choice_id, row.locale));
                continue;
            }

            let Some(locale) = locale_from_name_like_cpp(&row.locale) else {
                continue;
            };
            if locale == Locale::EnUS {
                continue;
            }

            self.locales
                .entry(row.choice_id)
                .or_default()
                .question
                .insert(locale, row.question);
        }
        report.loaded_choice_locale_entries = self.locales.len();

        for row in response_locale_rows {
            report.response_locale_rows_seen += 1;
            let Some(choice_locale) = self.locales.get_mut(&row.choice_id) else {
                report.skipped_response_locales_missing_choice_locale.push((
                    row.choice_id,
                    row.response_id,
                    row.locale,
                ));
                continue;
            };

            let Some(player_choice) = self.choices.get(&row.choice_id) else {
                report.skipped_response_locales_missing_choice_locale.push((
                    row.choice_id,
                    row.response_id,
                    row.locale,
                ));
                continue;
            };
            if player_choice
                .get_response_like_cpp(row.response_id)
                .is_none()
            {
                report.skipped_response_locales_missing_response.push((
                    row.choice_id,
                    row.response_id,
                    row.locale,
                ));
                continue;
            }

            let Some(locale) = locale_from_name_like_cpp(&row.locale) else {
                continue;
            };
            if locale == Locale::EnUS {
                continue;
            }

            let data = choice_locale.responses.entry(row.response_id).or_default();
            data.answer.insert(locale, row.answer);
            data.header.insert(locale, row.header);
            data.sub_header.insert(locale, row.sub_header);
            data.button_tooltip.insert(locale, row.button_tooltip);
            data.description.insert(locale, row.description);
            data.confirmation.insert(locale, row.confirmation);
            report.loaded_response_locale_rows += 1;
        }

        report
    }

    /// C++ `ObjectMgr::LoadPlayerChoices` core tables and base rewards.
    ///
    /// This slice intentionally stops before locales and live
    /// `DisplayPlayerChoice` packet wiring.
    /// C++ `ObjectMgr::GetPlayerChoice`.
    pub fn get_player_choice_like_cpp(&self, choice_id: i32) -> Option<&PlayerChoiceLikeCpp> {
        self.choices.get(&choice_id)
    }

    /// C++ `ObjectMgr::GetPlayerChoiceLocale`.
    pub fn get_player_choice_locale_like_cpp(
        &self,
        choice_id: i32,
    ) -> Option<&PlayerChoiceLocaleLikeCpp> {
        self.locales.get(&choice_id)
    }

    pub fn len(&self) -> usize {
        self.choices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.choices.is_empty()
    }
}

fn parse_bonus_list_ids_like_cpp(raw: &str) -> Vec<i32> {
    raw.split_whitespace()
        .filter_map(|token| token.parse::<i32>().ok())
        .collect()
}

fn locale_from_name_like_cpp(name: &str) -> Option<Locale> {
    match name {
        "enUS" => Some(Locale::EnUS),
        "koKR" => Some(Locale::KoKR),
        "frFR" => Some(Locale::FrFR),
        "deDE" => Some(Locale::DeDE),
        "zhCN" => Some(Locale::ZhCN),
        "zhTW" => Some(Locale::ZhTW),
        "esES" => Some(Locale::EsES),
        "esMX" => Some(Locale::EsMX),
        "ruRU" => Some(Locale::RuRU),
        "none" => Some(Locale::None),
        "ptBR" => Some(Locale::PtBR),
        "itIT" => Some(Locale::ItIT),
        _ => None,
    }
}

#[cfg(test)]
#[path = "player_choice/tests/mod.rs"]
mod tests;
