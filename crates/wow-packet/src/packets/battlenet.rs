// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battlenet service packets: BattlenetRequest (CMSG) and BattlenetResponse (SMSG).
//!
//! The client sends BattlenetRequest during character select to invoke
//! GameUtilitiesService RPCs. The server must always respond with a
//! BattlenetResponse — either with a result or an error code.

use wow_constants::ServerOpcodes;

use crate::{PacketError, ServerPacket, WorldPacket};

// ── MethodCall ──────────────────────────────────────────────────────

/// Encodes a BNet service call: serviceHash (high 32 bits of Type) +
/// methodId (low 32 bits of Type), plus ObjectId and Token.
#[derive(Debug, Clone, Default)]
pub struct MethodCall {
    /// High 32 bits = serviceHash, low 32 bits = methodId.
    pub method_type: u64,
    pub object_id: i64,
    pub token: u32,
}

impl MethodCall {
    pub fn service_hash(&self) -> u32 {
        ((self.method_type >> 32) & 0xFFFF_FFFF) as u32
    }

    pub fn method_id(&self) -> u32 {
        (self.method_type & 0xFFFF_FFFF) as u32
    }

    pub fn from_parts(service_hash: u32, method_id: u32, token: u32) -> Self {
        Self {
            method_type: ((service_hash as u64) << 32) | (method_id as u64),
            object_id: 1,
            token,
        }
    }

    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let method_type = pkt.read_uint64()?;
        let object_id = pkt.read_int64()?;
        let token = pkt.read_uint32()?;
        Ok(Self {
            method_type,
            object_id,
            token,
        })
    }

    pub fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint64(self.method_type);
        pkt.write_int64(self.object_id);
        pkt.write_uint32(self.token);
    }
}

// ── BattlenetRequest (CMSG 0x36FD) ─────────────────────────────────

/// Client → Server: invokes a BNet service method (GameUtilitiesService etc.).
#[derive(Debug)]
pub struct BattlenetRequest {
    pub method: MethodCall,
    pub data: Vec<u8>,
}

impl BattlenetRequest {
    pub fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let method = MethodCall::read(pkt)?;
        let proto_size = pkt.read_int32()?;
        let data = if proto_size > 0 {
            pkt.read_bytes(proto_size as usize)?
        } else {
            vec![]
        };
        Ok(Self { method, data })
    }
}

// ── BattlenetResponse (SMSG 0x2807) ────────────────────────────────

/// BNet RPC error codes (subset).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlenetRpcErrorCode {
    Ok = 0,
    RpcNotImplemented = 0x0000_0bc7,
}

/// Server → Client: response to a BattlenetRequest.
pub struct BattlenetResponse {
    pub status: BattlenetRpcErrorCode,
    pub method: MethodCall,
    pub data: Vec<u8>,
}

impl BattlenetResponse {
    /// Create an error response (no data).
    pub fn error(
        service_hash: u32,
        method_id: u32,
        token: u32,
        status: BattlenetRpcErrorCode,
    ) -> Self {
        Self {
            status,
            method: MethodCall::from_parts(service_hash, method_id, token),
            data: vec![],
        }
    }
}

impl ServerPacket for BattlenetResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlenetResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.status as u32);
        self.method.write(pkt);
        pkt.write_uint32(self.data.len() as u32);
        if !self.data.is_empty() {
            pkt.write_bytes(&self.data);
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_call_round_trip() {
        let mc = MethodCall::from_parts(0xDEAD_BEEF, 42, 99);
        assert_eq!(mc.service_hash(), 0xDEAD_BEEF);
        assert_eq!(mc.method_id(), 42);
        assert_eq!(mc.token, 99);

        let mut pkt = WorldPacket::new_server(ServerOpcodes::BattlenetResponse);
        mc.write(&mut pkt);
        pkt.reset_read();
        // skip opcode (2 bytes)
        let _ = pkt.read_uint16();
        let mc2 = MethodCall::read(&mut pkt).unwrap();
        assert_eq!(mc2.service_hash(), 0xDEAD_BEEF);
        assert_eq!(mc2.method_id(), 42);
        assert_eq!(mc2.token, 99);
    }

    #[test]
    fn battlenet_response_error_writes_correctly() {
        let resp = BattlenetResponse::error(
            0x12345678,
            1,
            7,
            BattlenetRpcErrorCode::RpcNotImplemented,
        );
        let bytes = resp.to_bytes();
        // opcode(2) + u32 status(4) + u64 type(8) + i64 objectId(8) + u32 token(4) + u32 dataSize(4) = 30
        assert_eq!(bytes.len(), 30);

        // Skip opcode (2 bytes), read status
        let status = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(status, 0x0bc7);

        // Type (serviceHash << 32 | methodId)
        let method_type = u64::from_le_bytes(bytes[6..14].try_into().unwrap());
        assert_eq!(method_type, (0x12345678u64 << 32) | 1);

        // ObjectId = 1
        let obj_id = i64::from_le_bytes(bytes[14..22].try_into().unwrap());
        assert_eq!(obj_id, 1);

        // Token = 7
        let token = u32::from_le_bytes(bytes[22..26].try_into().unwrap());
        assert_eq!(token, 7);

        // DataSize = 0
        let data_size = u32::from_le_bytes(bytes[26..30].try_into().unwrap());
        assert_eq!(data_size, 0);
    }

    #[test]
    fn battlenet_request_read() {
        // Build a fake BattlenetRequest packet payload
        let mut pkt = WorldPacket::new_server(ServerOpcodes::BattlenetResponse);
        // MethodCall
        pkt.write_uint64((0xAAAA_BBBBu64 << 32) | 5); // Type
        pkt.write_int64(1); // ObjectId
        pkt.write_uint32(42); // Token
        // Proto data
        pkt.write_int32(3); // size
        pkt.write_bytes(&[0x01, 0x02, 0x03]); // data

        pkt.reset_read();
        // skip opcode (2 bytes)
        let _ = pkt.read_uint16();
        let req = BattlenetRequest::read(&mut pkt).unwrap();
        assert_eq!(req.method.service_hash(), 0xAAAA_BBBB);
        assert_eq!(req.method.method_id(), 5);
        assert_eq!(req.method.token, 42);
        assert_eq!(req.data, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn battlenet_request_empty_data() {
        let mut pkt = WorldPacket::new_server(ServerOpcodes::BattlenetResponse);
        pkt.write_uint64(0);
        pkt.write_int64(1);
        pkt.write_uint32(0);
        pkt.write_int32(0); // zero-length data

        pkt.reset_read();
        let _ = pkt.read_uint16(); // skip opcode
        let req = BattlenetRequest::read(&mut pkt).unwrap();
        assert!(req.data.is_empty());
    }
}
