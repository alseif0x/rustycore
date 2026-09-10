//! SQLx-free World source contract for late LFG startup catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LfgDungeonTemplatePersistenceRowLikeCpp {
    pub dungeon_id: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub required_item_level: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LfgDungeonRewardPersistenceRowLikeCpp {
    pub dungeon_id: u32,
    pub max_level: u8,
    pub first_quest_id: u32,
    pub other_quest_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LfgWorldCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// Separate reads preserve C++'s entrance-position then reward startup order.
pub trait LfgWorldCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_lfg_dungeon_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        LfgWorldCatalogLoadOutcomeLikeCpp<LfgDungeonTemplatePersistenceRowLikeCpp>,
    >;

    fn load_lfg_dungeon_reward_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        LfgWorldCatalogLoadOutcomeLikeCpp<LfgDungeonRewardPersistenceRowLikeCpp>,
    >;
}
