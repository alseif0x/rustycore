//! SQLx-free Hotfix source contract for the Phase DB2 authorities.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseHotfixRowLikeCpp {
    pub id: u32,
    pub flags: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseGroupHotfixRowLikeCpp {
    pub id: u32,
    pub phase_id: u16,
    pub phase_group_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseHotfixLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// Independent reads preserve the represented startup fence between loading
/// and overlaying `Phase.db2` and loading `PhaseXPhaseGroup.db2`.
pub trait PhaseHotfixPersistencePortLikeCpp: Send + Sync {
    fn load_phase_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, PhaseHotfixLoadOutcomeLikeCpp<PhaseHotfixRowLikeCpp>>;

    fn load_phase_group_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, PhaseHotfixLoadOutcomeLikeCpp<PhaseGroupHotfixRowLikeCpp>>;
}
