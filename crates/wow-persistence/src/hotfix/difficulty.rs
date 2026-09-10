//! SQLx-free Hotfix contract for the effective Difficulty authority.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifficultyHotfixRowLikeCpp {
    pub id: u32,
    pub instance_type: u8,
    pub fallback_difficulty_id: u8,
    pub flags: u8,
    pub toggle_difficulty_id: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DifficultyHotfixRowsLikeCpp {
    pub official: Vec<DifficultyHotfixRowLikeCpp>,
    pub custom: Vec<DifficultyHotfixRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DifficultyHotfixLoadOutcomeLikeCpp {
    Loaded(DifficultyHotfixRowsLikeCpp),
    Failed { reason: String },
}

pub trait DifficultyHotfixPersistencePortLikeCpp: Send + Sync {
    fn load_difficulty_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, DifficultyHotfixLoadOutcomeLikeCpp>;
}
