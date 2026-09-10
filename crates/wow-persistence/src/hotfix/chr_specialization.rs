//! SQLx-free Hotfix contract for the effective ChrSpecialization authority.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChrSpecializationHotfixRowLikeCpp {
    pub id: u32,
    pub class_id: u8,
    pub order_index: i8,
    pub role: i8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChrSpecializationHotfixRowsLikeCpp {
    pub official: Vec<ChrSpecializationHotfixRowLikeCpp>,
    pub custom: Vec<ChrSpecializationHotfixRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChrSpecializationHotfixLoadOutcomeLikeCpp {
    Loaded(ChrSpecializationHotfixRowsLikeCpp),
    Failed { reason: String },
}

pub trait ChrSpecializationHotfixPersistencePortLikeCpp: Send + Sync {
    fn load_chr_specialization_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ChrSpecializationHotfixLoadOutcomeLikeCpp>;
}
