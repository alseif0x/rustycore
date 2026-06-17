// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadPlayerChoices` represented core store.

use std::collections::HashMap;

use anyhow::Result;
use wow_database::{WorldDatabase, WorldStatements};

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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerChoiceLoadReportLikeCpp {
    pub choice_rows_seen: usize,
    pub response_rows_seen: usize,
    pub reward_rows_seen: usize,
    pub reward_item_rows_seen: usize,
    pub reward_currency_rows_seen: usize,
    pub reward_faction_rows_seen: usize,
    pub reward_item_choice_rows_seen: usize,
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
    pub invalid_reward_titles: Vec<(i32, i32, i32)>,
    pub invalid_reward_packages: Vec<(i32, i32, i32)>,
    pub invalid_reward_skill_lines: Vec<(i32, i32, i32)>,
    pub rewards_pending: bool,
    pub locales_pending: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerChoiceStoreLikeCpp {
    choices: HashMap<i32, PlayerChoiceLikeCpp>,
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

        PlayerChoiceLoadOutcomeLikeCpp {
            store: Self { choices },
            report,
        }
    }

    /// C++ `ObjectMgr::LoadPlayerChoices` core tables and base rewards.
    ///
    /// This slice intentionally stops before reward item choices, MawPower,
    /// locales, and live `DisplayPlayerChoice` packet wiring.
    pub async fn load_core_like_cpp(
        db: &WorldDatabase,
        title_exists: impl Fn(u32) -> bool,
        quest_package_exists: impl Fn(u32) -> bool,
        skill_line_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
        currency_exists: impl Fn(u32) -> bool,
        faction_exists: impl Fn(u32) -> bool,
    ) -> Result<PlayerChoiceLoadOutcomeLikeCpp> {
        let mut choice_result = db
            .query(&db.prepare(WorldStatements::SEL_PLAYER_CHOICES))
            .await?;
        let mut choices = Vec::new();

        if !choice_result.is_empty() {
            loop {
                choices.push(PlayerChoiceRowLikeCpp {
                    choice_id: choice_result.read(0),
                    ui_texture_kit_id: choice_result.read(1),
                    sound_kit_id: choice_result.read(2),
                    close_sound_kit_id: choice_result.read(3),
                    duration: choice_result.read(4),
                    question: choice_result.read_string(5),
                    pending_choice_text: choice_result.read_string(6),
                    hide_warboard_header: choice_result.read(7),
                    keep_open_after_choice: choice_result.read(8),
                });

                if !choice_result.next_row() {
                    break;
                }
            }
        }

        let mut response_result = db
            .query(&db.prepare(WorldStatements::SEL_PLAYER_CHOICE_RESPONSES))
            .await?;
        let mut responses = Vec::new();

        if !response_result.is_empty() {
            loop {
                responses.push(PlayerChoiceResponseRowLikeCpp {
                    choice_id: response_result.read(0),
                    response_id: response_result.read(1),
                    response_identifier: response_result.read(2),
                    choice_art_file_id: response_result.read(3),
                    flags: response_result.read(4),
                    widget_set_id: response_result.read(5),
                    ui_texture_atlas_element_id: response_result.read(6),
                    sound_kit_id: response_result.read(7),
                    group_id: response_result.read(8),
                    ui_texture_kit_id: response_result.read(9),
                    answer: response_result.read_string(10),
                    header: response_result.read_string(11),
                    sub_header: response_result.read_string(12),
                    button_tooltip: response_result.read_string(13),
                    description: response_result.read_string(14),
                    confirmation: response_result.read_string(15),
                    reward_quest_id: if response_result.is_null(16) {
                        None
                    } else {
                        Some(response_result.read(16))
                    },
                });

                if !response_result.next_row() {
                    break;
                }
            }
        }

        let mut reward_result = db
            .query(&db.prepare(WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARDS))
            .await?;
        let mut rewards = Vec::new();

        if !reward_result.is_empty() {
            loop {
                rewards.push(PlayerChoiceResponseRewardRowLikeCpp {
                    choice_id: reward_result.read(0),
                    response_id: reward_result.read(1),
                    title_id: reward_result.read(2),
                    package_id: reward_result.read(3),
                    skill_line_id: reward_result.read(4),
                    skill_point_count: reward_result.read(5),
                    arena_point_count: reward_result.read(6),
                    honor_point_count: reward_result.read(7),
                    money: reward_result.read(8),
                    xp: reward_result.read(9),
                });

                if !reward_result.next_row() {
                    break;
                }
            }
        }

        let mut reward_item_result = db
            .query(&db.prepare(WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEMS))
            .await?;
        let mut reward_items = Vec::new();

        if !reward_item_result.is_empty() {
            loop {
                reward_items.push(PlayerChoiceResponseRewardItemRowLikeCpp {
                    choice_id: reward_item_result.read(0),
                    response_id: reward_item_result.read(1),
                    item_id: reward_item_result.read(2),
                    bonus_list_ids_raw: reward_item_result.read_string(3),
                    quantity: reward_item_result.read(4),
                });

                if !reward_item_result.next_row() {
                    break;
                }
            }
        }

        let mut reward_currency_result = db
            .query(&db.prepare(WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_CURRENCIES))
            .await?;
        let mut reward_currencies = Vec::new();

        if !reward_currency_result.is_empty() {
            loop {
                reward_currencies.push(PlayerChoiceResponseRewardCurrencyRowLikeCpp {
                    choice_id: reward_currency_result.read(0),
                    response_id: reward_currency_result.read(1),
                    currency_id: reward_currency_result.read(2),
                    quantity: reward_currency_result.read(3),
                });

                if !reward_currency_result.next_row() {
                    break;
                }
            }
        }

        let mut reward_faction_result = db
            .query(&db.prepare(WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_FACTIONS))
            .await?;
        let mut reward_factions = Vec::new();

        if !reward_faction_result.is_empty() {
            loop {
                reward_factions.push(PlayerChoiceResponseRewardFactionRowLikeCpp {
                    choice_id: reward_faction_result.read(0),
                    response_id: reward_faction_result.read(1),
                    faction_id: reward_faction_result.read(2),
                    quantity: reward_faction_result.read(3),
                });

                if !reward_faction_result.next_row() {
                    break;
                }
            }
        }

        let mut reward_item_choice_result = db
            .query(&db.prepare(WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEM_CHOICES))
            .await?;
        let mut reward_item_choices = Vec::new();

        if !reward_item_choice_result.is_empty() {
            loop {
                reward_item_choices.push(PlayerChoiceResponseRewardItemRowLikeCpp {
                    choice_id: reward_item_choice_result.read(0),
                    response_id: reward_item_choice_result.read(1),
                    item_id: reward_item_choice_result.read(2),
                    bonus_list_ids_raw: reward_item_choice_result.read_string(3),
                    quantity: reward_item_choice_result.read(4),
                });

                if !reward_item_choice_result.next_row() {
                    break;
                }
            }
        }

        Ok(
            Self::from_rows_rewards_items_currencies_factions_and_item_choices_like_cpp(
                choices,
                responses,
                rewards,
                reward_items,
                reward_currencies,
                reward_factions,
                reward_item_choices,
                title_exists,
                quest_package_exists,
                skill_line_exists,
                item_exists,
                currency_exists,
                faction_exists,
            ),
        )
    }

    /// C++ `ObjectMgr::GetPlayerChoice`.
    pub fn get_player_choice_like_cpp(&self, choice_id: i32) -> Option<&PlayerChoiceLikeCpp> {
        self.choices.get(&choice_id)
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

#[cfg(test)]
mod tests {
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
}
