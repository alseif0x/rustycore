//! Target DBQueryBulk/DBReply payloads, without legacy opcode framing.
//! 02245dcd HotfixPackets.cpp:58-76; native 70170 requests match 4+2+4*N.
//! Data is serialized by a typed store, never copied from a compressed DB2.

use super::hotfix::HotfixStatus;
use crate::WorldPacket;
use thiserror::Error;

pub const MAX_QUERIES: usize = (1 << 13) - 1;
pub const CLASSIC_QUERY_OPCODE: u32 = 0x440010;
pub const CLASSIC_REPLY_OPCODE: u32 = 0x4A0000;

#[derive(Debug, PartialEq, Eq)]
pub struct DBQueryBulk {
    pub table_hash: u32,
    pub record_ids: Vec<u32>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DBQueryError {
    #[error("DB query payload is truncated or has trailing bytes")]
    Length,
    #[error("DB reply data exceeds uint32 length")]
    ContentTooLarge,
}

impl DBQueryBulk {
    pub fn decode(payload: &[u8]) -> Result<Self, DBQueryError> {
        if payload.len() < 6 {
            return Err(DBQueryError::Length);
        }
        // ByteBuffer's bits are MSB-first. Reading the next uint32 aligns to
        // the byte boundary; the three padding bits are not part of the count.
        let count = (usize::from(payload[4]) << 5) | (usize::from(payload[5]) >> 3);
        if payload.len() != 6 + count * 4 {
            return Err(DBQueryError::Length);
        }
        Ok(Self {
            table_hash: u32::from_le_bytes(payload[..4].try_into().expect("checked header")),
            record_ids: payload[6..]
                .chunks_exact(4)
                .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("exact chunk")))
                .collect(),
        })
    }
}

/// No Debug: the data can contain private TACT record bytes.
pub struct DBReply<'a> {
    pub table_hash: u32,
    pub record_id: u32,
    pub timestamp: u32,
    pub status: HotfixStatus,
    pub data: &'a [u8],
}

impl DBReply<'_> {
    pub fn encode_payload(&self) -> Result<Vec<u8>, DBQueryError> {
        let size = u32::try_from(self.data.len()).map_err(|_| DBQueryError::ContentTooLarge)?;
        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(self.table_hash);
        packet.write_uint32(self.record_id);
        packet.write_uint32(self.timestamp);
        packet.write_bits(self.status as u32, 3);
        packet.write_uint32(size); // flushes the source's three status bits
        packet.write_bytes(self.data);
        Ok(packet.into_data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(ids: &[u32]) -> Vec<u8> {
        let mut bytes = 0xDF2F53CFu32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&((ids.len() as u16) << 3).to_be_bytes());
        for id in ids {
            bytes.extend_from_slice(&id.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn queries_match_native_counts_and_preserve_order_duplicates_and_unsigned_ids() {
        for count in [0, 44, 84, MAX_QUERIES] {
            let ids = vec![u32::MAX; count];
            let decoded = DBQueryBulk::decode(&query(&ids)).unwrap();
            assert_eq!(decoded.table_hash, 0xDF2F53CF);
            assert_eq!(decoded.record_ids, ids);
        }
        assert_eq!(
            DBQueryBulk::decode(&query(&[17, 2, 17]))
                .unwrap()
                .record_ids,
            [17, 2, 17]
        );
        let mut padding = query(&[17]);
        padding[5] |= 7;
        assert_eq!(DBQueryBulk::decode(&padding).unwrap().record_ids, [17]);
    }

    #[test]
    fn every_truncated_prefix_and_trailing_data_is_rejected() {
        let valid = query(&[1, 2]);
        for length in 0..valid.len() {
            assert_eq!(
                DBQueryBulk::decode(&valid[..length]),
                Err(DBQueryError::Length)
            );
        }
        let mut trailing = valid.clone();
        trailing.push(0);
        assert_eq!(DBQueryBulk::decode(&trailing), Err(DBQueryError::Length));
        let mut wrong_count = valid;
        wrong_count[5] = 24;
        assert_eq!(DBQueryBulk::decode(&wrong_count), Err(DBQueryError::Length));
    }

    #[test]
    fn reply_serializes_source_order_and_msb_status_without_prepending_record_id_to_data() {
        let data = [0xAB; 16]; // synthetic, not an acquired key
        let encoded = DBReply {
            table_hash: 0xDF2F53CF,
            record_id: u32::MAX,
            timestamp: 0x12345678,
            status: HotfixStatus::Valid,
            data: &data,
        }
        .encode_payload()
        .unwrap();
        let mut expected = vec![
            0xCF, 0x53, 0x2F, 0xDF, 255, 255, 255, 255, 0x78, 0x56, 0x34, 0x12, 0x20, 16, 0, 0, 0,
        ];
        expected.extend_from_slice(&data);
        assert_eq!(encoded, expected);
        let absent = DBReply {
            table_hash: 1,
            record_id: 2,
            timestamp: 3,
            status: HotfixStatus::Invalid,
            data: &[],
        }
        .encode_payload()
        .unwrap();
        assert_eq!(absent.len(), 17);
        assert_eq!(&absent[12..], &[0x60, 0, 0, 0, 0]);
    }
}
