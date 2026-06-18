// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;

use crate::{ClientPacket, PacketError, WorldPacket};

pub const SPEC_RESET_TALENTS_LIKE_CPP: u8 = 0;
pub const SPEC_RESET_SPECIALIZATION_LIKE_CPP: u8 = 1;
pub const SPEC_RESET_GLYPHS_LIKE_CPP: u8 = 2;
pub const SPEC_RESET_PET_TALENTS_LIKE_CPP: u8 = 3;

/// C++ `WorldPackets::Talent::ConfirmRespecWipe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmRespecWipe {
    pub respec_master: ObjectGuid,
    pub respec_type: u8,
}

/// C++ `WorldPackets::Talent::LearnTalents`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnTalents {
    pub talent_ids: Vec<u16>,
}

impl LearnTalents {
    pub fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let count = packet.read_bits(6)? as usize;
        let mut talent_ids = Vec::with_capacity(count);
        for _ in 0..count {
            talent_ids.push(packet.read_uint16()?);
        }
        Ok(Self { talent_ids })
    }
}

/// C++ `WorldPackets::Talent::LearnTalent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearnTalent {
    pub talent_id: i32,
    pub requested_rank: u16,
}

impl ClientPacket for LearnTalent {
    const OPCODE: ClientOpcodes = ClientOpcodes::LearnTalent;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            talent_id: packet.read_int32()?,
            requested_rank: packet.read_uint16()?,
        })
    }
}

impl ClientPacket for ConfirmRespecWipe {
    const OPCODE: ClientOpcodes = ClientOpcodes::ConfirmRespecWipe;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            respec_master: packet.read_guid()?,
            respec_type: packet.read_uint8()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::guid::HighGuid;

    #[test]
    fn learn_talents_reads_cpp_count_bits_then_uint16_ids() {
        let mut packet = WorldPacket::new_empty();
        packet.write_bits(3, 6);
        packet.flush_bits();
        packet.write_uint16(101);
        packet.write_uint16(202);
        packet.write_uint16(303);
        packet.reset_read();

        let parsed = LearnTalents::read(&mut packet).unwrap();

        assert_eq!(parsed.talent_ids, vec![101, 202, 303]);
        assert_eq!(packet.remaining(), 0);
    }

    #[test]
    fn learn_talent_reads_cpp_int32_then_uint16_rank() {
        let mut packet = WorldPacket::new_empty();
        packet.write_int32(101);
        packet.write_uint16(2);
        packet.reset_read();

        let parsed = LearnTalent::read(&mut packet).unwrap();

        assert_eq!(
            parsed,
            LearnTalent {
                talent_id: 101,
                requested_rank: 2,
            }
        );
        assert_eq!(packet.remaining(), 0);
    }

    #[test]
    fn confirm_respec_wipe_reads_cpp_guid_then_uint8_type() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9, 1);
        let mut packet = WorldPacket::new_empty();
        packet.write_guid(&guid);
        packet.write_uint8(SPEC_RESET_TALENTS_LIKE_CPP);
        packet.reset_read();

        let parsed = ConfirmRespecWipe::read(&mut packet).unwrap();

        assert_eq!(parsed.respec_master, guid);
        assert_eq!(parsed.respec_type, SPEC_RESET_TALENTS_LIKE_CPP);
        assert_eq!(packet.remaining(), 0);
    }
}
