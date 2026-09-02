//! SQLx-free startup source for the immutable ObjectMgr query catalogs.

use crate::PersistenceFutureLikeCpp;

pub const WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP: usize = 35;

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureQueryTemplatePersistenceRowLikeCpp {
    pub entry: u32,
    pub name: String,
    pub subname: String,
    pub title_alt: String,
    pub icon_name: String,
    pub creature_type: i32,
    pub creature_family: i32,
    pub classification: i32,
    pub kill_credits: [i32; 2],
    pub civilian: bool,
    pub racial_leader: bool,
    pub movement_id: i32,
    pub required_expansion: i32,
    pub vignette_id: i32,
    pub unit_class: i32,
    pub widget_set_id: i32,
    pub widget_set_unit_condition_id: i32,
    pub hp_multi: f32,
    pub energy_multi: f32,
    pub creature_difficulty_id: i32,
    pub type_flags: [u32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureQueryDisplayPersistenceRowLikeCpp {
    pub entry: u32,
    pub display_id: u32,
    pub scale: f32,
    pub probability: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureQueryLocalePersistenceRowLikeCpp {
    pub entry: u32,
    pub locale: String,
    pub name: String,
    pub subname: String,
    pub title_alt: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureQueryCatalogPersistenceRowsLikeCpp {
    pub templates: Vec<CreatureQueryTemplatePersistenceRowLikeCpp>,
    pub displays: Vec<CreatureQueryDisplayPersistenceRowLikeCpp>,
    pub locales: Vec<CreatureQueryLocalePersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectQueryTemplatePersistenceRowLikeCpp {
    pub entry: u32,
    pub go_type: i32,
    pub display_id: i32,
    pub name: String,
    pub icon_name: String,
    pub cast_bar_caption: String,
    pub unk_string: String,
    pub size: f32,
    pub data: [i32; WORLD_QUERY_GAMEOBJECT_DATA_COUNT_LIKE_CPP],
    pub content_tuning_id: i32,
    pub min_money: u32,
    pub max_money: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameObjectQueryLocalePersistenceRowLikeCpp {
    pub entry: u32,
    pub locale: String,
    pub name: String,
    pub cast_bar_caption: String,
    pub unk_string: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectQueryCatalogPersistenceRowsLikeCpp {
    pub templates: Vec<GameObjectQueryTemplatePersistenceRowLikeCpp>,
    pub locales: Vec<GameObjectQueryLocalePersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageTextPersistenceRowLikeCpp {
    pub id: u32,
    pub text: String,
    pub next_page_id: u32,
    pub player_condition_id: i32,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageTextLocalePersistenceRowLikeCpp {
    pub id: u32,
    pub locale: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageTextCatalogPersistenceRowsLikeCpp {
    pub pages: Vec<PageTextPersistenceRowLikeCpp>,
    pub locales: Vec<PageTextLocalePersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldQueryCatalogLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

pub trait WorldQueryCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_creature_query_catalog_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldQueryCatalogLoadOutcomeLikeCpp<CreatureQueryCatalogPersistenceRowsLikeCpp>,
    >;

    fn load_gameobject_query_catalog_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldQueryCatalogLoadOutcomeLikeCpp<GameObjectQueryCatalogPersistenceRowsLikeCpp>,
    >;

    fn load_page_text_catalog_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        WorldQueryCatalogLoadOutcomeLikeCpp<PageTextCatalogPersistenceRowsLikeCpp>,
    >;
}
