//! SQLx-free Character source contract for the reserved-player-name catalog.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedNamePersistenceRowLikeCpp {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservedNameCatalogLoadOutcomeLikeCpp {
    Loaded(Vec<ReservedNamePersistenceRowLikeCpp>),
    Failed { reason: String },
}

pub trait ReservedNameCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ReservedNameCatalogLoadOutcomeLikeCpp>;
}
