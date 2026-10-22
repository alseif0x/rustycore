// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Death, release and corpse-reclaim server packets (#43).
//!
//! C++ `WorldPackets::Misc::CorpseReclaimDelay`, `DeathReleaseLoc` and
//! `PreRessurect` (`MiscPackets.cpp:252-272`), sent by `Player::KillPlayer`,
//! `Player::BuildPlayerRepop`, `Player::RepopAtGraveyard` and
//! `Player::ResurrectPlayer`.

use super::*;

/// C++ `SMSG_CORPSE_RECLAIM_DELAY` (`MiscPackets.cpp:252-257`): the remaining
/// reclaim delay in milliseconds (`Player::SendCorpseReclaimDelay`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseReclaimDelay {
    pub remaining: u32,
}

impl ServerPacket for CorpseReclaimDelay {
    const OPCODE: ServerOpcodes = ServerOpcodes::CorpseReclaimDelay;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.remaining);
    }
}

/// C++ `SMSG_DEATH_RELEASE_LOC` (`MiscPackets.cpp:259-265`): `int32 MapID`
/// then `TaggedPosition<XYZ> Loc`. `Player::ResurrectPlayer` sends
/// `MapID = -1` with the default (zero) location.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeathReleaseLoc {
    pub map_id: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl DeathReleaseLoc {
    /// The `ResurrectPlayer` form: `MapID = -1`, zero location.
    pub const fn cleared_like_cpp() -> Self {
        Self {
            map_id: -1,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl ServerPacket for DeathReleaseLoc {
    const OPCODE: ServerOpcodes = ServerOpcodes::DeathReleaseLoc;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.map_id);
        pkt.write_float(self.x);
        pkt.write_float(self.y);
        pkt.write_float(self.z);
    }
}

/// C++ `SMSG_PRE_RESSURECT` (`MiscPackets.cpp:267-272`), the first packet of
/// `Player::BuildPlayerRepop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreRessurect {
    pub player_guid: ObjectGuid,
}

impl ServerPacket for PreRessurect {
    const OPCODE: ServerOpcodes = ServerOpcodes::PreRessurect;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.player_guid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpse_reclaim_delay_writes_cpp_remaining_like_cpp() {
        let bytes = CorpseReclaimDelay { remaining: 30_000 }.to_bytes();
        let mut packet = WorldPacket::from_bytes(&bytes);

        assert_eq!(
            packet.read_uint16().expect("opcode"),
            ServerOpcodes::CorpseReclaimDelay as u16
        );
        assert_eq!(packet.read_uint32().expect("remaining"), 30_000);
        assert!(packet.is_empty());
    }

    #[test]
    fn death_release_loc_writes_map_then_xyz_like_cpp() {
        let bytes = DeathReleaseLoc {
            map_id: 571,
            x: 1.5,
            y: -2.0,
            z: 3.25,
        }
        .to_bytes();
        let mut packet = WorldPacket::from_bytes(&bytes);

        assert_eq!(
            packet.read_uint16().expect("opcode"),
            ServerOpcodes::DeathReleaseLoc as u16
        );
        assert_eq!(packet.read_int32().expect("map"), 571);
        assert_eq!(packet.read_float().expect("x"), 1.5);
        assert_eq!(packet.read_float().expect("y"), -2.0);
        assert_eq!(packet.read_float().expect("z"), 3.25);
        assert!(packet.is_empty());
    }

    #[test]
    fn death_release_loc_cleared_matches_resurrect_player_like_cpp() {
        let bytes = DeathReleaseLoc::cleared_like_cpp().to_bytes();
        let mut packet = WorldPacket::from_bytes(&bytes);

        assert_eq!(
            packet.read_uint16().expect("opcode"),
            ServerOpcodes::DeathReleaseLoc as u16
        );
        assert_eq!(packet.read_int32().expect("map"), -1);
        for _ in 0..3 {
            assert_eq!(packet.read_float().expect("coordinate"), 0.0);
        }
        assert!(packet.is_empty());
    }

    #[test]
    fn pre_ressurect_writes_packed_player_guid_like_cpp() {
        let guid = ObjectGuid::create_player(1, 42);
        let bytes = PreRessurect { player_guid: guid }.to_bytes();
        let mut packet = WorldPacket::from_bytes(&bytes);

        assert_eq!(
            packet.read_uint16().expect("opcode"),
            ServerOpcodes::PreRessurect as u16
        );
        assert_eq!(packet.read_packed_guid().expect("guid"), guid);
        assert!(packet.is_empty());
    }
}
