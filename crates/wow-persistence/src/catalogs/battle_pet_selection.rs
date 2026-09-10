//! SQLx-free World source contract for battle-pet selection catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetBreedPersistenceRowLikeCpp {
    pub species_id: u32,
    pub breed_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetQualityPersistenceRowLikeCpp {
    pub species_id: u32,
    pub quality: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattlePetSelectionCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// The reads stay separate because C++ tolerates either missing table without
/// suppressing the other catalog.
pub trait BattlePetSelectionCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_breed_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        BattlePetSelectionCatalogLoadOutcomeLikeCpp<BattlePetBreedPersistenceRowLikeCpp>,
    >;

    fn load_quality_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        BattlePetSelectionCatalogLoadOutcomeLikeCpp<BattlePetQualityPersistenceRowLikeCpp>,
    >;
}
