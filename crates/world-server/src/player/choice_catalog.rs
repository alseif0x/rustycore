//! Composition adapter between SQLx-free PlayerChoice persistence rows and
//! the immutable `wow-data` catalog owner.

use anyhow::{Result, bail};
use wow_persistence::{
    PlayerChoiceCatalogLoadOutcomeLikeCpp, PlayerChoiceCatalogPersistencePortLikeCpp,
};

pub async fn load_core_like_cpp(
    persistence: &dyn PlayerChoiceCatalogPersistencePortLikeCpp,
    title_exists: impl Fn(u32) -> bool,
    quest_package_exists: impl Fn(u32) -> bool,
    skill_line_exists: impl Fn(u32) -> bool,
    item_exists: impl Fn(u32) -> bool,
    currency_exists: impl Fn(u32) -> bool,
    faction_exists: impl Fn(u32) -> bool,
) -> Result<wow_data::PlayerChoiceLoadOutcomeLikeCpp> {
    let rows = match persistence.load_core_rows_like_cpp().await {
        PlayerChoiceCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        PlayerChoiceCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };

    Ok(
        wow_data::PlayerChoiceStoreLikeCpp::from_rows_rewards_items_currencies_factions_and_item_choices_like_cpp(
            rows.choices.into_iter().map(|row| wow_data::PlayerChoiceRowLikeCpp {
                choice_id: row.choice_id,
                ui_texture_kit_id: row.ui_texture_kit_id,
                sound_kit_id: row.sound_kit_id,
                close_sound_kit_id: row.close_sound_kit_id,
                duration: row.duration,
                question: row.question,
                pending_choice_text: row.pending_choice_text,
                hide_warboard_header: row.hide_warboard_header,
                keep_open_after_choice: row.keep_open_after_choice,
            }),
            rows.responses.into_iter().map(|row| wow_data::PlayerChoiceResponseRowLikeCpp {
                choice_id: row.choice_id,
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
            }),
            rows.rewards.into_iter().map(|row| wow_data::PlayerChoiceResponseRewardRowLikeCpp {
                choice_id: row.choice_id,
                response_id: row.response_id,
                title_id: row.title_id,
                package_id: row.package_id,
                skill_line_id: row.skill_line_id,
                skill_point_count: row.skill_point_count,
                arena_point_count: row.arena_point_count,
                honor_point_count: row.honor_point_count,
                money: row.money,
                xp: row.xp,
            }),
            rows.reward_items.into_iter().map(map_reward_item_like_cpp),
            rows.reward_currencies.into_iter().map(|row| wow_data::PlayerChoiceResponseRewardCurrencyRowLikeCpp {
                choice_id: row.choice_id,
                response_id: row.response_id,
                currency_id: row.currency_id,
                quantity: row.quantity,
            }),
            rows.reward_factions.into_iter().map(|row| wow_data::PlayerChoiceResponseRewardFactionRowLikeCpp {
                choice_id: row.choice_id,
                response_id: row.response_id,
                faction_id: row.faction_id,
                quantity: row.quantity,
            }),
            rows.reward_item_choices.into_iter().map(map_reward_item_like_cpp),
            rows.maw_powers.into_iter().map(|row| wow_data::PlayerChoiceResponseMawPowerRowLikeCpp {
                choice_id: row.choice_id,
                response_id: row.response_id,
                type_art_file_id: row.type_art_file_id,
                rarity: row.rarity,
                rarity_color: row.rarity_color,
                spell_id: row.spell_id,
                max_stacks: row.max_stacks,
            }),
            title_exists,
            quest_package_exists,
            skill_line_exists,
            item_exists,
            currency_exists,
            faction_exists,
        ),
    )
}

fn map_reward_item_like_cpp(
    row: wow_persistence::PlayerChoiceResponseRewardItemRowLikeCpp,
) -> wow_data::PlayerChoiceResponseRewardItemRowLikeCpp {
    wow_data::PlayerChoiceResponseRewardItemRowLikeCpp {
        choice_id: row.choice_id,
        response_id: row.response_id,
        item_id: row.item_id,
        bonus_list_ids_raw: row.bonus_list_ids_raw,
        quantity: row.quantity,
    }
}

pub async fn load_locales_like_cpp(
    store: &mut wow_data::PlayerChoiceStoreLikeCpp,
    persistence: &dyn PlayerChoiceCatalogPersistencePortLikeCpp,
) -> Result<wow_data::PlayerChoiceLocaleLoadReportLikeCpp> {
    let rows = match persistence.load_locale_rows_like_cpp().await {
        PlayerChoiceCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        PlayerChoiceCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };

    Ok(store.load_locale_rows_like_cpp(
        rows.choices
            .into_iter()
            .map(|row| wow_data::PlayerChoiceLocaleRowLikeCpp {
                choice_id: row.choice_id,
                locale: row.locale,
                question: row.question,
            }),
        rows.responses
            .into_iter()
            .map(|row| wow_data::PlayerChoiceResponseLocaleRowLikeCpp {
                choice_id: row.choice_id,
                response_id: row.response_id,
                locale: row.locale,
                answer: row.answer,
                header: row.header,
                sub_header: row.sub_header,
                button_tooltip: row.button_tooltip,
                description: row.description,
                confirmation: row.confirmation,
            }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::{
        PersistenceFutureLikeCpp, PlayerChoiceCatalogCoreRowsLikeCpp,
        PlayerChoiceCatalogLocaleRowsLikeCpp, PlayerChoiceLocaleRowLikeCpp,
        PlayerChoiceResponseLocaleRowLikeCpp, PlayerChoiceResponseRowLikeCpp,
        PlayerChoiceRowLikeCpp,
    };

    #[derive(Clone)]
    struct FakePlayerChoiceCatalogPersistenceLikeCpp {
        core: PlayerChoiceCatalogLoadOutcomeLikeCpp<PlayerChoiceCatalogCoreRowsLikeCpp>,
        locales: PlayerChoiceCatalogLoadOutcomeLikeCpp<PlayerChoiceCatalogLocaleRowsLikeCpp>,
    }

    impl PlayerChoiceCatalogPersistencePortLikeCpp for FakePlayerChoiceCatalogPersistenceLikeCpp {
        fn load_core_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PlayerChoiceCatalogLoadOutcomeLikeCpp<PlayerChoiceCatalogCoreRowsLikeCpp>,
        > {
            Box::pin(async move { self.core.clone() })
        }

        fn load_locale_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            PlayerChoiceCatalogLoadOutcomeLikeCpp<PlayerChoiceCatalogLocaleRowsLikeCpp>,
        > {
            Box::pin(async move { self.locales.clone() })
        }
    }

    fn persistence_like_cpp() -> FakePlayerChoiceCatalogPersistenceLikeCpp {
        FakePlayerChoiceCatalogPersistenceLikeCpp {
            core: PlayerChoiceCatalogLoadOutcomeLikeCpp::Loaded(
                PlayerChoiceCatalogCoreRowsLikeCpp {
                    choices: vec![PlayerChoiceRowLikeCpp {
                        choice_id: 1,
                        ui_texture_kit_id: 2,
                        sound_kit_id: 3,
                        close_sound_kit_id: 4,
                        duration: 5,
                        question: "Question".to_string(),
                        pending_choice_text: "Pending".to_string(),
                        hide_warboard_header: 1,
                        keep_open_after_choice: 0,
                    }],
                    responses: vec![PlayerChoiceResponseRowLikeCpp {
                        choice_id: 1,
                        response_id: 10,
                        response_identifier: 11,
                        choice_art_file_id: 12,
                        flags: 13,
                        widget_set_id: 14,
                        ui_texture_atlas_element_id: 15,
                        sound_kit_id: 16,
                        group_id: 17,
                        ui_texture_kit_id: 18,
                        answer: "Answer".to_string(),
                        header: "Header".to_string(),
                        sub_header: "Sub".to_string(),
                        button_tooltip: "Tip".to_string(),
                        description: "Description".to_string(),
                        confirmation: "Confirm".to_string(),
                        reward_quest_id: None,
                    }],
                    ..Default::default()
                },
            ),
            locales: PlayerChoiceCatalogLoadOutcomeLikeCpp::Loaded(
                PlayerChoiceCatalogLocaleRowsLikeCpp {
                    choices: vec![PlayerChoiceLocaleRowLikeCpp {
                        choice_id: 1,
                        locale: "esES".to_string(),
                        question: "Pregunta".to_string(),
                    }],
                    responses: vec![PlayerChoiceResponseLocaleRowLikeCpp {
                        choice_id: 1,
                        response_id: 10,
                        locale: "esES".to_string(),
                        answer: "Respuesta".to_string(),
                        header: "Cabecera".to_string(),
                        sub_header: "Sub".to_string(),
                        button_tooltip: "Ayuda".to_string(),
                        description: "Descripción".to_string(),
                        confirmation: "Confirmar".to_string(),
                    }],
                },
            ),
        }
    }

    #[tokio::test]
    async fn typed_rows_feed_the_catalog_owner_without_changing_values() {
        let persistence = persistence_like_cpp();
        let mut outcome = load_core_like_cpp(
            &persistence,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
        )
        .await
        .unwrap();
        let choice = outcome.store.get_player_choice_like_cpp(1).unwrap();
        assert_eq!(choice.question, "Question");
        assert_eq!(choice.responses[0].answer, "Answer");

        let report = load_locales_like_cpp(&mut outcome.store, &persistence)
            .await
            .unwrap();
        assert_eq!(report.loaded_choice_locale_entries, 1);
        assert_eq!(report.loaded_response_locale_rows, 1);
    }

    #[tokio::test]
    async fn persistence_failure_stops_before_catalog_assembly() {
        let persistence = FakePlayerChoiceCatalogPersistenceLikeCpp {
            core: PlayerChoiceCatalogLoadOutcomeLikeCpp::Failed {
                reason: "world query failed".to_string(),
            },
            locales: PlayerChoiceCatalogLoadOutcomeLikeCpp::Loaded(Default::default()),
        };
        let result = load_core_like_cpp(
            &persistence,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
            |_| true,
        )
        .await;
        let Err(error) = result else {
            panic!("failed persistence must not assemble a catalog");
        };
        assert_eq!(error.to_string(), "world query failed");
    }
}
