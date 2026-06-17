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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerChoiceLoadReportLikeCpp {
    pub choice_rows_seen: usize,
    pub response_rows_seen: usize,
    /// C++ `responseCount`; increments only for responses attached to an existing choice.
    pub loaded_responses: usize,
    pub skipped_responses_missing_choice: Vec<(i32, i32)>,
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
        let mut choices = HashMap::new();
        let mut report = PlayerChoiceLoadReportLikeCpp {
            rewards_pending: true,
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
            });
            report.loaded_responses += 1;
        }

        PlayerChoiceLoadOutcomeLikeCpp {
            store: Self { choices },
            report,
        }
    }

    /// C++ `ObjectMgr::LoadPlayerChoices` core tables.
    ///
    /// This first slice intentionally represents only `playerchoice` and
    /// `playerchoice_response`. Rewards, MawPower, locales, and live
    /// `DisplayPlayerChoice` packet wiring remain tracked work.
    pub async fn load_core_like_cpp(db: &WorldDatabase) -> Result<PlayerChoiceLoadOutcomeLikeCpp> {
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

        Ok(Self::from_rows_like_cpp(choices, responses))
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

    #[test]
    fn player_choices_load_core_fields_and_response_order_like_cpp() {
        let outcome = PlayerChoiceStoreLikeCpp::from_rows_like_cpp(
            [choice(10, "question")],
            [response(10, 200, 2), response(10, 100, 1)],
        );

        assert_eq!(outcome.report.choice_rows_seen, 1);
        assert_eq!(outcome.report.response_rows_seen, 2);
        assert_eq!(outcome.report.loaded_responses, 2);
        assert!(outcome.report.rewards_pending);
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
}
