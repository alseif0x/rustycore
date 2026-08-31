//! SQLx-free World source contract for the exploration BaseXP catalog.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplorationBaseXpPersistenceRowLikeCpp {
    pub level: u8,
    pub base_xp: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplorationBaseXpCatalogLoadOutcomeLikeCpp {
    Loaded(Vec<ExplorationBaseXpPersistenceRowLikeCpp>),
    Failed { reason: String },
}

pub trait ExplorationBaseXpCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ExplorationBaseXpCatalogLoadOutcomeLikeCpp>;
}
