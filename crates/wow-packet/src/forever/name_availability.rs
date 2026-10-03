//! Payload-only Classic name-availability codecs.
//!
//! The request layout is pinned to `CharacterPackets.cpp:464-477` and
//! `CharacterPackets.h:302-311` at
//! `02245dcd245e7433e524577656177723d3e4992e`.  That source annotates the
//! bit layout as Classic 1.60.1.70009; a 1.60.1.70170 capture has observed the
//! request opcode, but has not independently proven a changed payload.
//!
//! This module deliberately stops at transport decoding/encoding.  Name
//! normalization, reservation, database lookup, and handler admission remain
//! owned by the world-session layer.

use crate::{PacketError, WorldPacket};
use thiserror::Error;

/// Classic client wire opcode observed for the request.  No framing is
/// written by this module.
pub const CHECK_CHARACTER_NAME_AVAILABILITY_OPCODE: u32 = 0x440071;

/// Classic server wire opcode for the fixed eight-byte result payload.
pub const CHECK_CHARACTER_NAME_AVAILABILITY_RESULT_OPCODE: u32 = 0x46001B;

const NAME_LENGTH_BITS: u32 = 6;
const UNKNOWN_BITS: u32 = 3;
const MAX_NAME_BYTES: usize = (1 << NAME_LENGTH_BITS) - 1;

/// A decoded `CMSG_CHECK_CHARACTER_NAME_AVAILABILITY` payload.
///
/// The source packet includes a surname even though the current C++ handler
/// ignores it.  Keep both strings private so accidental diagnostics cannot
/// render user-provided names; callers can inspect them through the narrow
/// getters when the owning handler needs them.
#[derive(Clone, PartialEq, Eq)]
pub struct CheckCharacterNameAvailability {
    sequence_index: u32,
    name: String,
    surname: String,
    unknown_bits: u8,
}

impl CheckCharacterNameAvailability {
    /// Decode only the payload; no 32-bit opcode or transport header is read.
    pub fn decode(payload: &[u8]) -> Result<Self, NameAvailabilityError> {
        let mut packet = WorldPacket::from_bytes(payload);
        let sequence_index = packet.read_uint32()?;
        let name_length = packet.read_bits(NAME_LENGTH_BITS)? as usize;
        let unknown_bits = packet.read_bits(UNKNOWN_BITS)? as u8;
        let surname_length = packet.read_bits(NAME_LENGTH_BITS)? as usize;
        packet.reset_bits();

        // Six-bit source lengths make each read bounded at 63 bytes.  Keep
        // these checks explicit so that this contract stays local if the
        // buffer reader is ever replaced.
        debug_assert!(name_length <= MAX_NAME_BYTES);
        debug_assert!(surname_length <= MAX_NAME_BYTES);
        let name = packet.read_string(name_length)?;
        let surname = packet.read_string(surname_length)?;

        if !packet.is_empty() {
            return Err(NameAvailabilityError::TrailingBytes {
                count: packet.remaining(),
            });
        }

        Ok(Self {
            sequence_index,
            name,
            surname,
            unknown_bits,
        })
    }

    pub fn sequence_index(&self) -> u32 {
        self.sequence_index
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn surname(&self) -> &str {
        &self.surname
    }

    pub fn unknown_bits(&self) -> u8 {
        self.unknown_bits
    }
}

/// Errors raised while decoding the bounded request payload.
#[derive(Debug, Error)]
pub enum NameAvailabilityError {
    #[error(transparent)]
    Packet(#[from] PacketError),
    #[error("name-availability payload has {count} trailing bytes")]
    TrailingBytes { count: usize },
}

/// The source `SMSG_CHECK_CHARACTER_NAME_AVAILABILITY_RESULT` payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckCharacterNameAvailabilityResult {
    sequence_index: u32,
    result: u32,
}

impl CheckCharacterNameAvailabilityResult {
    pub const fn new(sequence_index: u32, result: u32) -> Self {
        Self {
            sequence_index,
            result,
        }
    }

    pub const fn sequence_index(self) -> u32 {
        self.sequence_index
    }

    /// Raw source `ResponseCodes` value; interpretation belongs to the
    /// world-session owner rather than this payload codec.
    pub const fn result(self) -> u32 {
        self.result
    }

    /// Encode only the payload, in the source's little-endian uint32 order.
    pub fn encode_payload(self) -> Vec<u8> {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(self.sequence_index);
        packet.write_uint32(self.result);
        packet.into_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(sequence_index: u32, name: &str, surname: &str, unknown_bits: u8) -> Vec<u8> {
        assert!(name.len() <= MAX_NAME_BYTES);
        assert!(surname.len() <= MAX_NAME_BYTES);

        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(sequence_index);
        packet.write_bits(name.len() as u32, NAME_LENGTH_BITS);
        packet.write_bits(u32::from(unknown_bits & 0b111), UNKNOWN_BITS);
        packet.write_bits(surname.len() as u32, NAME_LENGTH_BITS);
        packet.flush_bits();
        packet.write_string(name);
        packet.write_string(surname);
        packet.into_data()
    }

    #[test]
    fn decodes_sequence_strings_and_unknown_bits_without_validating_flags() {
        let payload = request(0x1122_3344, "Málaga", "Łódź", 0b101);
        let decoded = CheckCharacterNameAvailability::decode(&payload).unwrap();

        assert_eq!(decoded.sequence_index(), 0x1122_3344);
        assert_eq!(decoded.name(), "Málaga");
        assert_eq!(decoded.surname(), "Łódź");
        assert_eq!(decoded.unknown_bits(), 0b101);
    }

    #[test]
    fn matches_independent_source_literal_not_only_writer_roundtrip() {
        // CharacterPackets.cpp:467: 10 04 followed by "gear" and "fd".
        let decoded = CheckCharacterNameAvailability::decode(&[
            42, 0, 0, 0, 0x10, 0x04, b'g', b'e', b'a', b'r', b'f', b'd',
        ])
        .unwrap();
        assert_eq!(decoded.sequence_index(), 42);
        assert_eq!(decoded.name(), "gear");
        assert_eq!(decoded.surname(), "fd");
        assert_eq!(decoded.unknown_bits(), 0);
    }

    #[test]
    fn accepts_the_source_six_bit_maximum_for_both_strings() {
        let payload = request(
            7,
            &"N".repeat(MAX_NAME_BYTES),
            &"S".repeat(MAX_NAME_BYTES),
            0,
        );
        let decoded = CheckCharacterNameAvailability::decode(&payload).unwrap();
        assert_eq!(decoded.name().len(), MAX_NAME_BYTES);
        assert_eq!(decoded.surname().len(), MAX_NAME_BYTES);
    }

    #[test]
    fn rejects_every_truncated_prefix() {
        let payload = request(9, "Character", "Surname", 0);
        for end in 0..payload.len() {
            assert!(
                CheckCharacterNameAvailability::decode(&payload[..end]).is_err(),
                "truncated prefix {end} unexpectedly decoded"
            );
        }
    }

    #[test]
    fn rejects_invalid_utf8_and_trailing_bytes() {
        let mut invalid_utf8 = request(1, "A", "B", 0);
        invalid_utf8[6] = 0xFF;
        assert!(matches!(
            CheckCharacterNameAvailability::decode(&invalid_utf8),
            Err(NameAvailabilityError::Packet(PacketError::StringError(_)))
        ));

        let mut trailing = request(2, "A", "B", 0);
        trailing.push(0xEE);
        assert!(matches!(
            CheckCharacterNameAvailability::decode(&trailing),
            Err(NameAvailabilityError::TrailingBytes { count: 1 })
        ));
    }

    #[test]
    fn response_is_exact_little_endian_sequence_and_raw_result() {
        for result in [0, 27, 98, 104] {
            assert_eq!(
                CheckCharacterNameAvailabilityResult::new(0x1122_3344, result).encode_payload(),
                vec![0x44, 0x33, 0x22, 0x11, result as u8, 0, 0, 0]
            );
        }
    }
}
