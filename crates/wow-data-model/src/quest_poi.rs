//! Shared immutable quest POI schemas.
//!
//! C++ a5f8da2e: ObjectMgr.h:779–825. Packet serialization remains
//! in wow-packet (QueryPackets.cpp:26–53,426–441).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPoiBlobPoint {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPoiBlobData {
    pub blob_index: i32,
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
    pub points: Vec<QuestPoiBlobPoint>,
    pub always_allow_merging_blobs: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPoiData {
    pub quest_id: i32,
    pub blobs: Vec<QuestPoiBlobData>,
}
