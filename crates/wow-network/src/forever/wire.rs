//! Bounded 32-bit-opcode framing and target AuthSession decoding.

use super::ForeverSocketError;

pub(super) const SERVER_HELLO: &[u8] = b"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n";
pub(super) const CLIENT_HELLO: &[u8] = b"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n";
pub(super) const HEADER_SIZE: usize = 16;
pub(super) const MAX_FRAME: usize = 0x10000;

/// Decrypted opcode/body. Debug deliberately omits the payload.
pub struct Frame {
    opcode: u32,
    payload: Vec<u8>,
}

impl Frame {
    pub fn opcode(&self) -> u32 {
        self.opcode
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub(super) fn decode(data: Vec<u8>) -> Result<Self, ForeverSocketError> {
        if !(4..MAX_FRAME).contains(&data.len()) {
            return Err(ForeverSocketError::Protocol);
        }
        let opcode = u32::from_le_bytes(data[..4].try_into().expect("checked size"));
        Ok(Self {
            opcode,
            payload: data[4..].to_vec(),
        })
    }
}

pub(super) fn frame_data(opcode: u32, payload: &[u8]) -> Result<Vec<u8>, ForeverSocketError> {
    if payload.len() >= MAX_FRAME - 4 {
        return Err(ForeverSocketError::Protocol);
    }
    let mut data = Vec::with_capacity(4 + payload.len());
    data.extend_from_slice(&opcode.to_le_bytes());
    data.extend_from_slice(payload);
    Ok(data)
}

pub(super) fn header(size: usize, tag: [u8; 12]) -> Result<[u8; HEADER_SIZE], ForeverSocketError> {
    if !(4..MAX_FRAME).contains(&size) {
        return Err(ForeverSocketError::Protocol);
    }
    let mut bytes = [0; HEADER_SIZE];
    bytes[..4].copy_from_slice(&(size as u32).to_le_bytes());
    bytes[4..].copy_from_slice(&tag);
    Ok(bytes)
}

pub(super) fn frame_size(bytes: &[u8; HEADER_SIZE]) -> Result<usize, ForeverSocketError> {
    let size = u32::from_le_bytes(bytes[..4].try_into().expect("fixed header")) as usize;
    if !(4..MAX_FRAME).contains(&size) {
        return Err(ForeverSocketError::Protocol);
    }
    Ok(size)
}

// No Debug: challenges, digest and ticket must never be rendered into logs.
pub(super) struct AuthSession {
    pub(super) region: u32,
    pub(super) district: u32,
    pub(super) realm: u32,
    pub(super) local_challenge: [u8; 32],
    pub(super) digest: [u8; 24],
    pub(super) ticket: String,
}

impl AuthSession {
    pub(super) fn decode(payload: &[u8]) -> Result<Self, ForeverSocketError> {
        // DOS8, region/district/realm12, local32, digest24, IPv6 bit+padding1,
        // uint32 ticket length. Unlike the reference truncation, fail closed.
        if payload.len() < 81 || payload[76] & 0x7F != 0 {
            return Err(ForeverSocketError::Protocol);
        }
        let size =
            u32::from_le_bytes(payload[77..81].try_into().expect("checked payload")) as usize;
        if !(1..=1024).contains(&size) || payload.len() != 81 + size {
            return Err(ForeverSocketError::Protocol);
        }
        let number = |start| {
            u32::from_le_bytes(
                payload[start..start + 4]
                    .try_into()
                    .expect("checked payload"),
            )
        };
        let ticket = std::str::from_utf8(&payload[81..])
            .map_err(|_| ForeverSocketError::Protocol)?
            .to_owned();
        Ok(Self {
            region: number(8),
            district: number(12),
            realm: number(16),
            local_challenge: payload[20..52].try_into().expect("checked payload"),
            digest: payload[52..76].try_into().expect("checked payload"),
            ticket,
        })
    }
}
