//! Names packets.
//!
//! Separated from query.rs under #689.

use super::*;

/// Trinity `MAX_DECLINED_NAME_CASES`.
pub const MAX_DECLINED_NAME_CASES_LIKE_CPP: usize = 5;

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
