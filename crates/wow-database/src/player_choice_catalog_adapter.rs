//! MariaDB adapter for C++ PlayerChoice startup catalog loading.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerChoiceCatalogCoreRowsLikeCpp,
    PlayerChoiceCatalogLoadOutcomeLikeCpp, PlayerChoiceCatalogLocaleRowsLikeCpp,
    PlayerChoiceCatalogPersistencePortLikeCpp, PlayerChoiceLocaleRowLikeCpp,
    PlayerChoiceResponseLocaleRowLikeCpp, PlayerChoiceResponseMawPowerRowLikeCpp,
    PlayerChoiceResponseRewardCurrencyRowLikeCpp, PlayerChoiceResponseRewardFactionRowLikeCpp,
    PlayerChoiceResponseRewardItemRowLikeCpp, PlayerChoiceResponseRewardRowLikeCpp,
    PlayerChoiceResponseRowLikeCpp, PlayerChoiceRowLikeCpp,
};

use crate::{DatabaseError, SqlResult, WorldDatabase, WorldStatements};

const CORE_STATEMENTS_LIKE_CPP: [WorldStatements; 8] = [
    WorldStatements::SEL_PLAYER_CHOICES,
    WorldStatements::SEL_PLAYER_CHOICE_RESPONSES,
    WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARDS,
    WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEMS,
    WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_CURRENCIES,
    WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_FACTIONS,
    WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEM_CHOICES,
    WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_MAW_POWERS,
];

const LOCALE_STATEMENTS_LIKE_CPP: [WorldStatements; 2] = [
    WorldStatements::SEL_PLAYER_CHOICE_LOCALES,
    WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_LOCALES,
];

async fn query_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> T,
) -> Result<Vec<T>, DatabaseError> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::new();
    if result.is_empty() {
        return Ok(rows);
    }
    loop {
        rows.push(decode(&result));
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

pub struct MariaDbPlayerChoiceCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbPlayerChoiceCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl PlayerChoiceCatalogPersistencePortLikeCpp
    for MariaDbPlayerChoiceCatalogPersistenceAdapterLikeCpp
{
    fn load_core_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerChoiceCatalogLoadOutcomeLikeCpp<PlayerChoiceCatalogCoreRowsLikeCpp>,
    > {
        Box::pin(async move {
            let loaded: Result<PlayerChoiceCatalogCoreRowsLikeCpp, DatabaseError> = async {
                Ok(PlayerChoiceCatalogCoreRowsLikeCpp {
                    choices: query_rows_like_cpp(
                        &self.world_db,
                        CORE_STATEMENTS_LIKE_CPP[0],
                        |row| PlayerChoiceRowLikeCpp {
                            choice_id: row.read(0),
                            ui_texture_kit_id: row.read(1),
                            sound_kit_id: row.read(2),
                            close_sound_kit_id: row.read(3),
                            duration: row.read(4),
                            question: row.read_string(5),
                            pending_choice_text: row.read_string(6),
                            hide_warboard_header: row.read(7),
                            keep_open_after_choice: row.read(8),
                        },
                    )
                    .await?,
                    responses: query_rows_like_cpp(
                        &self.world_db,
                        CORE_STATEMENTS_LIKE_CPP[1],
                        |row| PlayerChoiceResponseRowLikeCpp {
                            choice_id: row.read(0),
                            response_id: row.read(1),
                            response_identifier: row.read(2),
                            choice_art_file_id: row.read(3),
                            flags: row.read(4),
                            widget_set_id: row.read(5),
                            ui_texture_atlas_element_id: row.read(6),
                            sound_kit_id: row.read(7),
                            group_id: row.read(8),
                            ui_texture_kit_id: row.read(9),
                            answer: row.read_string(10),
                            header: row.read_string(11),
                            sub_header: row.read_string(12),
                            button_tooltip: row.read_string(13),
                            description: row.read_string(14),
                            confirmation: row.read_string(15),
                            reward_quest_id: (!row.is_null(16)).then(|| row.read(16)),
                        },
                    )
                    .await?,
                    rewards: query_rows_like_cpp(
                        &self.world_db,
                        CORE_STATEMENTS_LIKE_CPP[2],
                        |row| PlayerChoiceResponseRewardRowLikeCpp {
                            choice_id: row.read(0),
                            response_id: row.read(1),
                            title_id: row.read(2),
                            package_id: row.read(3),
                            skill_line_id: row.read(4),
                            skill_point_count: row.read(5),
                            arena_point_count: row.read(6),
                            honor_point_count: row.read(7),
                            money: row.read(8),
                            xp: row.read(9),
                        },
                    )
                    .await?,
                    reward_items: query_rows_like_cpp(
                        &self.world_db,
                        CORE_STATEMENTS_LIKE_CPP[3],
                        |row| PlayerChoiceResponseRewardItemRowLikeCpp {
                            choice_id: row.read(0),
                            response_id: row.read(1),
                            item_id: row.read(2),
                            bonus_list_ids_raw: row.read_string(3),
                            quantity: row.read(4),
                        },
                    )
                    .await?,
                    reward_currencies: query_rows_like_cpp(
                        &self.world_db,
                        CORE_STATEMENTS_LIKE_CPP[4],
                        |row| PlayerChoiceResponseRewardCurrencyRowLikeCpp {
                            choice_id: row.read(0),
                            response_id: row.read(1),
                            currency_id: row.read(2),
                            quantity: row.read(3),
                        },
                    )
                    .await?,
                    reward_factions: query_rows_like_cpp(
                        &self.world_db,
                        CORE_STATEMENTS_LIKE_CPP[5],
                        |row| PlayerChoiceResponseRewardFactionRowLikeCpp {
                            choice_id: row.read(0),
                            response_id: row.read(1),
                            faction_id: row.read(2),
                            quantity: row.read(3),
                        },
                    )
                    .await?,
                    reward_item_choices: query_rows_like_cpp(
                        &self.world_db,
                        CORE_STATEMENTS_LIKE_CPP[6],
                        |row| PlayerChoiceResponseRewardItemRowLikeCpp {
                            choice_id: row.read(0),
                            response_id: row.read(1),
                            item_id: row.read(2),
                            bonus_list_ids_raw: row.read_string(3),
                            quantity: row.read(4),
                        },
                    )
                    .await?,
                    maw_powers: query_rows_like_cpp(
                        &self.world_db,
                        CORE_STATEMENTS_LIKE_CPP[7],
                        |row| PlayerChoiceResponseMawPowerRowLikeCpp {
                            choice_id: row.read(0),
                            response_id: row.read(1),
                            type_art_file_id: row.read(2),
                            rarity: (!row.is_null(3)).then(|| row.read(3)),
                            rarity_color: (!row.is_null(4)).then(|| row.read(4)),
                            spell_id: row.read(5),
                            max_stacks: row.read(6),
                        },
                    )
                    .await?,
                })
            }
            .await;

            match loaded {
                Ok(rows) => PlayerChoiceCatalogLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => PlayerChoiceCatalogLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_locale_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerChoiceCatalogLoadOutcomeLikeCpp<PlayerChoiceCatalogLocaleRowsLikeCpp>,
    > {
        Box::pin(async move {
            let loaded: Result<PlayerChoiceCatalogLocaleRowsLikeCpp, DatabaseError> = async {
                Ok(PlayerChoiceCatalogLocaleRowsLikeCpp {
                    choices: query_rows_like_cpp(
                        &self.world_db,
                        LOCALE_STATEMENTS_LIKE_CPP[0],
                        |row| PlayerChoiceLocaleRowLikeCpp {
                            choice_id: row.read(0),
                            locale: row.read_string(1),
                            question: row.read_string(2),
                        },
                    )
                    .await?,
                    responses: query_rows_like_cpp(
                        &self.world_db,
                        LOCALE_STATEMENTS_LIKE_CPP[1],
                        |row| PlayerChoiceResponseLocaleRowLikeCpp {
                            choice_id: row.read(0),
                            response_id: row.read(1),
                            locale: row.read_string(2),
                            answer: row.read_string(3),
                            header: row.read_string(4),
                            sub_header: row.read_string(5),
                            button_tooltip: row.read_string(6),
                            description: row.read_string(7),
                            confirmation: row.read_string(8),
                        },
                    )
                    .await?,
                })
            }
            .await;

            match loaded {
                Ok(rows) => PlayerChoiceCatalogLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => PlayerChoiceCatalogLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_choice_statement_order_matches_cpp_startup_order() {
        assert_eq!(
            CORE_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_PLAYER_CHOICES,
                WorldStatements::SEL_PLAYER_CHOICE_RESPONSES,
                WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARDS,
                WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEMS,
                WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_CURRENCIES,
                WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_FACTIONS,
                WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEM_CHOICES,
                WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_MAW_POWERS,
            ]
        );
        assert_eq!(
            LOCALE_STATEMENTS_LIKE_CPP,
            [
                WorldStatements::SEL_PLAYER_CHOICE_LOCALES,
                WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_LOCALES,
            ]
        );
    }
}
