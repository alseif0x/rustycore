//! SQLx-free startup source contract for C++ trainer catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerSpellPersistenceRowLikeCpp {
    pub trainer_id: u32,
    pub spell_id: u32,
    pub money_cost: u32,
    pub req_skill_line: u32,
    pub req_skill_rank: u32,
    pub req_ability: [u32; 3],
    pub req_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerPersistenceRowLikeCpp {
    pub id: u32,
    pub trainer_type: u8,
    pub greeting: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerLocalePersistenceRowLikeCpp {
    pub id: u32,
    pub locale: String,
    pub greeting: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureTrainerPersistenceRowLikeCpp {
    pub creature_id: u32,
    pub trainer_id: u32,
    pub menu_id: u32,
    pub option_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrainerCatalogPersistenceRowsLikeCpp {
    pub trainer_spells: Vec<TrainerSpellPersistenceRowLikeCpp>,
    pub trainers: Vec<TrainerPersistenceRowLikeCpp>,
    pub trainer_locales: Vec<TrainerLocalePersistenceRowLikeCpp>,
    pub creature_trainers: Vec<CreatureTrainerPersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerCatalogLoadOutcomeLikeCpp {
    Loaded(TrainerCatalogPersistenceRowsLikeCpp),
    Failed { reason: String },
}

/// One ordered World-database capability matching C++
/// `ObjectMgr::LoadTrainers` followed by `LoadCreatureTrainers`.
pub trait TrainerCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_trainer_catalog_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, TrainerCatalogLoadOutcomeLikeCpp>;
}
