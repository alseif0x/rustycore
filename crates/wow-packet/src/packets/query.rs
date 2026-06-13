// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Query packets: QueryCreature, QueryGameObject and their responses.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::world_packet::PacketError;
use crate::{ClientPacket, ServerPacket, WorldPacket};

// ── Constants ────────────────────────────────────────────────────────

/// Maximum creature name slots (matches C# SharedConst.MaxCreatureNames).
const MAX_CREATURE_NAMES: usize = 4;

/// Maximum creature kill credit slots.
const MAX_CREATURE_KILL_CREDIT: usize = 2;

/// Trinity `Array<int32, 100>` cap for `CMSG_QUERY_QUEST_COMPLETION_NPCS`.
pub const MAX_QUERY_QUEST_COMPLETION_NPCS: usize = 100;

/// Trinity `MAX_DECLINED_NAME_CASES`.
pub const MAX_DECLINED_NAME_CASES_LIKE_CPP: usize = 5;

// ── CMSG_QUERY_CREATURE (0x3270) ─────────────────────────────────────

/// Client request for creature template data.
pub struct QueryCreature {
    pub creature_id: u32,
}

impl ClientPacket for QueryCreature {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryCreature;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let creature_id = packet.read_uint32()?;
        Ok(Self { creature_id })
    }
}

// ── SMSG_CREATURE_QUERY_RESPONSE (0x2914) ────────────────────────────

/// A single display variant for a creature.
pub struct CreatureXDisplay {
    pub creature_display_id: u32,
    pub scale: f32,
    pub probability: f32,
}

/// Creature display info block.
pub struct CreatureDisplayStats {
    pub displays: Vec<CreatureXDisplay>,
    pub total_probability: f32,
}

/// Full creature template stats for query response.
pub struct CreatureStats {
    pub title: String, // SubName in DB
    pub title_alt: String,
    pub cursor_name: String, // IconName in DB
    pub civilian: bool,
    pub leader: bool, // RacialLeader
    pub names: [String; MAX_CREATURE_NAMES],
    pub name_alts: [String; MAX_CREATURE_NAMES],
    pub flags: [u32; 2], // TypeFlags, TypeFlags2
    pub creature_type: i32,
    pub creature_family: i32,
    pub classification: i32,
    pub proxy_creature_ids: [i32; MAX_CREATURE_KILL_CREDIT],
    pub display: CreatureDisplayStats,
    pub hp_multi: f32,
    pub energy_multi: f32,
    pub quest_items: Vec<i32>,
    pub creature_movement_info_id: i32,
    pub health_scaling_expansion: i32,
    pub required_expansion: i32,
    pub vignette_id: i32,
    pub unit_class: i32,
    pub creature_difficulty_id: i32,
    pub widget_set_id: i32,
    pub widget_set_unit_condition_id: i32,
}

impl Default for CreatureStats {
    fn default() -> Self {
        Self {
            title: String::new(),
            title_alt: String::new(),
            cursor_name: String::new(),
            civilian: false,
            leader: false,
            names: Default::default(),
            name_alts: Default::default(),
            flags: [0; 2],
            creature_type: 0,
            creature_family: 0,
            classification: 0,
            proxy_creature_ids: [0; MAX_CREATURE_KILL_CREDIT],
            display: CreatureDisplayStats {
                displays: Vec::new(),
                total_probability: 0.0,
            },
            hp_multi: 1.0,
            energy_multi: 1.0,
            quest_items: Vec::new(),
            creature_movement_info_id: 0,
            health_scaling_expansion: 0,
            required_expansion: 0,
            vignette_id: 0,
            unit_class: 1,
            creature_difficulty_id: 0,
            widget_set_id: 0,
            widget_set_unit_condition_id: 0,
        }
    }
}

/// Server response with creature template data.
pub struct QueryCreatureResponse {
    pub creature_id: u32,
    pub allow: bool,
    pub stats: Option<CreatureStats>,
}

impl ServerPacket for QueryCreatureResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryCreatureResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.creature_id as i32);
        pkt.write_bit(self.allow);
        pkt.flush_bits();

        if !self.allow {
            return;
        }

        let default_stats;
        let stats = match &self.stats {
            Some(s) => s,
            None => {
                default_stats = CreatureStats::default();
                &default_stats
            }
        };

        // ── Bit-packed string lengths ────────────────────────────
        // C++ writes length() + 1 for every nullable string length bitfield,
        // including empty strings, but only emits string bytes when non-empty.
        let title_len = stats.title.len() as u32 + 1;
        let title_alt_len = stats.title_alt.len() as u32 + 1;
        let cursor_name_len = stats.cursor_name.len() as u32 + 1;

        pkt.write_bits(title_len, 11);
        pkt.write_bits(title_alt_len, 11);
        pkt.write_bits(cursor_name_len, 6);
        pkt.write_bit(stats.civilian);
        pkt.write_bit(stats.leader);

        // C# interleaves Name[i] and NameAlt[i] lengths in one loop
        for i in 0..MAX_CREATURE_NAMES {
            let name_len = stats.names[i].len() as u32 + 1;
            let alt_len = stats.name_alts[i].len() as u32 + 1;
            pkt.write_bits(name_len, 11);
            pkt.write_bits(alt_len, 11);
        }
        pkt.flush_bits();

        // ── Name strings (BEFORE integer fields!) ────────────────
        // C# writes names interleaved: Name[0], NameAlt[0], Name[1], NameAlt[1], ...
        for i in 0..MAX_CREATURE_NAMES {
            if !stats.names[i].is_empty() {
                pkt.write_cstring(&stats.names[i]);
            }
            if !stats.name_alts[i].is_empty() {
                pkt.write_cstring(&stats.name_alts[i]);
            }
        }

        // ── Integer fields ───────────────────────────────────────
        // Flags[2]
        pkt.write_uint32(stats.flags[0]);
        pkt.write_uint32(stats.flags[1]);

        // Type, Family, Classification
        pkt.write_int32(stats.creature_type);
        pkt.write_int32(stats.creature_family);
        pkt.write_int32(stats.classification);

        // PetSpellDataId — not used in 3.4.3, always 0
        pkt.write_int32(0);

        // ProxyCreatureID (kill credits)
        for i in 0..MAX_CREATURE_KILL_CREDIT {
            pkt.write_int32(stats.proxy_creature_ids[i]);
        }

        // Display info
        pkt.write_int32(stats.display.displays.len() as i32);
        pkt.write_float(stats.display.total_probability);

        for d in &stats.display.displays {
            pkt.write_int32(d.creature_display_id as i32);
            pkt.write_float(d.scale);
            pkt.write_float(d.probability);
        }

        // Multipliers
        pkt.write_float(stats.hp_multi);
        pkt.write_float(stats.energy_multi);

        // Quest items count
        pkt.write_int32(stats.quest_items.len() as i32);

        // Remaining integer fields
        pkt.write_int32(stats.creature_movement_info_id);
        pkt.write_int32(stats.health_scaling_expansion);
        pkt.write_int32(stats.required_expansion);
        pkt.write_int32(stats.vignette_id);
        pkt.write_int32(stats.unit_class);
        pkt.write_int32(stats.creature_difficulty_id);
        pkt.write_int32(stats.widget_set_id);
        pkt.write_int32(stats.widget_set_unit_condition_id);

        // ── Trailing strings ─────────────────────────────────────
        if !stats.title.is_empty() {
            pkt.write_cstring(&stats.title);
        }
        if !stats.title_alt.is_empty() {
            pkt.write_cstring(&stats.title_alt);
        }
        if !stats.cursor_name.is_empty() {
            pkt.write_cstring(&stats.cursor_name);
        }

        // Quest item IDs
        for &item_id in &stats.quest_items {
            pkt.write_int32(item_id);
        }
    }
}

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

// ── CMSG_QUERY_PAGE_TEXT (0x3274) ────────────────────────────────────

/// Client request for static page-text data.
pub struct QueryPageText {
    pub page_text_id: u32,
    pub item_guid: ObjectGuid,
}

impl ClientPacket for QueryPageText {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryPageText;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let page_text_id = packet.read_uint32()?;
        let guid_bytes = packet.read_bytes(16)?;
        let mut raw = [0_u8; 16];
        raw.copy_from_slice(&guid_bytes);
        Ok(Self {
            page_text_id,
            item_guid: ObjectGuid::from_raw_bytes(&raw),
        })
    }
}

/// One page in `SMSG_QUERY_PAGE_TEXT_RESPONSE`.
pub struct PageTextInfo {
    pub id: u32,
    pub next_page_id: u32,
    pub player_condition_id: i32,
    pub flags: u8,
    pub text: String,
}

/// Static page-text response, including linked pages by `NextPageID`.
pub struct QueryPageTextResponse {
    pub page_text_id: u32,
    pub allow: bool,
    pub pages: Vec<PageTextInfo>,
}

impl ServerPacket for QueryPageTextResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryPageTextResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.page_text_id);
        pkt.write_bit(self.allow);
        pkt.flush_bits();

        if !self.allow {
            return;
        }

        pkt.write_uint32(self.pages.len() as u32);
        for page in &self.pages {
            pkt.write_uint32(page.id);
            pkt.write_uint32(page.next_page_id);
            pkt.write_int32(page.player_condition_id);
            pkt.write_uint8(page.flags);
            pkt.write_bits(page.text.len() as u32, 12);
            pkt.flush_bits();
            pkt.write_string(&page.text);
        }
    }
}

// ── CMSG_QUERY_PET_NAME (0x3275) ────────────────────────────────────

/// Client request for an in-world pet/creature name.
pub struct QueryPetName {
    pub unit_guid: ObjectGuid,
}

impl ClientPacket for QueryPetName {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryPetName;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let guid_bytes = packet.read_bytes(16)?;
        let mut raw = [0_u8; 16];
        raw.copy_from_slice(&guid_bytes);
        Ok(Self {
            unit_guid: ObjectGuid::from_raw_bytes(&raw),
        })
    }
}

/// Declined pet names carried by `SMSG_QUERY_PET_NAME_RESPONSE`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PetDeclinedNamesLikeCpp {
    pub names: [String; MAX_DECLINED_NAME_CASES_LIKE_CPP],
}

/// C++ `WorldPackets::Query::QueryPetNameResponse`.
pub struct QueryPetNameResponse {
    pub unit_guid: ObjectGuid,
    pub allow: bool,
    pub has_declined: bool,
    pub declined_names: PetDeclinedNamesLikeCpp,
    pub timestamp: u32,
    pub name: String,
}

impl QueryPetNameResponse {
    pub fn not_allowed(unit_guid: ObjectGuid) -> Self {
        Self {
            unit_guid,
            allow: false,
            has_declined: false,
            declined_names: PetDeclinedNamesLikeCpp::default(),
            timestamp: 0,
            name: String::new(),
        }
    }
}

impl ServerPacket for QueryPetNameResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryPetNameResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bytes(&self.unit_guid.to_raw_bytes());
        pkt.write_bit(self.allow);

        if self.allow {
            pkt.write_bits(self.name.len() as u32, 8);
            pkt.write_bit(self.has_declined);

            for declined_name in &self.declined_names.names {
                pkt.write_bits(declined_name.len() as u32, 7);
            }

            for declined_name in &self.declined_names.names {
                pkt.write_string(declined_name);
            }

            pkt.write_uint32(self.timestamp);
            pkt.write_string(&self.name);
        }

        pkt.flush_bits();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_creature_response_not_found() {
        let resp = QueryCreatureResponse {
            creature_id: 12345,
            allow: false,
            stats: None,
        };
        let bytes = resp.to_bytes();
        // opcode(2) + creature_id(4) + bit(allow=false, flushed to 1 byte) = 7
        assert_eq!(bytes.len(), 7);
    }

    #[test]
    fn query_creature_response_found() {
        let mut stats = CreatureStats::default();
        stats.names[0] = "Test Creature".to_string();
        stats.creature_type = 1; // Beast
        stats.display = CreatureDisplayStats {
            displays: vec![CreatureXDisplay {
                creature_display_id: 856,
                scale: 1.0,
                probability: 1.0,
            }],
            total_probability: 1.0,
        };

        let resp = QueryCreatureResponse {
            creature_id: 100,
            allow: true,
            stats: Some(stats),
        };
        let bytes = resp.to_bytes();
        // Should be a reasonable size (name + fields)
        assert!(
            bytes.len() > 50,
            "Response too small: {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn query_creature_response_empty_string_lengths_match_cpp() {
        let resp = QueryCreatureResponse {
            creature_id: 100,
            allow: true,
            stats: Some(CreatureStats::default()),
        };
        let bytes = resp.to_bytes();

        // C++ writes 118 bits of string-length metadata after Allow, then
        // flushes to 15 bytes even when all strings are empty (length = 1).
        assert_eq!(bytes.len(), 106);
    }

    #[test]
    fn query_game_object_response_not_found() {
        let resp = QueryGameObjectResponse {
            game_object_id: 999,
            guid: ObjectGuid::EMPTY,
            allow: false,
            stats: None,
        };
        let bytes = resp.to_bytes();
        // C++ always writes uint32(statsData.size()), even when Allow=false.
        assert!(bytes.len() > 6);
        assert_eq!(&bytes[bytes.len() - 4..], &[0, 0, 0, 0]);
    }

    #[test]
    fn query_game_object_response_writes_data34_and_content_tuning() {
        let mut stats = GameObjectStats::default();
        stats.names[0] = "Test GameObject".to_string();
        stats.data[34] = 0x1122_3344;
        stats.content_tuning_id = 0x5566_7788;

        let resp = QueryGameObjectResponse {
            game_object_id: 100,
            guid: ObjectGuid::EMPTY,
            allow: true,
            stats: Some(stats),
        };

        let bytes = resp.to_bytes();
        assert!(bytes.windows(4).any(|w| w == 0x1122_3344i32.to_le_bytes()));
        assert!(bytes.windows(4).any(|w| w == 0x5566_7788i32.to_le_bytes()));
    }

    #[test]
    fn query_page_text_reads_cpp_page_id_then_raw_item_guid() {
        let guid =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Item, 0, 1, 0, 0, 7, 9);
        let mut data = (ClientOpcodes::QueryPageText as u16).to_le_bytes().to_vec();
        data.extend_from_slice(&123_u32.to_le_bytes());
        data.extend_from_slice(&guid.to_raw_bytes());
        let mut pkt = WorldPacket::from_bytes(&data);
        pkt.skip_opcode();

        let query = QueryPageText::read(&mut pkt).unwrap();
        assert_eq!(query.page_text_id, 123);
        assert_eq!(query.item_guid, guid);
    }

    #[test]
    fn query_page_text_response_writes_cpp_allow_false_shape() {
        let bytes = QueryPageTextResponse {
            page_text_id: 123,
            allow: false,
            pages: Vec::new(),
        }
        .to_bytes();

        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::QueryPageTextResponse as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..6], &123_u32.to_le_bytes());
        assert_eq!(bytes[6], 0x00);
        assert_eq!(bytes.len(), 7);
    }

    #[test]
    fn query_page_text_response_writes_pages_like_cpp() {
        let bytes = QueryPageTextResponse {
            page_text_id: 123,
            allow: true,
            pages: vec![PageTextInfo {
                id: 123,
                next_page_id: 124,
                player_condition_id: -5,
                flags: 7,
                text: "abc".to_string(),
            }],
        }
        .to_bytes();

        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::QueryPageTextResponse as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..6], &123_u32.to_le_bytes());
        assert_eq!(bytes[6], 0x80);
        assert_eq!(&bytes[7..11], &1_u32.to_le_bytes());
        assert_eq!(&bytes[11..15], &123_u32.to_le_bytes());
        assert_eq!(&bytes[15..19], &124_u32.to_le_bytes());
        assert_eq!(&bytes[19..23], &(-5_i32).to_le_bytes());
        assert_eq!(bytes[23], 7);
        assert_eq!(bytes[24], 0x00);
        assert_eq!(bytes[25], 0x30);
        assert_eq!(&bytes[26..29], b"abc");
        assert_eq!(bytes.len(), 29);
    }

    #[test]
    fn query_pet_name_reads_cpp_unit_guid() {
        let guid =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 7, 9);
        let mut data = (ClientOpcodes::QueryPetName as u16).to_le_bytes().to_vec();
        data.extend_from_slice(&guid.to_raw_bytes());
        let mut pkt = WorldPacket::from_bytes(&data);
        pkt.skip_opcode();

        let query = QueryPetName::read(&mut pkt).unwrap();
        assert_eq!(query.unit_guid, guid);
    }

    #[test]
    fn query_pet_name_response_writes_cpp_allow_false_shape() {
        let guid =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 7, 9);
        let bytes = QueryPetNameResponse::not_allowed(guid).to_bytes();

        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::QueryPetNameResponse as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(bytes[18], 0x00);
        assert_eq!(bytes.len(), 19);
    }

    #[test]
    fn query_pet_name_response_writes_name_timestamp_and_declined_like_cpp() {
        let guid =
            ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 571, 0, 7, 9);
        let response = QueryPetNameResponse {
            unit_guid: guid,
            allow: true,
            has_declined: true,
            declined_names: PetDeclinedNamesLikeCpp {
                names: [
                    "Alpha".to_string(),
                    "Beta".to_string(),
                    "Gamma".to_string(),
                    "Delta".to_string(),
                    "Epsilon".to_string(),
                ],
            },
            timestamp: 123_456,
            name: "Misha".to_string(),
        };

        let bytes = response.to_bytes();
        assert_eq!(
            bytes[0..2],
            (ServerOpcodes::QueryPetNameResponse as u16).to_le_bytes()
        );
        assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
        assert_eq!(bytes[18] & 0x80, 0x80);
        assert!(bytes.windows(4).any(|w| w == 123_456_u32.to_le_bytes()));
        assert!(bytes.windows(5).any(|w| w == b"Misha"));
        assert!(bytes.windows(5).any(|w| w == b"Alpha"));
        assert!(bytes.windows(7).any(|w| w == b"Epsilon"));
    }

    #[test]
    fn query_player_names_response_found() {
        let guid = ObjectGuid::create_player(1, 42);
        let data = PlayerGuidLookupData {
            name: "TestPlayer".to_string(),
            race: 1,
            sex: 0,
            class: 1,
            level: 10,
            guid_actual: guid,
            account_id: ObjectGuid::EMPTY,
            bnet_account_id: ObjectGuid::EMPTY,
            virtual_realm_address: 0x0100_0001,
            ..Default::default()
        };
        let resp = QueryPlayerNamesResponse {
            players: vec![NameCacheLookupResult {
                player: guid,
                result: 0,
                data: Some(data),
            }],
        };
        let bytes = resp.to_bytes();
        assert!(
            bytes.len() > 20,
            "Response too small: {} bytes",
            bytes.len()
        );
    }
}

// ── CMSG_QUERY_PLAYER_NAMES (0x3772) ──────────────────────────────

/// Client request for one or more player names.
pub struct QueryPlayerNames {
    pub players: Vec<ObjectGuid>,
}

impl ClientPacket for QueryPlayerNames {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryPlayerNames;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = packet.read_uint32()? as usize;
        // Sanity limit to prevent OOM
        let count = count.min(100); // Sanity cap
        let mut players = Vec::with_capacity(count);
        for _ in 0..count {
            players.push(packet.read_packed_guid()?);
        }
        Ok(Self { players })
    }
}

// ── SMSG_QUERY_PLAYER_NAMES_RESPONSE (0x301B) ────────────────────

/// Number of declined name cases (nominative through prepositional).
const MAX_DECLINED_NAME_CASES: usize = 5;

/// Player lookup data for a found character.
#[derive(Default)]
pub struct PlayerGuidLookupData {
    pub is_deleted: bool,
    pub account_id: ObjectGuid,
    pub bnet_account_id: ObjectGuid,
    pub guid_actual: ObjectGuid,
    pub guild_club_member_id: u64,
    pub virtual_realm_address: u32,
    pub race: u8,
    pub sex: u8,
    pub class: u8,
    pub level: u8,
    pub name: String,
    pub declined_names: [String; MAX_DECLINED_NAME_CASES],
}

/// A single lookup result in the response.
pub struct NameCacheLookupResult {
    pub player: ObjectGuid,
    /// 0 = Success (has data), non-zero = failure (no data).
    pub result: u8,
    pub data: Option<PlayerGuidLookupData>,
}

/// Server response with player name data.
pub struct QueryPlayerNamesResponse {
    pub players: Vec<NameCacheLookupResult>,
}

impl ServerPacket for QueryPlayerNamesResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryPlayerNamesResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.players.len() as i32);

        for entry in &self.players {
            // Result code: 0 = success
            pkt.write_uint8(entry.result);
            // Player GUID
            pkt.write_packed_guid(&entry.player);
            // HasData bit
            pkt.write_bit(entry.data.is_some());
            // HasUnused920 bit — always false
            pkt.write_bit(false);
            pkt.flush_bits();

            if let Some(data) = &entry.data {
                // ── PlayerGuidLookupData.Write ────────────────
                pkt.write_bit(data.is_deleted);
                // Name length (6 bits)
                pkt.write_bits(data.name.len() as u32, 6);
                // Declined name lengths (7 bits each, 5 cases)
                for dn in &data.declined_names {
                    pkt.write_bits(dn.len() as u32, 7);
                }
                // FlushBits is implicit — next byte write will flush

                // Declined name strings
                for dn in &data.declined_names {
                    if !dn.is_empty() {
                        pkt.write_string(dn);
                    }
                }

                // Account GUID (WowAccount)
                pkt.write_packed_guid(&data.account_id);
                // BNet Account GUID
                pkt.write_packed_guid(&data.bnet_account_id);
                // Player GUID (actual)
                pkt.write_packed_guid(&data.guid_actual);
                // Guild Club Member ID
                pkt.write_uint64(data.guild_club_member_id);
                // Virtual Realm Address
                pkt.write_uint32(data.virtual_realm_address);
                // Race, Sex, Class, Level, Unused915
                pkt.write_uint8(data.race);
                pkt.write_uint8(data.sex);
                pkt.write_uint8(data.class);
                pkt.write_uint8(data.level);
                pkt.write_uint8(0); // Unused915
                // Character name
                pkt.write_string(&data.name);
            }
        }
    }
}

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

// ── CMSG_QUERY_REALM_NAME (0x368A) ──────────────────────────────────

/// Client asks for the name of a realm given its VirtualRealmAddress.
pub struct QueryRealmName {
    pub virtual_realm_address: u32,
}

impl ClientPacket for QueryRealmName {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryRealmName;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let virtual_realm_address = pkt.read_uint32()?;
        Ok(Self {
            virtual_realm_address,
        })
    }
}

// ── SMSG_REALM_QUERY_RESPONSE (0x2913) ──────────────────────────────

/// Server response with realm name information.
pub struct RealmQueryResponse {
    pub virtual_realm_address: u32,
    pub lookup_state: u8,
    pub realm_name_actual: String,
    pub realm_name_normalized: String,
    pub is_local: bool,
}

impl ServerPacket for RealmQueryResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::RealmQueryResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.virtual_realm_address);
        pkt.write_uint8(self.lookup_state);

        if self.lookup_state == 0 {
            // VirtualRealmNameInfo.Write
            pkt.write_bit(self.is_local);
            pkt.write_bit(false); // IsInternalRealm
            pkt.write_bits(self.realm_name_actual.len() as u32, 8);
            pkt.write_bits(self.realm_name_normalized.len() as u32, 8);
            pkt.flush_bits();

            pkt.write_string(&self.realm_name_actual);
            pkt.write_string(&self.realm_name_normalized);
        }
    }
}

#[cfg(test)]
mod query_quest_completion_tests {
    use super::*;
    use num_traits::ToPrimitive;

    fn client_payload(bytes: &[u8]) -> WorldPacket {
        let mut data = Vec::from(
            ClientOpcodes::QueryQuestCompletionNpcs
                .to_u16()
                .unwrap()
                .to_le_bytes(),
        );
        data.extend_from_slice(bytes);
        let mut pkt = WorldPacket::from_bytes(&data);
        pkt.skip_opcode();
        pkt
    }

    #[test]
    fn query_quest_completion_parse_valid_ids() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&2u32.to_le_bytes());
        payload.extend_from_slice(&123i32.to_le_bytes());
        payload.extend_from_slice(&(-45i32).to_le_bytes());

        let mut pkt = client_payload(&payload);
        let parsed = QueryQuestCompletionNpcs::read(&mut pkt).unwrap();

        assert_eq!(parsed.quest_ids, vec![123, -45]);
    }

    #[test]
    fn query_quest_completion_rejects_short_payload() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(&[0x34, 0x12]);

        let mut pkt = client_payload(&payload);

        assert!(matches!(
            QueryQuestCompletionNpcs::read(&mut pkt),
            Err(PacketError::ReadPastEnd { .. })
        ));
    }

    #[test]
    fn query_quest_completion_rejects_over_cap() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&101u32.to_le_bytes());

        let mut pkt = client_payload(&payload);

        assert!(matches!(
            QueryQuestCompletionNpcs::read(&mut pkt),
            Err(PacketError::TooLarge { size: 101 })
        ));
    }

    #[test]
    fn query_quest_completion_response_serializes_creature_and_masked_go() {
        let response = QuestCompletionNpcResponse {
            quests: vec![QuestCompletionNpc {
                quest_id: 77,
                npcs: vec![1234, 0x8000_5678u32 as i32],
            }],
        };

        let bytes = response.to_bytes();

        let mut expected = Vec::from(
            ServerOpcodes::QuestCompletionNpcResponse
                .to_u16()
                .unwrap()
                .to_le_bytes(),
        );
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(&77i32.to_le_bytes());
        expected.extend_from_slice(&2u32.to_le_bytes());
        expected.extend_from_slice(&1234i32.to_le_bytes());
        expected.extend_from_slice(&0x8000_5678u32.to_le_bytes());

        assert_eq!(bytes, expected);
    }
}
