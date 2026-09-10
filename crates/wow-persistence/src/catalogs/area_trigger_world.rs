//! SQLx-free World-table source contract for represented AreaTrigger data.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerDestinationPersistenceRowLikeCpp {
    pub trigger_id: u32,
    pub target_map: u32,
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,
    pub target_orientation: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaTriggerScriptPersistenceRowLikeCpp {
    pub trigger_id: u32,
    pub script_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerTeleportPersistenceRowLikeCpp {
    pub trigger_id: u32,
    pub port_loc_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestAreaTriggerPersistenceRowLikeCpp {
    pub trigger_id: u32,
    pub quest_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TavernAreaTriggerPersistenceRowLikeCpp {
    pub trigger_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AreaTriggerWorldLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// Staged AreaTrigger World-table reads. Operations stay independent because
/// only a subset is currently composed during production startup.
pub trait AreaTriggerWorldCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_destination_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerDestinationPersistenceRowLikeCpp>,
    >;

    fn load_script_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerScriptPersistenceRowLikeCpp>,
    >;

    fn load_teleport_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<AreaTriggerTeleportPersistenceRowLikeCpp>,
    >;

    fn load_quest_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<QuestAreaTriggerPersistenceRowLikeCpp>,
    >;

    fn load_tavern_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        AreaTriggerWorldLoadOutcomeLikeCpp<TavernAreaTriggerPersistenceRowLikeCpp>,
    >;
}
