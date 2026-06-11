// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pet packet definitions.

use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;

use crate::{ServerPacket, WorldPacket};

pub const REACT_PASSIVE_LIKE_CPP: u8 = 0;
pub const REACT_DEFENSIVE_LIKE_CPP: u8 = 1;
pub const REACT_AGGRESSIVE_LIKE_CPP: u8 = 2;

pub const COMMAND_STAY_LIKE_CPP: u8 = 0;
pub const COMMAND_FOLLOW_LIKE_CPP: u8 = 1;
pub const COMMAND_ATTACK_LIKE_CPP: u8 = 2;

/// Mirrors C++ `WorldPackets::Pet::PetMode`.
pub struct PetMode {
    pub pet_guid: ObjectGuid,
    pub react_state: u8,
    pub command_state: u8,
    pub flag: u8,
}

impl PetMode {
    pub fn passive_follow(pet_guid: ObjectGuid) -> Self {
        Self {
            pet_guid,
            react_state: REACT_PASSIVE_LIKE_CPP,
            command_state: COMMAND_FOLLOW_LIKE_CPP,
            flag: 0,
        }
    }
}

impl ServerPacket for PetMode {
    const OPCODE: ServerOpcodes = ServerOpcodes::PetMode;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.pet_guid);
        pkt.write_uint16(u16::from(self.command_state) | (u16::from(self.flag) << 8));
        pkt.write_uint8(self.react_state);
    }
}

/// C++ `WorldPackets::Pet::PetStableResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PetStableResult {
    pub result: u8,
}

impl ServerPacket for PetStableResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::PetStableResult;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pet_mode_matches_cpp_opcode_and_tail() {
        let guid = ObjectGuid::create_player(1, 42);
        let pkt = PetMode::passive_follow(guid);
        let bytes = pkt.to_bytes();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x2588);
        assert!(bytes.len() > 5);
        assert_eq!(
            &bytes[bytes.len() - 3..bytes.len() - 1],
            &1u16.to_le_bytes()
        );
        assert_eq!(bytes[bytes.len() - 1], REACT_PASSIVE_LIKE_CPP);
    }

    #[test]
    fn pet_stable_result_matches_cpp_shape() {
        let bytes = PetStableResult { result: 2 }.to_bytes();
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::PetStableResult as u16
        );
        assert_eq!(bytes.len(), 2 + 1);

        let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
        assert_eq!(pkt.read_uint8().unwrap(), 2);
    }
}
