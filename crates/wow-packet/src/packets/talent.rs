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
