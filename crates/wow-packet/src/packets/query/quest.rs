//! Quest packets.
//!
//! Separated from query.rs under #689.

use super::*;

/// Trinity `Array<int32, 100>` cap for `CMSG_QUERY_QUEST_COMPLETION_NPCS`.
pub const MAX_QUERY_QUEST_COMPLETION_NPCS: usize = 100;

/// Trinity `Array<int32, 175>` payload for `CMSG_QUEST_POI_QUERY`.
pub const QUEST_POI_QUERY_MISSING_QUEST_POIS_LIKE_CPP: usize = 25;

// ── CMSG_QUERY_QUEST_COMPLETION_NPCS (0x3177) ───────────────────────

/// Client asks which Creature/GameObject entries can complete the supplied quests.
pub struct QueryQuestCompletionNpcs {
    pub quest_ids: Vec<i32>,
}

impl ClientPacket for QueryQuestCompletionNpcs {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryQuestCompletionNpcs;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = pkt.read_uint32()? as usize;
        if count > MAX_QUERY_QUEST_COMPLETION_NPCS {
            return Err(PacketError::TooLarge { size: count });
        }

        let mut quest_ids = Vec::with_capacity(count);
        for _ in 0..count {
            quest_ids.push(pkt.read_int32()?);
        }

        Ok(Self { quest_ids })
    }
}

// ── SMSG_QUEST_COMPLETION_NPC_RESPONSE (0x2A81) ─────────────────────

/// One quest's completion NPC/GO entry list. GameObjects carry the C++ high-bit mask.
pub struct QuestCompletionNpc {
    pub quest_id: i32,
    pub npcs: Vec<i32>,
}

/// Server response for `CMSG_QUERY_QUEST_COMPLETION_NPCS`.
pub struct QuestCompletionNpcResponse {
    pub quests: Vec<QuestCompletionNpc>,
}

impl ServerPacket for QuestCompletionNpcResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestCompletionNpcResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.quests.len() as u32);
        for quest in &self.quests {
            pkt.write_int32(quest.quest_id);
            pkt.write_uint32(quest.npcs.len() as u32);
            for &npc in &quest.npcs {
                pkt.write_int32(npc);
            }
        }
    }
}

// ── CMSG_QUEST_POI_QUERY (0x36B2) ──────────────────────────────────

/// Client asks for map POI blobs for missing quest tracker data.
///
/// C++ anchors:
/// - `QuestPOIQuery::Read`, `QueryPackets.cpp:418-423`: one signed count,
///   then the fixed quest-log-sized signed quest id array.
/// - `QuestPOIQuery`, `QueryPackets.h:323-331`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPoiQuery {
    pub missing_quest_count: i32,
    pub missing_quest_pois: [i32; QUEST_POI_QUERY_MISSING_QUEST_POIS_LIKE_CPP],
}

impl ClientPacket for QuestPoiQuery {
    const OPCODE: ClientOpcodes = ClientOpcodes::QuestPoiQuery;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let missing_quest_count = pkt.read_int32()?;
        let mut missing_quest_pois = [0; QUEST_POI_QUERY_MISSING_QUEST_POIS_LIKE_CPP];
        for quest_id in &mut missing_quest_pois {
            *quest_id = pkt.read_int32()?;
        }

        Ok(Self {
            missing_quest_count,
            missing_quest_pois,
        })
    }
}

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

/// Server response for `CMSG_QUEST_POI_QUERY`.
///
/// C++ anchors:
/// - `QuestPOIQueryResponse::Write`, `QueryPackets.cpp:426-441`.
/// - `operator<<(QuestPOIData const&)`, `QueryPackets.cpp:26-53`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPoiQueryResponse {
    pub quest_poi_data_stats: Vec<QuestPoiData>,
}

impl ServerPacket for QuestPoiQueryResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QuestPoiQueryResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        let count = self.quest_poi_data_stats.len() as i32;
        pkt.write_int32(count);
        pkt.write_int32(count);

        for quest_poi_data in &self.quest_poi_data_stats {
            pkt.write_int32(quest_poi_data.quest_id);
            pkt.write_int32(quest_poi_data.blobs.len() as i32);

            for blob in &quest_poi_data.blobs {
                pkt.write_int32(blob.blob_index);
                pkt.write_int32(blob.objective_index);
                pkt.write_int32(blob.quest_objective_id);
                pkt.write_int32(blob.quest_object_id);
                pkt.write_int32(blob.map_id);
                pkt.write_int32(blob.ui_map_id);
                pkt.write_int32(blob.priority);
                pkt.write_int32(blob.flags);
                pkt.write_int32(blob.world_effect_id);
                pkt.write_int32(blob.player_condition_id);
                pkt.write_int32(blob.navigation_player_condition_id);
                pkt.write_int32(blob.spawn_tracking_id);
                pkt.write_int32(blob.points.len() as i32);

                for point in &blob.points {
                    pkt.write_int16(point.x as i16);
                    pkt.write_int16(point.y as i16);
                    pkt.write_int16(point.z as i16);
                }

                pkt.write_bit(blob.always_allow_merging_blobs);
                pkt.flush_bits();
            }
        }
    }
}
