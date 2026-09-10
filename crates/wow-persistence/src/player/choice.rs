//! SQLx-free startup loading contract for the C++ PlayerChoice catalog.

use crate::PersistenceFutureLikeCpp;

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerChoiceCatalogCoreRowsLikeCpp {
    pub choices: Vec<PlayerChoiceRowLikeCpp>,
    pub responses: Vec<PlayerChoiceResponseRowLikeCpp>,
    pub rewards: Vec<PlayerChoiceResponseRewardRowLikeCpp>,
    pub reward_items: Vec<PlayerChoiceResponseRewardItemRowLikeCpp>,
    pub reward_currencies: Vec<PlayerChoiceResponseRewardCurrencyRowLikeCpp>,
    pub reward_factions: Vec<PlayerChoiceResponseRewardFactionRowLikeCpp>,
    pub reward_item_choices: Vec<PlayerChoiceResponseRewardItemRowLikeCpp>,
    pub maw_powers: Vec<PlayerChoiceResponseMawPowerRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerChoiceCatalogLocaleRowsLikeCpp {
    pub choices: Vec<PlayerChoiceLocaleRowLikeCpp>,
    pub responses: Vec<PlayerChoiceResponseLocaleRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerChoiceCatalogLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

/// C++ `ObjectMgr::LoadPlayerChoices` and `LoadPlayerChoicesLocale` data source.
/// The adapter owns statement identity, query order and concrete row decoding;
/// `wow-data` remains the catalog/validation owner.
pub trait PlayerChoiceCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_core_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerChoiceCatalogLoadOutcomeLikeCpp<PlayerChoiceCatalogCoreRowsLikeCpp>,
    >;

    fn load_locale_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerChoiceCatalogLoadOutcomeLikeCpp<PlayerChoiceCatalogLocaleRowsLikeCpp>,
    >;
}
