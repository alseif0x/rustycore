//! Payload-only Forever hotfix codecs.
//!
//! The layouts are from `HotfixPackets.cpp:91-124` at
//! `02245dcd245e7433e524577656177723d3e4992e`.  This module deliberately
//! treats the trailing HotfixContent as opaque, already-validated SQL
//! serialized record bytes.  It does not parse or manufacture WDC records.

use std::convert::TryFrom;

use thiserror::Error;

use crate::{PacketError, WorldPacket};

/// A bounded request decoder prevents a client count from driving allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotfixRequestDecoder {
    max_pushes: usize,
}

impl HotfixRequestDecoder {
    pub const fn new(max_pushes: usize) -> Self {
        Self { max_pushes }
    }

    pub const fn max_pushes(self) -> usize {
        self.max_pushes
    }

    /// Decode only the request payload (no opcode or world framing).
    ///
    /// C++ `HotfixRequest::Read` consumes `ClientBuild`, `DataBuild`, one
    /// uint32 count, and that many int32 push IDs.  Unlike the legacy reader,
    /// this bounded decoder also rejects bytes left after the final push ID.
    pub fn decode(self, payload: &[u8]) -> Result<HotfixRequest, HotfixPacketError> {
        let mut packet = WorldPacket::from_bytes(payload);
        let client_build = packet.read_uint32()?;
        let data_build = packet.read_uint32()?;
        let count = packet.read_uint32()? as usize;
        if count > self.max_pushes {
            return Err(HotfixPacketError::PushCount {
                count,
                maximum: self.max_pushes,
            });
        }

        let mut push_ids = Vec::with_capacity(count);
        for _ in 0..count {
            push_ids.push(packet.read_int32()?);
        }
        if !packet.is_empty() {
            return Err(HotfixPacketError::TrailingBytes {
                count: packet.remaining(),
            });
        }
        Ok(HotfixRequest {
            client_build,
            data_build,
            push_ids,
        })
    }
}

/// C++ `WorldPackets::Hotfix::HotfixRequest` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixRequest {
    pub client_build: u32,
    pub data_build: u32,
    pub push_ids: Vec<i32>,
}

impl HotfixRequest {
    pub fn decode(payload: &[u8], max_pushes: usize) -> Result<Self, HotfixPacketError> {
        HotfixRequestDecoder::new(max_pushes).decode(payload)
    }
}

/// `DB2Manager::HotfixRecord::Status` from `DB2Stores.h:404-418`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HotfixStatus {
    NotSet = 0,
    Valid = 1,
    RecordRemoved = 2,
    Invalid = 3,
    NotPublic = 4,
}

impl TryFrom<u8> for HotfixStatus {
    type Error = HotfixPacketError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NotSet),
            1 => Ok(Self::Valid),
            2 => Ok(Self::RecordRemoved),
            3 => Ok(Self::Invalid),
            4 => Ok(Self::NotPublic),
            value => Err(HotfixPacketError::InvalidStatus { value }),
        }
    }
}

/// One `HotfixConnect::HotfixData` record header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotfixConnectRecord {
    pub push_id: i32,
    pub unique_id: u32,
    pub table_hash: u32,
    pub record_id: i32,
    pub size: u32,
    pub status: HotfixStatus,
}

/// C++ `WorldPackets::Hotfix::HotfixConnect`, without opcode/framing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixConnectPayload {
    pub records: Vec<HotfixConnectRecord>,
    /// Concatenated, already-validated SQL serialized record blobs.
    pub content: Vec<u8>,
}

impl HotfixConnectPayload {
    pub fn encode_payload(&self) -> Result<Vec<u8>, HotfixPacketError> {
        let expected_content = self
            .records
            .iter()
            .try_fold(0usize, |total, record| {
                total.checked_add(record.size as usize)
            })
            .ok_or(HotfixPacketError::ContentTooLarge)?;
        if expected_content != self.content.len() {
            return Err(HotfixPacketError::ContentSizeMismatch {
                expected: expected_content,
                actual: self.content.len(),
            });
        }
        let record_count = u32::try_from(self.records.len()).map_err(|_| {
            HotfixPacketError::RecordCountTooLarge {
                count: self.records.len(),
            }
        })?;
        let content_count =
            u32::try_from(self.content.len()).map_err(|_| HotfixPacketError::ContentTooLarge)?;

        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(record_count);
        for record in &self.records {
            packet.write_int32(record.push_id);
            packet.write_uint32(record.unique_id);
            packet.write_uint32(record.table_hash);
            packet.write_int32(record.record_id);
            packet.write_uint32(record.size);
            packet.write_bits(record.status as u32, 3);
            packet.flush_bits();
        }
        packet.write_uint32(content_count);
        packet.write_bytes(&self.content);
        packet.flush_bits();
        Ok(packet.into_data())
    }
}

/// Errors raised before any untrusted hotfix request/content is accepted.
#[derive(Debug, Error)]
pub enum HotfixPacketError {
    #[error(transparent)]
    Packet(#[from] PacketError),
    #[error("hotfix push count {count} exceeds configured maximum {maximum}")]
    PushCount { count: usize, maximum: usize },
    #[error("hotfix request has {count} trailing bytes")]
    TrailingBytes { count: usize },
    #[error("hotfix record count {count} does not fit in uint32")]
    RecordCountTooLarge { count: usize },
    #[error("hotfix content length overflows the payload count")]
    ContentTooLarge,
    #[error("hotfix record sizes total {expected} bytes but content has {actual}")]
    ContentSizeMismatch { expected: usize, actual: usize },
    #[error("hotfix status {value} is outside DB2 status values 0..=4")]
    InvalidStatus { value: u8 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_bytes(push_ids: &[i32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12 + push_ids.len() * 4);
        bytes.extend_from_slice(&70170u32.to_le_bytes());
        bytes.extend_from_slice(&70170u32.to_le_bytes());
        bytes.extend_from_slice(&(push_ids.len() as u32).to_le_bytes());
        for push_id in push_ids {
            bytes.extend_from_slice(&push_id.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn request_roundtrips_exact_payload() {
        let decoded = HotfixRequestDecoder::new(4)
            .decode(&request_bytes(&[7, -2]))
            .unwrap();
        assert_eq!(decoded.client_build, 70170);
        assert_eq!(decoded.data_build, 70170);
        assert_eq!(decoded.push_ids, vec![7, -2]);
    }

    #[test]
    fn request_rejects_truncation_trailing_and_count_overflow() {
        let mut truncated = request_bytes(&[7]);
        truncated.pop();
        assert!(matches!(
            HotfixRequestDecoder::new(4).decode(&truncated),
            Err(HotfixPacketError::Packet(PacketError::ReadPastEnd { .. }))
        ));

        let mut trailing = request_bytes(&[]);
        trailing.push(0xEE);
        assert!(matches!(
            HotfixRequestDecoder::new(4).decode(&trailing),
            Err(HotfixPacketError::TrailingBytes { count: 1 })
        ));

        let mut too_many = request_bytes(&[]);
        too_many[8..12].copy_from_slice(&8u32.to_le_bytes());
        assert!(matches!(
            HotfixRequestDecoder::new(4).decode(&too_many),
            Err(HotfixPacketError::PushCount {
                count: 8,
                maximum: 4
            })
        ));
    }

    #[test]
    fn connect_encodes_status_bits_and_content_in_source_order() {
        let payload = HotfixConnectPayload {
            records: vec![HotfixConnectRecord {
                push_id: -2,
                unique_id: 7,
                table_hash: 0x1122_3344,
                record_id: -9,
                size: 2,
                status: HotfixStatus::Valid,
            }],
            content: vec![0xAA, 0xBB],
        }
        .encode_payload()
        .unwrap();
        assert_eq!(
            payload,
            vec![
                1, 0, 0, 0, 0xFE, 0xFF, 0xFF, 0xFF, 7, 0, 0, 0, 0x44, 0x33, 0x22, 0x11, 0xF7, 0xFF,
                0xFF, 0xFF, 2, 0, 0, 0, 0x20, 2, 0, 0, 0, 0xAA, 0xBB,
            ]
        );
    }

    #[test]
    fn connect_rejects_invalid_status_and_content_mismatch() {
        assert!(matches!(
            HotfixStatus::try_from(5),
            Err(HotfixPacketError::InvalidStatus { value: 5 })
        ));
        let payload = HotfixConnectPayload {
            records: vec![HotfixConnectRecord {
                push_id: 1,
                unique_id: 1,
                table_hash: 1,
                record_id: 1,
                size: 3,
                status: HotfixStatus::Invalid,
            }],
            content: vec![0, 1],
        };
        assert!(matches!(
            payload.encode_payload(),
            Err(HotfixPacketError::ContentSizeMismatch {
                expected: 3,
                actual: 2
            })
        ));
    }
}
