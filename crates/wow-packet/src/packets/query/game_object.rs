//! Game object packets.
//!
//! Separated from query.rs under #689.

use super::*;

// ── CMSG_QUERY_GAME_OBJECT (0x3271) ─────────────────────────────────

/// Client request for gameobject template data.
pub struct QueryGameObject {
    pub game_object_id: u32,
    pub guid: ObjectGuid,
}

impl ClientPacket for QueryGameObject {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryGameObject;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let game_object_id = packet.read_uint32()?;
        let guid = packet.read_packed_guid()?;
        Ok(Self {
            game_object_id,
            guid,
        })
    }
}

/// Maximum gameobject name slots.
const MAX_GAMEOBJECT_NAMES: usize = 4;

/// Maximum gameobject data fields.
const MAX_GAMEOBJECT_DATA: usize = 35;

/// Gameobject template stats for query response.
pub struct GameObjectStats {
    pub names: [String; MAX_GAMEOBJECT_NAMES],
    pub icon_name: String,
    pub cast_bar_caption: String,
    pub unk_string: String,
    pub go_type: i32,
    pub display_id: i32,
    pub data: [i32; MAX_GAMEOBJECT_DATA],
    pub size: f32,
    pub quest_items: Vec<i32>,
    pub content_tuning_id: i32,
}

impl Default for GameObjectStats {
    fn default() -> Self {
        Self {
            names: Default::default(),
            icon_name: String::new(),
            cast_bar_caption: String::new(),
            unk_string: String::new(),
            go_type: 0,
            display_id: 0,
            data: [0; MAX_GAMEOBJECT_DATA],
            size: 1.0,
            quest_items: Vec::new(),
            content_tuning_id: 0,
        }
    }
}

/// Server response with gameobject template data.
pub struct QueryGameObjectResponse {
    pub game_object_id: u32,
    pub guid: ObjectGuid,
    pub allow: bool,
    pub stats: Option<GameObjectStats>,
}

impl ServerPacket for QueryGameObjectResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryGameObjectResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.game_object_id as i32);
        pkt.write_packed_guid(&self.guid);
        pkt.write_bit(self.allow);
        pkt.flush_bits();

        if !self.allow {
            pkt.write_uint32(0);
            return;
        }

        let stats = match &self.stats {
            Some(s) => s,
            None => return,
        };

        // Build stats buffer so we can write size prefix
        let mut buf = WorldPacket::new_empty();

        // Type + DisplayID
        buf.write_int32(stats.go_type);
        buf.write_int32(stats.display_id);

        // Names[4] — null-terminated
        for name in &stats.names {
            buf.write_cstring(name);
        }

        // IconName, CastBarCaption, UnkString
        buf.write_cstring(&stats.icon_name);
        buf.write_cstring(&stats.cast_bar_caption);
        buf.write_cstring(&stats.unk_string);

        // Data[35] (Data0..Data34), matching C++ MAX_GAMEOBJECT_DATA.
        for &d in &stats.data {
            buf.write_int32(d);
        }

        // Size
        buf.write_float(stats.size);

        // QuestItems
        buf.write_uint8(stats.quest_items.len() as u8);
        for &item in &stats.quest_items {
            buf.write_int32(item);
        }

        // ContentTuningId
        buf.write_int32(stats.content_tuning_id);

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }
}
