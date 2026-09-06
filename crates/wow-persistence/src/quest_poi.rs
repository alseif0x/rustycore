//! Quest POI row projections and read-only cache loading capability.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPoiPointLoadRowLikeCpp {
    pub quest_id: i32,
    pub idx1: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPoiBlobLoadRowLikeCpp {
    pub quest_id: i32,
    pub blob_index: i32,
    pub idx1: i32,
    pub objective_index: i32,
    pub quest_objective_id: i32,
    pub quest_object_id: i32,
    pub map_id: i32,
    pub ui_map_id: i32,
    pub priority: i32,
    pub flags: i32,
    pub world_effect_id: i32,
    pub player_condition_id: i32,
    pub navigation_player_condition_id: i32,
    pub spawn_tracking_id: i32,
    pub always_allow_merging_blobs: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestPoiLoadStageLikeCpp {
    Points,
    Blobs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestPoiLoadOutcomeLikeCpp {
    Loaded {
        points: Vec<QuestPoiPointLoadRowLikeCpp>,
        blobs: Vec<QuestPoiBlobLoadRowLikeCpp>,
    },
    Failed {
        stage: QuestPoiLoadStageLikeCpp,
        reason: String,
    },
}

/// SQLx-free World-database capability for the represented quest POI cache.
pub trait QuestPoiPersistencePortLikeCpp: Send + Sync {
    fn load_quest_poi_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestPoiLoadOutcomeLikeCpp>;
}
