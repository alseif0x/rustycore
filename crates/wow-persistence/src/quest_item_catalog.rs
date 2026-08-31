//! SQLx-free World source contract for immutable C++ quest-item metadata.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectQuestItemPersistenceRowLikeCpp {
    pub gameobject_entry: u32,
    pub item_id: u32,
    pub idx: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureQuestItemPersistenceRowLikeCpp {
    pub creature_entry: u32,
    pub difficulty_id: u8,
    pub item_id: u32,
    pub idx: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestItemCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// C++ `ObjectMgr` World-table source for immutable quest-item metadata.
///
/// Gameobject and creature rows stay independent operations because C++ and
/// production validate and publish the two stores at consecutive fences.
pub trait QuestItemCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_gameobject_quest_item_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        QuestItemCatalogLoadOutcomeLikeCpp<GameObjectQuestItemPersistenceRowLikeCpp>,
    >;

    fn load_creature_quest_item_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        QuestItemCatalogLoadOutcomeLikeCpp<CreatureQuestItemPersistenceRowLikeCpp>,
    >;
}
