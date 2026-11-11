//! Battle.net RPC envelopes used during Forever character selection.

use crate::WorldPacket;

use super::{ForeverPacketError, count, finish};

pub const BATTLENET_REQUEST_OPCODE: u32 = 0x440124;
pub const BATTLENET_RESPONSE_OPCODE: u32 = 0x4602AF;
pub const RESOURCES_SERVICE_HASH: u32 = 0xECBE75BA;
pub const GET_CONTENT_HANDLE_METHOD_ID: u32 = 1;
pub const FRIENDS_SERVICE_HASH: u32 = 0x5869BE8C;
pub const GET_FRIENDS_METHOD_ID: u32 = 0x40000003;
pub const GET_RECEIVED_INVITATIONS_METHOD_ID: u32 = 0x40000004;
pub const GET_SENT_INVITATIONS_METHOD_ID: u32 = 0x40000005;
pub const BLOCK_LIST_SERVICE_HASH: u32 = 0x8E8F5FB0;
pub const SUBSCRIBE_METHOD_ID: u32 = 0x40000001;
pub const RPC_NOT_IMPLEMENTED: u32 = 0x00000BC7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlenetRequest {
    service_hash: u32,
    method_id: u32,
    object_id: u64,
    token: u32,
    data: Vec<u8>,
}

impl BattlenetRequest {
    pub fn decode(payload: &[u8]) -> Option<Self> {
        if payload.len() < 24 {
            return None;
        }
        let method_type = u64::from_le_bytes(payload[..8].try_into().ok()?);
        let data_size = u32::from_le_bytes(payload[20..24].try_into().ok()?) as usize;
        if data_size > 0xFFFF || payload.len() != 24 + data_size {
            return None;
        }
        Some(Self {
            service_hash: (method_type >> 32) as u32,
            method_id: method_type as u32,
            object_id: u64::from_le_bytes(payload[8..16].try_into().ok()?),
            token: u32::from_le_bytes(payload[16..20].try_into().ok()?),
            data: payload[24..].to_vec(),
        })
    }

    pub fn service_hash(&self) -> u32 {
        self.service_hash
    }

    pub fn method_id(&self) -> u32 {
        self.method_id
    }

    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    pub fn token(&self) -> u32 {
        self.token
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlenetResponse {
    status: u32,
    service_hash: u32,
    method_id: u32,
    object_id: u64,
    token: u32,
    data: Vec<u8>,
}

impl BattlenetResponse {
    pub fn success(service_hash: u32, method_id: u32, token: u32, data: Vec<u8>) -> Self {
        Self {
            status: 0,
            service_hash,
            method_id,
            object_id: 1,
            token,
            data,
        }
    }

    pub fn subscription(service_hash: u32, data: Vec<u8>) -> Self {
        Self {
            status: 0,
            service_hash,
            method_id: SUBSCRIBE_METHOD_ID,
            object_id: 0,
            token: 0,
            data,
        }
    }

    pub fn not_implemented(service_hash: u32, method_id: u32, token: u32) -> Self {
        Self {
            status: RPC_NOT_IMPLEMENTED,
            service_hash,
            method_id,
            object_id: 1,
            token,
            data: vec![],
        }
    }

    pub fn encode_payload(&self) -> Result<Vec<u8>, ForeverPacketError> {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(self.status);
        packet.write_uint64((u64::from(self.service_hash) << 32) | u64::from(self.method_id));
        packet.write_uint64(self.object_id);
        packet.write_uint32(self.token);
        packet.write_uint32(count(self.data.len(), "battlenet_response_data")?);
        packet.write_bytes(&self.data);
        Ok(finish(packet))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentHandleRequest {
    program: u32,
    stream: u32,
    version: u32,
}

impl ContentHandleRequest {
    pub fn decode(data: &[u8]) -> Option<Self> {
        if !matches!(data.len(), 10 | 15) || data[0] != 0x0D || data[5] != 0x15 {
            return None;
        }
        if data.len() == 15 && data[10] != 0x1D {
            return None;
        }
        Some(Self {
            program: u32::from_le_bytes(data[1..5].try_into().ok()?),
            stream: u32::from_le_bytes(data[6..10].try_into().ok()?),
            version: if data.len() == 15 {
                u32::from_le_bytes(data[11..15].try_into().ok()?)
            } else {
                0x656E5553
            },
        })
    }

    pub fn program(&self) -> u32 {
        self.program
    }

    pub fn stream(&self) -> u32 {
        self.stream
    }

    pub fn version(&self) -> u32 {
        self.version
    }
}

/// Official beta content handle from the 70291 capture in b6ae12c227. A zero
/// hash makes the client wait for a depot download instead of enumerating.
pub fn official_beta_content_handle(request: &ContentHandleRequest) -> Option<Vec<u8>> {
    if request.program != 0x424E || request.stream != 0x61706674 || request.version != 0x656E5553 {
        return None;
    }
    const HASH: [u8; 32] = [
        0x50, 0x63, 0x2D, 0x92, 0xDA, 0x0B, 0x7C, 0x34, 0xBA, 0x82, 0xAF, 0xF0, 0x31, 0x9E, 0x83,
        0x76, 0x06, 0x1D, 0xC0, 0xD0, 0x9B, 0x10, 0x94, 0xE2, 0xD3, 0x6D, 0x39, 0x22, 0x8C, 0x2D,
        0xAC, 0x0C,
    ];
    const URL: &str = "https://prod.depot.battle.net/${hash}.${usage}";
    let mut data = Vec::with_capacity(5 + 5 + 34 + 2 + URL.len());
    data.push(0x0D);
    data.extend_from_slice(&0x5858_u32.to_le_bytes());
    data.push(0x15);
    data.extend_from_slice(&0x70667479_u32.to_le_bytes());
    data.push(0x1A);
    data.push(32);
    data.extend_from_slice(&HASH);
    data.push(0x22);
    data.push(URL.len() as u8);
    data.extend_from_slice(URL.as_bytes());
    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Vec<u8> {
        let mut payload = ((u64::from(RESOURCES_SERVICE_HASH) << 32) | 1)
            .to_le_bytes()
            .to_vec();
        payload.extend_from_slice(&7_u64.to_le_bytes());
        payload.extend_from_slice(&9_u32.to_le_bytes());
        payload.extend_from_slice(&15_u32.to_le_bytes());
        for (tag, value) in [
            (0x0D_u8, 0x424E_u32),
            (0x15, 0x61706674),
            (0x1D, 0x656E5553),
        ] {
            payload.push(tag);
            payload.extend_from_slice(&value.to_le_bytes());
        }
        payload
    }

    #[test]
    fn content_handle_response_preserves_method_token_and_requested_stream() {
        let payload = request();
        let request = BattlenetRequest::decode(&payload).unwrap();
        assert_eq!(request.service_hash(), RESOURCES_SERVICE_HASH);
        assert_eq!(request.method_id(), GET_CONTENT_HANDLE_METHOD_ID);
        assert_eq!(request.object_id(), 7);
        assert_eq!(request.token(), 9);
        let content = ContentHandleRequest::decode(request.data()).unwrap();
        assert_eq!(content.program(), 0x424E);
        assert_eq!(content.stream(), 0x61706674);
        assert_eq!(content.version(), 0x656E5553);

        let response = BattlenetResponse::success(
            request.service_hash(),
            request.method_id(),
            request.token(),
            official_beta_content_handle(&content).unwrap(),
        )
        .encode_payload()
        .unwrap();
        assert_eq!(response.len(), 120);
        assert_eq!(&response[..4], &[0; 4]);
        assert_eq!(&response[4..12], &payload[..8]);
        assert_eq!(&response[12..20], &1_u64.to_le_bytes());
        assert_eq!(&response[20..24], &9_u32.to_le_bytes());
        assert_eq!(&response[24..28], &92_u32.to_le_bytes());
        assert_eq!(&response[28..33], &[0x0D, 0x58, 0x58, 0, 0]);
        assert_eq!(&response[33..38], &[0x15, 0x79, 0x74, 0x66, 0x70]);
        assert_eq!(&response[38..41], &[0x1A, 32, 0x50]);
        assert_eq!(&response[72..76], &[0x22, 46, b'h', b't']);
        let mut wrong_program = content;
        wrong_program.program = 0x574F5745;
        assert!(official_beta_content_handle(&wrong_program).is_none());
    }

    #[test]
    fn malformed_envelopes_and_required_content_fields_fail_closed() {
        let payload = request();
        for size in 0..payload.len() {
            assert!(BattlenetRequest::decode(&payload[..size]).is_none());
        }
        let mut excess = payload;
        excess.push(0);
        assert!(BattlenetRequest::decode(&excess).is_none());
        assert!(ContentHandleRequest::decode(&[]).is_none());
        assert!(ContentHandleRequest::decode(&[0; 10]).is_none());
        assert!(ContentHandleRequest::decode(&[0; 15]).is_none());
    }

    #[test]
    fn unsupported_rpc_has_status_and_no_response_body() {
        let response = BattlenetResponse::not_implemented(3, 4, 5)
            .encode_payload()
            .unwrap();
        assert_eq!(response.len(), 28);
        assert_eq!(&response[..4], &RPC_NOT_IMPLEMENTED.to_le_bytes());
        assert_eq!(&response[24..], &[0; 4]);
    }
}
