//! SQLx-free startup source for the C++ ObjectMgr quest catalog.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq)]
pub struct QuestTemplatePersistenceRowLikeCpp {
    pub id: u32,
    pub quest_type: u8,
    pub quest_level: i32,
    pub quest_max_scaling_level: i32,
    pub quest_package_id: u32,
    pub min_level: i32,
    pub quest_sort_id: i32,
    pub quest_info_id: u16,
    pub suggested_group_num: u8,
    pub reward_next_quest: u32,
    pub reward_xp_difficulty: u32,
    pub reward_xp_multiplier: f32,
    pub reward_money_difficulty: u32,
    pub reward_money_multiplier: f32,
    pub reward_bonus_money: u32,
    pub reward_display_spell: [u32; 3],
    pub reward_spell: u32,
    pub reward_honor: u32,
    pub reward_title_id: u32,
    pub reward_skill_line_id: u32,
    pub reward_skill_points: u32,
    pub reward_mail_template_id: u32,
    pub reward_mail_delay_secs: u32,
    pub reward_mail_sender_entry: u32,
    pub reward_faction_ids: [u32; 5],
    pub reward_faction_values: [i32; 5],
    pub reward_faction_overrides: [i32; 5],
    pub reward_faction_cap_in: [i32; 5],
    pub reward_faction_flags: u32,
    pub source_item_id: u32,
    pub source_item_count: u32,
    pub source_spell_id: u32,
    pub limit_time_secs: i64,
    pub expansion: i32,
    pub flags: u32,
    pub flags_ex: u32,
    pub flags_ex2: u32,
    pub special_flags: u32,
    pub reward_items: [u32; 4],
    pub reward_amounts: [u32; 4],
    pub reward_currencies: [u32; 4],
    pub reward_currency_amounts: [u32; 4],
    pub item_drop: [u32; 4],
    pub item_drop_quantity: [u32; 4],
    pub log_title: String,
    pub log_description: String,
    pub quest_description: String,
    pub area_description: String,
    pub quest_completion_log: String,
    pub allowable_races: u64,
    pub allowable_classes: u32,
    pub max_level: u8,
    pub prev_quest_id: i32,
    pub next_quest_id: u32,
    pub exclusive_group: i32,
    pub breadcrumb_for_quest_id: i32,
    pub required_min_rep_faction: u32,
    pub required_min_rep_value: i32,
    pub required_max_rep_faction: u32,
    pub required_max_rep_value: i32,
    pub required_skill_id: u32,
    pub required_skill_points: u32,
    pub reward_choice_items: [(u32, u32); 6],
    pub reward_choice_item_types: [u8; 6],
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuestObjectivePersistenceRowLikeCpp {
    pub id: u32,
    pub quest_id: u32,
    pub obj_type: u8,
    pub order: u8,
    pub storage_index: i8,
    pub object_id: i32,
    pub amount: i32,
    pub flags: u32,
    pub flags2: u32,
    pub progress_bar_weight: f32,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuestCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

pub trait QuestCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_quest_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        QuestCatalogLoadOutcomeLikeCpp<QuestTemplatePersistenceRowLikeCpp>,
    >;
    fn load_quest_special_flag_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>>;
    fn load_seasonal_quest_relation_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>>;
    fn load_quest_objective_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        QuestCatalogLoadOutcomeLikeCpp<QuestObjectivePersistenceRowLikeCpp>,
    >;
    fn load_creature_quest_starter_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>>;
    fn load_creature_quest_ender_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>>;
    fn load_gameobject_quest_starter_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>>;
    fn load_gameobject_quest_ender_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>>;
}
