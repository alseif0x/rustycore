//! Resurrection offer packets from `WorldPackets::Spells`.

use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;

use crate::{ServerPacket, WorldPacket};

/// C++ `WorldPackets::Spells::ResurrectRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResurrectRequest {
    pub offerer_guid: ObjectGuid,
    pub offerer_virtual_realm_address: u32,
    pub pet_number: u32,
    pub spell_id: i32,
    pub use_timer: bool,
    pub sickness: bool,
    pub name: String,
}

impl ServerPacket for ResurrectRequest {
    const OPCODE: ServerOpcodes = ServerOpcodes::ResurrectRequest;

    fn write(&self, packet: &mut WorldPacket) {
        packet.write_packed_guid(&self.offerer_guid);
        packet.write_uint32(self.offerer_virtual_realm_address);
        packet.write_uint32(self.pet_number);
        packet.write_int32(self.spell_id);
        packet.write_bits(self.name.len() as u32, 11);
        packet.write_bit(self.use_timer);
        packet.write_bit(self.sickness);
        packet.flush_bits();
        packet.write_string(&self.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resurrect_request_writes_empty_player_name_after_packed_offer_fields() {
        let bytes = ResurrectRequest {
            offerer_guid: ObjectGuid::new(1, 2),
            offerer_virtual_realm_address: 0x1122_3344,
            pet_number: 0x5566_7788,
            spell_id: -2,
            use_timer: false,
            sickness: false,
            name: String::new(),
        }
        .to_bytes();

        assert_eq!(
            bytes,
            [
                0x7e, 0x25, // opcode
                0x01, 0x01, 0x02, 0x01, // packed low/high masks and GUID bytes
                0x44, 0x33, 0x22, 0x11, // virtual realm address
                0x88, 0x77, 0x66, 0x55, // pet number
                0xfe, 0xff, 0xff, 0xff, // spell ID
                0x00, 0x00, // empty name length and false flags
            ]
        );
    }

    #[test]
    fn resurrect_request_writes_utf8_creature_name_with_byte_length() {
        let bytes = ResurrectRequest {
            offerer_guid: ObjectGuid::new(1, 2),
            offerer_virtual_realm_address: 0x0102_0304,
            pet_number: 0xa1b2_c3d4,
            spell_id: 0x1122_3344,
            use_timer: true,
            sickness: true,
            name: "Ánima".to_owned(),
        }
        .to_bytes();

        assert_eq!(
            bytes,
            [
                0x7e, 0x25, // opcode
                0x01, 0x01, 0x02, 0x01, // packed low/high masks and GUID bytes
                0x04, 0x03, 0x02, 0x01, // virtual realm address
                0xd4, 0xc3, 0xb2, 0xa1, // pet number
                0x44, 0x33, 0x22, 0x11, // spell ID
                0x00, 0xd8, // six UTF-8 bytes, timer and sickness bits
                0xc3, 0x81, 0x6e, 0x69, 0x6d, 0x61, // raw UTF-8 name
            ]
        );
    }

    #[test]
    fn resurrect_request_packs_every_timer_and_sickness_combination() {
        let cases: [(bool, bool, &[u8]); 4] = [
            (false, false, &[0x00, 0x00]),
            (true, false, &[0x00, 0x10]),
            (false, true, &[0x00, 0x08]),
            (true, true, &[0x00, 0x18]),
        ];

        for (use_timer, sickness, expected_bits) in cases {
            let bytes = ResurrectRequest {
                offerer_guid: ObjectGuid::EMPTY,
                offerer_virtual_realm_address: 0,
                pet_number: 0,
                spell_id: 0,
                use_timer,
                sickness,
                name: String::new(),
            }
            .to_bytes();

            assert_eq!(&bytes[16..], expected_bits);
        }
    }
}
