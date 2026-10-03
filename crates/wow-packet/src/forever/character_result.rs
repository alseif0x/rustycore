//! Payload-only Forever character-operation responses.
//!
//! `CharacterPackets.cpp:518-524` at
//! `02245dcd245e7433e524577656177723d3e4992e` writes a result code followed by
//! an ObjectGuid.  The core opcode is `0x4501AC`; the Classic opcode-table
//! translation in `Protocol/ClassicOpcodes.cpp` shifts this group/index to
//! `0x4601AB`.  The latter is source-derived, but remains a transport concern
//! and is not written by this payload codec.

use wow_core::ObjectGuid;

use crate::WorldPacket;

/// Core opcode for C++ `SMSG_CREATE_CHAR` (`Opcodes.h:1417`).
pub const CREATE_CHAR_CORE_OPCODE: u32 = 0x4501AC;

/// Classic translation of `SMSG_CREATE_CHAR` at the pinned source.
pub const CREATE_CHAR_CLASSIC_OPCODE: u32 = 0x4601AB;

/// C++ `WorldPackets::Character::CreateChar`, without its opcode/framing.
pub struct CreateCharResponse {
    pub code: u32,
    pub guid: ObjectGuid,
}

impl CreateCharResponse {
    /// Encode `uint32(Code)` followed by the source packed ObjectGuid.
    pub fn encode_payload(&self) -> Vec<u8> {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(self.code);
        packet.write_packed_guid(&self.guid);
        packet.into_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_char_writes_code_then_packed_guid() {
        let guid = ObjectGuid::create_player(1, 42);
        let payload = CreateCharResponse {
            code: 0x1122_3344,
            guid,
        }
        .encode_payload();

        assert_eq!(&payload[..4], &[0x44, 0x33, 0x22, 0x11]);
        let mut reader = WorldPacket::from_bytes(&payload[4..]);
        assert_eq!(reader.read_packed_guid().unwrap(), guid);
        assert!(reader.is_empty());
    }

    #[test]
    fn create_char_uses_literal_packed_guid_bytes() {
        let guid = ObjectGuid::new(0x0102_0304_0506_0708, 0x1112_1314_1516_1718);
        let payload = CreateCharResponse { code: 25, guid }.encode_payload();

        assert_eq!(
            payload,
            vec![
                25, 0, 0, 0, 0xFF, 0xFF, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11, 0x08,
                0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
            ]
        );
    }

    #[test]
    fn create_char_empty_guid_uses_zero_masks() {
        let payload = CreateCharResponse {
            code: 25,
            guid: ObjectGuid::EMPTY,
        }
        .encode_payload();

        assert_eq!(payload, vec![25, 0, 0, 0, 0, 0]);
    }
}
