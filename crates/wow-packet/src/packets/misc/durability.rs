// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durability packets.
//!
//! C++ `WorldPackets::Misc::DurabilityDamageDeath` (`MiscPackets.h:595-601`).

use super::*;

/// C++ `SMSG_DURABILITY_DAMAGE_DEATH` (`MiscPackets.cpp:481-486`), sent by
/// `Player::SendDurabilityLoss` after a lethal fall/environmental death.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurabilityDamageDeath {
    /// C++ `Percent`, already the configured percentage (not a fraction).
    pub percent: i32,
}

impl ServerPacket for DurabilityDamageDeath {
    const OPCODE: ServerOpcodes = ServerOpcodes::DurabilityDamageDeath;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.percent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durability_damage_death_writes_cpp_percent_like_cpp() {
        let bytes = DurabilityDamageDeath { percent: 10 }.to_bytes();
        let mut packet = WorldPacket::from_bytes(&bytes);

        assert_eq!(
            packet.read_uint16().expect("opcode"),
            ServerOpcodes::DurabilityDamageDeath as u16
        );
        assert_eq!(packet.read_int32().expect("percent"), 10);
        assert!(packet.is_empty());
    }
}
