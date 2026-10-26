//! SQLx-free startup contract for effective condition record presence.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionPresenceRows {
    pub player_condition_ids: Vec<u32>,
    pub unit_condition_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionPresenceLoadOutcome {
    Loaded(ConditionPresenceRows),
    Failed { reason: String },
}

pub trait ConditionPresencePersistencePort: Send + Sync {
    fn load_condition_presence_rows(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ConditionPresenceLoadOutcome>;
}
