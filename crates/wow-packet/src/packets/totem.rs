// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Totem packet definitions.

use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;

use crate::{ClientPacket, PacketError, WorldPacket};

/// C++ `WorldPackets::Totem::TotemDestroyed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotemDestroyed {
    pub slot: u8,
    pub totem_guid: ObjectGuid,
}

impl ClientPacket for TotemDestroyed {
    const OPCODE: ClientOpcodes = ClientOpcodes::TotemDestroyed;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        let slot = pkt.read_uint8()?;
        let totem_guid = pkt.read_packed_guid()?;
        Ok(Self { slot, totem_guid })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totem_destroyed_reads_cpp_slot_then_guid() {
        let totem_guid = ObjectGuid::create_player(1, 42);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(2);
        pkt.write_packed_guid(&totem_guid);
        pkt.reset_read();

        let parsed = TotemDestroyed::read(&mut pkt).unwrap();
        assert_eq!(parsed.slot, 2);
        assert_eq!(parsed.totem_guid, totem_guid);
        assert!(pkt.is_empty());
    }
}
