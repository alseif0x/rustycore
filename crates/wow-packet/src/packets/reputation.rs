// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Reputation packet definitions.

use wow_constants::{ClientOpcodes, ServerOpcodes};

pub use wow_constants::reputation::FACTION_COUNT_LIKE_CPP;

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};

/// SMSG_INITIALIZE_FACTIONS.
///
/// C++ writes all `(uint16 flags, int32 standing)` pairs first, then 1000
/// packed `FactionHasBonus` bits.
#[derive(Debug, Clone)]
pub struct InitializeFactions {
    pub faction_standings: [i32; FACTION_COUNT_LIKE_CPP],
    pub faction_has_bonus: [bool; FACTION_COUNT_LIKE_CPP],
    pub faction_flags: [u16; FACTION_COUNT_LIKE_CPP],
}

impl Default for InitializeFactions {
    fn default() -> Self {
        Self {
            faction_standings: [0; FACTION_COUNT_LIKE_CPP],
            faction_has_bonus: [false; FACTION_COUNT_LIKE_CPP],
            faction_flags: [0; FACTION_COUNT_LIKE_CPP],
        }
    }
}

impl ServerPacket for InitializeFactions {
    const OPCODE: ServerOpcodes = ServerOpcodes::InitializeFactions;

    fn write(&self, pkt: &mut WorldPacket) {
        for i in 0..FACTION_COUNT_LIKE_CPP {
            pkt.write_uint16(self.faction_flags[i]);
            pkt.write_int32(self.faction_standings[i]);
        }

        for has_bonus in self.faction_has_bonus {
            pkt.write_bit(has_bonus);
        }
        pkt.flush_bits();
    }
}

/// CMSG_REQUEST_FORCED_REACTIONS.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestForcedReactions;

impl ClientPacket for RequestForcedReactions {
    const OPCODE: ClientOpcodes = ClientOpcodes::RequestForcedReactions;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        packet.skip_opcode();
        Ok(Self)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ForcedReaction {
    pub faction: i32,
    pub reaction: i32,
}

/// SMSG_SET_FORCED_REACTIONS.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SetForcedReactions {
    pub reactions: Vec<ForcedReaction>,
}

impl ServerPacket for SetForcedReactions {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetForcedReactions;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.reactions.len() as u32);
        for reaction in &self.reactions {
            pkt.write_int32(reaction.faction);
            pkt.write_int32(reaction.reaction);
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FactionStandingData {
    pub index: i32,
    pub standing: i32,
}

/// SMSG_SET_FACTION_STANDING.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SetFactionStanding {
    pub bonus_from_achievement_system: f32,
    pub faction: Vec<FactionStandingData>,
    pub show_visual: bool,
}

impl ServerPacket for SetFactionStanding {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetFactionStanding;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_float(self.bonus_from_achievement_system);
        pkt.write_uint32(self.faction.len() as u32);
        for faction_standing in &self.faction {
            pkt.write_int32(faction_standing.index);
            pkt.write_int32(faction_standing.standing);
        }

        pkt.write_bit(self.show_visual);
        pkt.flush_bits();
    }
}

/// SMSG_SET_FACTION_VISIBLE.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SetFactionVisible {
    pub faction_index: u32,
}

impl ServerPacket for SetFactionVisible {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetFactionVisible;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.faction_index);
    }
}

/// SMSG_SET_FACTION_NOT_VISIBLE.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SetFactionNotVisible {
    pub faction_index: u32,
}

impl ServerPacket for SetFactionNotVisible {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetFactionNotVisible;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.faction_index);
    }
}

/// SMSG_SET_FACTION_AT_WAR.
///
/// Trinity registers the server opcode but does not define a writer in
/// `CharacterPackets`/`ReputationPackets`; C++ reputation state changes are
/// normally delivered through `SMSG_SET_FACTION_STANDING`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SetFactionAtWar {
    pub faction_index: u32,
}

impl ServerPacket for SetFactionAtWar {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetFactionAtWar;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.faction_index);
    }
}

/// CMSG_SET_FACTION_AT_WAR.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SetFactionAtWarRequest {
    pub faction_index: u16,
}

impl ClientPacket for SetFactionAtWarRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetFactionAtWar;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        packet.skip_opcode();
        Ok(Self {
            faction_index: packet.read_uint16()?,
        })
    }
}

/// CMSG_SET_FACTION_NOT_AT_WAR.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SetFactionNotAtWarRequest {
    pub faction_index: u16,
}

impl ClientPacket for SetFactionNotAtWarRequest {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetFactionNotAtWar;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        packet.skip_opcode();
        Ok(Self {
            faction_index: packet.read_uint16()?,
        })
    }
}

/// CMSG_SET_FACTION_INACTIVE.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SetFactionInactive {
    pub index: u32,
    pub state: bool,
}

impl ClientPacket for SetFactionInactive {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetFactionInactive;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        packet.skip_opcode();
        Ok(Self {
            index: packet.read_uint32()?,
            state: packet.read_bit()?,
        })
    }
}

/// CMSG_SET_WATCHED_FACTION.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SetWatchedFaction {
    pub faction_index: u32,
}

impl ClientPacket for SetWatchedFaction {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetWatchedFaction;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        packet.skip_opcode();
        Ok(Self {
            faction_index: packet.read_uint32()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServerPacket;

    fn opcode(bytes: &[u8]) -> u16 {
        u16::from_le_bytes([bytes[0], bytes[1]])
    }

    #[test]
    fn initialize_factions_matches_cpp_layout() {
        let mut packet = InitializeFactions::default();
        packet.faction_flags[2] = 0x1234;
        packet.faction_standings[2] = -123;
        packet.faction_has_bonus[2] = true;

        let bytes = packet.to_bytes();

        assert_eq!(opcode(&bytes), 0x2724);
        assert_eq!(bytes.len(), 2 + FACTION_COUNT_LIKE_CPP * 6 + 125);
        let entry_offset = 2 + 2 * 6;
        assert_eq!(
            u16::from_le_bytes([bytes[entry_offset], bytes[entry_offset + 1]]),
            0x1234
        );
        assert_eq!(
            i32::from_le_bytes([
                bytes[entry_offset + 2],
                bytes[entry_offset + 3],
                bytes[entry_offset + 4],
                bytes[entry_offset + 5],
            ]),
            -123
        );
        assert_eq!(bytes[2 + FACTION_COUNT_LIKE_CPP * 6], 0b0010_0000);
    }

    #[test]
    fn set_forced_reactions_matches_cpp_layout() {
        let bytes = SetForcedReactions {
            reactions: vec![
                ForcedReaction {
                    faction: 72,
                    reaction: 5,
                },
                ForcedReaction {
                    faction: 930,
                    reaction: -1,
                },
            ],
        }
        .to_bytes();

        assert_eq!(opcode(&bytes), 0x271d);
        assert_eq!(&bytes[2..6], &2u32.to_le_bytes());
        assert_eq!(&bytes[6..10], &72i32.to_le_bytes());
        assert_eq!(&bytes[10..14], &5i32.to_le_bytes());
        assert_eq!(&bytes[14..18], &930i32.to_le_bytes());
        assert_eq!(&bytes[18..22], &(-1i32).to_le_bytes());
    }

    #[test]
    fn set_faction_standing_matches_cpp_layout() {
        let bytes = SetFactionStanding {
            bonus_from_achievement_system: 1.5,
            faction: vec![
                FactionStandingData {
                    index: 7,
                    standing: 3000,
                },
                FactionStandingData {
                    index: 8,
                    standing: -42000,
                },
            ],
            show_visual: true,
        }
        .to_bytes();

        assert_eq!(opcode(&bytes), 0x272c);
        assert_eq!(&bytes[2..6], &1.5f32.to_bits().to_le_bytes());
        assert_eq!(&bytes[6..10], &2u32.to_le_bytes());
        assert_eq!(&bytes[10..14], &7i32.to_le_bytes());
        assert_eq!(&bytes[14..18], &3000i32.to_le_bytes());
        assert_eq!(&bytes[18..22], &8i32.to_le_bytes());
        assert_eq!(&bytes[22..26], &(-42000i32).to_le_bytes());
        assert_eq!(bytes[26], 0b1000_0000);
    }

    #[test]
    fn faction_visible_packets_match_cpp_payload() {
        let visible = SetFactionVisible { faction_index: 42 }.to_bytes();
        let not_visible = SetFactionNotVisible { faction_index: 42 }.to_bytes();

        assert_eq!(opcode(&visible), 0x272a);
        assert_eq!(&visible[2..6], &42u32.to_le_bytes());
        assert_eq!(opcode(&not_visible), 0x272b);
        assert_eq!(&not_visible[2..6], &42u32.to_le_bytes());
    }

    #[test]
    fn faction_client_packets_match_cpp_readers() {
        let mut at_war = WorldPacket::new_empty();
        at_war.write_uint16(ClientOpcodes::SetFactionAtWar as u16);
        at_war.write_uint16(7);
        assert_eq!(
            SetFactionAtWarRequest::read(&mut at_war).unwrap(),
            SetFactionAtWarRequest { faction_index: 7 }
        );

        let mut not_at_war = WorldPacket::new_empty();
        not_at_war.write_uint16(ClientOpcodes::SetFactionNotAtWar as u16);
        not_at_war.write_uint16(8);
        assert_eq!(
            SetFactionNotAtWarRequest::read(&mut not_at_war).unwrap(),
            SetFactionNotAtWarRequest { faction_index: 8 }
        );

        let mut inactive = WorldPacket::new_empty();
        inactive.write_uint16(ClientOpcodes::SetFactionInactive as u16);
        inactive.write_uint32(9);
        inactive.write_bit(true);
        inactive.flush_bits();
        assert_eq!(
            SetFactionInactive::read(&mut inactive).unwrap(),
            SetFactionInactive {
                index: 9,
                state: true
            }
        );

        let mut watched = WorldPacket::new_empty();
        watched.write_uint16(ClientOpcodes::SetWatchedFaction as u16);
        watched.write_uint32(10);
        assert_eq!(
            SetWatchedFaction::read(&mut watched).unwrap(),
            SetWatchedFaction { faction_index: 10 }
        );
    }
}
