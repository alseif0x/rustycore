// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Authentication packet definitions for the world server handshake.
//!
//! These packets handle the initial connection flow:
//! 1. Server → Client: [`AuthChallenge`]
//! 2. Client → Server: [`AuthSession`]
//! 3. Server → Client: [`AuthResponse`]
//! 4. Server → Client: [`EnterEncryptedMode`]
//! 5. Client → Server: [`EnterEncryptedModeAck`]

use wow_constants::{ClientOpcodes, ServerOpcodes};

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

// ── AuthChallenge (Server → Client) ──────────────────────────────

/// Server sends this after the connection string exchange.
///
/// Wire format: `[DosChallenge: 32B][Challenge: 16B][DosZeroBits: u8]`
#[derive(Debug, Clone)]
pub struct AuthChallenge {
    pub dos_challenge: [u8; 32],
    pub challenge: [u8; 16],
    pub dos_zero_bits: u8,
}

impl ServerPacket for AuthChallenge {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuthChallenge;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bytes(&self.dos_challenge);
        pkt.write_bytes(&self.challenge);
        pkt.write_uint8(self.dos_zero_bits);
    }
}

// ── AuthSession (Client → Server) ────────────────────────────────

/// Client sends this in response to [`AuthChallenge`].
///
/// Contains the HMAC-SHA256 digest (truncated to 24 bytes) for validation.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub dos_response: u64,
    pub region_id: u32,
    pub battlegroup_id: u32,
    pub realm_id: u32,
    pub local_challenge: [u8; 16],
    pub digest: [u8; 24],
    pub use_ipv6: bool,
    pub realm_join_ticket: String,
}

impl ClientPacket for AuthSession {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuthSession;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let dos_response = pkt.read_uint64()?;
        let region_id = pkt.read_uint32()?;
        let battlegroup_id = pkt.read_uint32()?;
        let realm_id = pkt.read_uint32()?;

        let mut local_challenge = [0u8; 16];
        for byte in &mut local_challenge {
            *byte = pkt.read_uint8()?;
        }

        let digest_vec = pkt.read_bytes(24)?;
        let mut digest = [0u8; 24];
        digest.copy_from_slice(&digest_vec);

        let use_ipv6 = pkt.has_bit()?;
        let ticket_size = pkt.read_int32()?;

        let realm_join_ticket = if ticket_size > 0 {
            pkt.read_string(ticket_size as usize)?
        } else {
            String::new()
        };

        Ok(Self {
            dos_response,
            region_id,
            battlegroup_id,
            realm_id,
            local_challenge,
            digest,
            use_ipv6,
            realm_join_ticket,
        })
    }
}

// ── AuthContinuedSession (Client → Server) ───────────────────────

/// Fast reconnection packet (skips full auth).
#[derive(Debug, Clone)]
pub struct AuthContinuedSession {
    pub dos_response: i64,
    pub key: i64,
    pub local_challenge: [u8; 16],
    pub digest: [u8; 24],
}

impl ClientPacket for AuthContinuedSession {
    const OPCODE: ClientOpcodes = ClientOpcodes::AuthContinuedSession;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let dos_response = pkt.read_int64()?;
        let key = pkt.read_int64()?;

        let challenge_vec = pkt.read_bytes(16)?;
        let mut local_challenge = [0u8; 16];
        local_challenge.copy_from_slice(&challenge_vec);

        let digest_vec = pkt.read_bytes(24)?;
        let mut digest = [0u8; 24];
        digest.copy_from_slice(&digest_vec);

        Ok(Self {
            dos_response,
            key,
            local_challenge,
            digest,
        })
    }
}

// ── AuthResponse (Server → Client) ──────────────────────────────

/// Authentication result sent after validating [`AuthSession`].
#[derive(Debug, Clone)]
pub struct AuthResponse {
    pub result: u32,
    pub success_info: Option<AuthSuccessInfo>,
    pub wait_info: Option<AuthWaitInfo>,
}

#[derive(Debug, Clone)]
pub struct AuthSuccessInfo {
    pub virtual_realm_address: u32,
    pub virtual_realms: Vec<VirtualRealmInfo>,
    pub time_rested: u32,
    pub active_expansion_level: u8,
    pub account_expansion_level: u8,
    pub time_seconds_until_pc_kick: u32,
    pub available_classes: Vec<RaceClassAvailability>,
    pub templates: Vec<CharacterTemplate>,
    pub currency_id: u32,
    pub time: i64,
    pub game_time_info: GameTimeInfo,
    pub is_expansion_trial: bool,
    pub force_character_template: bool,
    pub num_players_horde: Option<u16>,
    pub num_players_alliance: Option<u16>,
    pub expansion_trial_expiration: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct GameTimeInfo {
    pub billing_plan: u32,
    pub time_remain: u32,
    pub unknown735: u32,
    pub in_game_room: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct AuthWaitInfo {
    pub wait_count: u32,
    pub wait_time: u32,
    pub has_fcm: bool,
}

#[derive(Debug, Clone)]
pub struct VirtualRealmInfo {
    pub realm_address: u32,
    pub is_local: bool,
    pub is_internal_realm: bool,
    pub realm_name_actual: String,
    pub realm_name_normalized: String,
}

#[derive(Debug, Clone)]
pub struct RaceClassAvailability {
    pub race_id: u8,
    pub classes: Vec<ClassAvailability>,
}

#[derive(Debug, Clone, Copy)]
pub struct ClassAvailability {
    pub class_id: u8,
    pub active_expansion_level: u8,
    pub account_expansion_level: u8,
    pub min_active_expansion_level: u8,
}

#[derive(Debug, Clone)]
pub struct CharacterTemplate {
    pub template_set_id: i32,
    pub classes: Vec<CharacterTemplateClass>,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy)]
pub struct CharacterTemplateClass {
    pub faction_group: u8,
    pub class_id: u8,
}

impl ServerPacket for AuthResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::AuthResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.result);
        pkt.write_bit(self.success_info.is_some());
        pkt.write_bit(self.wait_info.is_some());
        pkt.flush_bits();

        if let Some(ref info) = self.success_info {
            write_success_info(pkt, info);
        }

        if let Some(ref wait) = self.wait_info {
            pkt.write_uint32(wait.wait_count);
            pkt.write_uint32(wait.wait_time);
            pkt.write_bit(wait.has_fcm);
            pkt.flush_bits();
        }
    }
}

fn write_success_info(pkt: &mut WorldPacket, info: &AuthSuccessInfo) {
    pkt.write_uint32(info.virtual_realm_address);
    pkt.write_int32(info.virtual_realms.len() as i32);
    pkt.write_uint32(info.time_rested);
    pkt.write_uint8(info.active_expansion_level);
    pkt.write_uint8(info.account_expansion_level);
    pkt.write_uint32(info.time_seconds_until_pc_kick);
    pkt.write_int32(info.available_classes.len() as i32);
    pkt.write_int32(info.templates.len() as i32);
    pkt.write_uint32(info.currency_id);
    pkt.write_int64(info.time);

    // AvailableClasses array
    for race in &info.available_classes {
        pkt.write_uint8(race.race_id);
        pkt.write_int32(race.classes.len() as i32);
        for class in &race.classes {
            pkt.write_uint8(class.class_id);
            pkt.write_uint8(class.active_expansion_level);
            pkt.write_uint8(class.account_expansion_level);
            pkt.write_uint8(class.min_active_expansion_level);
        }
    }

    // Bit fields
    pkt.write_bit(info.is_expansion_trial);
    pkt.write_bit(info.force_character_template);
    pkt.write_bit(info.num_players_horde.is_some());
    pkt.write_bit(info.num_players_alliance.is_some());
    pkt.write_bit(info.expansion_trial_expiration.is_some());
    pkt.write_bit(false); // NewBuildKeys (not implemented)
    pkt.flush_bits();

    // GameTimeInfo
    pkt.write_uint32(info.game_time_info.billing_plan);
    pkt.write_uint32(info.game_time_info.time_remain);
    pkt.write_uint32(info.game_time_info.unknown735);
    pkt.write_bit(info.game_time_info.in_game_room);
    pkt.write_bit(info.game_time_info.in_game_room);
    pkt.write_bit(info.game_time_info.in_game_room);
    pkt.flush_bits();

    // Optional fields
    if let Some(horde) = info.num_players_horde {
        pkt.write_uint16(horde);
    }
    if let Some(alliance) = info.num_players_alliance {
        pkt.write_uint16(alliance);
    }
    if let Some(expiry) = info.expansion_trial_expiration {
        pkt.write_int64(expiry);
    }

    // VirtualRealms array
    for realm in &info.virtual_realms {
        pkt.write_uint32(realm.realm_address);
        pkt.write_bit(realm.is_local);
        pkt.write_bit(realm.is_internal_realm);
        pkt.write_bits(realm.realm_name_actual.len() as u32, 8);
        pkt.write_bits(realm.realm_name_normalized.len() as u32, 8);
        pkt.flush_bits();
        pkt.write_string(&realm.realm_name_actual);
        pkt.write_string(&realm.realm_name_normalized);
    }

    // Templates array
    for tmpl in &info.templates {
        pkt.write_int32(tmpl.template_set_id);
        pkt.write_int32(tmpl.classes.len() as i32);
        for class in &tmpl.classes {
            pkt.write_uint8(class.class_id);
            pkt.write_uint8(class.faction_group);
        }
        pkt.write_bits(tmpl.name.len() as u32, 7);
        pkt.write_bits(tmpl.description.len() as u32, 10);
        pkt.flush_bits();
        pkt.write_string(&tmpl.name);
        pkt.write_string(&tmpl.description);
    }
}

// ── EnterEncryptedMode (Server → Client) ─────────────────────────

/// Signals the client to enable AES-GCM encryption.
///
/// Contains an Ed25519 signature proving the server has the private key.
#[derive(Debug, Clone)]
pub struct EnterEncryptedMode {
    pub signature: [u8; 64],
    pub enabled: bool,
}

impl ServerPacket for EnterEncryptedMode {
    const OPCODE: ServerOpcodes = ServerOpcodes::EnterEncryptedMode;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bytes(&self.signature);
        pkt.write_bit(self.enabled);
        pkt.flush_bits();
    }
}

// ── EnterEncryptedModeAck (Client → Server) ──────────────────────

/// Client acknowledges encryption mode — packet body is empty.
pub struct EnterEncryptedModeAck;

impl ClientPacket for EnterEncryptedModeAck {
    const OPCODE: ClientOpcodes = ClientOpcodes::EnterEncryptedModeAck;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

// ── ConnectTo (Server → Client) ──────────────────────────────────

/// Tells the client to connect to an instance server.
///
/// Wire format:
/// ```text
/// [Signature: 256B (RSA-PKCS1-SHA256, reversed)]
/// [AddressType: u8] [AddressBytes: 4B or 16B]
/// [Port: u16 LE]
/// [Serial: u32 LE]
/// [Con: u8 (ConnectionType)]
/// [Key: i64 LE (ConnectToKey.Raw)]
/// ```
#[derive(Debug, Clone)]
pub struct ConnectTo {
    /// RSA-2048 PKCS#1 v1.5 SHA-256 signature (256 bytes, reversed).
    pub signature: [u8; 256],
    /// Address of the instance server.
    pub address: ConnectToAddress,
    /// Port of the instance server.
    pub port: u16,
    /// Retry serial (e.g. WorldAttempt1, WorldAttempt2, ...).
    pub serial: ConnectToSerial,
    /// Connection type: 0 = Realm, 1 = Instance.
    pub con: u8,
    /// Packed ConnectToKey (account_id + connection_type + random key).
    pub key: i64,
}

/// Address type for ConnectTo.
#[derive(Debug, Clone)]
pub enum ConnectToAddress {
    IPv4([u8; 4]),
    IPv6([u8; 16]),
}

/// Serial values for ConnectTo retry logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ConnectToSerial {
    None = 0,
    Realm = 14,
    WorldAttempt1 = 17,
    WorldAttempt2 = 35,
    WorldAttempt3 = 53,
    WorldAttempt4 = 71,
    WorldAttempt5 = 89,
}

impl ConnectToSerial {
    /// Get the next retry serial, or `None` if all attempts exhausted.
    pub fn next(self) -> Option<Self> {
        match self {
            Self::WorldAttempt1 => Some(Self::WorldAttempt2),
            Self::WorldAttempt2 => Some(Self::WorldAttempt3),
            Self::WorldAttempt3 => Some(Self::WorldAttempt4),
            Self::WorldAttempt4 => Some(Self::WorldAttempt5),
            _ => None,
        }
    }
}

impl ServerPacket for ConnectTo {
    const OPCODE: ServerOpcodes = ServerOpcodes::ConnectTo;

    fn write(&self, pkt: &mut WorldPacket) {
        // 1. Signature (256 bytes)
        pkt.write_bytes(&self.signature);

        // 2. whereBuffer: address type + address bytes
        match &self.address {
            ConnectToAddress::IPv4(ip) => {
                pkt.write_uint8(1); // AddressType::IPv4
                pkt.write_bytes(ip);
            }
            ConnectToAddress::IPv6(ip) => {
                pkt.write_uint8(2); // AddressType::IPv6
                pkt.write_bytes(ip);
            }
        }

        // 3. Port
        pkt.write_uint16(self.port);

        // 4. Serial
        pkt.write_uint32(self.serial as u32);

        // 5. Connection type
        pkt.write_uint8(self.con);

        // 6. Key
        pkt.write_int64(self.key);
    }
}

// ── ConnectToKey ─────────────────────────────────────────────────

/// Packed key for the ConnectTo flow.
///
/// Bit layout of `raw` (i64):
/// - bits [31:0]  — AccountId (u32)
/// - bit  [32]    — ConnectionType (0 = Realm, 1 = Instance)
/// - bits [63:33] — Key (random 31-bit value)
#[derive(Debug, Clone, Copy)]
pub struct ConnectToKey {
    pub account_id: u32,
    pub connection_type: u8,
    pub key: u32,
}

impl ConnectToKey {
    /// Pack into a raw i64 value.
    pub fn raw(&self) -> i64 {
        let r = (self.account_id as u64)
            | ((self.connection_type as u64 & 1) << 32)
            | ((self.key as u64) << 33);
        r as i64
    }

    /// Unpack from a raw i64 value.
    pub fn from_raw(raw: i64) -> Self {
        let r = raw as u64;
        Self {
            account_id: (r & 0xFFFF_FFFF) as u32,
            connection_type: ((r >> 32) & 1) as u8,
            key: (r >> 33) as u32,
        }
    }
}

// ── ResumeComms (Server → Client) ───────────────────────────────

/// Sent on the instance socket after AuthContinuedSession succeeds.
///
/// Wire format: empty (opcode only). The ConnectionType is for routing
/// (determines which socket sends it), not part of the payload.
pub struct ResumeComms;

impl ServerPacket for ResumeComms {
    const OPCODE: ServerOpcodes = ServerOpcodes::ResumeComms;

    fn write(&self, _pkt: &mut WorldPacket) {
        // Empty payload (C# Write() method is empty)
    }
}

// ── ConnectToFailed (Client → Server) ───────────────────────────

/// Client reports that it could not connect to the instance server.
#[derive(Debug, Clone)]
pub struct ConnectToFailed {
    pub serial: ConnectToSerial,
}

impl ClientPacket for ConnectToFailed {
    const OPCODE: ClientOpcodes = ClientOpcodes::ConnectToFailed;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let serial_raw = pkt.read_uint32()?;
        let serial = match serial_raw {
            0 => ConnectToSerial::None,
            14 => ConnectToSerial::Realm,
            17 => ConnectToSerial::WorldAttempt1,
            35 => ConnectToSerial::WorldAttempt2,
            53 => ConnectToSerial::WorldAttempt3,
            71 => ConnectToSerial::WorldAttempt4,
            89 => ConnectToSerial::WorldAttempt5,
            _ => ConnectToSerial::None,
        };
        Ok(Self { serial })
    }
}

// ── Ping / Pong ──────────────────────────────────────────────────

/// Keepalive from client.
#[derive(Debug, Clone, Copy)]
pub struct Ping {
    pub serial: u32,
    pub latency: u32,
}

impl ClientPacket for Ping {
    const OPCODE: ClientOpcodes = ClientOpcodes::Ping;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let serial = pkt.read_uint32()?;
        let latency = pkt.read_uint32()?;
        Ok(Self { serial, latency })
    }
}

/// Keepalive response from server.
#[derive(Debug, Clone, Copy)]
pub struct Pong {
    pub serial: u32,
}

impl ServerPacket for Pong {
    const OPCODE: ServerOpcodes = ServerOpcodes::Pong;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.serial);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_challenge_roundtrip() {
        let challenge = AuthChallenge {
            dos_challenge: [0xAA; 32],
            challenge: [0xBB; 16],
            dos_zero_bits: 1,
        };

        let data = challenge.to_bytes();
        // opcode(2) + dos(32) + challenge(16) + bits(1) = 51
        assert_eq!(data.len(), 51);

        // Verify opcode
        let opcode = u16::from_le_bytes([data[0], data[1]]);
        assert_eq!(opcode, 0x3048); // AuthChallenge

        // Verify dos challenge
        assert_eq!(&data[2..34], &[0xAA; 32]);
        // Verify challenge
        assert_eq!(&data[34..50], &[0xBB; 16]);
        // Verify dos_zero_bits
        assert_eq!(data[50], 1);
    }

    #[test]
    fn auth_session_read() {
        let mut pkt = WorldPacket::new_empty();
        // Build a fake AuthSession payload (without opcode, since Read skips it)
        pkt.write_uint64(0x1234); // dos_response
        pkt.write_uint32(1); // region_id
        pkt.write_uint32(2); // battlegroup_id
        pkt.write_uint32(3); // realm_id
        pkt.write_bytes(&[0xCC; 16]); // local_challenge
        pkt.write_bytes(&[0xDD; 24]); // digest
        pkt.write_bit(false); // use_ipv6
        pkt.flush_bits();
        let ticket = b"test_ticket";
        pkt.write_int32(ticket.len() as i32);
        pkt.write_string("test_ticket");

        pkt.reset_read();
        let session = AuthSession::read(&mut pkt).unwrap();

        assert_eq!(session.dos_response, 0x1234);
        assert_eq!(session.region_id, 1);
        assert_eq!(session.battlegroup_id, 2);
        assert_eq!(session.realm_id, 3);
        assert_eq!(session.local_challenge, [0xCC; 16]);
        assert_eq!(session.digest, [0xDD; 24]);
        assert!(!session.use_ipv6);
        assert_eq!(session.realm_join_ticket, "test_ticket");
    }

    #[test]
    fn enter_encrypted_mode_write() {
        let pkt_data = EnterEncryptedMode {
            signature: [0xFF; 64],
            enabled: true,
        };

        let data = pkt_data.to_bytes();
        // opcode(2) + signature(64) + bit(1 byte padded) = 67
        assert_eq!(data.len(), 67);

        let opcode = u16::from_le_bytes([data[0], data[1]]);
        assert_eq!(opcode, 0x3049); // EnterEncryptedMode
    }

    #[test]
    fn ping_roundtrip() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(42);
        pkt.write_uint32(100);

        pkt.reset_read();
        let ping = Ping::read(&mut pkt).unwrap();
        assert_eq!(ping.serial, 42);
        assert_eq!(ping.latency, 100);
    }

    #[test]
    fn pong_write() {
        let pong = Pong { serial: 42 };
        let data = pong.to_bytes();
        assert_eq!(data.len(), 6); // opcode(2) + serial(4)
        let opcode = u16::from_le_bytes([data[0], data[1]]);
        assert_eq!(opcode, 0x304E); // Pong
    }

    #[test]
    fn auth_response_simple_error() {
        let resp = AuthResponse {
            result: 0x0A, // some error code
            success_info: None,
            wait_info: None,
        };

        let data = resp.to_bytes();
        // opcode(2) + result(4) + 2 bits + flush(1) = 7
        assert_eq!(data.len(), 7);
    }

    #[test]
    fn auth_continued_session_read() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int64(-1);
        pkt.write_int64(12345);
        pkt.write_bytes(&[0xEE; 16]);
        pkt.write_bytes(&[0xFF; 24]);

        pkt.reset_read();
        let session = AuthContinuedSession::read(&mut pkt).unwrap();
        assert_eq!(session.dos_response, -1);
        assert_eq!(session.key, 12345);
        assert_eq!(session.local_challenge, [0xEE; 16]);
        assert_eq!(session.digest, [0xFF; 24]);
    }

    #[test]
    fn connect_to_key_roundtrip() {
        let key = ConnectToKey {
            account_id: 42,
            connection_type: 1,
            key: 0x1234_5678,
        };
        let raw = key.raw();
        let decoded = ConnectToKey::from_raw(raw);

        assert_eq!(decoded.account_id, 42);
        assert_eq!(decoded.connection_type, 1);
        assert_eq!(decoded.key, 0x1234_5678);
    }

    #[test]
    fn connect_to_key_zero() {
        let key = ConnectToKey {
            account_id: 0,
            connection_type: 0,
            key: 0,
        };
        assert_eq!(key.raw(), 0);

        let decoded = ConnectToKey::from_raw(0);
        assert_eq!(decoded.account_id, 0);
        assert_eq!(decoded.connection_type, 0);
        assert_eq!(decoded.key, 0);
    }

    #[test]
    fn connect_to_key_max_account_id() {
        let key = ConnectToKey {
            account_id: 0xFFFF_FFFF,
            connection_type: 0,
            key: 0,
        };
        let decoded = ConnectToKey::from_raw(key.raw());
        assert_eq!(decoded.account_id, 0xFFFF_FFFF);
        assert_eq!(decoded.connection_type, 0);
    }

    #[test]
    fn connect_to_write_ipv4() {
        let pkt_data = ConnectTo {
            signature: [0xAA; 256],
            address: ConnectToAddress::IPv4([127, 0, 0, 1]),
            port: 8086,
            serial: ConnectToSerial::WorldAttempt1,
            con: 1,
            key: 12345,
        };

        let data = pkt_data.to_bytes();
        // opcode(2) + sig(256) + type(1) + ip(4) + port(2) + serial(4) + con(1) + key(8) = 278
        assert_eq!(data.len(), 278);

        // Verify opcode
        let opcode = u16::from_le_bytes([data[0], data[1]]);
        assert_eq!(opcode, ServerOpcodes::ConnectTo as u16);

        // Verify signature starts at offset 2
        assert_eq!(&data[2..258], &[0xAA; 256]);

        // Verify address type = 1 (IPv4)
        assert_eq!(data[258], 1);

        // Verify IP = 127.0.0.1
        assert_eq!(&data[259..263], &[127, 0, 0, 1]);

        // Verify port = 8086 LE
        assert_eq!(u16::from_le_bytes([data[263], data[264]]), 8086);

        // Verify serial = 17 (WorldAttempt1)
        assert_eq!(
            u32::from_le_bytes([data[265], data[266], data[267], data[268]]),
            17
        );

        // Verify con = 1
        assert_eq!(data[269], 1);

        // Verify key = 12345
        assert_eq!(
            i64::from_le_bytes([
                data[270], data[271], data[272], data[273],
                data[274], data[275], data[276], data[277],
            ]),
            12345
        );
    }

    #[test]
    fn resume_comms_write() {
        let pkt = ResumeComms;
        let data = pkt.to_bytes();
        // opcode only, no payload
        assert_eq!(data.len(), 2);
        let opcode = u16::from_le_bytes([data[0], data[1]]);
        assert_eq!(opcode, ServerOpcodes::ResumeComms as u16);
    }

    #[test]
    fn connect_to_failed_read() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(17); // WorldAttempt1

        pkt.reset_read();
        let failed = ConnectToFailed::read(&mut pkt).unwrap();
        assert_eq!(failed.serial, ConnectToSerial::WorldAttempt1);
    }

    #[test]
    fn connect_to_serial_next() {
        assert_eq!(
            ConnectToSerial::WorldAttempt1.next(),
            Some(ConnectToSerial::WorldAttempt2)
        );
        assert_eq!(
            ConnectToSerial::WorldAttempt5.next(),
            None
        );
    }
}
